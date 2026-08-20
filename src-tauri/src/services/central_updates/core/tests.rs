use super::super::types::GitHubUpdateSource;
use super::state::{
    find_remote_skill_candidate, is_fresh_update_available_state, reused_or_prepared_local_hash,
};
use super::*;
use crate::db::SkillRepository;
use crate::services::central_updates::fs::{
    collect_remote_skill_files, ensure_remote_skill_manifest, RemoteSkillFile,
};
use crate::services::central_updates::repository_sync::build_remote_missing_skills;
use crate::services::central_updates::snapshots::CentralUpdateRepositorySnapshot;
use crate::services::central_updates::{collect_remote_added_skills, repo_cache_key};
use crate::services::github_import::{GitHubRepoRef, GitHubRepoSnapshot};
use crate::test_support::mem_pool as setup_test_db;
use chrono::{Duration as ChronoDuration, Utc};
use sqlx::SqlitePool;
use std::path::Path;
use std::sync::Arc;
use tempfile::TempDir;

fn pinned_snapshot(snapshot: GitHubRepoSnapshot) -> Arc<CentralUpdateRepositorySnapshot> {
    let digest = crate::services::github_import::repository_snapshot_digest_from_local(&snapshot);
    Arc::new(CentralUpdateRepositorySnapshot::new(
        "a".repeat(40),
        digest,
        snapshot,
    ))
}

async fn setup_remote_test_db(remote_home: &Path) -> SqlitePool {
    crate::test_support::mem_pool_with_home(&remote_home.to_string_lossy()).await
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

async fn assign_test_repo(pool: &DbPool, skill_id: &str, source_path: &str) {
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

#[test]
fn missing_candidate_is_typed_as_remote_missing() {
    let source = GitHubUpdateSource {
        repo: test_repo(),
        source_path: "skills/missing".to_string(),
    };
    let snapshot = GitHubRepoSnapshot {
        files: HashMap::from([(
            "skills/other/SKILL.md".to_string(),
            b"---\nname: Other\n---".to_vec(),
        )]),
    };

    let error = find_remote_skill_candidate(&source, &snapshot).unwrap_err();

    assert!(matches!(error, RemoteSkillLoadError::RemoteMissing(_)));
    assert!(error
        .message()
        .contains("no longer contains an importable skill"));
}

#[test]
fn missing_source_path_is_remote_missing_reason() {
    let snapshot = GitHubRepoSnapshot {
        files: HashMap::from([(
            "skills/other/SKILL.md".to_string(),
            b"---\nname: Other\n---".to_vec(),
        )]),
    };

    let reason = collect_remote_skill_files(&snapshot, "skills/missing").unwrap_err();
    let error = RemoteSkillLoadError::remote_missing(reason.to_string());

    assert!(matches!(error, RemoteSkillLoadError::RemoteMissing(_)));
    assert!(error.message().contains("no longer available"));
}

#[test]
fn missing_skill_manifest_is_remote_missing_reason() {
    let files = vec![RemoteSkillFile {
        repo_path: "skills/demo/README.md".to_string(),
        relative_path: "README.md".to_string(),
        bytes: b"demo".to_vec(),
    }];

    let reason = ensure_remote_skill_manifest(&files).unwrap_err();
    let error = RemoteSkillLoadError::remote_missing(reason.to_string());

    assert!(matches!(error, RemoteSkillLoadError::RemoteMissing(_)));
    assert!(error.message().contains("SKILL.md"));
}

#[test]
fn update_counters_count_remote_missing_as_skipped() {
    let mut counters = UpdateCounters::default();
    let state = SkillUpdateState {
        skill_id: "missing".to_string(),
        source_type: "github".to_string(),
        source_url: Some("https://github.com/owner/repo".to_string()),
        ref_name: Some("main".to_string()),
        source_path: Some("skills/missing".to_string()),
        last_remote_hash: None,
        latest_remote_hash: None,
        last_checked_at: Some(Utc::now().to_rfc3339()),
        last_updated_at: None,
        status: SkillUpdateStatus::RemoteMissing,
        error: Some("removed remotely".to_string()),
    };

    update_counters_for_state(&mut counters, &state);

    assert_eq!(counters.completed, 1);
    assert_eq!(counters.skipped, 1);
    assert_eq!(counters.failed, 0);
}

#[tokio::test]
async fn collect_remote_added_skills_detects_repo_candidates_not_in_local_members() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let skill_dir = temp.path().join("existing");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("SKILL.md"), b"---\nname: Existing\n---").unwrap();
    let skill = make_central_skill("existing", &skill_dir);
    db::upsert_skill(&pool, &skill).await.unwrap();
    assign_test_repo(&pool, "existing", "skills/existing").await;
    let assignment = db::get_skill_repository_assignment(&pool, "existing")
        .await
        .unwrap();
    let repository = assignment.repository;
    let repository_ids = vec![repository.id.clone()];
    let repo = test_repo();
    let snapshots = HashMap::from([(
        repo_cache_key(&repo),
        pinned_snapshot(GitHubRepoSnapshot {
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
        }),
    )]);
    let mut failures = Vec::new();

    let added = collect_remote_added_skills(
        &pool,
        &repository_ids,
        &[(repository, repo)],
        &snapshots,
        &mut failures,
    )
    .await
    .unwrap();

    assert!(failures.is_empty());
    assert_eq!(added.remote_added.len(), 1);
    assert!(added.skipped_remote_added.is_empty());
    assert_eq!(added.remote_added[0].preview.skill_id, "new-skill");
    assert_eq!(
        added.remote_added[0].preview.source_path,
        "skills/new-skill"
    );
}

#[tokio::test]
async fn collect_remote_added_skills_splits_persisted_skips() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let skill_dir = temp.path().join("existing");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("SKILL.md"), b"---\nname: Existing\n---").unwrap();
    let skill = make_central_skill("existing", &skill_dir);
    db::upsert_skill(&pool, &skill).await.unwrap();
    assign_test_repo(&pool, "existing", "skills/existing").await;
    let assignment = db::get_skill_repository_assignment(&pool, "existing")
        .await
        .unwrap();
    let repository = assignment.repository;
    db::upsert_skill_repository_sync_skip(
        &pool,
        &repository.id,
        "skills/skipped-skill",
        "old-skipped-id",
        "Old Skipped Name",
    )
    .await
    .unwrap();
    let repository_ids = vec![repository.id.clone()];
    let repo = test_repo();
    let snapshots = HashMap::from([(
        repo_cache_key(&repo),
        pinned_snapshot(GitHubRepoSnapshot {
            files: HashMap::from([
                (
                    "skills/existing/SKILL.md".to_string(),
                    b"---\nname: Existing\n---".to_vec(),
                ),
                (
                    "skills/new-skill/SKILL.md".to_string(),
                    b"---\nname: New Skill\n---".to_vec(),
                ),
                (
                    "skills/skipped-skill/SKILL.md".to_string(),
                    b"---\nname: Skipped Skill\n---".to_vec(),
                ),
            ]),
        }),
    )]);
    let mut failures = Vec::new();

    let added = collect_remote_added_skills(
        &pool,
        &repository_ids,
        &[(repository.clone(), repo)],
        &snapshots,
        &mut failures,
    )
    .await
    .unwrap();

    assert!(failures.is_empty());
    assert_eq!(added.remote_added.len(), 1);
    assert_eq!(added.remote_added[0].preview.skill_id, "new-skill");
    assert_eq!(added.skipped_remote_added.len(), 1);
    assert_eq!(
        added.skipped_remote_added[0].preview.source_path,
        "skills/skipped-skill"
    );
    let skips = db::get_skill_repository_sync_skips(&pool, &[repository.id])
        .await
        .unwrap();
    assert_eq!(skips.len(), 1);
    assert_eq!(skips[0].skill_id, "skipped-skill");
    assert_eq!(skips[0].skill_name, "Skipped Skill");
}

#[test]
fn remote_missing_skills_include_repository_display_metadata() {
    let repository = SkillRepository {
        id: "github:owner-repo-main".to_string(),
        name: "fallback name".to_string(),
        source_type: "github".to_string(),
        owner: Some("owner".to_string()),
        repo: Some("repo".to_string()),
        branch: Some("main".to_string()),
        url: Some("https://github.com/owner/repo".to_string()),
        pinned: false,
        is_unknown: false,
        created_at: Utc::now().to_rfc3339(),
        updated_at: Utc::now().to_rfc3339(),
        last_synced_at: None,
    };
    let state = SkillUpdateState {
        skill_id: "missing".to_string(),
        source_type: "github".to_string(),
        source_url: Some("https://github.com/owner/repo".to_string()),
        ref_name: Some("main".to_string()),
        source_path: Some("skills/missing".to_string()),
        last_remote_hash: None,
        latest_remote_hash: None,
        last_checked_at: Some(Utc::now().to_rfc3339()),
        last_updated_at: None,
        status: SkillUpdateStatus::RemoteMissing,
        error: Some("removed remotely".to_string()),
    };
    let repo_by_id = HashMap::from([(repository.id.clone(), repository)]);

    let missing = build_remote_missing_skills(&repo_by_id, vec![state]);

    assert_eq!(missing.len(), 1);
    assert_eq!(
        missing[0].repository_id.as_deref(),
        Some("github:owner-repo-main")
    );
    assert_eq!(missing[0].repository_name, "owner/repo");
    assert_eq!(
        missing[0].repo.as_ref().map(|repo| repo.branch.as_str()),
        Some("main")
    );
    assert_eq!(missing[0].state.skill_id, "missing");
}

#[test]
fn remote_missing_skills_allow_unmapped_repository_source() {
    let state = SkillUpdateState {
        skill_id: "missing".to_string(),
        source_type: "github".to_string(),
        source_url: Some("https://github.com/unknown/source".to_string()),
        ref_name: Some("main".to_string()),
        source_path: Some("skills/missing".to_string()),
        last_remote_hash: None,
        latest_remote_hash: None,
        last_checked_at: Some(Utc::now().to_rfc3339()),
        last_updated_at: None,
        status: SkillUpdateStatus::RemoteMissing,
        error: Some("removed remotely".to_string()),
    };

    let missing = build_remote_missing_skills(&HashMap::new(), vec![state]);

    assert_eq!(missing.len(), 1);
    assert!(missing[0].repository_id.is_none());
    assert_eq!(missing[0].repository_name, "Unknown source");
    assert!(missing[0].repo.is_none());
    assert_eq!(missing[0].state.skill_id, "missing");
}

#[tokio::test]
async fn keep_remote_missing_detaches_source_without_deleting_skill() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let skill_dir = temp.path().join("keep-local");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("SKILL.md"), b"---\nname: Keep Local\n---").unwrap();
    let skill = make_central_skill("keep-local", &skill_dir);
    db::upsert_skill(&pool, &skill).await.unwrap();
    db::assign_github_repository_to_skill(
        &pool,
        "owner",
        "repo",
        "main",
        "https://github.com/owner/repo",
        "keep-local",
        "skills/keep-local",
    )
    .await
    .unwrap();
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
            status: SkillUpdateStatus::RemoteMissing,
            error: Some("removed remotely".to_string()),
        },
    )
    .await
    .unwrap();

    let kept = keep_remote_missing_central_skills_impl(&pool, &["keep-local".to_string()])
        .await
        .unwrap();

    assert_eq!(kept, vec!["keep-local"]);
    assert!(skill_dir.exists());
    assert!(db::get_skill_by_id(&pool, "keep-local")
        .await
        .unwrap()
        .is_some());
    let assignment = db::get_skill_repository_assignment(&pool, "keep-local")
        .await
        .unwrap();
    assert!(assignment.is_source_unknown);
    assert!(
        db::get_skill_update_states_for_skills(&pool, &["keep-local".to_string()])
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn keep_remote_missing_rejects_non_remote_missing_state() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let skill_dir = temp.path().join("available");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("SKILL.md"), b"---\nname: Available\n---").unwrap();
    let skill = make_central_skill("available", &skill_dir);
    db::upsert_skill(&pool, &skill).await.unwrap();
    db::assign_github_repository_to_skill(
        &pool,
        "owner",
        "repo",
        "main",
        "https://github.com/owner/repo",
        "available",
        "skills/available",
    )
    .await
    .unwrap();
    db::upsert_skill_update_state(
        &pool,
        &SkillUpdateState {
            skill_id: "available".to_string(),
            source_type: "github".to_string(),
            source_url: Some("https://github.com/owner/repo".to_string()),
            ref_name: Some("main".to_string()),
            source_path: Some("skills/available".to_string()),
            last_remote_hash: Some("fnv1a64:old".to_string()),
            latest_remote_hash: Some("fnv1a64:new".to_string()),
            last_checked_at: Some(Utc::now().to_rfc3339()),
            last_updated_at: None,
            status: SkillUpdateStatus::UpdateAvailable,
            error: None,
        },
    )
    .await
    .unwrap();

    let error = keep_remote_missing_central_skills_impl(&pool, &["available".to_string()])
        .await
        .unwrap_err();

    assert!(error.to_string().contains("not marked as removed remotely"));
    assert!(matches!(error, CentralUpdatesError::NotRemoteMissing(_)));
    let assignment = db::get_skill_repository_assignment(&pool, "available")
        .await
        .unwrap();
    assert!(!assignment.is_source_unknown);
    assert_eq!(
        db::get_skill_update_states_for_skills(&pool, &["available".to_string()])
            .await
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn load_selected_central_skills_filters_in_db_and_preserves_requested_order() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    for id in ["one", "two", "three"] {
        let skill_dir = temp.path().join(id);
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), b"---\nname: Demo\n---").unwrap();
        db::upsert_skill(&pool, &make_central_skill(id, &skill_dir))
            .await
            .unwrap();
    }

    let skills =
        load_selected_central_skills(&pool, Some(&["three".to_string(), "one".to_string()]))
            .await
            .unwrap();

    assert_eq!(
        skills.into_iter().map(|skill| skill.id).collect::<Vec<_>>(),
        vec!["three", "one"]
    );
}

#[tokio::test]
async fn prepare_skill_updates_batches_assignments_and_local_hashes() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let first_dir = temp.path().join("first");
    let second_dir = temp.path().join("second");
    std::fs::create_dir_all(&first_dir).unwrap();
    std::fs::create_dir_all(&second_dir).unwrap();
    std::fs::write(first_dir.join("SKILL.md"), b"---\nname: First\n---").unwrap();
    std::fs::write(second_dir.join("SKILL.md"), b"---\nname: Second\n---").unwrap();
    let first = make_central_skill("first", &first_dir);
    let second = make_central_skill("second", &second_dir);
    db::upsert_skill(&pool, &first).await.unwrap();
    db::upsert_skill(&pool, &second).await.unwrap();
    assign_test_repo(&pool, "first", "skills/first").await;
    assign_test_repo(&pool, "second", "skills/second").await;

    let prepared = prepare_skill_updates(
        &pool,
        &CentralFs::Local,
        vec![first.clone(), second.clone()],
        None,
        false,
    )
    .await
    .unwrap();

    assert_eq!(prepared.len(), 2);
    assert_eq!(
        prepared[0]
            .source
            .as_ref()
            .map(|source| source.source_path.as_str()),
        Some("skills/first")
    );
    assert!(prepared[0]
        .local_hash
        .as_deref()
        .is_some_and(|hash| hash.starts_with("sha256-manifest:")));
    assert!(prepared[1].local_hash.is_some());
}

#[tokio::test]
async fn prepare_skill_updates_reuses_fresh_update_available_local_hash() {
    let pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let skill_dir = temp.path().join("fresh");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("SKILL.md"), b"---\nname: Fresh\n---").unwrap();
    let skill = make_central_skill("fresh", &skill_dir);
    db::upsert_skill(&pool, &skill).await.unwrap();
    assign_test_repo(&pool, "fresh", "skills/fresh").await;
    db::upsert_skill_update_state(
        &pool,
        &SkillUpdateState {
            skill_id: "fresh".to_string(),
            source_type: "github".to_string(),
            source_url: Some("https://github.com/owner/repo".to_string()),
            ref_name: Some("main".to_string()),
            source_path: Some("skills/fresh".to_string()),
            last_remote_hash: Some("sha256-manifest:cached-local".to_string()),
            latest_remote_hash: Some("sha256-manifest:remote".to_string()),
            last_checked_at: Some(Utc::now().to_rfc3339()),
            last_updated_at: None,
            status: SkillUpdateStatus::UpdateAvailable,
            error: None,
        },
    )
    .await
    .unwrap();

    let prepared = prepare_skill_updates(&pool, &CentralFs::Local, vec![skill], None, true)
        .await
        .unwrap();

    assert_eq!(prepared[0].local_hash, None);
    assert_eq!(
        reused_or_prepared_local_hash(&prepared[0]).as_deref(),
        Some("sha256-manifest:cached-local")
    );
}

#[test]
fn stale_update_available_state_does_not_reuse_local_hash() {
    let stale = SkillUpdateState {
        skill_id: "stale".to_string(),
        source_type: "github".to_string(),
        source_url: Some("https://github.com/owner/repo".to_string()),
        ref_name: Some("main".to_string()),
        source_path: Some("skills/stale".to_string()),
        last_remote_hash: Some("sha256-manifest:old".to_string()),
        latest_remote_hash: Some("sha256-manifest:new".to_string()),
        last_checked_at: Some((Utc::now() - ChronoDuration::minutes(11)).to_rfc3339()),
        last_updated_at: None,
        status: SkillUpdateStatus::UpdateAvailable,
        error: None,
    };

    assert!(!is_fresh_update_available_state(&stale));
}

#[tokio::test]
async fn prepared_state_helpers_use_active_pool_assignment() {
    let local_pool = setup_test_db().await;
    let temp = TempDir::new().unwrap();
    let local_dir = temp.path().join("local-active-only");
    std::fs::create_dir_all(&local_dir).unwrap();
    std::fs::write(local_dir.join("SKILL.md"), b"---\nname: Local\n---").unwrap();
    let local_skill = make_central_skill("shared-id", &local_dir);
    db::upsert_skill(&local_pool, &local_skill).await.unwrap();
    assign_test_repo(&local_pool, "shared-id", "skills/local").await;

    let remote_home = temp.path().join("remote-home");
    let remote_pool = setup_remote_test_db(&remote_home).await;
    let remote_dir = remote_home.join(".skillsmanage/skills/shared-id");
    std::fs::create_dir_all(&remote_dir).unwrap();
    std::fs::write(remote_dir.join("SKILL.md"), b"---\nname: Remote\n---").unwrap();
    let remote_skill = make_central_skill("shared-id", &remote_dir);
    db::upsert_skill(&remote_pool, &remote_skill).await.unwrap();
    assign_test_repo(&remote_pool, "shared-id", "skills/remote").await;

    let prepared = prepare_skill_updates(
        &remote_pool,
        &CentralFs::Local,
        vec![remote_skill.clone()],
        None,
        false,
    )
    .await
    .unwrap();
    let state =
        unsupported_state_from_assignment(&remote_skill, &prepared[0].assignment, Some("probe"));

    assert_eq!(
        prepared[0].source.as_ref().unwrap().source_path,
        "skills/remote"
    );
    assert_eq!(state.source_path.as_deref(), Some("skills/remote"));
}
