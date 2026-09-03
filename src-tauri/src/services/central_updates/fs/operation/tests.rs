use super::*;
use crate::db;
use crate::services::central_operation::{OperationKind, OperationManifest};
use crate::services::central_updates::fs::RemoteSkillFile;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const CRASH_HELPER_ENV: &str = "SKILLPORT_OPERATION_CRASH_HELPER";
const CRASH_PHASE_ENV: &str = "SKILLPORT_OPERATION_CRASH_PHASE";
const CRASH_ROOT_ENV: &str = "SKILLPORT_OPERATION_CRASH_ROOT";
const CRASH_READY_ENV: &str = "SKILLPORT_OPERATION_CRASH_READY";

fn write(skill_id: &str, target: PathBuf, content: &[u8]) -> CentralSkillWrite {
    CentralSkillWrite {
        skill_id: skill_id.to_string(),
        target_dir: target,
        files: vec![RemoteSkillFile {
            repo_path: "SKILL.md".to_string(),
            relative_path: "SKILL.md".to_string(),
            bytes: content.to_vec(),
        }],
    }
}

async fn insert_row(
    pool: &db::DbPool,
    operation_id: &str,
    skill_id: &str,
    manifest: &UpdateManifest,
) {
    let json = serde_json::to_string(&OperationManifest::Update(manifest.clone())).unwrap();
    db::insert_fs_db_operation(
        pool,
        db::NewFsDbOperation {
            id: operation_id,
            batch_id: None,
            target_id: "local",
            target_kind: "local",
            operation_kind: OperationKind::CentralUpdate.as_str(),
            skill_id,
            manifest_version: MANIFEST_VERSION,
            manifest_json: &json,
            old_fingerprint: manifest.old_fingerprint.as_deref(),
            new_fingerprint: Some(&manifest.new_fingerprint),
        },
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn central_operation_crash_process_helper() {
    if std::env::var_os(CRASH_HELPER_ENV).is_none() {
        return;
    }
    let phase = std::env::var(CRASH_PHASE_ENV).unwrap();
    let root = PathBuf::from(std::env::var_os(CRASH_ROOT_ENV).unwrap());
    let ready = PathBuf::from(std::env::var_os(CRASH_READY_ENV).unwrap());
    let pool = db::open_database(&root.join("db.sqlite")).await.unwrap();
    let fs = CentralFs::Local;
    let target = root.join("skill-a");
    let operation_id = format!("crash-{phase}");
    let write = write("skill-a", target, b"new-after-crash");
    let manifest = fs
        .build_operation_update_manifest(&operation_id, &write, Vec::new())
        .await
        .unwrap();
    insert_row(&pool, &operation_id, "skill-a", &manifest).await;

    if phase != "prepared" {
        fs.stage_operation_update(&manifest, &write).await.unwrap();
        db::transition_fs_db_operation(&pool, &operation_id, "prepared", "fs_staged")
            .await
            .unwrap();
    }
    if phase == "prepared" || phase == "staged" {
        pause_for_parent(&ready);
    }

    let mut transaction = pool.begin().await.unwrap();
    fs.swap_operation_update(&manifest).await.unwrap();
    db::transition_fs_db_operation_in_transaction(
        &mut transaction,
        &operation_id,
        "fs_staged",
        "fs_swapped",
    )
    .await
    .unwrap();
    if phase == "swapped" {
        pause_for_parent(&ready);
    }
    db::transition_fs_db_operation_in_transaction(
        &mut transaction,
        &operation_id,
        "fs_swapped",
        "db_committed",
    )
    .await
    .unwrap();
    transaction.commit().await.unwrap();
    if phase == "db_commit" {
        pause_for_parent(&ready);
    }

    db::transition_fs_db_operation(&pool, &operation_id, "db_committed", "copies_pending")
        .await
        .unwrap();
    if phase == "copies_pending" {
        pause_for_parent(&ready);
    }
    fs.finalize_operation_update(&manifest).await.unwrap();
    pause_for_parent(&ready);
}

fn pause_for_parent(ready: &Path) -> ! {
    std::fs::write(ready, b"ready").unwrap();
    loop {
        std::thread::sleep(Duration::from_secs(1));
    }
}

#[tokio::test]
async fn subprocess_kill_phase_matrix_converges_to_old_or_new_state() {
    for phase in [
        "prepared",
        "staged",
        "swapped",
        "db_commit",
        "copies_pending",
        "pre_completion",
    ] {
        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path();
        let target = root.join("skill-a");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("SKILL.md"), b"old-before-crash").unwrap();
        let pool = db::open_database(&root.join("db.sqlite")).await.unwrap();
        pool.close().await;
        let ready = root.join("ready");
        let mut child = Command::new(std::env::current_exe().unwrap())
            .arg("--exact")
            .arg("services::central_updates::fs::operation::tests::central_operation_crash_process_helper")
            .arg("--nocapture")
            .env(CRASH_HELPER_ENV, "1")
            .env(CRASH_PHASE_ENV, phase)
            .env(CRASH_ROOT_ENV, root)
            .env(CRASH_READY_ENV, &ready)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        wait_for_ready(&mut child, &ready, phase);
        child.kill().unwrap();
        child.wait().unwrap();

        let pool = db::open_database(&root.join("db.sqlite")).await.unwrap();
        crate::services::central_updates::core::recover_pending_update_operations(
            &pool,
            &CentralFs::Local,
        )
        .await
        .unwrap();
        let row = db::get_fs_db_operation(&pool, &format!("crash-{phase}"))
            .await
            .unwrap()
            .unwrap();
        let expects_new = matches!(phase, "db_commit" | "copies_pending" | "pre_completion");
        assert_eq!(
            row.phase,
            if expects_new {
                "completed"
            } else {
                "rolled_back"
            },
            "phase {phase}"
        );
        assert_eq!(
            std::fs::read(target.join("SKILL.md")).unwrap(),
            if expects_new {
                b"new-after-crash".as_slice()
            } else {
                b"old-before-crash".as_slice()
            },
            "phase {phase}"
        );
        let OperationManifest::Update(manifest) =
            serde_json::from_str::<OperationManifest>(&row.manifest_json).unwrap()
        else {
            panic!("crash row must contain an update manifest");
        };
        for path in [manifest.staging, manifest.backup, manifest.marker] {
            assert!(
                std::fs::symlink_metadata(&path).is_err(),
                "phase {phase} left operation artifact {path}"
            );
        }
        pool.close().await;
    }
}

fn wait_for_ready(child: &mut std::process::Child, ready: &Path, phase: &str) {
    let started = Instant::now();
    while !ready.exists() {
        if let Some(status) = child.try_wait().unwrap() {
            panic!("crash helper exited before phase {phase}: {status}");
        }
        assert!(
            started.elapsed() < Duration::from_secs(15),
            "crash helper did not reach phase {phase}"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
}

#[tokio::test]
async fn local_update_restore_and_finalize_are_idempotent() {
    let temp = tempfile::TempDir::new().unwrap();
    let target = temp.path().join("skill-a");
    std::fs::create_dir_all(&target).unwrap();
    std::fs::write(target.join("SKILL.md"), b"old").unwrap();
    let fs = CentralFs::Local;

    let old_write = write("skill-a", target.clone(), b"new-one");
    let old_manifest = fs
        .build_operation_update_manifest("rollback-op", &old_write, Vec::new())
        .await
        .unwrap();
    fs.stage_operation_update(&old_manifest, &old_write)
        .await
        .unwrap();
    fs.swap_operation_update(&old_manifest).await.unwrap();
    fs.rollback_operation_update(&old_manifest, OperationPhase::FsStaged)
        .await
        .unwrap();
    fs.rollback_operation_update(&old_manifest, OperationPhase::FsStaged)
        .await
        .unwrap();
    assert_eq!(std::fs::read(target.join("SKILL.md")).unwrap(), b"old");

    let new_write = write("skill-a", target.clone(), b"new-two");
    let new_manifest = fs
        .build_operation_update_manifest("finalize-op", &new_write, Vec::new())
        .await
        .unwrap();
    fs.stage_operation_update(&new_manifest, &new_write)
        .await
        .unwrap();
    fs.swap_operation_update(&new_manifest).await.unwrap();
    fs.finalize_operation_update(&new_manifest).await.unwrap();
    fs.finalize_operation_update(&new_manifest).await.unwrap();
    assert_eq!(std::fs::read(target.join("SKILL.md")).unwrap(), b"new-two");
}

#[tokio::test]
async fn tampered_staging_fails_closed_and_keeps_row_artifacts() {
    let temp = tempfile::TempDir::new().unwrap();
    let target = temp.path().join("skill-a");
    std::fs::create_dir_all(&target).unwrap();
    std::fs::write(target.join("SKILL.md"), b"old").unwrap();
    let fs = CentralFs::Local;
    let write = write("skill-a", target.clone(), b"new");
    let manifest = fs
        .build_operation_update_manifest("collision-op", &write, Vec::new())
        .await
        .unwrap();
    let pool = crate::test_support::mem_pool().await;
    insert_row(&pool, "collision-op", "skill-a", &manifest).await;
    db::transition_fs_db_operation(&pool, "collision-op", "prepared", "fs_staged")
        .await
        .unwrap();
    fs.stage_operation_update(&manifest, &write).await.unwrap();
    std::fs::write(
        PathBuf::from(&manifest.staging).join("SKILL.md"),
        b"tampered",
    )
    .unwrap();

    let error = fs.swap_operation_update(&manifest).await.unwrap_err();
    match error {
        crate::services::central_updates::CentralUpdatesError::CentralOperation(
            crate::services::central_operation::CentralOperationError::RecoveryCollision { code },
        ) => assert_eq!(code, "update_swap_collision"),
        other => panic!("expected swap collision, got {other:?}"),
    }
    assert_eq!(std::fs::read(target.join("SKILL.md")).unwrap(), b"old");
    assert!(std::fs::symlink_metadata(&manifest.staging).is_ok());
    assert!(std::fs::symlink_metadata(&manifest.marker).is_ok());
    let row = db::get_fs_db_operation(&pool, "collision-op")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.phase, "fs_staged");
}
