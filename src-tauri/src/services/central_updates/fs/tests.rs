use super::batch::{build_skill_batch_archive, parse_batch_rows};
use super::*;
use crate::targets::{
    CommandRunner, ConnectedRemoteTarget, ConnectedSshTarget, ConnectedWslTarget,
    RemoteTargetConfig, RunnerError, SshAuthMethod, WslTargetConfig,
};
use crate::test_support::FakeRunner;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tempfile::TempDir;

fn fake_remote_fs() -> (Arc<FakeRunner>, CentralFs) {
    let runner = Arc::new(FakeRunner::new());
    let target = RemoteTargetConfig {
        id: "test-ssh".to_string(),
        label: "Test SSH".to_string(),
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
    };
    let connection = ConnectedSshTarget::for_tests_with_runner(target, runner.clone());
    (
        runner,
        CentralFs::Remote(Box::new(ConnectedRemoteTarget::Ssh(connection))),
    )
}

fn fake_remote_update_filesystems() -> Vec<(Arc<FakeRunner>, CentralFs)> {
    let (ssh_runner, ssh) = fake_remote_fs();
    let wsl_runner = Arc::new(FakeRunner::new());
    let wsl = ConnectedWslTarget::for_tests_with_runner(
        WslTargetConfig {
            id: "test-wsl".to_string(),
            label: "Test WSL".to_string(),
            distribution: "TestDistro".to_string(),
            remote_home: "/home/tester".to_string(),
            remote_os: "linux".to_string(),
            symlink_enabled: true,
        },
        wsl_runner.clone(),
    );
    vec![
        (ssh_runner, ssh),
        (
            wsl_runner,
            CentralFs::Remote(Box::new(ConnectedRemoteTarget::Wsl(wsl))),
        ),
    ]
}

fn remote_hash_output(root: &str, file_digest: &str) -> String {
    format!("ROOT\t{root}\n{file_digest}\tSKILL.md\nEND\t{root}\n")
}

fn sample_write(index: usize) -> CentralSkillWrite {
    let skill_id = format!("demo-{index}");
    CentralSkillWrite {
        target_dir: PathBuf::from(format!("/home/tester/.skillsmanage/skills/{skill_id}")),
        skill_id,
        files: vec![RemoteSkillFile {
            repo_path: format!("skills/demo-{index}/SKILL.md"),
            relative_path: "SKILL.md".to_string(),
            bytes: format!("---\nname: Demo {index}\n---\n").into_bytes(),
        }],
    }
}

fn successful_rows(start: usize, count: usize) -> String {
    (start..start + count)
        .map(|index| format!("OK\tdemo-{index}\n"))
        .collect()
}

fn operation_stage(index: usize) -> OperationUpdateStage {
    let write = sample_write(index);
    let operation_id = format!("op-{index}");
    let parent = "/home/tester/.skillsmanage/skills";
    let file_digest = format!("{:x}", Sha256::digest(&write.files[0].bytes));
    OperationUpdateStage {
        manifest: crate::services::central_operation::UpdateManifest {
            version: crate::services::central_operation::MANIFEST_VERSION,
            operation_id: operation_id.clone(),
            target: posix_path(&write.target_dir),
            staging: format!("{parent}/.skillport-update-staging-{operation_id}"),
            backup: format!("{parent}/.skillport-update-backup-{operation_id}"),
            marker: format!("{parent}/.skillport-operation-marker-{operation_id}"),
            had_target: false,
            old_fingerprint: None,
            new_fingerprint: hash_entries(vec![("SKILL.md".to_string(), file_digest)]),
            copies: Vec::new(),
        },
        write,
    }
}

fn successful_stage_hashes(stages: &[OperationUpdateStage]) -> String {
    stages
        .iter()
        .map(|stage| {
            let digest = format!("{:x}", Sha256::digest(&stage.write.files[0].bytes));
            remote_hash_output(&stage.manifest.staging, &digest)
        })
        .collect()
}

struct CancellingRunner {
    inner: FakeRunner,
    cancel: Arc<AtomicBool>,
}

#[cfg(windows)]
async fn cleanup_remote_root(fs: &CentralFs, root: &str) {
    let CentralFs::Remote(connection) = fs else {
        unreachable!();
    };
    connection
        .run_script("rm -rf -- \"$1\"", &[root])
        .await
        .unwrap();
}

#[async_trait::async_trait]
impl CommandRunner for CancellingRunner {
    async fn run(
        &self,
        request: crate::targets::ProcessRequest<'_>,
    ) -> Result<std::process::Output, RunnerError> {
        let result = self.inner.run(request).await;
        self.cancel.store(true, Ordering::SeqCst);
        result
    }
}

#[test]
fn hash_entries_is_stable_across_input_order() {
    let one = hex_digest(&Sha256::digest(b"one"));
    let two = hex_digest(&Sha256::digest(b"two"));
    let left = hash_entries(vec![
        ("b.txt".to_string(), two.clone()),
        ("a.txt".to_string(), one.clone()),
    ]);
    let right = hash_entries(vec![("a.txt".to_string(), one), ("b.txt".to_string(), two)]);

    assert_eq!(left, right);
    assert!(left.starts_with("sha256-manifest:"));
}

#[test]
fn local_hash_changes_when_file_content_changes() {
    let temp = TempDir::new().unwrap();
    let skill_dir = temp.path().join("demo");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("SKILL.md"), b"one").unwrap();

    let first = hash_local_directory(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("SKILL.md"), b"two").unwrap();
    let second = hash_local_directory(&skill_dir).unwrap();

    assert_ne!(first, second);
}

#[test]
fn remote_hash_output_matches_local_manifest_rule() {
    let skill_md = hex_digest(&Sha256::digest(b"one"));
    let readme = hex_digest(&Sha256::digest(b"two"));
    let output = format!(
        "ROOT\t/skills/demo\n{skill_md}  ./SKILL.md\n{readme}  ./README.md\nEND\t/skills/demo\n"
    );

    let parsed = parse_remote_hash_output(&output).unwrap();
    let expected = hash_entries(vec![
        ("README.md".to_string(), readme),
        ("SKILL.md".to_string(), skill_md),
    ]);

    assert_eq!(parsed.get("/skills/demo"), Some(&expected));
}

#[test]
fn github_snapshot_hash_matches_local_directory_hash() {
    let snapshot = GitHubRepoSnapshot {
        files: HashMap::from([
            ("skills/demo/SKILL.md".to_string(), b"one".to_vec()),
            ("skills/demo/README.md".to_string(), b"two".to_vec()),
        ]),
    };
    let files = collect_remote_skill_files(&snapshot, "skills/demo").unwrap();
    let remote_hash = hash_remote_files(&snapshot, &files).unwrap();

    let temp = TempDir::new().unwrap();
    let skill_dir = temp.path().join("demo");
    std::fs::create_dir_all(&skill_dir).unwrap();
    std::fs::write(skill_dir.join("SKILL.md"), b"one").unwrap();
    std::fs::write(skill_dir.join("README.md"), b"two").unwrap();

    assert_eq!(remote_hash, hash_local_directory(&skill_dir).unwrap());
}

#[test]
fn github_root_snapshot_hash_matches_complete_local_directory() {
    let snapshot = GitHubRepoSnapshot {
        files: HashMap::from([
            ("SKILL.md".to_string(), b"one".to_vec()),
            ("README.md".to_string(), b"two".to_vec()),
            ("references/guide.md".to_string(), b"three".to_vec()),
            ("scripts/run.py".to_string(), b"four".to_vec()),
        ]),
    };
    let files = collect_remote_skill_files(&snapshot, ".").unwrap();
    let relative_paths = files
        .iter()
        .map(|file| file.relative_path.as_str())
        .collect::<Vec<_>>();

    assert_eq!(
        relative_paths,
        vec![
            "README.md",
            "SKILL.md",
            "references/guide.md",
            "scripts/run.py",
        ]
    );

    let temp = TempDir::new().unwrap();
    let skill_dir = temp.path().join("root-skill");
    std::fs::create_dir_all(skill_dir.join("references")).unwrap();
    std::fs::create_dir_all(skill_dir.join("scripts")).unwrap();
    std::fs::write(skill_dir.join("SKILL.md"), b"one").unwrap();
    std::fs::write(skill_dir.join("README.md"), b"two").unwrap();
    std::fs::write(skill_dir.join("references/guide.md"), b"three").unwrap();
    std::fs::write(skill_dir.join("scripts/run.py"), b"four").unwrap();

    let remote_hash = hash_remote_files(&snapshot, &files).unwrap();
    assert_eq!(remote_hash, hash_local_directory(&skill_dir).unwrap());
}

#[test]
fn collect_remote_skill_files_requires_source_path() {
    let snapshot = GitHubRepoSnapshot {
        files: HashMap::from([(
            "skills/demo/SKILL.md".to_string(),
            b"---\nname: Demo\n---".to_vec(),
        )]),
    };

    let files = collect_remote_skill_files(&snapshot, "skills/demo").unwrap();

    assert_eq!(files.len(), 1);
    assert_eq!(files[0].relative_path, "SKILL.md");
}

#[test]
fn batch_archive_rejects_unsafe_skill_ids_before_execution() {
    let mut write = sample_write(0);
    write.skill_id = "../escape".to_string();

    let error = build_skill_batch_archive(&[write]).unwrap_err();

    assert!(error.to_string().contains("unsafe for a batch update"));
}

#[test]
fn batch_row_parser_preserves_partial_success() {
    let expected = vec!["demo-0".to_string(), "demo-1".to_string()];
    let parsed = parse_batch_rows(
        b"OK\tdemo-0\nERR\tdemo-1\tswap_failed\n",
        &expected,
        "Central write",
    )
    .unwrap();

    assert!(parsed[0].1.is_ok());
    assert!(parsed[1].1.is_err());
}

#[tokio::test]
async fn remote_hash_fallback_bounds_entries_at_thirty_two_mib_plus_one() {
    let (runner, fs) = fake_remote_fs();
    let root = PathBuf::from("/home/tester/.skillsmanage/skills/demo");
    runner.push_output(86, "", "");
    runner.push_success("");
    runner.push_success("SKILL.md\tfile\t\n");
    runner.push_success("content");

    let hashes = fs
        .hash_directories(std::slice::from_ref(&root))
        .await
        .unwrap();
    assert!(hashes.contains_key(&root));

    let calls = runner.calls();
    let read = &calls[3];
    assert!(read.args.last().unwrap().contains("bs=33554433 count=1"));
    assert_eq!(read.policy.stdout_limit, 33_554_433);
}

#[tokio::test]
async fn ssh_and_wsl_fake_runners_cover_update_stage_swap_and_phase_loss_rollback() {
    use crate::services::central_operation::{UpdateManifest, MANIFEST_VERSION};

    let old_file_digest = format!("{:x}", Sha256::digest(b"old"));
    let new_file_digest = format!("{:x}", Sha256::digest(b"new"));
    let old_fingerprint = hash_entries(vec![("SKILL.md".to_string(), old_file_digest.clone())]);
    let new_fingerprint = hash_entries(vec![("SKILL.md".to_string(), new_file_digest.clone())]);
    for (runner, fs) in fake_remote_update_filesystems() {
        let target = "/home/tester/.skillsmanage/skills/demo";
        let staging = "/home/tester/.skillsmanage/skills/.skillport-update-staging-op-demo";
        let backup = "/home/tester/.skillsmanage/skills/.skillport-update-backup-op-demo";
        let marker = "/home/tester/.skillsmanage/skills/.skillport-operation-marker-op-demo";
        let manifest = UpdateManifest {
            version: MANIFEST_VERSION,
            operation_id: "op-demo".to_string(),
            target: target.to_string(),
            staging: staging.to_string(),
            backup: backup.to_string(),
            marker: marker.to_string(),
            had_target: true,
            old_fingerprint: Some(old_fingerprint.clone()),
            new_fingerprint: new_fingerprint.clone(),
            copies: Vec::new(),
        };
        let write = CentralSkillWrite {
            skill_id: "demo".to_string(),
            target_dir: PathBuf::from(target),
            files: vec![RemoteSkillFile {
                repo_path: "SKILL.md".to_string(),
                relative_path: "SKILL.md".to_string(),
                bytes: b"new".to_vec(),
            }],
        };

        runner.push_success("STAGED\n");
        runner.push_success(&remote_hash_output(staging, &new_file_digest));
        runner.push_success(&remote_hash_output(staging, &new_file_digest));
        runner.push_success(&remote_hash_output(target, &old_file_digest));
        runner.push_success("SWAPPED\n");
        runner.push_success("");
        runner.push_success("");
        runner.push_output(1, "", "");
        runner.push_success(&remote_hash_output(target, &new_file_digest));
        runner.push_success(&remote_hash_output(backup, &old_file_digest));
        runner.push_success("ROLLED_BACK\n");

        fs.stage_operation_update(&manifest, &write).await.unwrap();
        fs.swap_operation_update(&manifest).await.unwrap();
        fs.rollback_operation_update(
            &manifest,
            crate::services::central_operation::OperationPhase::FsStaged,
        )
        .await
        .unwrap();
        assert_eq!(runner.calls().len(), 11);
    }
}

#[tokio::test]
async fn ssh_and_wsl_remote_update_finalize_is_idempotent_after_cleanup() {
    use crate::services::central_operation::{UpdateManifest, MANIFEST_VERSION};

    let file_digest = format!("{:x}", Sha256::digest(b"new"));
    let new_fingerprint = hash_entries(vec![("SKILL.md".to_string(), file_digest.clone())]);
    for (runner, fs) in fake_remote_update_filesystems() {
        let target = "/home/tester/.skillsmanage/skills/demo";
        let manifest = UpdateManifest {
            version: MANIFEST_VERSION,
            operation_id: "op-finalize".to_string(),
            target: target.to_string(),
            staging: "/home/tester/.skillsmanage/skills/.skillport-update-staging-op-finalize"
                .to_string(),
            backup: "/home/tester/.skillsmanage/skills/.skillport-update-backup-op-finalize"
                .to_string(),
            marker: "/home/tester/.skillsmanage/skills/.skillport-operation-marker-op-finalize"
                .to_string(),
            had_target: true,
            old_fingerprint: Some(hash_entries(vec![(
                "SKILL.md".to_string(),
                format!("{:x}", Sha256::digest(b"old")),
            )])),
            new_fingerprint: new_fingerprint.clone(),
            copies: Vec::new(),
        };

        for _ in 0..2 {
            runner.push_success(&remote_hash_output(target, &file_digest));
            runner.push_output(1, "", "");
            runner.push_success("FINALIZED\n");
            fs.finalize_operation_update(&manifest).await.unwrap();
        }

        let calls = runner.calls();
        assert_eq!(calls.len(), 6);
        let finalize_calls = calls
            .iter()
            .filter(|call| {
                call.stdin.as_deref().is_some_and(|stdin| {
                    String::from_utf8_lossy(stdin).contains("[ ! -e \"$marker\" ]")
                })
            })
            .count();
        assert_eq!(finalize_calls, 2);
    }
}

#[tokio::test]
async fn ssh_and_wsl_rollback_restores_backup_when_failed_swap_left_target_missing() {
    use crate::services::central_operation::{OperationPhase, UpdateManifest, MANIFEST_VERSION};

    let old_file_digest = format!("{:x}", Sha256::digest(b"old"));
    let old_fingerprint = hash_entries(vec![("SKILL.md".to_string(), old_file_digest.clone())]);
    let new_fingerprint = hash_entries(vec![(
        "SKILL.md".to_string(),
        format!("{:x}", Sha256::digest(b"new")),
    )]);
    for (runner, fs) in fake_remote_update_filesystems() {
        let manifest = UpdateManifest {
            version: MANIFEST_VERSION,
            operation_id: "op-missing-target".to_string(),
            target: "/home/tester/.skillsmanage/skills/demo".to_string(),
            staging: "/home/tester/.skillsmanage/skills/.skillport-update-staging".to_string(),
            backup: "/home/tester/.skillsmanage/skills/.skillport-update-backup".to_string(),
            marker: "/home/tester/.skillsmanage/skills/.skillport-operation-marker".to_string(),
            had_target: true,
            old_fingerprint: Some(old_fingerprint.clone()),
            new_fingerprint: new_fingerprint.clone(),
            copies: Vec::new(),
        };

        runner.push_success("");
        runner.push_output(1, "", "");
        runner.push_success("");
        runner.push_success(&remote_hash_output(&manifest.backup, &old_file_digest));
        runner.push_success("ROLLED_BACK\n");

        fs.rollback_operation_update(&manifest, OperationPhase::FsStaged)
            .await
            .unwrap();
        assert_eq!(runner.calls().len(), 5);
    }
}

#[tokio::test]
async fn remote_batch_writes_use_one_process_per_sixteen_skills() {
    let (runner, fs) = fake_remote_fs();
    runner.push_success(&successful_rows(0, 16));
    runner.push_success(&successful_rows(16, 16));
    runner.push_success(&successful_rows(32, 1));

    let outcomes = fs
        .write_skill_dirs_atomic_cancellable((0..33).map(sample_write).collect(), None)
        .await;

    assert_eq!(outcomes.len(), 33);
    assert!(outcomes.iter().all(|outcome| outcome.result.is_ok()));
    let calls = runner.calls();
    assert_eq!(calls.len(), 3);
    assert!(calls.iter().all(|call| call.stdin.is_some()));
    assert!(calls
        .iter()
        .all(|call| call.policy.class.label() == "bulk_transfer"));
}

#[tokio::test]
async fn remote_durable_staging_uses_one_archive_process_per_sixteen_skills() {
    let (runner, fs) = fake_remote_fs();
    let stages = (0..33).map(operation_stage).collect::<Vec<_>>();
    runner.push_success(&successful_rows(0, 16));
    runner.push_success(&successful_rows(16, 16));
    runner.push_success(&successful_rows(32, 1));
    runner.push_success(&successful_stage_hashes(&stages[..32]));
    runner.push_success(&successful_stage_hashes(&stages[32..]));

    let outcomes = fs.stage_operation_updates(stages, None).await;

    assert_eq!(outcomes.len(), 33);
    assert!(outcomes.iter().all(|outcome| outcome.result.is_ok()));
    let calls = runner.calls();
    assert_eq!(calls.len(), 5);
    assert_eq!(
        calls
            .iter()
            .filter(|call| {
                call.stdin
                    .as_deref()
                    .is_some_and(|bytes| bytes.starts_with(&[0x1f, 0x8b]))
            })
            .count(),
        3,
        "33 durable writes must use ceil(33 / 16) archive uploads"
    );
}

#[tokio::test]
async fn remote_copy_refresh_uses_one_process_per_thirty_two_targets() {
    let (runner, fs) = fake_remote_fs();
    runner.push_success(&successful_rows(0, 32));
    runner.push_success(&successful_rows(32, 32));
    runner.push_success(&successful_rows(64, 1));
    let copies = (0..65)
        .map(|index| CopyRefreshRequest {
            skill_id: format!("demo-{index}"),
            source_dir: PathBuf::from(format!("/home/tester/.skillsmanage/skills/demo-{index}")),
            target: format!("/home/tester/.agents/skills/demo-{index}"),
        })
        .collect();

    let outcomes = fs.refresh_copy_installs_cancellable(copies, None).await;

    assert_eq!(outcomes.len(), 65);
    assert!(outcomes.iter().all(|outcome| outcome.result.is_ok()));
    assert_eq!(runner.calls().len(), 3);
}

#[tokio::test]
async fn remote_batch_write_checks_cancellation_between_chunks() {
    let cancel = Arc::new(AtomicBool::new(false));
    let runner = Arc::new(CancellingRunner {
        inner: FakeRunner::new(),
        cancel: cancel.clone(),
    });
    runner.inner.push_success(&successful_rows(0, 16));
    let target = RemoteTargetConfig {
        id: "test-ssh".to_string(),
        label: "Test SSH".to_string(),
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
    };
    let connection = ConnectedSshTarget::for_tests_with_runner(target, runner.clone());
    let fs = CentralFs::Remote(Box::new(ConnectedRemoteTarget::Ssh(connection)));

    let outcomes = fs
        .write_skill_dirs_atomic_cancellable(
            (0..33).map(sample_write).collect(),
            Some(cancel.as_ref()),
        )
        .await;

    assert_eq!(runner.inner.calls().len(), 1);
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| outcome.result.is_ok())
            .count(),
        16
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| {
                matches!(&outcome.result, Err(CentralUpdatesError::BatchCancelled))
            })
            .count(),
        17
    );
}

#[tokio::test]
#[ignore = "performance benchmark"]
async fn local_ten_skill_batch_benchmark() {
    use std::time::Instant;

    let temp = TempDir::new().unwrap();
    let root = temp.path().join("batch-benchmark");
    let writes = (0..10)
        .map(|index| {
            let skill_id = format!("bench-{index}");
            CentralSkillWrite {
                target_dir: root.join("central").join(&skill_id),
                skill_id,
                files: vec![RemoteSkillFile {
                    repo_path: format!("skills/bench-{index}/SKILL.md"),
                    relative_path: "SKILL.md".to_string(),
                    bytes: format!("---\nname: Bench {index}\n---\n").into_bytes(),
                }],
            }
        })
        .collect::<Vec<_>>();
    let roots = writes
        .iter()
        .map(|write| write.target_dir.clone())
        .collect::<Vec<_>>();
    let copies = writes
        .iter()
        .map(|write| CopyRefreshRequest {
            skill_id: write.skill_id.clone(),
            source_dir: write.target_dir.clone(),
            target: root
                .join("copies")
                .join(&write.skill_id)
                .to_string_lossy()
                .into_owned(),
        })
        .collect::<Vec<_>>();
    let fs = CentralFs::Local;
    let mut samples = Vec::new();
    for _ in 0..5 {
        if root.exists() {
            std::fs::remove_dir_all(&root).unwrap();
        }
        let started = Instant::now();
        fs.hash_directories(&roots).await.unwrap();
        let write_outcomes = fs
            .write_skill_dirs_atomic_cancellable(writes.clone(), None)
            .await;
        assert!(write_outcomes
            .into_iter()
            .all(|outcome| outcome.result.is_ok()));
        let copy_outcomes = fs
            .refresh_copy_installs_cancellable(copies.clone(), None)
            .await;
        assert!(copy_outcomes
            .into_iter()
            .all(|outcome| outcome.result.is_ok()));
        samples.push(started.elapsed().as_millis());
    }
    samples.sort_unstable();
    let p50 = samples[samples.len() / 2];
    println!("LOCAL_10_SKILL batch_ms={samples:?} batch_p50={p50}");
}

#[cfg(windows)]
#[tokio::test]
#[ignore = "requires SKILLPORT_TEST_WSL_DISTRO and writes only under WSL /tmp"]
async fn live_wsl_ten_skill_batch_benchmark() {
    use crate::targets::{open_wsl_target, ConnectedRemoteTarget, WslTargetConfig};
    use std::time::Instant;

    let distribution = std::env::var("SKILLPORT_TEST_WSL_DISTRO")
        .expect("set SKILLPORT_TEST_WSL_DISTRO to an installed distribution");
    let target = WslTargetConfig {
        id: "benchmark-wsl".to_string(),
        label: "Benchmark WSL".to_string(),
        distribution,
        remote_home: "/tmp".to_string(),
        remote_os: "linux".to_string(),
        symlink_enabled: true,
    };
    let connection = open_wsl_target(&target).unwrap();
    let fs = CentralFs::Remote(Box::new(ConnectedRemoteTarget::Wsl(connection)));
    let root = format!("/tmp/skillport-batch-bench-{}", Uuid::new_v4());
    let writes = (0..10)
        .map(|index| {
            let skill_id = format!("bench-{index}");
            CentralSkillWrite {
                target_dir: PathBuf::from(format!("{root}/central/{skill_id}")),
                skill_id,
                files: vec![RemoteSkillFile {
                    repo_path: format!("skills/bench-{index}/SKILL.md"),
                    relative_path: "SKILL.md".to_string(),
                    bytes: format!("---\nname: Bench {index}\n---\n").into_bytes(),
                }],
            }
        })
        .collect::<Vec<_>>();
    let roots = writes
        .iter()
        .map(|write| write.target_dir.clone())
        .collect::<Vec<_>>();
    let copies = writes
        .iter()
        .map(|write| CopyRefreshRequest {
            skill_id: write.skill_id.clone(),
            source_dir: write.target_dir.clone(),
            target: format!("{root}/copies/{}", write.skill_id),
        })
        .collect::<Vec<_>>();

    let mut legacy_samples = Vec::new();
    let mut batch_samples = Vec::new();
    for _ in 0..5 {
        cleanup_remote_root(&fs, &root).await;
        let started = Instant::now();
        fs.hash_directories(&roots).await.unwrap();
        for write in writes.clone() {
            let outcomes = fs
                .write_skill_dirs_atomic_cancellable(vec![write], None)
                .await;
            assert!(outcomes.into_iter().all(|outcome| outcome.result.is_ok()));
        }
        for copy in copies.clone() {
            let outcomes = fs.refresh_copy_installs_cancellable(vec![copy], None).await;
            assert!(outcomes.into_iter().all(|outcome| outcome.result.is_ok()));
        }
        legacy_samples.push(started.elapsed().as_millis());

        cleanup_remote_root(&fs, &root).await;
        let started = Instant::now();
        fs.hash_directories(&roots).await.unwrap();
        let write_outcomes = fs
            .write_skill_dirs_atomic_cancellable(writes.clone(), None)
            .await;
        assert!(write_outcomes
            .into_iter()
            .all(|outcome| outcome.result.is_ok()));
        let copy_outcomes = fs
            .refresh_copy_installs_cancellable(copies.clone(), None)
            .await;
        assert!(copy_outcomes
            .into_iter()
            .all(|outcome| outcome.result.is_ok()));
        batch_samples.push(started.elapsed().as_millis());
    }
    cleanup_remote_root(&fs, &root).await;

    legacy_samples.sort_unstable();
    batch_samples.sort_unstable();
    let legacy_p50 = legacy_samples[legacy_samples.len() / 2];
    let batch_p50 = batch_samples[batch_samples.len() / 2];
    println!(
        "WSL_10_SKILL legacy_ms={legacy_samples:?} batch_ms={batch_samples:?} legacy_p50={legacy_p50} batch_p50={batch_p50}"
    );
    assert!(
        batch_p50 * 10 <= legacy_p50 * 4,
        "batch apply must be at least 60% faster"
    );
}
