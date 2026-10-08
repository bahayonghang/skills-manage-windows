use super::*;
use crate::targets::{
    ConnectedRemoteTarget, ConnectedSshTarget, ConnectedWslTarget, RemoteTargetConfig,
    SshAuthMethod, WslTargetConfig,
};
use crate::test_support::FakeRunner;
#[cfg(windows)]
use std::os::windows::fs::FileTypeExt;
use std::sync::Arc;

fn fake_connections() -> Vec<(Arc<FakeRunner>, ConnectedRemoteTarget)> {
    let ssh_runner = Arc::new(FakeRunner::new());
    let ssh = ConnectedSshTarget::for_tests_with_runner(
        RemoteTargetConfig {
            id: "ssh-operation-test".to_string(),
            label: "SSH operation test".to_string(),
            host: "example.invalid".to_string(),
            username: "tester".to_string(),
            port: 22,
            auth_method: SshAuthMethod::Key,
            key_path: "~/.ssh/id_ed25519".to_string(),
            credential_key: None,
            protected_password: None,
            password: None,
            remote_home: "/home/tester".to_string(),
            remote_os: "linux".to_string(),
            symlink_enabled: true,
        },
        ssh_runner.clone(),
    );
    let wsl_runner = Arc::new(FakeRunner::new());
    let wsl = ConnectedWslTarget::for_tests_with_runner(
        WslTargetConfig {
            id: "wsl-operation-test".to_string(),
            label: "WSL operation test".to_string(),
            distribution: "TestDistro".to_string(),
            remote_home: "/home/tester".to_string(),
            remote_os: "linux".to_string(),
            symlink_enabled: true,
        },
        wsl_runner.clone(),
    );
    vec![
        (ssh_runner, ConnectedRemoteTarget::Ssh(ssh)),
        (wsl_runner, ConnectedRemoteTarget::Wsl(wsl)),
    ]
}

#[tokio::test]
async fn local_delete_stage_restore_and_finalize_are_idempotent() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("skill-a");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("SKILL.md"), "before").unwrap();
    let manifest = build_local_delete_manifest("op-a", vec![target.clone()])
        .await
        .unwrap();
    stage_delete_local(&manifest).await.unwrap();
    assert!(!target.exists());
    restore_delete_local(&manifest).await.unwrap();
    assert_eq!(
        fs::read_to_string(target.join("SKILL.md")).unwrap(),
        "before"
    );

    let manifest = build_local_delete_manifest("op-b", vec![target.clone()])
        .await
        .unwrap();
    stage_delete_local(&manifest).await.unwrap();
    finalize_delete_local(&manifest).await.unwrap();
    finalize_delete_local(&manifest).await.unwrap();
    assert!(!target.exists());
}

#[tokio::test]
async fn local_delete_finalize_removes_directory_symlink_without_following_target() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("central-skill");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("SKILL.md"), "keep").unwrap();
    let link = temp.path().join("agent-skill");
    crate::test_support::symlink_dir(&target, &link);

    let manifest = build_local_delete_manifest("op-dir-link", vec![link.clone()])
        .await
        .unwrap();
    assert_eq!(manifest.paths.len(), 1);
    assert!(manifest.paths[0].expected_present);
    assert_eq!(Path::new(&manifest.paths[0].original), link.as_path());

    stage_delete_local(&manifest).await.unwrap();

    let backup = Path::new(&manifest.paths[0].backup);
    let marker = Path::new(&manifest.paths[0].marker);
    assert_eq!(
        fs::symlink_metadata(&link).unwrap_err().kind(),
        std::io::ErrorKind::NotFound
    );
    let backup_type = fs::symlink_metadata(backup).unwrap().file_type();
    assert!(backup_type.is_symlink());
    #[cfg(windows)]
    assert!(backup_type.is_symlink_dir());

    finalize_delete_local(&manifest).await.unwrap();

    assert_eq!(
        fs::symlink_metadata(backup).unwrap_err().kind(),
        std::io::ErrorKind::NotFound
    );
    assert_eq!(
        fs::symlink_metadata(marker).unwrap_err().kind(),
        std::io::ErrorKind::NotFound
    );
    assert!(target.is_dir());
    assert_eq!(fs::read_to_string(target.join("SKILL.md")).unwrap(), "keep");
}

#[tokio::test]
async fn local_delete_restore_preserves_collision_evidence() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("skill-a");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("SKILL.md"), "before").unwrap();
    let manifest = build_local_delete_manifest("op-a", vec![target.clone()])
        .await
        .unwrap();
    stage_delete_local(&manifest).await.unwrap();
    fs::create_dir(&target).unwrap();
    fs::write(target.join("SKILL.md"), "new-user-data").unwrap();
    let error = restore_delete_local(&manifest).await.unwrap_err();
    assert!(matches!(
        error,
        CentralOperationError::RecoveryCollision { .. }
    ));
    assert!(Path::new(&manifest.paths[0].backup).exists());
}

#[tokio::test]
async fn ssh_and_wsl_fake_runners_cover_delete_stage_and_phase_loss_restore() {
    let digest = "a".repeat(64);
    for (runner, connection) in fake_connections() {
        runner.push_success("");
        runner.push_success(&digest);
        runner.push_success("STAGED\n");
        runner.push_success(&digest);
        runner.push_success("RESTORED\n");
        let manifest = build_remote_delete_manifest(
            &connection,
            "remote-op",
            vec!["/home/tester/.skillsmanage/skills/demo".to_string()],
        )
        .await
        .unwrap();
        stage_delete_remote(&connection, &manifest).await.unwrap();
        restore_delete_remote(&connection, &manifest).await.unwrap();
        let calls = runner.calls();
        assert_eq!(calls.len(), 5);
        assert!(calls.iter().any(|call| {
            call.args
                .iter()
                .any(|argument| argument.contains("skillport-delete-backup"))
        }));
    }
}

#[tokio::test]
async fn ssh_and_wsl_delete_finalize_is_idempotent_after_cleanup() {
    let digest = "a".repeat(64);
    for (runner, connection) in fake_connections() {
        let manifest = DeleteManifest {
            version: MANIFEST_VERSION,
            operation_id: "remote-finalize-op".to_string(),
            paths: vec![ManagedPath {
                original: "/home/tester/.skillsmanage/skills/demo".to_string(),
                backup: "/home/tester/.skillsmanage/skills/.skillport-delete-backup".to_string(),
                marker: "/home/tester/.skillsmanage/skills/.skillport-operation.marker".to_string(),
                expected_present: true,
                fingerprint: Some(digest.clone()),
            }],
        };
        for _ in 0..2 {
            runner.push_success("MISSING\n");
            runner.push_success("FINALIZED\n");
            finalize_delete_remote(&connection, &manifest)
                .await
                .unwrap();
        }
        assert_eq!(runner.calls().len(), 4);
    }
}

#[cfg(windows)]
#[tokio::test]
#[ignore = "requires SKILLPORT_TEST_WSL_DISTRO and writes only under WSL /tmp"]
async fn live_wsl_delete_stage_and_restore_smoke() {
    let distribution = std::env::var("SKILLPORT_TEST_WSL_DISTRO")
        .expect("set SKILLPORT_TEST_WSL_DISTRO to an installed distribution");
    let target = WslTargetConfig {
        id: "operation-wsl-smoke".to_string(),
        label: "Operation WSL smoke".to_string(),
        distribution,
        remote_home: "/tmp".to_string(),
        remote_os: "linux".to_string(),
        symlink_enabled: true,
    };
    let connection = ConnectedRemoteTarget::Wsl(
        crate::targets::open_wsl_target(&target).expect("open WSL target"),
    );
    let root = format!("/tmp/skillport-operation-smoke-{}", uuid::Uuid::new_v4());
    let skill = format!("{root}/demo");
    connection
        .run_script(
            "set -eu; mkdir -p -- \"$1\"; printf before > \"$1/SKILL.md\"",
            &[&skill],
        )
        .await
        .unwrap();
    let manifest = build_remote_delete_manifest(&connection, "wsl-smoke-op", vec![skill.clone()])
        .await
        .unwrap();
    stage_delete_remote(&connection, &manifest).await.unwrap();
    restore_delete_remote(&connection, &manifest).await.unwrap();
    assert_eq!(
        connection
            .run_script("cat -- \"$1/SKILL.md\"", &[&skill])
            .await
            .unwrap(),
        "before"
    );
    connection
        .run_script("rm -rf -- \"$1\"", &[&root])
        .await
        .unwrap();
}
