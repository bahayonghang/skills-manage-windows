//! Full coverage tests for Phase P3 inventory commands.
//!
//! 风格参考 `central_updates/tests.rs`：用内存 SQLite + TempDir + 预填
//! `central_update_snapshots` 缓存，避免触网；只测 `_impl` 内核版本，
//! 不依赖 `State<AppState>`。
//!
//! 分组：
//! - A: refresh / get / clear 行为
//! - B: apply 顺序与 partial success
//! - C: scan_platform_duplicate_skills
//! - D: 取消信号（暂留位）

use super::*;
use crate::commands::central_updates::repo_cache_key;
use crate::commands::central_updates_fs::CentralFs;
use crate::commands::github_import::GitHubRepoRef;
use crate::db;
use crate::db::{AgentSkillObservation, Skill, SkillInstallation, SkillUpdateState};
use crate::services::central_skills::BatchDeleteCentralSkillRequest;
use crate::services::github_import::GitHubRepoSnapshot;
use crate::targets::ActiveTarget;
use crate::CentralUpdateSnapshotCache;
use chrono::Utc;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::path::Path;
use tempfile::TempDir;

/*
 * ========================================================================
 * 通用 helpers
 * ========================================================================
 */

async fn setup_test_db() -> SqlitePool {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    db::init_database(&pool).await.unwrap();
    pool
}

/// 用 TempDir 当 `~`，central 目录就落在 `{home}/.skillsmanage/skills/`，
/// 测试可以安全清理。delete_central_skill_impl 会去看这个目录是否合法。
async fn setup_test_db_with_home(home: &Path) -> SqlitePool {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    db::init_database_for_remote_home(&pool, &home.to_string_lossy())
        .await
        .unwrap();
    pool
}

fn make_central_skill(id: &str, dir: &Path) -> Skill {
    Skill {
        id: id.to_string(),
        name: id.to_string(),
        description: Some(format!("Desc for {id}")),
        file_path: dir.join("SKILL.md").to_string_lossy().into_owned(),
        canonical_path: Some(dir.to_string_lossy().into_owned()),
        is_central: true,
        source: Some("github:owner/repo".to_string()),
        content: None,
        scanned_at: Utc::now().to_rfc3339(),
        fs_created_at: None,
        fs_updated_at: None,
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
    reqwest::Client::builder().build().unwrap()
}

fn scope_all() -> SkillRefreshScope {
    SkillRefreshScope {
        kind: SkillRefreshScopeKind::All,
        skill_ids: None,
        repository_ids: None,
    }
}

fn scope_skills(ids: Vec<&str>) -> SkillRefreshScope {
    SkillRefreshScope {
        kind: SkillRefreshScopeKind::Skills,
        skill_ids: Some(ids.into_iter().map(String::from).collect()),
        repository_ids: None,
    }
}

fn scope_repos(ids: Vec<&str>) -> SkillRefreshScope {
    SkillRefreshScope {
        kind: SkillRefreshScopeKind::Repositories,
        skill_ids: None,
        repository_ids: Some(ids.into_iter().map(String::from).collect()),
    }
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

    let reloaded = get_skill_update_inventory_impl(&pool).await.unwrap();
    assert_eq!(reloaded.updatable.len(), 1);
    assert_eq!(reloaded.updatable[0].state.skill_id, "with-update");
    assert_eq!(reloaded.remote_missing.len(), 1);
    assert_eq!(reloaded.remote_missing[0].state.skill_id, "missing-local");
}

#[tokio::test]
async fn refresh_persists_non_actionable_state_to_clear_stale_update() {
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

    let inventory = get_skill_update_inventory_impl(&pool).await.unwrap();
    assert!(inventory.updatable.is_empty());
    let states = db::get_skill_update_states_for_skills(&pool, &["already-fresh".to_string()])
        .await
        .unwrap();
    assert_eq!(states.len(), 1);
    assert_eq!(states[0].status, SkillUpdateStatus::UpToDate.to_string());
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
    let reloaded = get_skill_update_inventory_impl(&pool).await.unwrap();
    assert!(reloaded.remote_added.is_empty());
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
    db::upsert_skill_update_state(
        &pool,
        &SkillUpdateState {
            skill_id: "with-update".to_string(),
            source_type: "github".to_string(),
            source_url: Some("https://github.com/owner/repo".to_string()),
            ref_name: Some("main".to_string()),
            source_path: Some("skills/with-update".to_string()),
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

    let inventory = get_skill_update_inventory_impl(&pool).await.unwrap();

    assert_eq!(inventory.updatable.len(), 1);
    assert_eq!(inventory.updatable[0].state.skill_id, "with-update");
    assert_eq!(inventory.remote_added.len(), 1);
    assert_eq!(inventory.remote_added[0].skill_id, "persisted");
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
    apply_remove_platform_duplicates_step(&pool, vec![], &mut result).await;
    apply_remove_deleted_platform_copies_step(&pool, &ActiveTarget::Local, vec![], &mut result)
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

/*
 * ========================================================================
 * C. scan_platform_duplicate_skills
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
