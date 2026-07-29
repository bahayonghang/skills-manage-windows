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
        },
        refresh_copies: false,
    }
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

    assert!(outcomes[0].result.is_err());
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

    assert!(outcomes[0].result.is_err());
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
