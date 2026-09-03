use super::*;
use crate::db;
use crate::services::github_import::DuplicateResolution;
use serde_json::Value;
use tempfile::TempDir;

#[derive(Debug, PartialEq)]
struct AuthoritativeSnapshot {
    memberships: Vec<(String, Option<String>, String)>,
    update_states: Vec<Value>,
    skips: Vec<Value>,
}

async fn seed_repository_state() -> (DbPool, TempDir, String) {
    let pool = crate::test_support::mem_pool().await;
    let temp = TempDir::new().expect("create repository-sync tempdir");
    let skill = crate::test_support::central_skill_row("existing", &temp.path().join("existing"));
    db::upsert_skill(&pool, &skill)
        .await
        .expect("seed central skill");
    let repository = db::assign_github_repository_to_skill(
        &pool,
        "owner",
        "repo",
        "main",
        "https://github.com/owner/repo",
        &skill.id,
        "skills/existing",
    )
    .await
    .expect("assign repository membership");
    db::upsert_skill_update_state(
        &pool,
        &SkillUpdateState {
            skill_id: skill.id,
            source_type: "github".to_string(),
            source_url: Some("https://github.com/owner/repo".to_string()),
            ref_name: Some("main".to_string()),
            source_path: Some("skills/existing".to_string()),
            last_remote_hash: Some("old".to_string()),
            latest_remote_hash: None,
            last_checked_at: Some("2026-08-30T00:00:00Z".to_string()),
            last_updated_at: None,
            status: SkillUpdateStatus::RemoteMissing,
            error: Some("removed remotely".to_string()),
        },
    )
    .await
    .expect("seed update state");
    (pool, temp, repository.id)
}

async fn authoritative_snapshot(pool: &DbPool, repository_id: &str) -> AuthoritativeSnapshot {
    let repository_ids = vec![repository_id.to_string()];
    let memberships = db::get_central_repository_members_by_repositories(pool, &repository_ids)
        .await
        .expect("load repository memberships")
        .into_iter()
        .map(|member| (member.skill_id, member.source_path, member.repository.id))
        .collect();
    let update_states = db::get_skill_update_states(pool)
        .await
        .expect("load update states")
        .into_iter()
        .map(|state| serde_json::to_value(state).expect("serialize update state"))
        .collect();
    let skips = db::get_skill_repository_sync_skips(pool, &repository_ids)
        .await
        .expect("load repository skips")
        .into_iter()
        .map(|skip| serde_json::to_value(skip).expect("serialize repository skip"))
        .collect();
    AuthoritativeSnapshot {
        memberships,
        update_states,
        skips,
    }
}

fn skip_request(repository_id: &str, source_path: &str) -> CentralRepositoryAdditionSkipRequest {
    CentralRepositoryAdditionSkipRequest {
        repository_id: repository_id.to_string(),
        source_path: source_path.to_string(),
        skill_id: source_path.replace('/', "-"),
        skill_name: source_path.to_string(),
    }
}

#[tokio::test]
async fn late_invalid_path_rejects_heterogeneous_decisions_before_any_write() {
    let (pool, _temp, repository_id) = seed_repository_state().await;
    db::upsert_skill_repository_sync_skip(
        &pool,
        &repository_id,
        "skills/existing-skip",
        "existing-skip",
        "Existing Skip",
    )
    .await
    .expect("seed existing skip");
    let before = authoritative_snapshot(&pool, &repository_id).await;

    let error = apply_central_repository_sync_impl(
        None,
        &pool,
        &ActiveTarget::Local,
        None,
        CentralRepositorySyncDecisions {
            keep_skill_ids: Vec::new(),
            delete_requests: Vec::new(),
            skip_additions: vec![skip_request(&repository_id, "skills/new-skip")],
            unskip_additions: vec![CentralRepositoryAdditionUnskipRequest {
                repository_id: repository_id.clone(),
                source_path: "skills/existing-skip".to_string(),
            }],
            additions: vec![CentralRepositoryAddedSkillSelection {
                repository_id: repository_id.clone(),
                selections: vec![GitHubSkillImportSelection {
                    source_path: "skills/valid/../../escape".to_string(),
                    resolution: DuplicateResolution::Skip,
                    renamed_skill_id: None,
                }],
            }],
        },
    )
    .await
    .expect_err("late traversal path must reject the complete decision batch");

    assert!(matches!(
        error,
        CentralUpdatesError::UnsupportedRepoPath(path)
            if path == "skills/valid/../../escape"
    ));
    assert_eq!(authoritative_snapshot(&pool, &repository_id).await, before);
}

#[tokio::test]
async fn second_skip_or_unskip_write_failure_rolls_back_and_retry_succeeds() {
    let (pool, _temp, repository_id) = seed_repository_state().await;
    db::upsert_skill_repository_sync_skip(
        &pool,
        &repository_id,
        "skills/remove-me",
        "remove-me",
        "Remove Me",
    )
    .await
    .expect("seed skip to remove");
    let before = authoritative_snapshot(&pool, &repository_id).await;
    sqlx::query(
        "CREATE TRIGGER fail_second_repository_sync_write
         BEFORE DELETE ON skill_repository_sync_skips
         WHEN OLD.source_path = 'skills/remove-me'
         BEGIN
           SELECT RAISE(FAIL, 'injected second repository-sync write failure');
         END",
    )
    .execute(&pool)
    .await
    .expect("install second-write failure trigger");

    let decisions = || CentralRepositorySyncDecisions {
        keep_skill_ids: Vec::new(),
        delete_requests: Vec::new(),
        additions: Vec::new(),
        skip_additions: vec![skip_request(&repository_id, "skills/new-skip")],
        unskip_additions: vec![CentralRepositoryAdditionUnskipRequest {
            repository_id: repository_id.clone(),
            source_path: "skills/remove-me".to_string(),
        }],
    };
    let error =
        apply_central_repository_sync_impl(None, &pool, &ActiveTarget::Local, None, decisions())
            .await
            .expect_err("second skip/unskip write must fail");

    assert!(matches!(error, CentralUpdatesError::Db(_)));
    assert_eq!(authoritative_snapshot(&pool, &repository_id).await, before);

    sqlx::query("DROP TRIGGER fail_second_repository_sync_write")
        .execute(&pool)
        .await
        .expect("remove second-write failure trigger");
    let result =
        apply_central_repository_sync_impl(None, &pool, &ActiveTarget::Local, None, decisions())
            .await
            .expect("retry repository-sync decisions");

    assert_eq!(result.skipped_additions.len(), 1);
    assert_eq!(result.unskipped_additions.len(), 1);
    let after = authoritative_snapshot(&pool, &repository_id).await;
    assert_eq!(after.memberships, before.memberships);
    assert_eq!(after.update_states, before.update_states);
    assert_eq!(after.skips.len(), 1);
    assert_eq!(after.skips[0]["source_path"], "skills/new-skip");
}
