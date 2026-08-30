//! Remote install and update coverage (AC1–AC10). GitHub/SSH/npm are fakes.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use sha2::{Digest, Sha256};

use super::argv::{
    build_add_global_argv, parse_skill_source, quote_remote_cli_command, SKILLS_CLI_NPM_SPEC,
};
use super::error::SkillsCliError;
use super::list_global;
use super::updates::{
    apply_updates, check_updates, load_update_inventory_for_pool, retry_update_recovery,
    set_apply_fault, verify_update_baseline, ApplyFault, FakeSkillsCliGithub, GithubObserveResult,
    NoopProgress, SkillsCliApplySelection, SkillsCliApplyUpdateRequest, SkillsCliUpdateProgress,
    SkillsCliUpdateStatus, UpdateProgressEmitter,
};
use super::{add_global, preview_source, reveal_skill_folder, SkillsCliTransport};
use crate::ipc_error::public_message_for_code;
use crate::services::github_import::{skill_content_digest_from_file_bytes, GitHubRepoSnapshot};
use crate::targets::{ConnectedRemoteTarget, ConnectedSshTarget, ProcessClass};
use crate::test_support::{mem_pool_with_home, FakeRunner};

const HOME: &str = "/mnt/remote-seam-home";
const SENTINEL: &str = "SENTINEL_TOKEN_SKILLS_CLI_STDERR";
const PLANTED_URL: &str = "https://example.invalid/secret-token-path";
const PLANTED_TOKEN: &str = "ghp_PLANTED_NOT_FOR_REMOTE";
const SHA_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const SHA_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const LAUNCHER: &str = "NODE=/usr/bin/node\nNPX=/usr/lib/node_modules/npm/bin/npx-cli.js\n";

fn ssh_config() -> crate::targets::RemoteTargetConfig {
    crate::targets::RemoteTargetConfig {
        id: "ssh-cli-test".to_string(),
        label: "SSH".to_string(),
        host: "example.invalid".to_string(),
        username: "alice".to_string(),
        port: 22,
        auth_method: crate::targets::SshAuthMethod::Key,
        key_path: "~/.ssh/id_ed25519".to_string(),
        credential_key: None,
        protected_password: None,
        password: None,
        remote_home: HOME.to_string(),
        remote_os: "linux".to_string(),
        symlink_enabled: true,
    }
}

fn remote_tx(runner: Arc<FakeRunner>) -> SkillsCliTransport {
    SkillsCliTransport::for_tests_remote(ConnectedRemoteTarget::Ssh(
        ConnectedSshTarget::for_tests_with_runner(ssh_config(), runner),
    ))
}

fn probe_line(path: &str, kind: &str, target: &str) -> String {
    format!("{path}\t{kind}\t{target}\n")
}

fn lock_github(name: &str) -> String {
    format!(
        r#"{{"version":3,"skills":{{"{name}":{{"sourceUrl":"https://github.com/owner/repo","sourceType":"github"}}}}}}"#
    )
}

fn skill_digest(body: &str) -> String {
    skill_content_digest_from_file_bytes(&[("SKILL.md".to_string(), body.as_bytes().to_vec())])
}

fn file_sha_hex(body: &str) -> String {
    crate::hashing::encode_lower_hex(Sha256::digest(body.as_bytes()).as_ref())
}

fn hash_output(root: &str, body: &str) -> String {
    format!(
        "ROOT\t{root}\n{}\t{}\tSKILL.md\nEND\t{root}\n",
        file_sha_hex(body),
        body.len()
    )
}

fn snapshot_with(path: &str, body: &str) -> GitHubRepoSnapshot {
    let mut files = std::collections::HashMap::new();
    files.insert(path.to_string(), body.as_bytes().to_vec());
    GitHubRepoSnapshot { files }
}

fn recorded_blob(runner: &FakeRunner) -> String {
    let mut blob = String::new();
    for call in runner.calls().iter() {
        blob.push_str(&call.program);
        blob.push('\n');
        for arg in &call.args {
            blob.push_str(arg);
            blob.push('\n');
        }
        if let Some(stdin) = &call.stdin {
            blob.push_str(&String::from_utf8_lossy(stdin));
            blob.push('\n');
        }
        for (key, value) in &call.env {
            blob.push_str(key);
            blob.push('\n');
            if let Some(item) = value {
                blob.push_str(item);
                blob.push('\n');
            }
        }
    }
    blob
}

fn assert_credential_boundary(runner: &FakeRunner) {
    let blob = recorded_blob(runner);
    assert!(!blob.contains(PLANTED_TOKEN), "{blob}");
    assert!(!blob.contains("Authorization: Bearer"), "{blob}");
    assert!(!blob.contains("curl.conf"), "{blob}");
    assert!(!blob.contains("Authorization"), "{blob}");
}

struct RecordingProgress(Mutex<Vec<SkillsCliUpdateProgress>>);

impl UpdateProgressEmitter for RecordingProgress {
    fn emit_update_progress(&self, payload: &SkillsCliUpdateProgress) {
        self.0.lock().unwrap().push(payload.clone());
    }
}

async fn four_platform_pool() -> crate::db::DbPool {
    let pool = mem_pool_with_home(HOME).await;
    sqlx::query("DELETE FROM agents WHERE id NOT IN ('zed', 'claude-code')")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE agents SET is_enabled = 1 WHERE id IN ('zed', 'claude-code')")
        .execute(&pool)
        .await
        .unwrap();
    pool
}

fn canonical(name: &str) -> String {
    format!("{HOME}/.agents/skills/{name}")
}

#[test]
fn ac1_remote_argv_shape_matches_pin() {
    let launcher = super::NodeLauncher {
        program: std::path::PathBuf::from("/usr/bin/node"),
        npx_js: std::path::PathBuf::from("/usr/lib/node_modules/npm/bin/npx-cli.js"),
    };
    let source = parse_skill_source("owner/repo").unwrap();
    let argv = build_add_global_argv(
        &launcher,
        &source,
        &["demo".to_string()],
        &["cursor".to_string()],
    );
    let quoted = quote_remote_cli_command(&launcher.program, &argv);
    assert!(quoted.contains("--yes"));
    assert!(quoted.contains(&format!("--package={SKILLS_CLI_NPM_SPEC}")));
    assert!(argv.iter().any(|item| item == "-g"));
    assert!(argv.iter().any(|item| item == "-y"));
    assert!(argv.iter().any(|item| item == "-a"));
    assert!(argv.iter().any(|item| item == "-s"));
    assert!(quoted.contains("'-g'") || quoted.contains("-g"));
    assert!(!quoted.contains("--all"));
    assert!(!quoted.contains("npx.cmd"));
    assert!(!quoted.contains("cmd /c"));
    assert!(!quoted.contains("--copy"));
    assert_eq!(SKILLS_CLI_NPM_SPEC, "skills");
    assert!(!argv.iter().any(|item| item == "--copy"));
    assert!(!argv.iter().any(|item| item == "*" || item.contains('*')));
}

#[test]
fn ac1_remote_quote_posixifies_windows_separators() {
    let program = std::path::PathBuf::from("/usr/bin/node");
    let argv = vec![
        r"\usr\lib\node_modules\npm\bin\npx-cli.js".to_string(),
        "--yes".to_string(),
        format!("--package={SKILLS_CLI_NPM_SPEC}"),
        "--".to_string(),
        "skills".to_string(),
        "add".to_string(),
        "owner/repo".to_string(),
        "-g".to_string(),
        "-y".to_string(),
        "-a".to_string(),
        "cursor".to_string(),
        "-s".to_string(),
        "demo".to_string(),
    ];
    let quoted = quote_remote_cli_command(&program, &argv);
    assert!(quoted.contains("/usr/bin/node"));
    assert!(quoted.contains("/usr/lib/node_modules/npm/bin/npx-cli.js"));
    assert!(quoted.contains("--yes"));
    assert!(!quoted.contains('\\'));
    assert!(!quoted.contains("npx.cmd"));
    assert!(!quoted.contains("cmd /c"));
    assert!(!quoted.contains("--all"));
}

#[tokio::test]
async fn ac1_remote_preview_quotes_pin_without_npx_cmd() {
    let runner = Arc::new(FakeRunner::new());
    runner.push_success(LAUNCHER);
    runner.push_success("- demo\n");
    let tx = remote_tx(runner.clone());
    let preview = preview_source(&tx, "owner/repo").await.unwrap();
    assert_eq!(preview.skills, vec!["demo".to_string()]);
    let blob = recorded_blob(runner.as_ref());
    assert!(blob.contains("--yes"));
    assert!(blob.contains(SKILLS_CLI_NPM_SPEC));
    assert!(!blob.contains("npx.cmd"));
    assert!(!blob.contains("cmd /c"));
    assert!(!blob.contains("--all"));
    let calls = runner.calls();
    let launcher_stdin =
        String::from_utf8_lossy(calls[0].stdin.as_deref().unwrap_or(&[])).into_owned();
    assert!(
        launcher_stdin.contains("/home/linuxbrew/.linuxbrew/bin"),
        "{launcher_stdin}"
    );
    assert!(
        launcher_stdin.contains("../lib/node_modules/npm/bin/npx-cli.js"),
        "{launcher_stdin}"
    );
    assert!(!launcher_stdin.contains("bash -lc"), "{launcher_stdin}");
    assert!(!launcher_stdin.contains("zsh -lic"), "{launcher_stdin}");
    let preview = calls.last().expect("preview invocation");
    assert_eq!(preview.policy.class, ProcessClass::Standard);
}

#[tokio::test]
async fn ac2_forbidden_source_rejected_before_remote() {
    let runner = Arc::new(FakeRunner::new());
    let tx = remote_tx(runner.clone());
    let error = preview_source(&tx, "own&er/repo").await.unwrap_err();
    assert!(matches!(error, SkillsCliError::SourceInvalid));
    assert_eq!(runner.calls().len(), 0);
    let add_err = add_global(
        &tx,
        "a|b",
        vec!["demo".to_string()],
        vec!["cursor".to_string()],
        None,
    )
    .await
    .unwrap_err();
    assert!(matches!(add_err, SkillsCliError::SourceInvalid));
    assert_eq!(runner.calls().len(), 0);
}

#[tokio::test]
async fn ac3_remote_add_timeout_and_bulk_stdout_cap() {
    let runner = Arc::new(FakeRunner::new());
    runner.push_success(LAUNCHER);
    runner.push_timeout();
    let tx = remote_tx(runner.clone());
    let error = add_global(
        &tx,
        "owner/repo",
        vec!["demo".to_string()],
        vec!["cursor".to_string()],
        None,
    )
    .await
    .unwrap_err();
    assert!(matches!(error, SkillsCliError::Timeout(_)));
    assert_eq!(error.ipc_code(), "skills_cli.timeout");
    let calls = runner.calls();
    let add = calls.last().expect("add invocation");
    assert_eq!(add.policy.class, ProcessClass::BulkTransfer);
    assert_eq!(add.policy.stdout_limit, 32 * 1024 * 1024);
    assert_eq!(add.policy.deadline, Duration::from_secs(15 * 60));
}

#[tokio::test]
async fn ac4_remote_add_nonzero_is_cli_failed_zero_write() {
    let pool = four_platform_pool().await;
    let runner = Arc::new(FakeRunner::new());
    runner.push_success(LAUNCHER);
    runner.push_output(1, "", SENTINEL);
    let tx = remote_tx(runner.clone());
    let error = add_global(
        &tx,
        "owner/repo",
        vec!["demo".to_string()],
        vec!["cursor".to_string()],
        None,
    )
    .await
    .unwrap_err();
    assert!(matches!(error, SkillsCliError::CliFailed));
    assert_eq!(error.ipc_code(), "skills_cli.cli_failed");
    assert_eq!(tx.write_count(), 0);
    let pending: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM skills_cli_update_operations")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(pending, 0);
}

#[tokio::test]
async fn ac5_remote_topology_and_local_modified() {
    let pool = four_platform_pool().await;
    let name = "demo";
    let root = canonical(name);
    let old = "old";
    let runner = Arc::new(FakeRunner::new());
    runner.push_success(&lock_github(name));
    runner.push_success(&probe_line(&root, "dir", ""));
    runner.push_success(&hash_output(&root, old));
    let tx = remote_tx(runner.clone());
    let github = FakeSkillsCliGithub::new();
    github.set_result(
        "owner/repo@main",
        GithubObserveResult {
            revision_sha: SHA_A.to_string(),
            snapshot: snapshot_with("demo/SKILL.md", old),
            etag: None,
            rate_limit_remaining: Some(50),
            rate_limit_reset_at: None,
        },
    );
    check_updates(&tx, &pool, &github, &NoopProgress, "job-c", None)
        .await
        .unwrap();

    runner.push_success(&lock_github(name));
    runner.push_success(&probe_line(&root, "dir", ""));
    runner.push_success(&hash_output(&root, old));
    verify_update_baseline(&tx, &pool, &[name.to_string()], None)
        .await
        .unwrap();

    runner.push_success(&lock_github(name));
    runner.push_success(&probe_line(&root, "dir", ""));
    runner.push_success(&hash_output(&root, "changed-locally"));
    github.set_result(
        "owner/repo@main",
        GithubObserveResult {
            revision_sha: SHA_A.to_string(),
            snapshot: snapshot_with("demo/SKILL.md", old),
            etag: None,
            rate_limit_remaining: Some(50),
            rate_limit_reset_at: None,
        },
    );
    let modified = check_updates(&tx, &pool, &github, &NoopProgress, "job-mod", None)
        .await
        .unwrap();
    assert!(modified
        .skills
        .iter()
        .any(|row| row.skill_name == name && row.status == SkillsCliUpdateStatus::LocalModified));

    let zed =
        sqlx::query_scalar::<_, String>("SELECT global_skills_dir FROM agents WHERE id = 'zed'")
            .fetch_one(&pool)
            .await
            .unwrap();
    let slot = format!("{zed}/{name}");
    let conflict_runner = Arc::new(FakeRunner::new());
    conflict_runner.push_success(&lock_github(name));
    conflict_runner.push_success(&format!(
        "{}{}",
        probe_line(&root, "dir", ""),
        probe_line(&slot, "dir", "")
    ));
    let conflict_tx = remote_tx(conflict_runner.clone());
    let snapshot = list_global(&conflict_tx, &pool).await.unwrap();
    let skill = snapshot
        .skills
        .iter()
        .find(|skill| skill.name == name)
        .unwrap();
    assert!(skill
        .placements
        .iter()
        .any(|placement| matches!(placement.state, super::SkillsCliPlacementState::DirectCopy)));
    assert!(skill.placements.iter().all(|p| p.install_origin.is_none()));

    conflict_runner.push_success(&lock_github(name));
    conflict_runner.push_success(&format!(
        "{}{}",
        probe_line(&root, "dir", ""),
        probe_line(&slot, "dir", "")
    ));
    let topology_err = apply_updates(
        &conflict_tx,
        &pool,
        &github,
        &NoopProgress,
        &SkillsCliApplyUpdateRequest {
            job_id: "job-topo".into(),
            repository_key: "owner/repo@main".into(),
            selections: vec![SkillsCliApplySelection {
                skill_name: name.into(),
                skill_path: "demo".into(),
                expected_installed_revision: Some(SHA_A.to_string()),
                expected_installed_local_digest: Some("unused".into()),
                expected_pending_revision: SHA_B.to_string(),
                expected_pending_digest: "unused".into(),
            }],
        },
        None,
    )
    .await
    .unwrap_err();
    assert!(matches!(
        topology_err,
        SkillsCliError::UpdateTopologyConflict
    ));
    assert_eq!(
        topology_err.ipc_code(),
        "skills_cli.update_topology_conflict"
    );
}

#[tokio::test]
async fn ac6_ac7_ac8_ac9_remote_apply_journal_and_credentials() {
    let pool = four_platform_pool().await;
    let name = "demo";
    let root = canonical(name);
    let old = "old";
    let new = "new";
    let runner = Arc::new(FakeRunner::new());
    let progress = RecordingProgress(Mutex::new(Vec::new()));
    let github = FakeSkillsCliGithub::new();
    github.plant_auth(PLANTED_TOKEN);
    github.set_result(
        "owner/repo@main",
        GithubObserveResult {
            revision_sha: SHA_A.to_string(),
            snapshot: snapshot_with("demo/SKILL.md", old),
            etag: None,
            rate_limit_remaining: Some(50),
            rate_limit_reset_at: None,
        },
    );
    github.set_sha_snapshot(SHA_B, snapshot_with("demo/SKILL.md", new));

    runner.push_success(&lock_github(name));
    runner.push_success(&probe_line(&root, "dir", ""));
    runner.push_success(&hash_output(&root, old));
    let tx = remote_tx(runner.clone());
    check_updates(&tx, &pool, &github, &progress, "job-check", None)
        .await
        .unwrap();
    runner.push_success(&lock_github(name));
    runner.push_success(&probe_line(&root, "dir", ""));
    runner.push_success(&hash_output(&root, old));
    verify_update_baseline(&tx, &pool, &[name.to_string()], None)
        .await
        .unwrap();

    github.set_result(
        "owner/repo@main",
        GithubObserveResult {
            revision_sha: SHA_B.to_string(),
            snapshot: snapshot_with("demo/SKILL.md", new),
            etag: None,
            rate_limit_remaining: Some(50),
            rate_limit_reset_at: None,
        },
    );
    runner.push_success(&lock_github(name));
    runner.push_success(&probe_line(&root, "dir", ""));
    runner.push_success(&hash_output(&root, old));
    let inventory = check_updates(&tx, &pool, &github, &progress, "job-avail", None)
        .await
        .unwrap();
    let row = inventory
        .skills
        .iter()
        .find(|row| row.skill_name == name)
        .unwrap();
    let pending_digest = row.pending_upstream_digest.clone().unwrap();
    assert_eq!(pending_digest, skill_digest(new));

    let request = SkillsCliApplyUpdateRequest {
        job_id: "job-apply".into(),
        repository_key: "owner/repo@main".into(),
        selections: vec![SkillsCliApplySelection {
            skill_name: name.into(),
            skill_path: "demo".into(),
            expected_installed_revision: row.installed_revision_sha.clone(),
            expected_installed_local_digest: row.installed_local_digest.clone(),
            expected_pending_revision: SHA_B.to_string(),
            expected_pending_digest: pending_digest.clone(),
        }],
    };

    fn queue_list(runner: &FakeRunner, root: &str) {
        runner.push_success(&lock_github("demo"));
        runner.push_success(&probe_line(root, "dir", ""));
    }

    fn queue_apply_until_prepared(runner: &FakeRunner, root: &str, old: &str) {
        queue_list(runner, root);
        queue_list(runner, root);
        runner.push_success(&hash_output(root, old));
        runner.push_success(&lock_github("demo"));
    }

    fn queue_apply_until_cli_started(runner: &FakeRunner, root: &str, old: &str) {
        queue_apply_until_prepared(runner, root, old);
        runner.push_success("");
        runner.push_success("");
    }

    fn queue_apply_until_db_committed(runner: &FakeRunner, root: &str, old: &str, new: &str) {
        queue_apply_until_cli_started(runner, root, old);
        runner.push_success("");
        runner.push_success(&hash_output(root, new));
    }

    set_apply_fault(Some(ApplyFault::Prepared));
    queue_apply_until_prepared(runner.as_ref(), &root, old);
    let prepared_err = apply_updates(&tx, &pool, &github, &progress, &request, None)
        .await
        .unwrap_err();
    set_apply_fault(None);
    assert!(matches!(
        prepared_err,
        SkillsCliError::UpdateRecoveryRequired
    ));
    runner.push_success("");
    let pending = load_update_inventory_for_pool(&pool)
        .await
        .unwrap()
        .pending_recovery
        .expect("prepared journal");
    let recovered = retry_update_recovery(&tx, &pool, &pending.operation_id, None)
        .await
        .unwrap();
    assert_eq!(recovered.phase, "rolled_back");

    set_apply_fault(Some(ApplyFault::CliStarted));
    queue_apply_until_cli_started(runner.as_ref(), &root, old);
    let started_err = apply_updates(&tx, &pool, &github, &progress, &request, None)
        .await
        .unwrap_err();
    set_apply_fault(None);
    assert!(matches!(
        started_err,
        SkillsCliError::UpdateRecoveryRequired
    ));
    let pending = load_update_inventory_for_pool(&pool)
        .await
        .unwrap()
        .pending_recovery
        .expect("cli_started journal");
    let backup = format!(
        "{HOME}/.agents/skills/.skillport-update-op-{}/demo",
        pending.operation_id
    );
    runner.push_success(&format!(
        "{}{}",
        hash_output(&root, old),
        hash_output(&backup, old)
    ));
    runner.push_success("");
    runner.push_success("");
    let recovered = retry_update_recovery(&tx, &pool, &pending.operation_id, None)
        .await
        .unwrap();
    assert_eq!(recovered.phase, "rolled_back");

    set_apply_fault(Some(ApplyFault::DbCommitted));
    queue_apply_until_db_committed(runner.as_ref(), &root, old, new);
    let db_err = apply_updates(&tx, &pool, &github, &progress, &request, None)
        .await
        .unwrap_err();
    set_apply_fault(None);
    assert!(matches!(db_err, SkillsCliError::UpdateRecoveryRequired));
    let pending = load_update_inventory_for_pool(&pool)
        .await
        .unwrap()
        .pending_recovery
        .expect("db_committed journal");
    runner.push_success("");
    let recovered = retry_update_recovery(&tx, &pool, &pending.operation_id, None)
        .await
        .unwrap();
    assert_eq!(recovered.phase, "completed");

    let phases: Vec<String> = progress
        .0
        .lock()
        .unwrap()
        .iter()
        .map(|item| item.phase.clone())
        .collect();
    assert!(phases.iter().any(|phase| phase == "prepare"));
    assert!(phases.iter().any(|phase| phase == "completed"));

    let blob = recorded_blob(runner.as_ref());
    assert!(blob.contains("tar -x"));
    let tar_class = {
        let calls = runner.calls();
        calls
            .iter()
            .find(|call| {
                call.policy.class == ProcessClass::BulkTransfer
                    && (call.args.iter().any(|arg| arg.contains("tar -x"))
                        || call.stdin.as_ref().is_some_and(|stdin| {
                            stdin.len() > 262 && stdin.get(257..262) == Some(b"ustar")
                        }))
            })
            .expect("tar stdin bulk-transfer call")
            .policy
            .class
    };
    assert_eq!(tar_class, ProcessClass::BulkTransfer);
    assert_eq!(github.planted_auth().as_deref(), Some(PLANTED_TOKEN));
    assert_credential_boundary(runner.as_ref());
    assert!(!github
        .call_keys()
        .iter()
        .any(|item| item.contains(PLANTED_TOKEN)));

    runner.push_success(&lock_github(name));
    runner.push_success(&probe_line(&root, "dir", ""));
    let snapshot = list_global(&tx, &pool).await.unwrap();
    let skill = snapshot
        .skills
        .iter()
        .find(|skill| skill.name == name)
        .unwrap();
    assert!(skill.placements.iter().all(|p| p.install_origin.is_none()));
}

#[tokio::test]
async fn ac10_stderr_and_url_never_enter_ipc_or_logs() {
    let (logs, _guard) = {
        let logs: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        let log_buffer = logs.clone();
        struct SharedLogBuffer(Arc<Mutex<Vec<u8>>>);
        impl std::io::Write for SharedLogBuffer {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(buf);
                Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let subscriber = tracing_subscriber::fmt()
            .with_writer(move || SharedLogBuffer(log_buffer.clone()))
            .with_ansi(false)
            .compact()
            .finish();
        (logs, tracing::subscriber::set_default(subscriber))
    };
    let runner = Arc::new(FakeRunner::new());
    runner.push_success(LAUNCHER);
    runner.push_output(1, PLANTED_URL, SENTINEL);
    let tx = remote_tx(runner);
    let error = add_global(
        &tx,
        "owner/repo",
        vec!["demo".to_string()],
        vec!["cursor".to_string()],
        None,
    )
    .await
    .unwrap_err();
    let message = public_message_for_code(error.ipc_code()).unwrap();
    assert!(!message.contains(SENTINEL));
    assert!(!message.contains(PLANTED_URL));
    let logged = String::from_utf8(logs.lock().unwrap().clone()).unwrap();
    assert!(logged.contains("target_kind"));
    assert!(logged.contains("ssh") || logged.contains("local"));
    assert!(!logged.contains(SENTINEL));
    assert!(!logged.contains(PLANTED_URL));
}

#[tokio::test]
async fn reveal_stays_locked_on_remote() {
    let runner = Arc::new(FakeRunner::new());
    let tx = remote_tx(runner.clone());
    let error = reveal_skill_folder(&tx, "demo").unwrap_err();
    assert!(matches!(error, SkillsCliError::LocalTargetOnly));
    assert_eq!(error.ipc_code(), "skills_cli.local_target_only");
    assert_eq!(runner.calls().len(), 0);
    assert_eq!(tx.write_count(), 0);
}

#[test]
fn remote_apply_source_never_copies_curl_auth() {
    for source in [
        include_str!("updates/apply/remote.rs"),
        include_str!("remote_scripts.rs"),
        include_str!("transport.rs"),
        include_str!("updates/github.rs"),
        include_str!("argv.rs"),
    ] {
        assert!(!source.contains("Authorization: Bearer"), "{source}");
        assert!(!source.contains("curl.conf"), "{source}");
    }
}
