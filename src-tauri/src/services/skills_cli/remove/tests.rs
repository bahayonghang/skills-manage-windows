use super::*;
use crate::services::installation::fs_util::create_skills_cli_directory_link;
use crate::test_support::{mem_pool, set_agent_dir};
use tempfile::TempDir;

fn lock_json(name: &str) -> String {
    format!(r#"{{"version":3,"skills":{{"{name}":{{"source":"owner/repo"}}}}}}"#)
}

async fn harness() -> (DbPool, TempDir, PathBuf, PathBuf, PathBuf, PathBuf) {
    let pool = mem_pool().await;
    let temp = TempDir::new().unwrap();
    let canonical_root = temp.path().join("universal");
    let cursor = temp.path().join("cursor");
    let amp = temp.path().join("amp");
    std::fs::create_dir_all(canonical_root.join("demo")).unwrap();
    std::fs::write(canonical_root.join("demo/SKILL.md"), b"owned").unwrap();
    std::fs::create_dir_all(&cursor).unwrap();
    std::fs::create_dir_all(amp.join("demo")).unwrap();
    std::fs::write(amp.join("demo/copy.bin"), b"retain-me").unwrap();
    set_agent_dir(&pool, "cursor", &cursor).await;
    set_agent_dir(&pool, "amp", &amp).await;
    create_skills_cli_directory_link(&canonical_root.join("demo"), &cursor.join("demo")).unwrap();
    let lock_path = temp.path().join(".skill-lock.json");
    std::fs::write(&lock_path, lock_json("demo")).unwrap();
    let recovery = temp.path().join("recovery");
    (
        pool,
        temp,
        canonical_root,
        lock_path,
        recovery,
        amp.join("demo/copy.bin"),
    )
}

#[tokio::test]
async fn preview_has_no_paths_or_argv_and_conflict_blocks() {
    let (pool, temp, canonical_root, lock_path, _, _) = harness().await;
    let plan = preview_remove_global_at(&pool, "demo", &canonical_root, &lock_path)
        .await
        .unwrap();
    let serialized = serde_json::to_string(&plan).unwrap();
    assert!(!serialized.contains("universal"));
    assert!(!serialized.contains(canonical_root.to_string_lossy().as_ref()));
    assert!(!serialized.contains("--keep-links"));
    assert!(!serialized.contains("--force"));
    assert!(!serialized.contains("skills remove"));
    assert!(plan.confirmable);
    assert_eq!(plan.retained_direct_copies[0].agent_id, "amp");

    let zed = temp.path().join("zed");
    std::fs::create_dir_all(&zed).unwrap();
    std::fs::write(zed.join("demo"), b"not-a-dir").unwrap();
    set_agent_dir(&pool, "zed", &zed).await;
    let blocked = preview_remove_global_at(&pool, "demo", &canonical_root, &lock_path)
        .await
        .unwrap();
    assert!(!blocked.confirmable);
    assert_eq!(blocked.conflicts.len(), 1);
    assert_eq!(blocked.conflicts[0].agent_id, "zed");
    let blocked_json = serde_json::to_string(&blocked).unwrap();
    assert!(!blocked_json.contains(zed.to_string_lossy().as_ref()));
    assert!(!blocked_json.contains("--keep-links"));
    assert!(!blocked_json.contains("not-a-dir"));
}

#[tokio::test]
async fn remove_preserves_direct_copy_bytes_and_drops_canonical_and_link() {
    let (pool, _temp, canonical_root, lock_path, recovery, copy) = harness().await;
    let result = remove_global_at(
        &pool,
        "demo",
        None,
        &canonical_root,
        &lock_path,
        Some(_temp.path().join("mutation.lock")),
        recovery.clone(),
        Duration::from_secs(2),
    )
    .await
    .unwrap();
    assert!(result.removed_canonical);
    assert!(result
        .removed_managed_agent_ids
        .contains(&"cursor".to_string()));
    assert!(result
        .retained_direct_copy_agent_ids
        .contains(&"amp".to_string()));
    assert!(!canonical_root.join("demo").exists());
    assert!(!_temp.path().join("cursor/demo").exists());
    assert_eq!(std::fs::read(copy).unwrap(), b"retain-me");
    let lock = std::fs::read_to_string(&lock_path).unwrap();
    assert!(!lock.contains("\"demo\""));
    assert!(!recovery.join("demo.json").exists());
}

#[tokio::test]
async fn conflict_is_zero_write() {
    let (pool, temp, canonical_root, lock_path, recovery, copy) = harness().await;
    let zed = temp.path().join("zed");
    std::fs::create_dir_all(&zed).unwrap();
    std::fs::write(zed.join("demo"), b"not-a-dir").unwrap();
    set_agent_dir(&pool, "zed", &zed).await;
    let before_lock = std::fs::read(&lock_path).unwrap();
    let before_canonical = std::fs::read(canonical_root.join("demo/SKILL.md")).unwrap();
    let err = remove_global_at(
        &pool,
        "demo",
        None,
        &canonical_root,
        &lock_path,
        Some(temp.path().join("mutation.lock")),
        recovery,
        Duration::from_secs(2),
    )
    .await
    .unwrap_err();
    assert!(matches!(err, SkillsCliError::PlacementConflict));
    assert_eq!(std::fs::read(&lock_path).unwrap(), before_lock);
    assert_eq!(
        std::fs::read(canonical_root.join("demo/SKILL.md")).unwrap(),
        before_canonical
    );
    assert_eq!(std::fs::read(copy).unwrap(), b"retain-me");
    assert!(temp.path().join("cursor/demo").exists());
}

#[tokio::test]
async fn prepared_fault_rolls_back() {
    let (pool, temp, canonical_root, lock_path, recovery, _) = harness().await;
    set_remove_fault(Some(RemoveFault::AfterPrepared));
    let err = remove_global_at(
        &pool,
        "demo",
        None,
        &canonical_root,
        &lock_path,
        Some(temp.path().join("mutation.lock")),
        recovery.clone(),
        Duration::from_secs(2),
    )
    .await
    .unwrap_err();
    set_remove_fault(None);
    assert!(matches!(err, SkillsCliError::RecoveryRequired));
    recover_pending_for_skill_at(&recovery, &canonical_root, &lock_path, "demo").unwrap();
    assert!(canonical_root.join("demo").is_dir());
    assert!(std::fs::read_to_string(&lock_path)
        .unwrap()
        .contains("demo"));
}

#[tokio::test]
async fn fingerprint_drift_fail_closed() {
    let (pool, temp, canonical_root, lock_path, recovery, copy) = harness().await;
    set_remove_fault(Some(RemoveFault::FingerprintDrift));
    let err = remove_global_at(
        &pool,
        "demo",
        None,
        &canonical_root,
        &lock_path,
        Some(temp.path().join("mutation.lock")),
        recovery.clone(),
        Duration::from_secs(2),
    )
    .await
    .unwrap_err();
    set_remove_fault(None);
    assert!(matches!(err, SkillsCliError::RecoveryRequired));
    assert_eq!(std::fs::read(copy).unwrap(), b"retain-me");
    recover_pending_for_skill_at(&recovery, &canonical_root, &lock_path, "demo").ok();
}

#[tokio::test]
async fn injected_phase_faults_converge_or_fail_closed() {
    for fault in [
        RemoveFault::AfterCanonicalRename,
        RemoveFault::AfterLinkRemove,
        RemoveFault::BeforeLockReplace,
        RemoveFault::AfterLockReplace,
        RemoveFault::AfterCleanupBackup,
    ] {
        let (pool, temp, canonical_root, lock_path, recovery, copy) = harness().await;
        set_remove_fault(Some(fault));
        let err = remove_global_at(
            &pool,
            "demo",
            None,
            &canonical_root,
            &lock_path,
            Some(temp.path().join("mutation.lock")),
            recovery.clone(),
            Duration::from_secs(2),
        )
        .await
        .unwrap_err();
        set_remove_fault(None);
        assert!(matches!(err, SkillsCliError::RecoveryRequired), "{fault:?}");
        assert_eq!(std::fs::read(&copy).unwrap(), b"retain-me", "{fault:?}");
        recover_pending_for_skill_at(&recovery, &canonical_root, &lock_path, "demo").unwrap();
        let lock = std::fs::read_to_string(&lock_path).unwrap();
        match fault {
            RemoveFault::AfterLockReplace | RemoveFault::AfterCleanupBackup => {
                assert!(!lock.contains("\"demo\""), "{fault:?}");
                assert!(!canonical_root.join("demo").exists(), "{fault:?}");
            }
            _ => {
                assert!(lock.contains("demo"), "{fault:?}");
                assert!(canonical_root.join("demo").is_dir(), "{fault:?}");
                assert!(temp.path().join("cursor/demo").exists(), "{fault:?}");
            }
        }
        assert!(!recovery.join("demo.json").exists(), "{fault:?}");
    }
}
