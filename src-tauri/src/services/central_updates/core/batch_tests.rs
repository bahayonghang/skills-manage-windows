use super::*;
use crate::services::central_updates::fs::RemoteSkillFile;
use crate::services::central_updates::types::GitHubUpdateSource;
use crate::services::github_import::{GitHubRepoRef, RemoteSkillCandidate};

fn update_plan(target_dir: std::path::PathBuf) -> SkillUpdatePlan {
    let skill_id = "db-fail";
    SkillUpdatePlan {
        skill: Skill {
            id: skill_id.to_string(),
            uid: "uid-db-fail".to_string(),
            name: "Old".to_string(),
            description: None,
            file_path: target_dir.join("SKILL.md").to_string_lossy().into_owned(),
            canonical_path: Some(target_dir.to_string_lossy().into_owned()),
            is_central: true,
            source: Some("github:owner/repo".to_string()),
            content: None,
            scanned_at: Utc::now().to_rfc3339(),
            fs_created_at: None,
            fs_updated_at: None,
        },
        remote: RemoteSkillContent {
            source: GitHubUpdateSource {
                repo: GitHubRepoRef {
                    owner: "owner".to_string(),
                    repo: "repo".to_string(),
                    branch: "main".to_string(),
                    normalized_url: "https://github.com/owner/repo".to_string(),
                },
                source_path: "skills/db-fail".to_string(),
            },
            candidate: RemoteSkillCandidate {
                source_path: "skills/db-fail".to_string(),
                skill_id: skill_id.to_string(),
                skill_name: "New".to_string(),
                description: None,
                plugin_name: None,
                root_directory: "skills".to_string(),
                skill_directory_name: skill_id.to_string(),
                download_url: "https://github.com/owner/repo/archive/main.zip".to_string(),
            },
            files: vec![RemoteSkillFile {
                repo_path: "skills/db-fail/SKILL.md".to_string(),
                relative_path: "SKILL.md".to_string(),
                bytes: b"new".to_vec(),
            }],
            remote_hash: "remote-new".to_string(),
            local_hash: "local-old".to_string(),
            target_dir,
            resolved_commit_sha: None,
            content_digest: None,
        },
        refresh_copies: false,
    }
}

fn named_update_plan(skill_id: &str, target_dir: std::path::PathBuf) -> SkillUpdatePlan {
    let mut plan = update_plan(target_dir.clone());
    plan.skill.id = skill_id.to_string();
    plan.skill.uid = format!("uid-{skill_id}");
    plan.skill.name = format!("Old {skill_id}");
    plan.skill.file_path = target_dir.join("SKILL.md").to_string_lossy().into_owned();
    plan.skill.canonical_path = Some(target_dir.to_string_lossy().into_owned());
    plan.remote.source.source_path = format!("skills/{skill_id}");
    plan.remote.candidate.source_path = format!("skills/{skill_id}");
    plan.remote.candidate.skill_id = skill_id.to_string();
    plan.remote.candidate.skill_name = format!("New {skill_id}");
    plan.remote.candidate.skill_directory_name = skill_id.to_string();
    plan.remote.target_dir = target_dir;
    plan.remote.files = vec![RemoteSkillFile {
        repo_path: format!("skills/{skill_id}/SKILL.md"),
        relative_path: "SKILL.md".to_string(),
        bytes: format!("new {skill_id}").into_bytes(),
    }];
    plan
}

async fn insert_pending_delete_collision(
    pool: &DbPool,
    root: &std::path::Path,
    skill_id: &str,
) -> String {
    let operation_id = format!("pending-delete-{skill_id}");
    let manifest = crate::services::central_operation::OperationManifest::Delete(
        crate::services::central_operation::DeleteManifest {
            version: MANIFEST_VERSION,
            operation_id: operation_id.clone(),
            paths: vec![crate::services::central_operation::ManagedPath {
                original: root
                    .join(format!("{skill_id}-original"))
                    .to_string_lossy()
                    .into_owned(),
                backup: root
                    .join(format!("{skill_id}-backup"))
                    .to_string_lossy()
                    .into_owned(),
                marker: root
                    .join(format!("{skill_id}-marker"))
                    .to_string_lossy()
                    .into_owned(),
                expected_present: true,
                fingerprint: None,
            }],
        },
    );
    let manifest_json = serde_json::to_string(&manifest).unwrap();
    db::insert_fs_db_operation(
        pool,
        db::NewFsDbOperation {
            id: &operation_id,
            batch_id: None,
            target_id: "local",
            target_kind: "local",
            operation_kind: "central_delete",
            skill_id,
            manifest_version: MANIFEST_VERSION,
            manifest_json: &manifest_json,
            old_fingerprint: None,
            new_fingerprint: None,
        },
    )
    .await
    .unwrap();
    operation_id
}

#[test]
fn remote_central_skill_persistence_paths_remain_posix_on_windows() {
    let target = std::path::Path::new("/home/tester/.skillsmanage/skills/safe-skill");
    for kind in [
        crate::targets::TargetKind::Ssh,
        crate::targets::TargetKind::Wsl,
    ] {
        let (file_path, canonical_path) = central_skill_persistence_paths(kind, target);
        assert_eq!(
            canonical_path,
            "/home/tester/.skillsmanage/skills/safe-skill"
        );
        assert_eq!(
            file_path,
            "/home/tester/.skillsmanage/skills/safe-skill/SKILL.md"
        );
        assert!(!file_path.contains('\\'));
    }
}

#[tokio::test]
async fn pending_recovery_failure_blocks_only_the_matching_selected_skill() {
    let pool = crate::test_support::mem_pool().await;
    let temp = tempfile::TempDir::new().unwrap();
    let skill_a_target = temp.path().join("skill-a");
    let skill_b_target = temp.path().join("skill-b");
    for target in [&skill_a_target, &skill_b_target] {
        std::fs::create_dir_all(target).unwrap();
        std::fs::write(target.join("SKILL.md"), b"old").unwrap();
    }
    let pending_operation = insert_pending_delete_collision(&pool, temp.path(), "skill-a").await;

    let outcomes = update_skills_batch(
        &pool,
        &CentralFs::Local,
        vec![
            named_update_plan("skill-a", skill_a_target.clone()),
            named_update_plan("skill-b", skill_b_target.clone()),
        ],
        None,
    )
    .await;

    let recovery_error = outcomes[0].result.as_ref().unwrap_err();
    assert_eq!(recovery_error.phase, CentralUpdateFailurePhase::Recovery);
    assert_eq!(
        recovery_error.error().stable_error_code(),
        "central_operation.delete_restore_collision"
    );
    assert_eq!(
        recovery_error.error().diagnostic_category(),
        "central_updates.central_operation"
    );
    assert!(outcomes[1].result.is_ok(), "{:?}", outcomes[1].result);
    assert_eq!(
        std::fs::read(skill_a_target.join("SKILL.md")).unwrap(),
        b"old"
    );
    assert_eq!(
        std::fs::read(skill_b_target.join("SKILL.md")).unwrap(),
        b"new skill-b"
    );
    let pending_row = db::get_fs_db_operation(&pool, &pending_operation)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(pending_row.phase, "prepared");
    assert_eq!(
        pending_row.last_error_code.as_deref(),
        Some("delete_restore_collision")
    );
    let skill_b_phase = sqlx::query_scalar::<_, String>(
        "SELECT phase FROM fs_db_operations WHERE skill_id = 'skill-b'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(skill_b_phase, "completed");
}

#[tokio::test]
async fn unselected_pending_recovery_is_not_retried_or_updated() {
    let pool = crate::test_support::mem_pool().await;
    let temp = tempfile::TempDir::new().unwrap();
    let skill_b_target = temp.path().join("skill-b");
    std::fs::create_dir_all(&skill_b_target).unwrap();
    std::fs::write(skill_b_target.join("SKILL.md"), b"old").unwrap();
    let pending_operation = insert_pending_delete_collision(&pool, temp.path(), "skill-a").await;
    sqlx::query(
        "UPDATE fs_db_operations
         SET updated_at = '2000-01-01T00:00:00Z'
         WHERE id = ?",
    )
    .bind(&pending_operation)
    .execute(&pool)
    .await
    .unwrap();

    let outcomes = update_skills_batch(
        &pool,
        &CentralFs::Local,
        vec![named_update_plan("skill-b", skill_b_target.clone())],
        None,
    )
    .await;

    assert!(outcomes[0].result.is_ok(), "{:?}", outcomes[0].result);
    let pending_row = db::get_fs_db_operation(&pool, &pending_operation)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(pending_row.phase, "prepared");
    assert_eq!(pending_row.updated_at, "2000-01-01T00:00:00Z");
    assert!(pending_row.last_error_code.is_none());
    assert_eq!(
        std::fs::read(skill_b_target.join("SKILL.md")).unwrap(),
        b"new skill-b"
    );
}

#[tokio::test]
async fn selected_recoverable_delete_finishes_before_the_new_update() {
    let pool = crate::test_support::mem_pool().await;
    let temp = tempfile::TempDir::new().unwrap();
    let target = temp.path().join("recover-then-update");
    std::fs::create_dir_all(&target).unwrap();
    std::fs::write(target.join("SKILL.md"), b"old").unwrap();
    let operation_id = "recoverable-delete";
    let manifest = crate::services::central_operation::OperationManifest::Delete(
        crate::services::central_operation::DeleteManifest {
            version: MANIFEST_VERSION,
            operation_id: operation_id.to_string(),
            paths: vec![crate::services::central_operation::ManagedPath {
                original: temp
                    .path()
                    .join("already-missing")
                    .to_string_lossy()
                    .into_owned(),
                backup: temp
                    .path()
                    .join("missing-backup")
                    .to_string_lossy()
                    .into_owned(),
                marker: temp
                    .path()
                    .join("missing-marker")
                    .to_string_lossy()
                    .into_owned(),
                expected_present: false,
                fingerprint: None,
            }],
        },
    );
    let manifest_json = serde_json::to_string(&manifest).unwrap();
    db::insert_fs_db_operation(
        &pool,
        db::NewFsDbOperation {
            id: operation_id,
            batch_id: None,
            target_id: "local",
            target_kind: "local",
            operation_kind: "central_delete",
            skill_id: "recover-then-update",
            manifest_version: MANIFEST_VERSION,
            manifest_json: &manifest_json,
            old_fingerprint: None,
            new_fingerprint: None,
        },
    )
    .await
    .unwrap();

    let outcomes = update_skills_batch(
        &pool,
        &CentralFs::Local,
        vec![named_update_plan("recover-then-update", target.clone())],
        None,
    )
    .await;

    assert!(outcomes[0].result.is_ok(), "{:?}", outcomes[0].result);
    assert_eq!(
        db::get_fs_db_operation(&pool, operation_id)
            .await
            .unwrap()
            .unwrap()
            .phase,
        "rolled_back"
    );
    let completed_updates = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM fs_db_operations
         WHERE skill_id = 'recover-then-update'
           AND operation_kind = 'central_update'
           AND phase = 'completed'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(completed_updates, 1);
    assert_eq!(
        std::fs::read(target.join("SKILL.md")).unwrap(),
        b"new recover-then-update"
    );
}

#[tokio::test]
async fn duplicate_skill_requests_keep_first_result_order_and_do_not_double_mutate() {
    let pool = crate::test_support::mem_pool().await;
    let temp = tempfile::TempDir::new().unwrap();
    let target = temp.path().join("duplicate-skill");
    std::fs::create_dir_all(&target).unwrap();
    std::fs::write(target.join("SKILL.md"), b"old").unwrap();

    let outcomes = update_skills_batch(
        &pool,
        &CentralFs::Local,
        vec![
            named_update_plan("duplicate-skill", target.clone()),
            named_update_plan("duplicate-skill", target.clone()),
        ],
        None,
    )
    .await;

    assert!(outcomes[0].result.is_ok());
    assert_eq!(
        outcomes[1].result.as_ref().unwrap_err().phase,
        CentralUpdateFailurePhase::Prepare
    );
    let journal_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM fs_db_operations WHERE skill_id = 'duplicate-skill'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(journal_count, 1);
}

#[tokio::test]
async fn duplicate_skill_request_does_not_duplicate_a_recovery_failure() {
    let pool = crate::test_support::mem_pool().await;
    let temp = tempfile::TempDir::new().unwrap();
    let target = temp.path().join("duplicate-recovery");
    std::fs::create_dir_all(&target).unwrap();
    std::fs::write(target.join("SKILL.md"), b"old").unwrap();
    insert_pending_delete_collision(&pool, temp.path(), "duplicate-recovery").await;

    let outcomes = update_skills_batch(
        &pool,
        &CentralFs::Local,
        vec![
            named_update_plan("duplicate-recovery", target.clone()),
            named_update_plan("duplicate-recovery", target.clone()),
        ],
        None,
    )
    .await;

    assert_eq!(
        outcomes[0].result.as_ref().unwrap_err().phase,
        CentralUpdateFailurePhase::Recovery
    );
    assert_eq!(
        outcomes[1].result.as_ref().unwrap_err().phase,
        CentralUpdateFailurePhase::Prepare
    );
    let journal_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM fs_db_operations WHERE skill_id = 'duplicate-recovery'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(journal_count, 1);
}

#[tokio::test]
async fn duplicate_skill_request_keeps_prepare_failure_on_global_recovery_error() {
    let pool = crate::test_support::mem_pool().await;
    let temp = tempfile::TempDir::new().unwrap();
    let target = temp.path().join("duplicate-global-recovery");
    pool.close().await;

    let outcomes = update_skills_batch(
        &pool,
        &CentralFs::Local,
        vec![
            named_update_plan("duplicate-global-recovery", target.clone()),
            named_update_plan("duplicate-global-recovery", target),
        ],
        None,
    )
    .await;

    assert_eq!(
        outcomes[0].result.as_ref().unwrap_err().phase,
        CentralUpdateFailurePhase::Recovery
    );
    assert_eq!(
        outcomes[1].result.as_ref().unwrap_err().phase,
        CentralUpdateFailurePhase::Prepare
    );
}

#[tokio::test]
async fn db_apply_failure_releases_transaction_before_fs_rollback() {
    let pool = crate::test_support::mem_pool().await;
    let temp = tempfile::TempDir::new().unwrap();
    let target = temp.path().join("db-fail");
    std::fs::create_dir_all(&target).unwrap();
    std::fs::write(target.join("SKILL.md"), b"old").unwrap();
    sqlx::query(
        "CREATE TRIGGER fail_db_apply BEFORE INSERT ON skills
             WHEN NEW.id = 'db-fail'
             BEGIN SELECT RAISE(FAIL, 'forced db apply failure'); END",
    )
    .execute(&pool)
    .await
    .unwrap();

    let outcomes = update_skills_batch(
        &pool,
        &CentralFs::Local,
        vec![update_plan(target.clone())],
        None,
    )
    .await;

    assert_eq!(
        outcomes[0].result.as_ref().unwrap_err().phase,
        CentralUpdateFailurePhase::DatabaseCommit
    );
    assert_eq!(std::fs::read(target.join("SKILL.md")).unwrap(), b"old");
    let phase = sqlx::query_scalar::<_, String>(
        "SELECT phase FROM fs_db_operations WHERE skill_id = 'db-fail'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(phase, "rolled_back");
}

#[tokio::test]
async fn partial_local_stage_is_cleaned_and_journal_rolls_back() {
    let pool = crate::test_support::mem_pool().await;
    let temp = tempfile::TempDir::new().unwrap();
    let target = temp.path().join("partial-stage");
    std::fs::create_dir_all(&target).unwrap();
    std::fs::write(target.join("SKILL.md"), b"old").unwrap();
    let mut plan = update_plan(target.clone());
    plan.skill.id = "partial-stage".to_string();
    plan.skill.uid = "uid-partial-stage".to_string();
    plan.remote.candidate.skill_id = "partial-stage".to_string();
    plan.remote.target_dir = target.clone();
    plan.remote.files = vec![
        RemoteSkillFile {
            repo_path: "skills/partial-stage/SKILL.md".to_string(),
            relative_path: "SKILL.md".to_string(),
            bytes: b"new".to_vec(),
        },
        RemoteSkillFile {
            repo_path: "skills/partial-stage/SKILL.md/child".to_string(),
            relative_path: "SKILL.md/child".to_string(),
            bytes: b"cannot-be-written".to_vec(),
        },
    ];

    let outcomes = update_skills_batch(&pool, &CentralFs::Local, vec![plan], None).await;

    assert_eq!(
        outcomes[0].result.as_ref().unwrap_err().phase,
        CentralUpdateFailurePhase::Stage
    );
    assert_eq!(std::fs::read(target.join("SKILL.md")).unwrap(), b"old");
    let row = sqlx::query_as::<_, db::FsDbOperationRow>(
        "SELECT * FROM fs_db_operations WHERE skill_id = 'partial-stage'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.phase, "rolled_back");
    assert!(std::fs::read_dir(temp.path()).unwrap().all(|entry| !entry
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".skillport-")));
}

#[tokio::test]
async fn cancellation_after_durable_staging_finishes_the_operation() {
    let pool = crate::test_support::mem_pool().await;
    let temp = tempfile::TempDir::new().unwrap();
    let target = temp.path().join("cancel-after-stage");
    std::fs::create_dir_all(&target).unwrap();
    std::fs::write(target.join("SKILL.md"), b"old").unwrap();
    let mut plan = update_plan(target.clone());
    plan.skill.id = "cancel-after-stage".to_string();
    plan.skill.uid = "uid-cancel-after-stage".to_string();
    plan.remote.candidate.skill_id = "cancel-after-stage".to_string();
    plan.remote.target_dir = target.clone();
    plan.remote.files = std::iter::once(RemoteSkillFile {
        repo_path: "skills/cancel-after-stage/SKILL.md".to_string(),
        relative_path: "SKILL.md".to_string(),
        bytes: b"new".to_vec(),
    })
    .chain((0..128).map(|index| RemoteSkillFile {
        repo_path: format!("skills/cancel-after-stage/data/{index}.bin"),
        relative_path: format!("data/{index}.bin"),
        bytes: vec![index as u8; 64 * 1024],
    }))
    .collect();

    let cancel = std::sync::Arc::new(AtomicBool::new(false));
    let watcher_cancel = cancel.clone();
    let watcher_root = temp.path().to_path_buf();
    let watcher = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while std::time::Instant::now() < deadline {
            let marker_exists = std::fs::read_dir(&watcher_root)
                .into_iter()
                .flatten()
                .flatten()
                .any(|entry| {
                    entry
                        .file_name()
                        .to_string_lossy()
                        .starts_with(".skillport-operation-marker-")
                });
            if marker_exists {
                watcher_cancel.store(true, Ordering::SeqCst);
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        panic!("durable staging marker was not observed");
    });

    let outcomes =
        update_skills_batch(&pool, &CentralFs::Local, vec![plan], Some(cancel.as_ref())).await;
    watcher.join().unwrap();

    assert!(cancel.load(Ordering::SeqCst));
    assert!(outcomes[0].result.is_ok(), "{:?}", outcomes[0].result);
    assert_eq!(std::fs::read(target.join("SKILL.md")).unwrap(), b"new");
    let phase = sqlx::query_scalar::<_, String>(
        "SELECT phase FROM fs_db_operations WHERE skill_id = 'cancel-after-stage'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(phase, "completed");
}

#[tokio::test]
async fn journal_error_marker_failure_is_never_best_effort() {
    let pool = crate::test_support::mem_pool().await;
    let temp = tempfile::TempDir::new().unwrap();
    let operation_id = "journal-marker-fail";
    let manifest = crate::services::central_operation::UpdateManifest {
        version: crate::services::central_operation::MANIFEST_VERSION,
        operation_id: operation_id.to_string(),
        target: temp.path().join("target").to_string_lossy().into_owned(),
        staging: temp.path().join("staging").to_string_lossy().into_owned(),
        backup: temp.path().join("backup").to_string_lossy().into_owned(),
        marker: temp.path().join("marker").to_string_lossy().into_owned(),
        had_target: false,
        old_fingerprint: None,
        new_fingerprint: "sha256-manifest:test".to_string(),
        copies: Vec::new(),
    };
    let manifest_json = serde_json::to_string(
        &crate::services::central_operation::OperationManifest::Update(manifest.clone()),
    )
    .unwrap();
    db::insert_fs_db_operation(
        &pool,
        db::NewFsDbOperation {
            id: operation_id,
            batch_id: None,
            target_id: "local",
            target_kind: "local",
            operation_kind: "central_update",
            skill_id: "skill-a",
            manifest_version: crate::services::central_operation::MANIFEST_VERSION,
            manifest_json: &manifest_json,
            old_fingerprint: None,
            new_fingerprint: Some(&manifest.new_fingerprint),
        },
    )
    .await
    .unwrap();
    sqlx::query(
        "CREATE TRIGGER fail_journal_error_marker BEFORE UPDATE ON fs_db_operations
             WHEN NEW.last_error_code IS NOT NULL
             BEGIN SELECT RAISE(FAIL, 'forced journal marker failure'); END",
    )
    .execute(&pool)
    .await
    .unwrap();

    let error = settle_failed_stage(
        &pool,
        &CentralFs::Local,
        operation_id,
        &manifest,
        CentralUpdatesError::Batch("original stage failure".to_string()),
    )
    .await;

    assert!(matches!(error, CentralUpdatesError::Db(_)));
    let row = db::get_fs_db_operation(&pool, operation_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.phase, "prepared");
    assert!(row.last_error_code.is_none());
}
