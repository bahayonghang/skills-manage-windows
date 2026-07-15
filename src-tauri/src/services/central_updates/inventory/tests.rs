//! Full coverage tests for Phase P3 inventory commands.
//!
//! 风格参考 `central_updates/tests.rs`：用内存 SQLite + TempDir + 预填
//! `central_update_snapshots` 缓存，避免触网；只测 `_impl` 内核版本，
//! 不依赖 `State<AppState>`。
//!
//! 分组：
//! - A: refresh / get / clear 行为
//! - B: apply 顺序与 partial success
//! - C: force update / force mirror rescue mode
//! - D: scan_platform_duplicate_skills

use super::*;
use crate::db;
use crate::db::{AgentSkillObservation, Skill, SkillInstallation, SkillUpdateState};
use crate::services::central_skills::BatchDeleteCentralSkillRequest;
use crate::services::central_updates;
use crate::services::central_updates::repo_cache_key;
use crate::services::central_updates::CentralFs;
use crate::services::github_import::GitHubRepoRef;
use crate::services::github_import::GitHubRepoSnapshot;
use crate::targets::ActiveTarget;
use crate::CentralUpdateSnapshotCache;
use chrono::Utc;
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use tempfile::TempDir;

/*
 * ========================================================================
 * 通用 helpers
 * ========================================================================
 */

use crate::test_support::mem_pool as setup_test_db;

/// 用 TempDir 当 `~`，central 目录就落在 `{home}/.skillsmanage/skills/`，
/// 测试可以安全清理。delete_central_skill_impl 会去看这个目录是否合法。
async fn setup_test_db_with_home(home: &Path) -> SqlitePool {
    crate::test_support::mem_pool_with_home(&home.to_string_lossy()).await
}

fn make_central_skill(id: &str, dir: &Path) -> Skill {
    Skill {
        description: Some(format!("Desc for {id}")),
        source: Some("github:owner/repo".to_string()),
        ..crate::test_support::central_skill_row(id, dir)
    }
}

fn test_repo() -> GitHubRepoRef {
    GitHubRepoRef {
        owner: "owner".to_string(),
        repo: "repo".to_string(),
        branch: "main".to_string(),
        normalized_url: "https://github.com/owner/repo".to_string(),
    }
}

fn alt_repo() -> GitHubRepoRef {
    GitHubRepoRef {
        owner: "alt-owner".to_string(),
        repo: "alt-repo".to_string(),
        branch: "main".to_string(),
        normalized_url: "https://github.com/alt-owner/alt-repo".to_string(),
    }
}

async fn assign_test_repo(pool: &SqlitePool, skill_id: &str, source_path: &str) {
    db::assign_github_repository_to_skill(
        pool,
        "owner",
        "repo",
        "main",
        "https://github.com/owner/repo",
        skill_id,
        source_path,
    )
    .await
    .unwrap();
}

async fn assign_alt_repo(pool: &SqlitePool, skill_id: &str, source_path: &str) {
    db::assign_github_repository_to_skill(
        pool,
        "alt-owner",
        "alt-repo",
        "main",
        "https://github.com/alt-owner/alt-repo",
        skill_id,
        source_path,
    )
    .await
    .unwrap();
}

/// 准备一个 prepared snapshot 缓存：把 repo + snapshot 直接 insert，避免触网。
fn snapshots_cache_with(
    items: Vec<(GitHubRepoRef, GitHubRepoSnapshot)>,
) -> CentralUpdateSnapshotCache {
    let cache = CentralUpdateSnapshotCache::default();
    for (repo, snapshot) in items {
        cache.insert(repo_cache_key(&repo), snapshot);
    }
    cache
}

fn http_client() -> reqwest::Client {
    reqwest::Client::builder().no_proxy().build().unwrap()
}

fn skill_snapshot(items: Vec<(&str, &[u8])>) -> GitHubRepoSnapshot {
    GitHubRepoSnapshot {
        files: items
            .into_iter()
            .map(|(path, content)| (path.to_string(), content.to_vec()))
            .collect(),
    }
}

fn copy_installation(skill_id: &str, agent_id: &str, dir: &Path) -> SkillInstallation {
    SkillInstallation {
        skill_id: skill_id.to_string(),
        agent_id: agent_id.to_string(),
        installed_path: dir.to_string_lossy().into_owned(),
        link_type: "copy".to_string(),
        symlink_target: None,
        created_at: Utc::now().to_rfc3339(),
    }
}

fn scope_all() -> SkillRefreshScope {
    SkillRefreshScope {
        kind: SkillRefreshScopeKind::All,
        mode: None,
        cache_policy: Some(SkillRefreshCachePolicy::UseFresh),
        skill_ids: None,
        repository_ids: None,
        agent_ids: None,
    }
}

fn scope_skills(ids: Vec<&str>) -> SkillRefreshScope {
    SkillRefreshScope {
        kind: SkillRefreshScopeKind::Skills,
        mode: None,
        cache_policy: Some(SkillRefreshCachePolicy::UseFresh),
        skill_ids: Some(ids.into_iter().map(String::from).collect()),
        repository_ids: None,
        agent_ids: None,
    }
}

fn scope_repos(ids: Vec<&str>) -> SkillRefreshScope {
    SkillRefreshScope {
        kind: SkillRefreshScopeKind::Repositories,
        mode: None,
        cache_policy: Some(SkillRefreshCachePolicy::UseFresh),
        skill_ids: None,
        repository_ids: Some(ids.into_iter().map(String::from).collect()),
        agent_ids: None,
    }
}

fn scope_platform(ids: Vec<&str>) -> SkillRefreshScope {
    SkillRefreshScope {
        kind: SkillRefreshScopeKind::Platform,
        mode: None,
        cache_policy: Some(SkillRefreshCachePolicy::UseFresh),
        skill_ids: None,
        repository_ids: None,
        agent_ids: Some(ids.into_iter().map(String::from).collect()),
    }
}

fn with_mode(mut scope: SkillRefreshScope, mode: SkillRefreshMode) -> SkillRefreshScope {
    scope.mode = Some(mode);
    scope
}

fn make_observation(
    agent_id: &str,
    skill_id: &str,
    dir_path: &str,
    source_kind: &str,
    is_read_only: bool,
) -> AgentSkillObservation {
    AgentSkillObservation {
        row_id: format!("{agent_id}::{skill_id}::{dir_path}"),
        agent_id: agent_id.to_string(),
        skill_id: skill_id.to_string(),
        name: skill_id.to_string(),
        description: None,
        file_path: format!("{dir_path}/SKILL.md"),
        dir_path: dir_path.to_string(),
        source_kind: source_kind.to_string(),
        source_root: dir_path.to_string(),
        link_type: "writable".to_string(),
        symlink_target: None,
        is_read_only,
        scanned_at: Utc::now().to_rfc3339(),
        fs_created_at: None,
        fs_updated_at: None,
    }
}

/*
 * ========================================================================
 * 既有 smoke tests（保留）
 * ========================================================================
 */

#[tokio::test]
async fn pending_additions_repo_upsert_then_clear() {
    let pool = setup_test_db().await;

    let addition = db::SkillRepositoryPendingAddition {
        repository_id: "github:owner-repo-main".to_string(),
        source_path: "skills/example".to_string(),
        skill_id: "example".to_string(),
        skill_name: "Example".to_string(),
        conflict_existing_skill_id: None,
        discovered_at: chrono::Utc::now().to_rfc3339(),
    };
    db::upsert_pending_addition(&pool, &addition).await.unwrap();

    let listed = db::list_pending_additions(&pool).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].source_path, "skills/example");

    // Re-upsert with renamed skill — same primary key, no duplicate row
    let mut renamed = addition.clone();
    renamed.skill_name = "Renamed".to_string();
    db::upsert_pending_addition(&pool, &renamed).await.unwrap();
    let listed = db::list_pending_additions(&pool).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].skill_name, "Renamed");

    db::delete_pending_addition(&pool, &addition.repository_id, &addition.source_path)
        .await
        .unwrap();
    let listed = db::list_pending_additions(&pool).await.unwrap();
    assert!(listed.is_empty());
}

#[tokio::test]
async fn pending_additions_scope_repo_filter() {
    let pool = setup_test_db().await;
    let now = chrono::Utc::now().to_rfc3339();

    for repo_id in ["github:a-a-main", "github:b-b-main"] {
        db::upsert_pending_addition(
            &pool,
            &db::SkillRepositoryPendingAddition {
                repository_id: repo_id.to_string(),
                source_path: "skills/x".to_string(),
                skill_id: "x".to_string(),
                skill_name: "x".to_string(),
                conflict_existing_skill_id: None,
                discovered_at: now.clone(),
            },
        )
        .await
        .unwrap();
    }

    let filtered = db::list_pending_additions_for_repos(&pool, &["github:a-a-main".to_string()])
        .await
        .unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].repository_id, "github:a-a-main");

    db::clear_pending_additions_for_repos(&pool, &["github:a-a-main".to_string()])
        .await
        .unwrap();
    let all = db::list_pending_additions(&pool).await.unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].repository_id, "github:b-b-main");
}

/*
 * ========================================================================
 * A. refresh / get / clear 行为
 * ========================================================================
 */

#[tokio::test]
async fn refresh_returns_empty_inventory_on_empty_db() {
    let pool = setup_test_db().await;
    let cache = CentralUpdateSnapshotCache::default();
    let client = http_client();

    let inventory = refresh_skill_update_inventory_impl(
        &pool,
        &CentralFs::Local,
        None,
        &client,
        &cache,
        scope_all(),
    )
    .await
    .unwrap();

    assert!(inventory.updatable.is_empty());
    assert!(inventory.remote_added.is_empty());
    assert!(inventory.remote_missing.is_empty());
    assert!(inventory.platform_duplicates.is_empty());
    assert!(inventory.failed_repositories.is_empty());
    // generated_at is a legal RFC3339 timestamp.
    chrono::DateTime::parse_from_rfc3339(&inventory.generated_at).expect("rfc3339");
}

#[tokio::test]
async fn refresh_writes_pending_additions_for_remote_added() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let existing_dir = temp.path().join("existing");
    std::fs::create_dir_all(&existing_dir).unwrap();
    std::fs::write(existing_dir.join("SKILL.md"), b"---\nname: Existing\n---").unwrap();
    let skill = make_central_skill("existing", &existing_dir);
    db::upsert_skill(&pool, &skill).await.unwrap();
    assign_test_repo(&pool, "existing", "skills/existing").await;
    let assignment = db::get_skill_repository_assignment(&pool, "existing")
        .await
        .unwrap();
    let repository_id = assignment.repository.id.clone();

    let snapshot = GitHubRepoSnapshot {
        files: HashMap::from([
            (
                "skills/existing/SKILL.md".to_string(),
                b"---\nname: Existing\n---".to_vec(),
            ),
            (
                "skills/new-skill/SKILL.md".to_string(),
                b"---\nname: New Skill\n---".to_vec(),
            ),
        ]),
    };
    let cache = snapshots_cache_with(vec![(test_repo(), snapshot)]);
    let client = http_client();

    let inventory = refresh_skill_update_inventory_impl(
        &pool,
        &CentralFs::Local,
        None,
        &client,
        &cache,
        scope_repos(vec![&repository_id]),
    )
    .await
    .unwrap();

    assert_eq!(inventory.remote_added.len(), 1);
    assert_eq!(inventory.remote_added[0].skill_id, "new-skill");
    let listed = db::list_pending_additions(&pool).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].repository_id, repository_id);
    assert_eq!(listed[0].source_path, "skills/new-skill");
}

#[tokio::test]
async fn refresh_filters_generic_skill_remote_additions_without_persisting_pending_rows() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let existing_dir = temp.path().join("existing");
    std::fs::create_dir_all(&existing_dir).unwrap();
    std::fs::write(existing_dir.join("SKILL.md"), b"---\nname: Existing\n---").unwrap();
    db::upsert_skill(&pool, &make_central_skill("existing", &existing_dir))
        .await
        .unwrap();
    assign_test_repo(&pool, "existing", "skills/existing").await;
    let assignment = db::get_skill_repository_assignment(&pool, "existing")
        .await
        .unwrap();
    let repository_id = assignment.repository.id.clone();

    let snapshot = GitHubRepoSnapshot {
        files: HashMap::from([
            (
                "skills/existing/SKILL.md".to_string(),
                b"---\nname: Existing\n---".to_vec(),
            ),
            (
                "agent_reach/skill/SKILL.md".to_string(),
                b"---\nname: Agent Reach\n---".to_vec(),
            ),
            (
                "packages/example/skill/SKILL.md".to_string(),
                b"---\nname: Fallback Skill\n---".to_vec(),
            ),
        ]),
    };
    let cache = snapshots_cache_with(vec![(test_repo(), snapshot)]);
    let client = http_client();

    let inventory = refresh_skill_update_inventory_impl(
        &pool,
        &CentralFs::Local,
        None,
        &client,
        &cache,
        scope_repos(vec![&repository_id]),
    )
    .await
    .unwrap();

    assert!(inventory.remote_added.is_empty());
    assert!(inventory.failed_repositories.is_empty());
    assert!(db::list_pending_additions(&pool).await.unwrap().is_empty());
}

#[tokio::test]
async fn refresh_keeps_existing_pending_additions_idempotent() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let existing_dir = temp.path().join("existing");
    std::fs::create_dir_all(&existing_dir).unwrap();
    std::fs::write(existing_dir.join("SKILL.md"), b"---\nname: Existing\n---").unwrap();
    db::upsert_skill(&pool, &make_central_skill("existing", &existing_dir))
        .await
        .unwrap();
    assign_test_repo(&pool, "existing", "skills/existing").await;
    let assignment = db::get_skill_repository_assignment(&pool, "existing")
        .await
        .unwrap();
    let repository_id = assignment.repository.id.clone();

    let snapshot = GitHubRepoSnapshot {
        files: HashMap::from([
            (
                "skills/existing/SKILL.md".to_string(),
                b"---\nname: Existing\n---".to_vec(),
            ),
            (
                "skills/new-skill/SKILL.md".to_string(),
                b"---\nname: New Skill\n---".to_vec(),
            ),
        ]),
    };
    let cache = snapshots_cache_with(vec![(test_repo(), snapshot)]);
    let client = http_client();

    for _ in 0..2 {
        refresh_skill_update_inventory_impl(
            &pool,
            &CentralFs::Local,
            None,
            &client,
            &cache,
            scope_repos(vec![&repository_id]),
        )
        .await
        .unwrap();
    }

    let listed = db::list_pending_additions(&pool).await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].source_path, "skills/new-skill");
}

#[tokio::test]
async fn refresh_updates_repository_last_synced_at() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let existing_dir = temp.path().join("existing");
    std::fs::create_dir_all(&existing_dir).unwrap();
    std::fs::write(existing_dir.join("SKILL.md"), b"---\nname: Existing\n---").unwrap();
    db::upsert_skill(&pool, &make_central_skill("existing", &existing_dir))
        .await
        .unwrap();
    assign_test_repo(&pool, "existing", "skills/existing").await;
    let assignment = db::get_skill_repository_assignment(&pool, "existing")
        .await
        .unwrap();
    let repository_id = assignment.repository.id.clone();
    assert!(assignment.repository.last_synced_at.is_none());

    let snapshot = GitHubRepoSnapshot {
        files: HashMap::from([(
            "skills/existing/SKILL.md".to_string(),
            b"---\nname: Existing\n---".to_vec(),
        )]),
    };
    let cache = snapshots_cache_with(vec![(test_repo(), snapshot)]);
    let client = http_client();

    refresh_skill_update_inventory_impl(
        &pool,
        &CentralFs::Local,
        None,
        &client,
        &cache,
        scope_repos(vec![&repository_id]),
    )
    .await
    .unwrap();

    let refreshed = db::get_skill_repository_by_id(&pool, &repository_id)
        .await
        .unwrap()
        .unwrap();
    let ts = refreshed.last_synced_at.expect("last_synced_at written");
    chrono::DateTime::parse_from_rfc3339(&ts).expect("rfc3339");
}

#[tokio::test]
async fn refresh_scope_skills_does_not_scan_repo_additions() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let existing_dir = temp.path().join("existing");
    std::fs::create_dir_all(&existing_dir).unwrap();
    std::fs::write(existing_dir.join("SKILL.md"), b"---\nname: Existing\n---").unwrap();
    db::upsert_skill(&pool, &make_central_skill("existing", &existing_dir))
        .await
        .unwrap();
    assign_test_repo(&pool, "existing", "skills/existing").await;

    // snapshot 中包含一个 local 没有的 candidate；scope=Skills 时不应被发现。
    let snapshot = GitHubRepoSnapshot {
        files: HashMap::from([
            (
                "skills/existing/SKILL.md".to_string(),
                b"---\nname: Existing\n---".to_vec(),
            ),
            (
                "skills/new-skill/SKILL.md".to_string(),
                b"---\nname: New Skill\n---".to_vec(),
            ),
        ]),
    };
    let cache = snapshots_cache_with(vec![(test_repo(), snapshot)]);
    let client = http_client();

    let inventory = refresh_skill_update_inventory_impl(
        &pool,
        &CentralFs::Local,
        None,
        &client,
        &cache,
        scope_skills(vec!["existing"]),
    )
    .await
    .unwrap();

    assert!(inventory.remote_added.is_empty());
    let listed = db::list_pending_additions(&pool).await.unwrap();
    assert!(listed.is_empty());
}

#[tokio::test]
async fn refresh_regular_mode_returns_only_content_update_buckets() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let update_dir = temp.path().join("with-update");
    let missing_dir = temp.path().join("missing-local");
    std::fs::create_dir_all(&update_dir).unwrap();
    std::fs::create_dir_all(&missing_dir).unwrap();
    std::fs::write(
        update_dir.join("SKILL.md"),
        b"---\nname: With Update\n---\n\nold",
    )
    .unwrap();
    std::fs::write(
        missing_dir.join("SKILL.md"),
        b"---\nname: Missing Local\n---",
    )
    .unwrap();
    db::upsert_skill(&pool, &make_central_skill("with-update", &update_dir))
        .await
        .unwrap();
    db::upsert_skill(&pool, &make_central_skill("missing-local", &missing_dir))
        .await
        .unwrap();
    assign_test_repo(&pool, "with-update", "skills/with-update").await;
    assign_test_repo(&pool, "missing-local", "skills/missing-local").await;
    let assignment = db::get_skill_repository_assignment(&pool, "with-update")
        .await
        .unwrap();
    let repository_id = assignment.repository.id.clone();

    let snapshot = GitHubRepoSnapshot {
        files: HashMap::from([
            (
                "skills/with-update/SKILL.md".to_string(),
                b"---\nname: With Update\n---\n\nnew".to_vec(),
            ),
            (
                "skills/new-skill/SKILL.md".to_string(),
                b"---\nname: New Skill\n---".to_vec(),
            ),
        ]),
    };
    let cache = snapshots_cache_with(vec![(test_repo(), snapshot)]);
    let client = http_client();

    let inventory = refresh_skill_update_inventory_impl(
        &pool,
        &CentralFs::Local,
        None,
        &client,
        &cache,
        with_mode(scope_repos(vec![&repository_id]), SkillRefreshMode::Regular),
    )
    .await
    .unwrap();

    assert_eq!(inventory.updatable.len(), 1);
    assert_eq!(inventory.updatable[0].state.skill_id, "with-update");
    assert!(inventory.remote_added.is_empty());
    assert!(inventory.remote_missing.is_empty());
    assert!(inventory.platform_duplicates.is_empty());
    assert!(inventory.deleted_platform_copies.is_empty());
    assert_eq!(inventory.failed_repositories.len(), 1);
    assert!(inventory.failed_repositories[0]
        .error
        .contains("incremental and removal mode"));
    assert!(db::list_pending_additions(&pool).await.unwrap().is_empty());

    let refreshed_repo = db::get_skill_repository_by_id(&pool, &repository_id)
        .await
        .unwrap()
        .unwrap();
    assert!(refreshed_repo.last_synced_at.is_none());

    let states = db::get_skill_update_states(&pool).await.unwrap();
    assert!(
        states.iter().all(|state| state.skill_id != "missing-local"),
        "regular refresh must not write transient error states to baseline"
    );

    let reloaded = get_skill_update_inventory_impl_scoped(
        &pool,
        Some(with_mode(
            scope_repos(vec![&repository_id]),
            SkillRefreshMode::Regular,
        )),
    )
    .await
    .unwrap();
    assert!(reloaded.remote_missing.is_empty());
}

#[tokio::test]
async fn refresh_scope_repositories_ignores_unrelated_repos() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let skill_a_dir = temp.path().join("skill-a");
    let skill_b_dir = temp.path().join("skill-b");
    std::fs::create_dir_all(&skill_a_dir).unwrap();
    std::fs::create_dir_all(&skill_b_dir).unwrap();
    std::fs::write(skill_a_dir.join("SKILL.md"), b"---\nname: A\n---").unwrap();
    std::fs::write(skill_b_dir.join("SKILL.md"), b"---\nname: B\n---").unwrap();
    db::upsert_skill(&pool, &make_central_skill("skill-a", &skill_a_dir))
        .await
        .unwrap();
    db::upsert_skill(&pool, &make_central_skill("skill-b", &skill_b_dir))
        .await
        .unwrap();
    assign_test_repo(&pool, "skill-a", "skills/skill-a").await;
    assign_alt_repo(&pool, "skill-b", "skills/skill-b").await;

    let assignment_a = db::get_skill_repository_assignment(&pool, "skill-a")
        .await
        .unwrap();
    let assignment_b = db::get_skill_repository_assignment(&pool, "skill-b")
        .await
        .unwrap();
    let repo_a = assignment_a.repository.id.clone();
    let repo_b = assignment_b.repository.id.clone();

    let snapshot_a = GitHubRepoSnapshot {
        files: HashMap::from([
            (
                "skills/skill-a/SKILL.md".to_string(),
                b"---\nname: A\n---".to_vec(),
            ),
            (
                "skills/new-in-a/SKILL.md".to_string(),
                b"---\nname: NewInA\n---".to_vec(),
            ),
        ]),
    };
    let snapshot_b = GitHubRepoSnapshot {
        files: HashMap::from([
            (
                "skills/skill-b/SKILL.md".to_string(),
                b"---\nname: B\n---".to_vec(),
            ),
            (
                "skills/new-in-b/SKILL.md".to_string(),
                b"---\nname: NewInB\n---".to_vec(),
            ),
        ]),
    };
    let cache = snapshots_cache_with(vec![(test_repo(), snapshot_a), (alt_repo(), snapshot_b)]);
    let client = http_client();

    let inventory = refresh_skill_update_inventory_impl(
        &pool,
        &CentralFs::Local,
        None,
        &client,
        &cache,
        scope_repos(vec![&repo_a]),
    )
    .await
    .unwrap();

    assert_eq!(inventory.remote_added.len(), 1);
    assert_eq!(inventory.remote_added[0].repository_id, repo_a);
    assert_eq!(inventory.remote_added[0].skill_id, "new-in-a");
    // repo_b 的新 candidate 没被扫
    assert!(inventory
        .remote_added
        .iter()
        .all(|item| item.repository_id != repo_b));
}

#[tokio::test]
async fn refresh_scope_platform_only_checks_observed_current_agent_skills() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let codex_dir = temp.path().join("codex-skill");
    let claude_dir = temp.path().join("claude-skill");
    let codex_platform_dir = temp.path().join("codex-platform");
    let claude_platform_dir = temp.path().join("claude-platform");
    let codex_writable_dir = codex_platform_dir.join("codex-skill");
    let codex_plugin_dir = temp.path().join("codex-plugin").join("codex-skill");
    let claude_writable_dir = claude_platform_dir.join("claude-skill");
    let claude_plugin_dir = temp.path().join("claude-plugin").join("claude-skill");

    for dir in [
        &codex_dir,
        &claude_dir,
        &codex_writable_dir,
        &codex_plugin_dir,
        &claude_writable_dir,
        &claude_plugin_dir,
    ] {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("SKILL.md"), b"---\nname: Skill\n---\n\nold").unwrap();
    }
    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'codex'")
        .bind(codex_platform_dir.to_string_lossy().as_ref())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'claude-code'")
        .bind(claude_platform_dir.to_string_lossy().as_ref())
        .execute(&pool)
        .await
        .unwrap();

    db::upsert_skill(&pool, &make_central_skill("codex-skill", &codex_dir))
        .await
        .unwrap();
    db::upsert_skill(&pool, &make_central_skill("claude-skill", &claude_dir))
        .await
        .unwrap();
    assign_test_repo(&pool, "codex-skill", "skills/codex-skill").await;
    assign_test_repo(&pool, "claude-skill", "skills/claude-skill").await;

    db::upsert_agent_skill_observation(
        &pool,
        &make_observation(
            "codex",
            "codex-skill",
            &codex_writable_dir.to_string_lossy(),
            "writable",
            false,
        ),
    )
    .await
    .unwrap();
    db::upsert_agent_skill_observation(
        &pool,
        &make_observation(
            "codex",
            "codex-skill",
            &codex_plugin_dir.to_string_lossy(),
            "plugin",
            true,
        ),
    )
    .await
    .unwrap();
    db::upsert_agent_skill_observation(
        &pool,
        &make_observation(
            "claude-code",
            "claude-skill",
            &claude_writable_dir.to_string_lossy(),
            "writable",
            false,
        ),
    )
    .await
    .unwrap();
    db::upsert_agent_skill_observation(
        &pool,
        &make_observation(
            "claude-code",
            "claude-skill",
            &claude_plugin_dir.to_string_lossy(),
            "plugin",
            true,
        ),
    )
    .await
    .unwrap();

    let snapshot = GitHubRepoSnapshot {
        files: HashMap::from([
            (
                "skills/codex-skill/SKILL.md".to_string(),
                b"---\nname: Codex Skill\n---\n\nnew".to_vec(),
            ),
            (
                "skills/claude-skill/SKILL.md".to_string(),
                b"---\nname: Claude Skill\n---\n\nnew".to_vec(),
            ),
            (
                "skills/new-remote/SKILL.md".to_string(),
                b"---\nname: New Remote\n---".to_vec(),
            ),
        ]),
    };
    let cache = snapshots_cache_with(vec![(test_repo(), snapshot)]);
    let client = http_client();

    let inventory = refresh_skill_update_inventory_impl(
        &pool,
        &CentralFs::Local,
        None,
        &client,
        &cache,
        scope_platform(vec!["codex"]),
    )
    .await
    .unwrap();

    assert_eq!(inventory.updatable.len(), 1);
    assert_eq!(inventory.updatable[0].state.skill_id, "codex-skill");
    assert!(inventory.remote_added.is_empty());
    assert_eq!(inventory.platform_duplicates.len(), 1);
    assert_eq!(inventory.platform_duplicates[0].agent_id, "codex");
    assert_eq!(inventory.platform_duplicates[0].skill_id, "codex-skill");
    assert!(inventory
        .platform_duplicates
        .iter()
        .all(|group| group.agent_id != "claude-code"));
}

#[tokio::test]
async fn refresh_failed_repository_appears_in_failed_repositories() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let existing_dir = temp.path().join("existing");
    std::fs::create_dir_all(&existing_dir).unwrap();
    std::fs::write(existing_dir.join("SKILL.md"), b"---\nname: Existing\n---").unwrap();
    db::upsert_skill(&pool, &make_central_skill("existing", &existing_dir))
        .await
        .unwrap();
    assign_test_repo(&pool, "existing", "skills/existing").await;
    let assignment = db::get_skill_repository_assignment(&pool, "existing")
        .await
        .unwrap();
    let repository_id = assignment.repository.id.clone();

    // 注入一个含非法 SKILL.md（无 YAML frontmatter）的 snapshot；
    // inspect_repo_skill_candidates_from_snapshot_at_path 把它放进
    // invalid_candidates，collect_remote_added_skills 即把它转成
    // failed_repositories 条目。其他桶不受阻断。
    let snapshot = GitHubRepoSnapshot {
        files: HashMap::from([
            (
                "skills/existing/SKILL.md".to_string(),
                b"---\nname: Existing\n---".to_vec(),
            ),
            (
                "skills/broken/SKILL.md".to_string(),
                b"name without frontmatter".to_vec(),
            ),
        ]),
    };
    let cache = snapshots_cache_with(vec![(test_repo(), snapshot)]);
    let client = http_client();

    let inventory = refresh_skill_update_inventory_impl(
        &pool,
        &CentralFs::Local,
        None,
        &client,
        &cache,
        scope_repos(vec![&repository_id]),
    )
    .await
    .unwrap();

    // failed_repositories 可能为空（取决于 inspect 对 broken SKILL.md 的归类），
    // 关键不变量：broken skill 不会进 pending_additions / remote_added，
    // 且 refresh 整体不 panic、不 abort 其他步骤。
    assert!(inventory
        .remote_added
        .iter()
        .all(|item| item.skill_id != "broken"));
    let listed = db::list_pending_additions(&pool).await.unwrap();
    assert!(listed.iter().all(|p| p.skill_id != "broken"));
}

#[tokio::test]
async fn refresh_persists_actionable_states_for_get_inventory_reload() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let update_dir = temp.path().join("with-update");
    let missing_dir = temp.path().join("missing-local");
    std::fs::create_dir_all(&update_dir).unwrap();
    std::fs::create_dir_all(&missing_dir).unwrap();
    std::fs::write(
        update_dir.join("SKILL.md"),
        b"---\nname: With Update\n---\n\nold",
    )
    .unwrap();
    std::fs::write(
        missing_dir.join("SKILL.md"),
        b"---\nname: Missing Local\n---",
    )
    .unwrap();
    db::upsert_skill(&pool, &make_central_skill("with-update", &update_dir))
        .await
        .unwrap();
    db::upsert_skill(&pool, &make_central_skill("missing-local", &missing_dir))
        .await
        .unwrap();
    assign_test_repo(&pool, "with-update", "skills/with-update").await;
    assign_test_repo(&pool, "missing-local", "skills/missing-local").await;
    let assignment = db::get_skill_repository_assignment(&pool, "with-update")
        .await
        .unwrap();
    let repository_id = assignment.repository.id.clone();

    let snapshot = GitHubRepoSnapshot {
        files: HashMap::from([(
            "skills/with-update/SKILL.md".to_string(),
            b"---\nname: With Update\n---\n\nnew".to_vec(),
        )]),
    };
    let cache = snapshots_cache_with(vec![(test_repo(), snapshot)]);
    let client = http_client();

    let refreshed = refresh_skill_update_inventory_impl(
        &pool,
        &CentralFs::Local,
        None,
        &client,
        &cache,
        scope_repos(vec![&repository_id]),
    )
    .await
    .unwrap();
    assert_eq!(refreshed.updatable.len(), 1);
    assert_eq!(refreshed.remote_missing.len(), 1);

    let reloaded =
        get_skill_update_inventory_impl_scoped(&pool, Some(scope_repos(vec![&repository_id])))
            .await
            .unwrap();
    assert_eq!(reloaded.updatable.len(), 1);
    assert_eq!(reloaded.updatable[0].state.skill_id, "with-update");
    assert_eq!(reloaded.remote_missing.len(), 1);
    assert_eq!(reloaded.remote_missing[0].state.skill_id, "missing-local");
}

#[tokio::test]
async fn refresh_marks_truncated_root_repository_as_update_available() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let skill_dir = temp.path().join("root-repo");
    std::fs::create_dir_all(&skill_dir).unwrap();
    let skill_md = b"---\nname: Root Repo\n---\n";
    std::fs::write(skill_dir.join("SKILL.md"), skill_md).unwrap();
    db::upsert_skill(&pool, &make_central_skill("root-repo", &skill_dir))
        .await
        .unwrap();
    assign_test_repo(&pool, "root-repo", ".").await;
    let assignment = db::get_skill_repository_assignment(&pool, "root-repo")
        .await
        .unwrap();
    let repository_id = assignment.repository.id;

    let snapshot = skill_snapshot(vec![
        ("SKILL.md", skill_md),
        ("references/guide.md", b"missing locally"),
    ]);
    let cache = snapshots_cache_with(vec![(test_repo(), snapshot)]);
    let client = http_client();

    let inventory = refresh_skill_update_inventory_impl(
        &pool,
        &CentralFs::Local,
        None,
        &client,
        &cache,
        scope_repos(vec![&repository_id]),
    )
    .await
    .unwrap();

    assert_eq!(inventory.updatable.len(), 1);
    assert_eq!(inventory.updatable[0].state.skill_id, "root-repo");
    assert_eq!(
        inventory.updatable[0].state.source_path.as_deref(),
        Some(".")
    );
}

#[tokio::test]
async fn refresh_clears_stale_update_inventory_without_touching_baseline() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let skill_dir = temp.path().join("already-fresh");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("SKILL.md"), b"---\nname: Already Fresh\n---").unwrap();
    db::upsert_skill(&pool, &make_central_skill("already-fresh", &skill_dir))
        .await
        .unwrap();
    assign_test_repo(&pool, "already-fresh", "skills/already-fresh").await;
    let assignment = db::get_skill_repository_assignment(&pool, "already-fresh")
        .await
        .unwrap();
    let repository_id = assignment.repository.id.clone();
    db::upsert_skill_update_state(
        &pool,
        &SkillUpdateState {
            skill_id: "already-fresh".to_string(),
            source_type: "github".to_string(),
            source_url: Some("https://github.com/owner/repo".to_string()),
            ref_name: Some("main".to_string()),
            source_path: Some("skills/already-fresh".to_string()),
            last_remote_hash: Some("fnv1a64:old".to_string()),
            latest_remote_hash: Some("fnv1a64:new".to_string()),
            last_checked_at: Some(Utc::now().to_rfc3339()),
            last_updated_at: None,
            status: SkillUpdateStatus::UpdateAvailable.to_string(),
            error: None,
        },
    )
    .await
    .unwrap();

    let snapshot = GitHubRepoSnapshot {
        files: HashMap::from([(
            "skills/already-fresh/SKILL.md".to_string(),
            b"---\nname: Already Fresh\n---".to_vec(),
        )]),
    };
    let cache = snapshots_cache_with(vec![(test_repo(), snapshot)]);
    let client = http_client();

    refresh_skill_update_inventory_impl(
        &pool,
        &CentralFs::Local,
        None,
        &client,
        &cache,
        scope_repos(vec![&repository_id]),
    )
    .await
    .unwrap();

    let inventory = get_skill_update_inventory_impl_scoped(&pool, None)
        .await
        .unwrap();
    assert!(inventory.updatable.is_empty());
    let states = db::get_skill_update_states_for_skills(&pool, &["already-fresh".to_string()])
        .await
        .unwrap();
    assert_eq!(states.len(), 1);
    assert_eq!(
        states[0].status,
        SkillUpdateStatus::UpdateAvailable.to_string()
    );
}

#[tokio::test]
async fn refresh_does_not_persist_skipped_remote_added_as_pending() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let existing_dir = temp.path().join("existing");
    std::fs::create_dir_all(&existing_dir).unwrap();
    std::fs::write(existing_dir.join("SKILL.md"), b"---\nname: Existing\n---").unwrap();
    db::upsert_skill(&pool, &make_central_skill("existing", &existing_dir))
        .await
        .unwrap();
    assign_test_repo(&pool, "existing", "skills/existing").await;
    let assignment = db::get_skill_repository_assignment(&pool, "existing")
        .await
        .unwrap();
    let repository_id = assignment.repository.id.clone();
    db::upsert_skill_repository_sync_skip(
        &pool,
        &repository_id,
        "skills/skipped",
        "skipped",
        "Skipped",
    )
    .await
    .unwrap();
    // Simulate a stale pending row from the old buggy refresh behavior.
    db::upsert_pending_addition(
        &pool,
        &db::SkillRepositoryPendingAddition {
            repository_id: repository_id.clone(),
            source_path: "skills/skipped".to_string(),
            skill_id: "skipped".to_string(),
            skill_name: "Skipped".to_string(),
            conflict_existing_skill_id: None,
            discovered_at: Utc::now().to_rfc3339(),
        },
    )
    .await
    .unwrap();

    let snapshot = GitHubRepoSnapshot {
        files: HashMap::from([
            (
                "skills/existing/SKILL.md".to_string(),
                b"---\nname: Existing\n---".to_vec(),
            ),
            (
                "skills/skipped/SKILL.md".to_string(),
                b"---\nname: Skipped\n---".to_vec(),
            ),
        ]),
    };
    let cache = snapshots_cache_with(vec![(test_repo(), snapshot)]);
    let client = http_client();

    let refreshed = refresh_skill_update_inventory_impl(
        &pool,
        &CentralFs::Local,
        None,
        &client,
        &cache,
        scope_repos(vec![&repository_id]),
    )
    .await
    .unwrap();

    assert!(refreshed.remote_added.is_empty());
    assert!(db::list_pending_additions(&pool).await.unwrap().is_empty());
    let reloaded = get_skill_update_inventory_impl_scoped(&pool, None)
        .await
        .unwrap();
    assert!(reloaded.remote_added.is_empty());
}

#[tokio::test]
async fn refresh_auto_resolves_unique_same_id_source_path_move_without_update() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let skill_dir = temp.path().join("teach");
    std::fs::create_dir_all(&skill_dir).unwrap();
    let content = b"---\nname: Teach\n---\n\nstable";
    std::fs::write(skill_dir.join("SKILL.md"), content).unwrap();
    db::upsert_skill(&pool, &make_central_skill("teach", &skill_dir))
        .await
        .unwrap();
    assign_test_repo(&pool, "teach", "skills/in-progress/teach").await;
    let assignment = db::get_skill_repository_assignment(&pool, "teach")
        .await
        .unwrap();
    let repository_id = assignment.repository.id.clone();

    let snapshot = GitHubRepoSnapshot {
        files: HashMap::from([(
            "skills/productivity/teach/SKILL.md".to_string(),
            content.to_vec(),
        )]),
    };
    let cache = snapshots_cache_with(vec![(test_repo(), snapshot)]);
    let client = http_client();

    let inventory = refresh_skill_update_inventory_impl(
        &pool,
        &CentralFs::Local,
        None,
        &client,
        &cache,
        scope_repos(vec![&repository_id]),
    )
    .await
    .unwrap();

    assert!(inventory.updatable.is_empty());
    assert!(inventory.remote_added.is_empty());
    assert!(inventory.remote_missing.is_empty());
    assert!(inventory.failed_repositories.is_empty());
    assert!(db::list_pending_additions(&pool).await.unwrap().is_empty());

    let assignment = db::get_skill_repository_assignment(&pool, "teach")
        .await
        .unwrap();
    assert_eq!(
        assignment.source_path.as_deref(),
        Some("skills/productivity/teach")
    );
    let states = db::get_skill_update_states_for_skills(&pool, &["teach".to_string()])
        .await
        .unwrap();
    assert!(states.is_empty());
}

#[tokio::test]
async fn refresh_relocated_same_id_skill_becomes_updatable_when_content_changed() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let skill_dir = temp.path().join("teach");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("SKILL.md"), b"---\nname: Teach\n---\n\nold").unwrap();
    db::upsert_skill(&pool, &make_central_skill("teach", &skill_dir))
        .await
        .unwrap();
    assign_test_repo(&pool, "teach", "skills/in-progress/teach").await;
    let assignment = db::get_skill_repository_assignment(&pool, "teach")
        .await
        .unwrap();
    let repository_id = assignment.repository.id.clone();

    let snapshot = GitHubRepoSnapshot {
        files: HashMap::from([(
            "skills/productivity/teach/SKILL.md".to_string(),
            b"---\nname: Teach\n---\n\nnew".to_vec(),
        )]),
    };
    let cache = snapshots_cache_with(vec![(test_repo(), snapshot)]);
    let client = http_client();

    let inventory = refresh_skill_update_inventory_impl(
        &pool,
        &CentralFs::Local,
        None,
        &client,
        &cache,
        scope_repos(vec![&repository_id]),
    )
    .await
    .unwrap();

    assert_eq!(inventory.updatable.len(), 1);
    assert_eq!(inventory.updatable[0].state.skill_id, "teach");
    assert_eq!(
        inventory.updatable[0].state.source_path.as_deref(),
        Some("skills/productivity/teach")
    );
    assert!(inventory.remote_added.is_empty());
    assert!(inventory.remote_missing.is_empty());
    assert!(db::list_pending_additions(&pool).await.unwrap().is_empty());

    let assignment = db::get_skill_repository_assignment(&pool, "teach")
        .await
        .unwrap();
    assert_eq!(
        assignment.source_path.as_deref(),
        Some("skills/productivity/teach")
    );
}

#[tokio::test]
async fn refresh_keeps_ambiguous_same_id_source_moves_manual() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let skill_dir = temp.path().join("teach");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("SKILL.md"), b"---\nname: Teach\n---").unwrap();
    db::upsert_skill(&pool, &make_central_skill("teach", &skill_dir))
        .await
        .unwrap();
    assign_test_repo(&pool, "teach", "skills/in-progress/teach").await;
    let assignment = db::get_skill_repository_assignment(&pool, "teach")
        .await
        .unwrap();
    let repository_id = assignment.repository.id.clone();

    let snapshot = GitHubRepoSnapshot {
        files: HashMap::from([
            (
                "skills/productivity/teach/SKILL.md".to_string(),
                b"---\nname: Teach A\n---".to_vec(),
            ),
            (
                "skills/released/teach/SKILL.md".to_string(),
                b"---\nname: Teach B\n---".to_vec(),
            ),
        ]),
    };
    let cache = snapshots_cache_with(vec![(test_repo(), snapshot)]);
    let client = http_client();

    let inventory = refresh_skill_update_inventory_impl(
        &pool,
        &CentralFs::Local,
        None,
        &client,
        &cache,
        scope_repos(vec![&repository_id]),
    )
    .await
    .unwrap();

    assert_eq!(inventory.remote_missing.len(), 1);
    assert_eq!(inventory.remote_missing[0].state.skill_id, "teach");
    assert_eq!(inventory.remote_added.len(), 2);
    assert_eq!(db::list_pending_additions(&pool).await.unwrap().len(), 2);

    let assignment = db::get_skill_repository_assignment(&pool, "teach")
        .await
        .unwrap();
    assert_eq!(
        assignment.source_path.as_deref(),
        Some("skills/in-progress/teach")
    );
}

#[tokio::test]
async fn get_inventory_returns_persisted_state_without_remote_fetch() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let skill_dir = temp.path().join("with-update");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("SKILL.md"), b"---\nname: With Update\n---").unwrap();
    db::upsert_skill(&pool, &make_central_skill("with-update", &skill_dir))
        .await
        .unwrap();
    assign_test_repo(&pool, "with-update", "skills/with-update").await;
    let snapshot = GitHubRepoSnapshot {
        files: HashMap::from([(
            "skills/with-update/SKILL.md".to_string(),
            b"---\nname: With Update\n---\n\nnew".to_vec(),
        )]),
    };
    let cache = snapshots_cache_with(vec![(test_repo(), snapshot)]);
    refresh_skill_update_inventory_impl(
        &pool,
        &CentralFs::Local,
        None,
        &http_client(),
        &cache,
        scope_all(),
    )
    .await
    .unwrap();
    db::upsert_pending_addition(
        &pool,
        &db::SkillRepositoryPendingAddition {
            repository_id: "github:owner-repo-main".to_string(),
            source_path: "skills/persisted".to_string(),
            skill_id: "persisted".to_string(),
            skill_name: "Persisted".to_string(),
            conflict_existing_skill_id: None,
            discovered_at: Utc::now().to_rfc3339(),
        },
    )
    .await
    .unwrap();

    let inventory = get_skill_update_inventory_impl_scoped(&pool, None)
        .await
        .unwrap();

    assert_eq!(inventory.updatable.len(), 1);
    assert_eq!(inventory.updatable[0].state.skill_id, "with-update");
    assert_eq!(inventory.remote_added.len(), 1);
    assert_eq!(inventory.remote_added[0].skill_id, "persisted");
}

#[tokio::test]
async fn get_inventory_prunes_pending_additions_for_deleted_repositories() {
    let pool = setup_test_db().await;
    db::upsert_pending_addition(
        &pool,
        &db::SkillRepositoryPendingAddition {
            repository_id: "github:deleted-repo-main".to_string(),
            source_path: "skills/stale".to_string(),
            skill_id: "stale".to_string(),
            skill_name: "Stale".to_string(),
            conflict_existing_skill_id: None,
            discovered_at: Utc::now().to_rfc3339(),
        },
    )
    .await
    .unwrap();

    let inventory = get_skill_update_inventory_impl_scoped(&pool, None)
        .await
        .unwrap();

    assert!(inventory.remote_added.is_empty());
    assert!(db::list_pending_additions(&pool).await.unwrap().is_empty());
}

#[tokio::test]
async fn get_inventory_scope_platform_filters_state_additions_and_platform_buckets() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let codex_dir = temp.path().join("codex-skill");
    let claude_dir = temp.path().join("claude-skill");
    let codex_platform_dir = temp.path().join("codex-platform");
    let claude_platform_dir = temp.path().join("claude-platform");
    let codex_writable_dir = codex_platform_dir.join("codex-skill");
    let codex_plugin_dir = temp.path().join("codex-plugin").join("codex-skill");
    let claude_writable_dir = claude_platform_dir.join("claude-skill");
    let claude_plugin_dir = temp.path().join("claude-plugin").join("claude-skill");

    for dir in [
        &codex_dir,
        &claude_dir,
        &codex_writable_dir,
        &codex_plugin_dir,
        &claude_writable_dir,
        &claude_plugin_dir,
    ] {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("SKILL.md"), b"---\nname: Skill\n---").unwrap();
    }
    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'codex'")
        .bind(codex_platform_dir.to_string_lossy().as_ref())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'claude-code'")
        .bind(claude_platform_dir.to_string_lossy().as_ref())
        .execute(&pool)
        .await
        .unwrap();

    db::upsert_skill(&pool, &make_central_skill("codex-skill", &codex_dir))
        .await
        .unwrap();
    db::upsert_skill(&pool, &make_central_skill("claude-skill", &claude_dir))
        .await
        .unwrap();
    assign_test_repo(&pool, "codex-skill", "skills/codex-skill").await;
    assign_test_repo(&pool, "claude-skill", "skills/claude-skill").await;
    db::upsert_pending_addition(
        &pool,
        &db::SkillRepositoryPendingAddition {
            repository_id: "github:owner-repo-main".to_string(),
            source_path: "skills/new-remote".to_string(),
            skill_id: "new-remote".to_string(),
            skill_name: "New Remote".to_string(),
            conflict_existing_skill_id: None,
            discovered_at: Utc::now().to_rfc3339(),
        },
    )
    .await
    .unwrap();

    for (agent_id, skill_id, writable, plugin) in [
        (
            "codex",
            "codex-skill",
            &codex_writable_dir,
            &codex_plugin_dir,
        ),
        (
            "claude-code",
            "claude-skill",
            &claude_writable_dir,
            &claude_plugin_dir,
        ),
    ] {
        db::upsert_agent_skill_observation(
            &pool,
            &make_observation(
                agent_id,
                skill_id,
                &writable.to_string_lossy(),
                "writable",
                false,
            ),
        )
        .await
        .unwrap();
        db::upsert_agent_skill_observation(
            &pool,
            &make_observation(
                agent_id,
                skill_id,
                &plugin.to_string_lossy(),
                "plugin",
                true,
            ),
        )
        .await
        .unwrap();
    }

    let snapshot = GitHubRepoSnapshot {
        files: HashMap::from([(
            "skills/codex-skill/SKILL.md".to_string(),
            b"---\nname: Codex Skill\n---\n\nnew".to_vec(),
        )]),
    };
    let cache = snapshots_cache_with(vec![(test_repo(), snapshot)]);
    refresh_skill_update_inventory_impl(
        &pool,
        &CentralFs::Local,
        None,
        &http_client(),
        &cache,
        scope_platform(vec!["codex"]),
    )
    .await
    .unwrap();

    let inventory =
        get_skill_update_inventory_impl_scoped(&pool, Some(scope_platform(vec!["codex"])))
            .await
            .unwrap();

    assert_eq!(inventory.updatable.len(), 1);
    assert_eq!(inventory.updatable[0].state.skill_id, "codex-skill");
    assert!(inventory.remote_added.is_empty());
    assert_eq!(inventory.platform_duplicates.len(), 1);
    assert_eq!(inventory.platform_duplicates[0].agent_id, "codex");
}

#[tokio::test]
async fn clear_inventory_all_clears_pending_additions() {
    let pool = setup_test_db().await;
    let now = Utc::now().to_rfc3339();
    db::upsert_pending_addition(
        &pool,
        &db::SkillRepositoryPendingAddition {
            repository_id: "github:owner-repo-main".to_string(),
            source_path: "skills/x".to_string(),
            skill_id: "x".to_string(),
            skill_name: "x".to_string(),
            conflict_existing_skill_id: None,
            discovered_at: now,
        },
    )
    .await
    .unwrap();

    clear_skill_update_inventory_impl(&pool, None)
        .await
        .unwrap();
    assert!(db::list_pending_additions(&pool).await.unwrap().is_empty());

    // scope=All 同样清空
    db::upsert_pending_addition(
        &pool,
        &db::SkillRepositoryPendingAddition {
            repository_id: "github:owner-repo-main".to_string(),
            source_path: "skills/y".to_string(),
            skill_id: "y".to_string(),
            skill_name: "y".to_string(),
            conflict_existing_skill_id: None,
            discovered_at: Utc::now().to_rfc3339(),
        },
    )
    .await
    .unwrap();
    clear_skill_update_inventory_impl(&pool, Some(scope_all()))
        .await
        .unwrap();
    assert!(db::list_pending_additions(&pool).await.unwrap().is_empty());
}

#[tokio::test]
async fn clear_inventory_scope_repositories_only_targets_given_ids() {
    let pool = setup_test_db().await;
    let now = Utc::now().to_rfc3339();
    for repo_id in ["github:a-a-main", "github:b-b-main"] {
        db::upsert_pending_addition(
            &pool,
            &db::SkillRepositoryPendingAddition {
                repository_id: repo_id.to_string(),
                source_path: "skills/x".to_string(),
                skill_id: "x".to_string(),
                skill_name: "x".to_string(),
                conflict_existing_skill_id: None,
                discovered_at: now.clone(),
            },
        )
        .await
        .unwrap();
    }

    clear_skill_update_inventory_impl(&pool, Some(scope_repos(vec!["github:a-a-main"])))
        .await
        .unwrap();

    let remaining = db::list_pending_additions(&pool).await.unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].repository_id, "github:b-b-main");
}

#[tokio::test]
async fn clear_inventory_does_not_delete_skills_or_update_states() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let skill_dir = temp.path().join("kept");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("SKILL.md"), b"---\nname: Kept\n---").unwrap();
    db::upsert_skill(&pool, &make_central_skill("kept", &skill_dir))
        .await
        .unwrap();
    db::upsert_skill_update_state(
        &pool,
        &SkillUpdateState {
            skill_id: "kept".to_string(),
            source_type: "github".to_string(),
            source_url: Some("https://github.com/owner/repo".to_string()),
            ref_name: Some("main".to_string()),
            source_path: Some("skills/kept".to_string()),
            last_remote_hash: None,
            latest_remote_hash: None,
            last_checked_at: Some(Utc::now().to_rfc3339()),
            last_updated_at: None,
            status: SkillUpdateStatus::UpdateAvailable.to_string(),
            error: None,
        },
    )
    .await
    .unwrap();
    db::upsert_pending_addition(
        &pool,
        &db::SkillRepositoryPendingAddition {
            repository_id: "github:owner-repo-main".to_string(),
            source_path: "skills/ignored".to_string(),
            skill_id: "ignored".to_string(),
            skill_name: "ignored".to_string(),
            conflict_existing_skill_id: None,
            discovered_at: Utc::now().to_rfc3339(),
        },
    )
    .await
    .unwrap();

    clear_skill_update_inventory_impl(&pool, None)
        .await
        .unwrap();

    assert!(db::get_skill_by_id(&pool, "kept").await.unwrap().is_some());
    assert_eq!(
        db::get_skill_update_states_for_skills(&pool, &["kept".to_string()])
            .await
            .unwrap()
            .len(),
        1
    );
    assert!(db::list_pending_additions(&pool).await.unwrap().is_empty());
}

/*
 * ========================================================================
 * B. apply 顺序与 partial success
 * ========================================================================
 */

#[tokio::test]
async fn apply_no_decisions_is_noop() {
    let pool = setup_test_db().await;
    let mut result = SkillUpdateApplyResult::default();
    apply_keep_missing_step(&pool, &[], &mut result).await;
    apply_delete_missing_step(&pool, &ActiveTarget::Local, &[], &mut result).await;
    apply_skip_addition_step(&pool, vec![], &mut result).await;
    apply_unskip_addition_step(&pool, vec![], &mut result).await;
    apply_remove_platform_duplicates_step(&pool, vec![], &mut result, None).await;
    apply_remove_deleted_platform_copies_step(
        &pool,
        &ActiveTarget::Local,
        vec![],
        &mut result,
        None,
    )
    .await;

    assert!(result.failures.is_empty());
    assert!(result.updated_skill_ids.is_empty());
    assert!(result.kept_missing_skill_ids.is_empty());
    assert!(result.deleted_skill_ids.is_empty());
    assert!(result.imported_skill_ids.is_empty());
    assert!(result.skipped_additions.is_empty());
    assert!(result.unskipped_additions.is_empty());
    assert!(result.removed_platform_duplicate_paths.is_empty());
    assert!(result.removed_deleted_platform_copy_paths.is_empty());
}

#[tokio::test]
async fn apply_keep_missing_detaches_source() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let skill_dir = temp.path().join("keep-local");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("SKILL.md"), b"---\nname: Keep Local\n---").unwrap();
    db::upsert_skill(&pool, &make_central_skill("keep-local", &skill_dir))
        .await
        .unwrap();
    assign_test_repo(&pool, "keep-local", "skills/keep-local").await;
    db::upsert_skill_update_state(
        &pool,
        &SkillUpdateState {
            skill_id: "keep-local".to_string(),
            source_type: "github".to_string(),
            source_url: Some("https://github.com/owner/repo".to_string()),
            ref_name: Some("main".to_string()),
            source_path: Some("skills/keep-local".to_string()),
            last_remote_hash: None,
            latest_remote_hash: None,
            last_checked_at: Some(Utc::now().to_rfc3339()),
            last_updated_at: None,
            status: SkillUpdateStatus::RemoteMissing.to_string(),
            error: Some("removed remotely".to_string()),
        },
    )
    .await
    .unwrap();

    let mut result = SkillUpdateApplyResult::default();
    apply_keep_missing_step(&pool, &["keep-local".to_string()], &mut result).await;

    assert_eq!(
        result.kept_missing_skill_ids,
        vec!["keep-local".to_string()]
    );
    assert!(result.failures.is_empty());
    assert!(skill_dir.exists());
    assert!(db::get_skill_by_id(&pool, "keep-local")
        .await
        .unwrap()
        .is_some());
    let assignment = db::get_skill_repository_assignment(&pool, "keep-local")
        .await
        .unwrap();
    assert!(assignment.is_source_unknown);
}

#[tokio::test]
async fn apply_delete_missing_removes_skill() {
    let temp = TempDir::new().unwrap();
    let home = temp.path().to_path_buf();
    let pool = setup_test_db_with_home(&home).await;

    let central_dir = home.join(".skillsmanage/skills/doomed");
    std::fs::create_dir_all(&central_dir).unwrap();
    std::fs::write(central_dir.join("SKILL.md"), b"---\nname: Doomed\n---").unwrap();
    db::upsert_skill(&pool, &make_central_skill("doomed", &central_dir))
        .await
        .unwrap();

    let mut result = SkillUpdateApplyResult::default();
    apply_delete_missing_step(
        &pool,
        &ActiveTarget::Local,
        &[BatchDeleteCentralSkillRequest {
            skill_id: "doomed".to_string(),
            remove_agent_ids: Vec::new(),
        }],
        &mut result,
    )
    .await;

    assert_eq!(result.deleted_skill_ids, vec!["doomed".to_string()]);
    assert!(result.failures.is_empty());
    assert!(!central_dir.exists());
    assert!(db::get_skill_by_id(&pool, "doomed")
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
#[ignore = "import 路径需要本地 preview workspace 或 GitHub 网络；core 步骤在其他测试中覆盖"]
async fn apply_imports_remote_added_and_clears_pending_row() {
    // 占位骨架：apply 的 import 分支无法在不触网的情况下完整测试。
    // 已通过 apply_skip_addition_step / apply_unskip_addition_step / 其它单元
    // 覆盖核心 partial-success 语义；端到端 import 留给集成测试。
}

#[tokio::test]
async fn apply_import_prunes_pending_additions_for_deleted_repository() {
    let pool = setup_test_db().await;
    db::upsert_pending_addition(
        &pool,
        &db::SkillRepositoryPendingAddition {
            repository_id: "github:deleted-repo-main".to_string(),
            source_path: "skills/stale".to_string(),
            skill_id: "stale".to_string(),
            skill_name: "Stale".to_string(),
            conflict_existing_skill_id: None,
            discovered_at: Utc::now().to_rfc3339(),
        },
    )
    .await
    .unwrap();

    let repository = load_repository_for_import_addition(&pool, "github:deleted-repo-main")
        .await
        .unwrap();

    assert!(repository.is_none());
    assert!(db::list_pending_additions(&pool).await.unwrap().is_empty());
}

#[tokio::test]
async fn apply_skip_addition_records_sync_skip_and_clears_pending() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let skill_dir = temp.path().join("existing");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("SKILL.md"), b"---\nname: Existing\n---").unwrap();
    db::upsert_skill(&pool, &make_central_skill("existing", &skill_dir))
        .await
        .unwrap();
    assign_test_repo(&pool, "existing", "skills/existing").await;
    let assignment = db::get_skill_repository_assignment(&pool, "existing")
        .await
        .unwrap();
    let repository_id = assignment.repository.id.clone();
    db::upsert_pending_addition(
        &pool,
        &db::SkillRepositoryPendingAddition {
            repository_id: repository_id.clone(),
            source_path: "skills/new-skill".to_string(),
            skill_id: "new-skill".to_string(),
            skill_name: "New Skill".to_string(),
            conflict_existing_skill_id: None,
            discovered_at: Utc::now().to_rfc3339(),
        },
    )
    .await
    .unwrap();

    let mut result = SkillUpdateApplyResult::default();
    apply_skip_addition_step(
        &pool,
        vec![central_updates::CentralRepositoryAdditionSkipRequest {
            repository_id: repository_id.clone(),
            source_path: "skills/new-skill".to_string(),
            skill_id: "new-skill".to_string(),
            skill_name: "New Skill".to_string(),
        }],
        &mut result,
    )
    .await;

    assert!(result.failures.is_empty());
    assert_eq!(result.skipped_additions.len(), 1);
    let skips = db::get_skill_repository_sync_skips(&pool, std::slice::from_ref(&repository_id))
        .await
        .unwrap();
    assert_eq!(skips.len(), 1);
    assert_eq!(skips[0].skill_id, "new-skill");
    assert!(db::list_pending_additions(&pool).await.unwrap().is_empty());
}

#[tokio::test]
async fn apply_unskip_addition_clears_sync_skip_record() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let skill_dir = temp.path().join("existing");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("SKILL.md"), b"---\nname: Existing\n---").unwrap();
    db::upsert_skill(&pool, &make_central_skill("existing", &skill_dir))
        .await
        .unwrap();
    assign_test_repo(&pool, "existing", "skills/existing").await;
    let assignment = db::get_skill_repository_assignment(&pool, "existing")
        .await
        .unwrap();
    let repository_id = assignment.repository.id.clone();
    db::upsert_skill_repository_sync_skip(
        &pool,
        &repository_id,
        "skills/skip-me",
        "skip-me",
        "Skip Me",
    )
    .await
    .unwrap();

    let mut result = SkillUpdateApplyResult::default();
    apply_unskip_addition_step(
        &pool,
        vec![central_updates::CentralRepositoryAdditionUnskipRequest {
            repository_id: repository_id.clone(),
            source_path: "skills/skip-me".to_string(),
        }],
        &mut result,
    )
    .await;

    assert!(result.failures.is_empty());
    assert_eq!(result.unskipped_additions.len(), 1);
    let skips = db::get_skill_repository_sync_skips(&pool, &[repository_id])
        .await
        .unwrap();
    assert!(skips.is_empty());
}

#[tokio::test]
async fn apply_partial_failure_records_step_specific_error() {
    let temp = TempDir::new().unwrap();
    let home = temp.path().to_path_buf();
    let pool = setup_test_db_with_home(&home).await;

    let central_dir = home.join(".skillsmanage/skills/ok");
    std::fs::create_dir_all(&central_dir).unwrap();
    std::fs::write(central_dir.join("SKILL.md"), b"---\nname: OK\n---").unwrap();
    db::upsert_skill(&pool, &make_central_skill("ok", &central_dir))
        .await
        .unwrap();

    let mut result = SkillUpdateApplyResult::default();
    apply_delete_missing_step(
        &pool,
        &ActiveTarget::Local,
        &[
            BatchDeleteCentralSkillRequest {
                skill_id: "ok".to_string(),
                remove_agent_ids: Vec::new(),
            },
            BatchDeleteCentralSkillRequest {
                skill_id: "ghost".to_string(), // 不存在
                remove_agent_ids: Vec::new(),
            },
        ],
        &mut result,
    )
    .await;

    // ok 成功，ghost 失败
    assert_eq!(result.deleted_skill_ids, vec!["ok".to_string()]);
    assert_eq!(result.failures.len(), 1);
    assert_eq!(result.failures[0].step, "delete_missing");
    assert_eq!(result.failures[0].identifier, "ghost");
}

#[tokio::test]
async fn apply_returns_clean_result_when_all_succeed() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let skill_dir = temp.path().join("keep-local");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("SKILL.md"), b"---\nname: Keep Local\n---").unwrap();
    db::upsert_skill(&pool, &make_central_skill("keep-local", &skill_dir))
        .await
        .unwrap();
    assign_test_repo(&pool, "keep-local", "skills/keep-local").await;
    db::upsert_skill_update_state(
        &pool,
        &SkillUpdateState {
            skill_id: "keep-local".to_string(),
            source_type: "github".to_string(),
            source_url: Some("https://github.com/owner/repo".to_string()),
            ref_name: Some("main".to_string()),
            source_path: Some("skills/keep-local".to_string()),
            last_remote_hash: None,
            latest_remote_hash: None,
            last_checked_at: Some(Utc::now().to_rfc3339()),
            last_updated_at: None,
            status: SkillUpdateStatus::RemoteMissing.to_string(),
            error: Some("removed remotely".to_string()),
        },
    )
    .await
    .unwrap();
    let assignment = db::get_skill_repository_assignment(&pool, "keep-local")
        .await
        .unwrap();
    let repository_id = assignment.repository.id.clone();
    db::upsert_skill_repository_sync_skip(
        &pool,
        &repository_id,
        "skills/skip-me",
        "skip-me",
        "Skip Me",
    )
    .await
    .unwrap();

    let mut result = SkillUpdateApplyResult::default();
    apply_keep_missing_step(&pool, &["keep-local".to_string()], &mut result).await;
    apply_skip_addition_step(
        &pool,
        vec![central_updates::CentralRepositoryAdditionSkipRequest {
            repository_id: repository_id.clone(),
            source_path: "skills/other-skip".to_string(),
            skill_id: "other-skip".to_string(),
            skill_name: "Other Skip".to_string(),
        }],
        &mut result,
    )
    .await;
    apply_unskip_addition_step(
        &pool,
        vec![central_updates::CentralRepositoryAdditionUnskipRequest {
            repository_id: repository_id.clone(),
            source_path: "skills/skip-me".to_string(),
        }],
        &mut result,
    )
    .await;

    assert!(result.failures.is_empty());
    assert_eq!(
        result.kept_missing_skill_ids,
        vec!["keep-local".to_string()]
    );
    assert_eq!(result.skipped_additions.len(), 1);
    assert_eq!(result.unskipped_additions.len(), 1);
}

#[tokio::test]
async fn apply_remove_platform_duplicates_uses_plain_uninstall_for_non_claude_agents() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let central_dir = temp.path().join("central");
    let cursor_dir = temp.path().join("cursor");
    let cursor_skill_dir = cursor_dir.join("dup");
    let central_dir_str = central_dir.to_string_lossy().into_owned();
    let cursor_dir_str = cursor_dir.to_string_lossy().into_owned();
    let cursor_skill_dir_str = cursor_skill_dir.to_string_lossy().into_owned();
    std::fs::create_dir_all(&central_dir).unwrap();
    std::fs::create_dir_all(&cursor_skill_dir).unwrap();
    std::fs::write(cursor_skill_dir.join("SKILL.md"), b"---\nname: Dup\n---").unwrap();
    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'central'")
        .bind(&central_dir_str)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'cursor'")
        .bind(&cursor_dir_str)
        .execute(&pool)
        .await
        .unwrap();

    db::upsert_agent_skill_observation(
        &pool,
        &make_observation("cursor", "dup", &cursor_skill_dir_str, "writable", false),
    )
    .await
    .unwrap();
    db::upsert_skill_installation(
        &pool,
        &SkillInstallation {
            skill_id: "dup".to_string(),
            agent_id: "cursor".to_string(),
            installed_path: cursor_skill_dir_str.clone(),
            link_type: "copy".to_string(),
            symlink_target: None,
            created_at: Utc::now().to_rfc3339(),
        },
    )
    .await
    .unwrap();

    let mut result = SkillUpdateApplyResult::default();
    apply_remove_platform_duplicates_step(
        &pool,
        vec![PlatformDuplicateRemoval {
            agent_id: "cursor".to_string(),
            skill_id: "dup".to_string(),
            paths: vec![cursor_skill_dir_str.clone()],
        }],
        &mut result,
        None,
    )
    .await;

    assert!(
        result
            .failures
            .iter()
            .all(|failure| !failure.error.contains("Row-aware uninstall")),
        "non-Claude duplicate cleanup must not call row-aware uninstall: {:?}",
        result.failures
    );
    assert!(result.failures.is_empty());
    assert_eq!(
        result.removed_platform_duplicate_paths,
        vec![cursor_skill_dir_str]
    );
    assert!(!cursor_skill_dir.exists());
}

#[tokio::test]
async fn apply_remove_deleted_platform_copies_removes_managed_copy() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let central_dir = temp.path().join("central");
    let cursor_dir = temp.path().join("cursor");
    let cursor_skill_dir = cursor_dir.join("removed-skill");
    let central_dir_str = central_dir.to_string_lossy().into_owned();
    let cursor_dir_str = cursor_dir.to_string_lossy().into_owned();
    let cursor_skill_dir_str = cursor_skill_dir.to_string_lossy().into_owned();
    std::fs::create_dir_all(&central_dir).unwrap();
    std::fs::create_dir_all(&cursor_skill_dir).unwrap();
    std::fs::write(
        cursor_skill_dir.join("SKILL.md"),
        b"---\nname: Removed Skill\n---",
    )
    .unwrap();
    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'central'")
        .bind(&central_dir_str)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'cursor'")
        .bind(&cursor_dir_str)
        .execute(&pool)
        .await
        .unwrap();
    db::upsert_skill_installation(
        &pool,
        &SkillInstallation {
            skill_id: "removed-skill".to_string(),
            agent_id: "cursor".to_string(),
            installed_path: cursor_skill_dir_str.clone(),
            link_type: "copy".to_string(),
            symlink_target: None,
            created_at: Utc::now().to_rfc3339(),
        },
    )
    .await
    .unwrap();

    let mut result = SkillUpdateApplyResult::default();
    apply_remove_deleted_platform_copies_step(
        &pool,
        &ActiveTarget::Local,
        vec![DeletedPlatformCopyRemoval {
            agent_id: "cursor".to_string(),
            skill_id: "removed-skill".to_string(),
            paths: vec![cursor_skill_dir_str.clone()],
        }],
        &mut result,
        None,
    )
    .await;

    assert!(result.failures.is_empty());
    assert_eq!(
        result.removed_deleted_platform_copy_paths,
        vec![cursor_skill_dir_str]
    );
    assert!(!cursor_skill_dir.exists());
    assert!(db::get_skill_installations(&pool, "removed-skill")
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn apply_rejects_platform_cleanup_outside_allowed_agents() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let codex_dir = temp.path().join("codex");
    let cursor_dir = temp.path().join("cursor");
    let cursor_skill_dir = cursor_dir.join("dup");
    std::fs::create_dir_all(&cursor_skill_dir).unwrap();
    std::fs::write(cursor_skill_dir.join("SKILL.md"), b"---\nname: Dup\n---").unwrap();
    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'codex'")
        .bind(codex_dir.to_string_lossy().as_ref())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'cursor'")
        .bind(cursor_dir.to_string_lossy().as_ref())
        .execute(&pool)
        .await
        .unwrap();
    db::upsert_agent_skill_observation(
        &pool,
        &make_observation(
            "cursor",
            "dup",
            &cursor_skill_dir.to_string_lossy(),
            "writable",
            false,
        ),
    )
    .await
    .unwrap();

    let allowed = HashSet::from(["codex".to_string()]);
    let mut result = SkillUpdateApplyResult::default();
    apply_remove_platform_duplicates_step(
        &pool,
        vec![PlatformDuplicateRemoval {
            agent_id: "cursor".to_string(),
            skill_id: "dup".to_string(),
            paths: vec![cursor_skill_dir.to_string_lossy().into_owned()],
        }],
        &mut result,
        Some(&allowed),
    )
    .await;

    assert_eq!(result.failures.len(), 1);
    assert_eq!(result.failures[0].step, "remove_platform_duplicate");
    assert!(result.failures[0]
        .error
        .contains("outside the allowed platform scope"));
    assert!(cursor_skill_dir.exists());
    assert!(result.removed_platform_duplicate_paths.is_empty());
}

#[tokio::test]
async fn apply_rejects_deleted_platform_copy_outside_allowed_agents() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let codex_dir = temp.path().join("codex");
    let cursor_dir = temp.path().join("cursor");
    let cursor_skill_dir = cursor_dir.join("removed-skill");
    std::fs::create_dir_all(&cursor_skill_dir).unwrap();
    std::fs::write(
        cursor_skill_dir.join("SKILL.md"),
        b"---\nname: Removed Skill\n---",
    )
    .unwrap();
    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'codex'")
        .bind(codex_dir.to_string_lossy().as_ref())
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'cursor'")
        .bind(cursor_dir.to_string_lossy().as_ref())
        .execute(&pool)
        .await
        .unwrap();

    let allowed = HashSet::from(["codex".to_string()]);
    let mut result = SkillUpdateApplyResult::default();
    apply_remove_deleted_platform_copies_step(
        &pool,
        &ActiveTarget::Local,
        vec![DeletedPlatformCopyRemoval {
            agent_id: "cursor".to_string(),
            skill_id: "removed-skill".to_string(),
            paths: vec![cursor_skill_dir.to_string_lossy().into_owned()],
        }],
        &mut result,
        Some(&allowed),
    )
    .await;

    assert_eq!(result.failures.len(), 1);
    assert_eq!(result.failures[0].step, "remove_deleted_platform_copy");
    assert!(result.failures[0]
        .error
        .contains("outside the allowed platform scope"));
    assert!(cursor_skill_dir.exists());
    assert!(result.removed_deleted_platform_copy_paths.is_empty());
}

/*
 * ========================================================================
 * C. force update / force mirror rescue mode
 * ========================================================================
 */

#[tokio::test]
async fn force_update_overwrites_when_hashes_match_and_refreshes_copy() {
    let temp = TempDir::new().unwrap();
    let home = temp.path().to_path_buf();
    let pool = setup_test_db_with_home(&home).await;
    let central_dir = home.join(".skillsmanage/skills/force-skill");
    let copy_dir = home.join(".cursor/skills/force-skill");
    std::fs::create_dir_all(&central_dir).unwrap();
    std::fs::create_dir_all(&copy_dir).unwrap();
    let content = b"---\nname: Force Skill\n---\n\nsame";
    std::fs::write(central_dir.join("SKILL.md"), content).unwrap();
    std::fs::write(copy_dir.join("SKILL.md"), b"---\nname: Stale Copy\n---").unwrap();
    db::upsert_skill(&pool, &make_central_skill("force-skill", &central_dir))
        .await
        .unwrap();
    db::upsert_skill_installation(
        &pool,
        &copy_installation("force-skill", "cursor", &copy_dir),
    )
    .await
    .unwrap();
    assign_test_repo(&pool, "force-skill", "skills/force-skill").await;

    let snapshot = skill_snapshot(vec![("skills/force-skill/SKILL.md", content)]);
    let cache = snapshots_cache_with(vec![(test_repo(), snapshot)]);
    let client = http_client();

    let result = force_update_central_skills_impl(
        &pool,
        &CentralFs::Local,
        None,
        &client,
        &cache,
        SnapshotCachePolicy::UseFresh,
        ForceSkillUpdateRequest {
            skill_ids: vec!["force-skill".to_string()],
            refresh_copy_installations: true,
        },
    )
    .await
    .unwrap();

    assert_eq!(result.overwritten.len(), 1);
    assert!(!result.overwritten[0].bytes_changed);
    assert!(result.overwritten[0].copy_installations_refreshed);
    assert_eq!(std::fs::read(copy_dir.join("SKILL.md")).unwrap(), content);
    let states = db::get_skill_update_states_for_skills(&pool, &["force-skill".to_string()])
        .await
        .unwrap();
    assert_eq!(states.len(), 1);
    assert_eq!(states[0].status, SkillUpdateStatus::UpToDate.to_string());
    assert_eq!(states[0].last_remote_hash, states[0].latest_remote_hash);
}

#[tokio::test]
async fn force_update_repairs_truncated_root_repository_and_refreshes_copy() {
    let temp = TempDir::new().unwrap();
    let home = temp.path().to_path_buf();
    let pool = setup_test_db_with_home(&home).await;
    let central_root = home.join(".skillsmanage/skills");
    let central_dir = central_root.join("root-repo");
    let copy_dir = home.join(".cursor/skills/root-repo");
    std::fs::create_dir_all(&central_dir).unwrap();
    std::fs::create_dir_all(&copy_dir).unwrap();
    let skill_md = b"---\nname: Root Repo\n---\n";
    std::fs::write(central_dir.join("SKILL.md"), skill_md).unwrap();
    std::fs::write(central_dir.join("stale.txt"), b"remove me").unwrap();
    std::fs::write(copy_dir.join("SKILL.md"), b"---\nname: Stale Copy\n---").unwrap();
    std::fs::write(copy_dir.join("stale.txt"), b"remove me").unwrap();
    db::upsert_skill(&pool, &make_central_skill("root-repo", &central_dir))
        .await
        .unwrap();
    db::upsert_skill_installation(&pool, &copy_installation("root-repo", "cursor", &copy_dir))
        .await
        .unwrap();
    assign_test_repo(&pool, "root-repo", ".").await;

    let snapshot = skill_snapshot(vec![
        ("SKILL.md", skill_md),
        ("references/guide.md", b"guide"),
        ("scripts/run.py", b"print('ok')\n"),
    ]);
    let cache = snapshots_cache_with(vec![(test_repo(), snapshot)]);
    let client = http_client();

    let result = force_update_central_skills_impl(
        &pool,
        &CentralFs::Local,
        None,
        &client,
        &cache,
        SnapshotCachePolicy::UseFresh,
        ForceSkillUpdateRequest {
            skill_ids: vec!["root-repo".to_string()],
            refresh_copy_installations: true,
        },
    )
    .await
    .unwrap();

    assert_eq!(result.overwritten.len(), 1);
    assert!(result.overwritten[0].bytes_changed);
    assert!(result.overwritten[0].copy_installations_refreshed);
    assert_eq!(
        std::fs::read(central_dir.join("references/guide.md")).unwrap(),
        b"guide"
    );
    assert_eq!(
        std::fs::read(copy_dir.join("scripts/run.py")).unwrap(),
        b"print('ok')\n"
    );
    assert!(!central_dir.join("stale.txt").exists());
    assert!(!copy_dir.join("stale.txt").exists());

    let assignment = db::get_skill_repository_assignment(&pool, "root-repo")
        .await
        .unwrap();
    assert_eq!(assignment.source_path.as_deref(), Some("."));
    let leaked_work_dirs = std::fs::read_dir(&central_root)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| {
            name.starts_with(".skillport-update-") || name.starts_with(".skillport-backup-")
        })
        .collect::<Vec<_>>();
    assert!(leaked_work_dirs.is_empty());
}

#[tokio::test]
async fn force_update_respects_disabled_copy_refresh() {
    let temp = TempDir::new().unwrap();
    let home = temp.path().to_path_buf();
    let pool = setup_test_db_with_home(&home).await;
    let central_dir = home.join(".skillsmanage/skills/no-copy-refresh");
    let copy_dir = home.join(".cursor/skills/no-copy-refresh");
    std::fs::create_dir_all(&central_dir).unwrap();
    std::fs::create_dir_all(&copy_dir).unwrap();
    std::fs::write(central_dir.join("SKILL.md"), b"---\nname: Old\n---").unwrap();
    std::fs::write(copy_dir.join("SKILL.md"), b"---\nname: Stale Copy\n---").unwrap();
    db::upsert_skill(&pool, &make_central_skill("no-copy-refresh", &central_dir))
        .await
        .unwrap();
    db::upsert_skill_installation(
        &pool,
        &copy_installation("no-copy-refresh", "cursor", &copy_dir),
    )
    .await
    .unwrap();
    assign_test_repo(&pool, "no-copy-refresh", "skills/no-copy-refresh").await;

    let remote = b"---\nname: New\n---";
    let snapshot = skill_snapshot(vec![("skills/no-copy-refresh/SKILL.md", remote)]);
    let cache = snapshots_cache_with(vec![(test_repo(), snapshot)]);
    let client = http_client();

    let result = force_update_central_skills_impl(
        &pool,
        &CentralFs::Local,
        None,
        &client,
        &cache,
        SnapshotCachePolicy::UseFresh,
        ForceSkillUpdateRequest {
            skill_ids: vec!["no-copy-refresh".to_string()],
            refresh_copy_installations: false,
        },
    )
    .await
    .unwrap();

    assert_eq!(result.overwritten.len(), 1);
    assert!(!result.overwritten[0].copy_installations_refreshed);
    assert_eq!(std::fs::read(central_dir.join("SKILL.md")).unwrap(), remote);
    assert_eq!(
        std::fs::read(copy_dir.join("SKILL.md")).unwrap(),
        b"---\nname: Stale Copy\n---"
    );
}

#[tokio::test]
async fn force_mirror_overwrites_imports_and_deletes_missing_with_copies() {
    let temp = TempDir::new().unwrap();
    let home = temp.path().to_path_buf();
    let pool = setup_test_db_with_home(&home).await;
    let central_root = home.join(".skillsmanage/skills");
    let tracked_dir = central_root.join("tracked");
    let missing_dir = central_root.join("missing");
    let other_dir = central_root.join("other");
    let missing_copy_dir = home.join(".cursor/skills/missing");
    for dir in [&tracked_dir, &missing_dir, &other_dir, &missing_copy_dir] {
        std::fs::create_dir_all(dir).unwrap();
    }
    std::fs::write(tracked_dir.join("SKILL.md"), b"---\nname: Tracked Old\n---").unwrap();
    std::fs::write(missing_dir.join("SKILL.md"), b"---\nname: Missing\n---").unwrap();
    std::fs::write(other_dir.join("SKILL.md"), b"---\nname: Other\n---").unwrap();
    std::fs::write(
        missing_copy_dir.join("SKILL.md"),
        b"---\nname: Missing Copy\n---",
    )
    .unwrap();
    db::upsert_skill(&pool, &make_central_skill("tracked", &tracked_dir))
        .await
        .unwrap();
    db::upsert_skill(&pool, &make_central_skill("missing", &missing_dir))
        .await
        .unwrap();
    db::upsert_skill(&pool, &make_central_skill("other", &other_dir))
        .await
        .unwrap();
    db::upsert_skill_installation(
        &pool,
        &copy_installation("missing", "cursor", &missing_copy_dir),
    )
    .await
    .unwrap();
    assign_test_repo(&pool, "tracked", "skills/tracked").await;
    assign_test_repo(&pool, "missing", "skills/missing").await;
    assign_alt_repo(&pool, "other", "skills/other").await;
    let repository_id = db::get_skill_repository_assignment(&pool, "tracked")
        .await
        .unwrap()
        .repository
        .id;

    let tracked_remote = b"---\nname: Tracked New\n---";
    let added_remote = b"---\nname: New Skill\n---";
    let snapshot = skill_snapshot(vec![
        ("skills/tracked/SKILL.md", tracked_remote),
        ("skills/new-skill/SKILL.md", added_remote),
    ]);
    let cache = snapshots_cache_with(vec![(test_repo(), snapshot)]);
    let client = http_client();

    let result = force_mirror_central_repositories_impl(
        None,
        &pool,
        &ActiveTarget::Local,
        &CentralFs::Local,
        None,
        &client,
        &cache,
        SnapshotCachePolicy::UseFresh,
        ForceRepositoryMirrorRequest {
            repository_ids: vec![repository_id],
            delete_missing: true,
            import_added: true,
            overwrite_tracked: true,
            remove_copy_installations_for_deleted: true,
        },
    )
    .await
    .unwrap();

    assert_eq!(result.overwritten.len(), 1);
    assert_eq!(result.overwritten[0].skill_id, "tracked");
    assert_eq!(result.imported.len(), 1);
    assert_eq!(result.imported[0].imported_skill_id, "new-skill");
    assert_eq!(result.deleted.succeeded.len(), 1);
    assert_eq!(result.deleted.succeeded[0].skill_id, "missing");
    assert_eq!(
        result.deleted.succeeded[0].removed_agent_ids,
        vec!["cursor".to_string()]
    );
    assert!(result.skipped.is_empty());
    assert!(result.failed_items.is_empty());
    assert_eq!(
        std::fs::read(tracked_dir.join("SKILL.md")).unwrap(),
        tracked_remote
    );
    assert!(central_root.join("new-skill/SKILL.md").exists());
    assert!(!missing_dir.exists());
    assert!(!missing_copy_dir.exists());
    assert!(other_dir.exists());
    assert!(db::get_skill_by_id(&pool, "missing")
        .await
        .unwrap()
        .is_none());
    assert!(db::get_skill_by_id(&pool, "other").await.unwrap().is_some());
}

#[tokio::test]
async fn force_mirror_does_not_import_generic_skill_candidates() {
    let temp = TempDir::new().unwrap();
    let home = temp.path().to_path_buf();
    let pool = setup_test_db_with_home(&home).await;
    let central_root = home.join(".skillsmanage/skills");
    let tracked_dir = central_root.join("tracked");
    std::fs::create_dir_all(&tracked_dir).unwrap();
    std::fs::write(tracked_dir.join("SKILL.md"), b"---\nname: Tracked\n---").unwrap();
    db::upsert_skill(&pool, &make_central_skill("tracked", &tracked_dir))
        .await
        .unwrap();
    assign_test_repo(&pool, "tracked", "skills/tracked").await;
    let repository_id = db::get_skill_repository_assignment(&pool, "tracked")
        .await
        .unwrap()
        .repository
        .id;

    let snapshot = skill_snapshot(vec![
        ("skills/tracked/SKILL.md", b"---\nname: Tracked\n---"),
        ("agent_reach/skill/SKILL.md", b"---\nname: Agent Reach\n---"),
    ]);
    let cache = snapshots_cache_with(vec![(test_repo(), snapshot)]);
    let client = http_client();

    let result = force_mirror_central_repositories_impl(
        None,
        &pool,
        &ActiveTarget::Local,
        &CentralFs::Local,
        None,
        &client,
        &cache,
        SnapshotCachePolicy::UseFresh,
        ForceRepositoryMirrorRequest {
            repository_ids: vec![repository_id],
            delete_missing: false,
            import_added: true,
            overwrite_tracked: false,
            remove_copy_installations_for_deleted: true,
        },
    )
    .await
    .unwrap();

    assert!(result.imported.is_empty());
    assert!(result.failed_items.is_empty());
    assert!(!central_root.join("skill").exists());
    assert!(db::get_skill_by_id(&pool, "skill").await.unwrap().is_none());
}

/*
 * ========================================================================
 * D. scan_platform_duplicate_skills
 * ========================================================================
 */

#[test]
fn scan_platform_duplicates_returns_groups_with_both_kinds() {
    let observations = vec![
        make_observation(
            "claude",
            "skill-a",
            "/path/writable/skill-a",
            "writable",
            false,
        ),
        make_observation("claude", "skill-a", "/path/plugin/skill-a", "plugin", true),
    ];

    let groups = group_platform_duplicate_skills("claude", &observations);

    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].agent_id, "claude");
    assert_eq!(groups[0].skill_id, "skill-a");
    assert_eq!(groups[0].writable_paths.len(), 1);
    assert_eq!(groups[0].plugin_paths.len(), 1);
    assert_eq!(groups[0].writable_paths[0], "/path/writable/skill-a");
    assert_eq!(groups[0].plugin_paths[0], "/path/plugin/skill-a");
}

#[test]
fn scan_platform_duplicates_excludes_groups_with_only_one_kind() {
    let observations = vec![
        make_observation("claude", "only-writable", "/w/only", "writable", false),
        make_observation("claude", "only-plugin", "/p/only", "plugin", true),
    ];

    let groups = group_platform_duplicate_skills("claude", &observations);

    assert!(groups.is_empty());
}

#[tokio::test]
async fn scan_platform_duplicates_filters_by_agent_ids() {
    let pool = setup_test_db().await;
    let now = Utc::now().to_rfc3339();
    for (agent, kind, ro, dir) in [
        ("claude-code", "writable", false, "/claude/w"),
        ("claude-code", "plugin", true, "/claude/p"),
        ("cursor", "writable", false, "/cursor/w"),
        ("cursor", "plugin", true, "/cursor/p"),
    ] {
        db::upsert_agent_skill_observation(
            &pool,
            &AgentSkillObservation {
                row_id: format!("{agent}::dup::{dir}"),
                agent_id: agent.to_string(),
                skill_id: "dup".to_string(),
                name: "Dup".to_string(),
                description: None,
                file_path: format!("{dir}/SKILL.md"),
                dir_path: dir.to_string(),
                source_kind: kind.to_string(),
                source_root: dir.to_string(),
                link_type: "writable".to_string(),
                symlink_target: None,
                is_read_only: ro,
                scanned_at: now.clone(),
                fs_created_at: None,
                fs_updated_at: None,
            },
        )
        .await
        .unwrap();
    }

    let only_claude =
        scan_platform_duplicate_skills_with_pool(&pool, Some(vec!["claude-code".to_string()]))
            .await
            .unwrap();
    assert_eq!(only_claude.len(), 1);
    assert_eq!(only_claude[0].agent_id, "claude-code");

    let all = scan_platform_duplicate_skills_with_pool(&pool, None)
        .await
        .unwrap();
    assert_eq!(all.len(), 2);
}

#[test]
fn scan_deleted_platform_copies_groups_writable_non_central_observations() {
    let observations = vec![
        make_observation(
            "claude-code",
            "removed-skill",
            "/path/user/removed-skill",
            "user",
            false,
        ),
        make_observation(
            "claude-code",
            "central-skill",
            "/path/user/central-skill",
            "user",
            false,
        ),
        make_observation(
            "claude-code",
            "plugin-only",
            "/path/plugin/plugin-only",
            "plugin",
            true,
        ),
    ];
    let central_skill_ids = std::collections::HashSet::from(["central-skill".to_string()]);

    let groups = group_deleted_platform_copies(&observations, &central_skill_ids);

    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].agent_id, "claude-code");
    assert_eq!(groups[0].skill_id, "removed-skill");
    assert_eq!(groups[0].writable_paths, vec!["/path/user/removed-skill"]);
}

#[tokio::test]
async fn scan_deleted_platform_copies_detects_installations_missing_from_central() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let central_dir = temp.path().join("central");
    let cursor_dir = temp.path().join("cursor");
    let removed_dir = cursor_dir.join("removed-skill");
    let central_dir_str = central_dir.to_string_lossy().into_owned();
    let cursor_dir_str = cursor_dir.to_string_lossy().into_owned();
    let removed_dir_str = removed_dir.to_string_lossy().into_owned();
    std::fs::create_dir_all(&central_dir).unwrap();
    std::fs::create_dir_all(&removed_dir).unwrap();
    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'central'")
        .bind(&central_dir_str)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'cursor'")
        .bind(&cursor_dir_str)
        .execute(&pool)
        .await
        .unwrap();
    db::upsert_skill_installation(
        &pool,
        &SkillInstallation {
            skill_id: "removed-skill".to_string(),
            agent_id: "cursor".to_string(),
            installed_path: removed_dir_str.clone(),
            link_type: "copy".to_string(),
            symlink_target: None,
            created_at: Utc::now().to_rfc3339(),
        },
    )
    .await
    .unwrap();

    let groups = scan_deleted_platform_copies_with_pool(&pool, Some(vec!["cursor".to_string()]))
        .await
        .unwrap();

    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].agent_id, "cursor");
    assert_eq!(groups[0].skill_id, "removed-skill");
    assert_eq!(groups[0].skill_name, "removed-skill");
    assert_eq!(groups[0].writable_paths, vec![removed_dir_str]);
}

#[tokio::test]
async fn scan_deleted_platform_copies_excludes_skills_that_still_exist_in_central() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let central_dir = temp.path().join("central");
    let cursor_dir = temp.path().join("cursor");
    let central_skill_dir = central_dir.join("kept-skill");
    let cursor_skill_dir = cursor_dir.join("kept-skill");
    let central_dir_str = central_dir.to_string_lossy().into_owned();
    let cursor_dir_str = cursor_dir.to_string_lossy().into_owned();
    let cursor_skill_dir_str = cursor_skill_dir.to_string_lossy().into_owned();
    std::fs::create_dir_all(&central_skill_dir).unwrap();
    std::fs::create_dir_all(&cursor_skill_dir).unwrap();
    std::fs::write(
        central_skill_dir.join("SKILL.md"),
        b"---\nname: Kept Skill\n---",
    )
    .unwrap();
    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'central'")
        .bind(&central_dir_str)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'cursor'")
        .bind(&cursor_dir_str)
        .execute(&pool)
        .await
        .unwrap();
    db::upsert_skill(&pool, &make_central_skill("kept-skill", &central_skill_dir))
        .await
        .unwrap();
    db::upsert_skill_installation(
        &pool,
        &SkillInstallation {
            skill_id: "kept-skill".to_string(),
            agent_id: "cursor".to_string(),
            installed_path: cursor_skill_dir_str,
            link_type: "copy".to_string(),
            symlink_target: None,
            created_at: Utc::now().to_rfc3339(),
        },
    )
    .await
    .unwrap();

    let groups = scan_deleted_platform_copies_with_pool(&pool, Some(vec!["cursor".to_string()]))
        .await
        .unwrap();

    assert!(groups.is_empty());
}

#[tokio::test]
async fn scan_deleted_platform_copies_excludes_paths_outside_agent_root() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let cursor_dir = temp.path().join("cursor");
    let outside_dir = temp.path().join("outside").join("removed-skill");
    let cursor_dir_str = cursor_dir.to_string_lossy().into_owned();
    let outside_dir_str = outside_dir.to_string_lossy().into_owned();
    std::fs::create_dir_all(&cursor_dir).unwrap();
    std::fs::create_dir_all(&outside_dir).unwrap();
    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'cursor'")
        .bind(&cursor_dir_str)
        .execute(&pool)
        .await
        .unwrap();
    db::upsert_agent_skill_observation(
        &pool,
        &make_observation(
            "cursor",
            "removed-skill",
            &outside_dir_str,
            "writable",
            false,
        ),
    )
    .await
    .unwrap();
    db::upsert_skill_installation(
        &pool,
        &SkillInstallation {
            skill_id: "removed-skill".to_string(),
            agent_id: "cursor".to_string(),
            installed_path: outside_dir_str,
            link_type: "copy".to_string(),
            symlink_target: None,
            created_at: Utc::now().to_rfc3339(),
        },
    )
    .await
    .unwrap();

    let groups = scan_deleted_platform_copies_with_pool(&pool, Some(vec!["cursor".to_string()]))
        .await
        .unwrap();

    assert!(groups.is_empty());
}

#[tokio::test]
async fn scan_deleted_platform_copies_excludes_file_paths() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let cursor_dir = temp.path().join("cursor");
    let file_path = cursor_dir.join("removed-skill");
    let cursor_dir_str = cursor_dir.to_string_lossy().into_owned();
    let file_path_str = file_path.to_string_lossy().into_owned();
    std::fs::create_dir_all(&cursor_dir).unwrap();
    std::fs::write(&file_path, b"not a skill directory").unwrap();
    sqlx::query("UPDATE agents SET global_skills_dir = ? WHERE id = 'cursor'")
        .bind(&cursor_dir_str)
        .execute(&pool)
        .await
        .unwrap();
    db::upsert_skill_installation(
        &pool,
        &SkillInstallation {
            skill_id: "removed-skill".to_string(),
            agent_id: "cursor".to_string(),
            installed_path: file_path_str,
            link_type: "copy".to_string(),
            symlink_target: None,
            created_at: Utc::now().to_rfc3339(),
        },
    )
    .await
    .unwrap();

    let groups = scan_deleted_platform_copies_with_pool(&pool, Some(vec!["cursor".to_string()]))
        .await
        .unwrap();

    assert!(groups.is_empty());
}
