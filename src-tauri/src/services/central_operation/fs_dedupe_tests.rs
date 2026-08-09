use super::*;
use crate::targets::{
    ConnectedRemoteTarget, ConnectedSshTarget, ConnectedWslTarget, RemoteTargetConfig,
    SshAuthMethod, WslTargetConfig,
};
use crate::test_support::FakeRunner;
use std::sync::Arc;

fn fake_connections() -> Vec<(Arc<FakeRunner>, ConnectedRemoteTarget)> {
    let ssh_runner = Arc::new(FakeRunner::new());
    let ssh = ConnectedSshTarget::for_tests_with_runner(
        RemoteTargetConfig {
            id: "ssh-dedupe-test".to_string(),
            label: "SSH dedupe test".to_string(),
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
            id: "wsl-dedupe-test".to_string(),
            label: "WSL dedupe test".to_string(),
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
async fn local_delete_manifest_deduplicates_equivalent_paths_before_staging() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("shared-skill");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("SKILL.md"), "before").unwrap();
    let manifest = build_local_delete_manifest(
        "op-dedup-local",
        vec![target.clone(), target.clone(), target.join(".")],
    )
    .await
    .unwrap();
    assert_eq!(manifest.paths.len(), 1);
    stage_delete_local(&manifest).await.unwrap();
    restore_delete_local(&manifest).await.unwrap();
    assert_eq!(
        fs::read_to_string(target.join("SKILL.md")).unwrap(),
        "before"
    );
}

#[tokio::test]
async fn remote_delete_manifest_normalizes_and_deduplicates_before_inspection() {
    let digest = "b".repeat(64);
    for (runner, connection) in fake_connections() {
        runner.push_success("");
        runner.push_success(&digest);
        let manifest = build_remote_delete_manifest(
            &connection,
            "op-dedup-remote",
            vec![
                "/home/tester//skills/./demo".to_string(),
                "/home/tester/skills/demo".to_string(),
            ],
        )
        .await
        .unwrap();
        assert_eq!(manifest.paths.len(), 1);
        assert_eq!(manifest.paths[0].original, "/home/tester/skills/demo");
        assert_eq!(runner.calls().len(), 2);
    }
}
