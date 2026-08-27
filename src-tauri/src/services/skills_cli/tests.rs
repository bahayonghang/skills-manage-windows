//! Table-driven Skills CLI service tests. Node/npx never leave this process:
//! every spawn goes through [`FakeCliRunner`].

use async_trait::async_trait;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tempfile::TempDir;

use super::argv::{
    build_add_global_argv, build_list_global_argv, build_node_version_argv, build_preview_argv,
    build_remove_global_argv, parse_skill_source, resolve_node_launcher_from_dirs,
    resolve_node_program_from_dirs, NodeLauncher, SkillSource, SKILLS_CLI_NPM_SPEC,
};
use super::error::SkillsCliError;
use super::lock::{
    annotate_platform_install_origins_with, classify_local_path_origin, load_cli_lock_ownership,
    LinkOrigin,
};
use super::runner::{
    bulk_transfer_policy, map_runner_error, standard_policy, CliOutput, RunnerRequest,
    SkillsCliRunner,
};
use super::{
    add_global, add_global_with_lock_at, doctor_with_program, install_targets, list_global,
    list_global_at, preview_source_with_launcher, AddGlobalLockRequest, SkillsCliCapability,
    SkillsCliInstallKind, SkillsCliManagedLinkKind, SkillsCliPlacementState,
    SkillsCliSourceTypeBucket, SkillsCliTransport, SKILLS_CLI_AGENT_MAP, SKILLS_CLI_UNSUPPORTED,
};
use crate::db::{self, SkillForAgent};
use crate::ipc_error::{public_message_for_code, IpcError};
use crate::services::central_mutation::{
    acquire_central_mutation_guard_at, acquire_target_mutation_guard,
};
use crate::services::central_updates::inventory::{
    apply_remove_deleted_platform_copies_step, set_leftover_guard_timeout,
    DeletedPlatformCopyRemoval, SkillUpdateApplyResult,
};
use crate::targets::{
    ActiveTarget, ProcessClass, ProcessPolicy, RemoteTargetConfig, RunnerError, RunnerPhase,
    SshAuthMethod, TargetsError, WslTargetConfig,
};
use crate::test_support::{exit_status, mem_pool, mem_pool_with_home, set_agent_dir, symlink_dir};

fn fake_launcher() -> NodeLauncher {
    NodeLauncher {
        program: PathBuf::from(if cfg!(windows) {
            r"C:\Program Files\nodejs\node.exe"
        } else {
            "/usr/bin/node"
        }),
        npx_js: PathBuf::from(if cfg!(windows) {
            r"C:\Program Files\nodejs\node_modules\npm\bin\npx-cli.js"
        } else {
            "/usr/lib/node_modules/npm/bin/npx-cli.js"
        }),
    }
}

fn ok_output(stdout: &str) -> CliOutput {
    CliOutput {
        status_success: true,
        exit_code: Some(0),
        stdout: stdout.as_bytes().to_vec(),
        stderr: Vec::new(),
    }
}

struct RecordedRun {
    args: Vec<String>,
    class: ProcessClass,
    stdout_limit: usize,
    deadline: Duration,
    cancel_observed: bool,
}

struct FakeCliRunner {
    next: Mutex<VecDeque<Result<CliOutput, SkillsCliError>>>,
    recorded: Mutex<Vec<RecordedRun>>,
}

impl FakeCliRunner {
    fn new() -> Self {
        Self {
            next: Mutex::new(VecDeque::new()),
            recorded: Mutex::new(Vec::new()),
        }
    }

    fn push_ok(&self, stdout: &str) {
        self.next.lock().unwrap().push_back(Ok(ok_output(stdout)));
    }

    fn push_err(&self, error: SkillsCliError) {
        self.next.lock().unwrap().push_back(Err(error));
    }

    fn push_output(&self, output: CliOutput) {
        self.next.lock().unwrap().push_back(Ok(output));
    }

    fn recorded(&self) -> Vec<RecordedRun> {
        self.recorded.lock().unwrap().drain(..).collect()
    }
}

#[async_trait]
impl SkillsCliRunner for FakeCliRunner {
    async fn run(&self, request: RunnerRequest<'_>) -> Result<CliOutput, SkillsCliError> {
        let cancel_observed = request
            .cancel
            .map(|flag| flag.load(Ordering::SeqCst))
            .unwrap_or(false);
        self.recorded.lock().unwrap().push(RecordedRun {
            args: request.args.clone(),
            class: request.policy.class,
            stdout_limit: request.policy.stdout_limit,
            deadline: request.policy.deadline,
            cancel_observed,
        });
        if cancel_observed {
            return Err(SkillsCliError::Cancelled);
        }
        self.next
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| panic!("FakeCliRunner has no queued response"))
    }
}

fn ssh_target() -> ActiveTarget {
    ActiveTarget::Ssh(Box::new(RemoteTargetConfig {
        id: "ssh-cli-test".to_string(),
        label: "SSH".to_string(),
        host: "example.invalid".to_string(),
        username: "alice".to_string(),
        port: 22,
        auth_method: SshAuthMethod::Key,
        key_path: "~/.ssh/id_ed25519".to_string(),
        credential_key: None,
        protected_password: None,
        password: None,
        remote_home: "/home/alice".to_string(),
        remote_os: "linux".to_string(),
        symlink_enabled: true,
    }))
}

fn wsl_target() -> ActiveTarget {
    ActiveTarget::Wsl(Box::new(WslTargetConfig {
        id: "wsl-cli-test".to_string(),
        label: "WSL".to_string(),
        distribution: "TestDistro".to_string(),
        remote_home: "/home/alice".to_string(),
        remote_os: "linux".to_string(),
        symlink_enabled: true,
    }))
}

fn assert_npx_prefix(args: &[String], launcher: &NodeLauncher) {
    assert_eq!(args[0], launcher.npx_js.to_string_lossy());
    assert_eq!(args[1], "--yes");
    assert_eq!(args[2], format!("--package={SKILLS_CLI_NPM_SPEC}"));
    assert_eq!(args[3], "--");
    assert_eq!(args[4], "skills");
    let joined = args.join(" ");
    assert!(!joined.contains("npx.cmd"));
    assert!(!args.iter().any(|arg| arg == "--all" || arg == "*"));
    assert!(!args
        .iter()
        .any(|arg| arg.eq_ignore_ascii_case("npx.cmd") || arg.contains("cmd /c")));
}

fn skill_for_agent(dir_path: &Path, link_type: &str) -> SkillForAgent {
    SkillForAgent {
        id: "demo".to_string(),
        row_id: "demo-row".to_string(),
        name: "demo".to_string(),
        description: None,
        file_path: dir_path.join("SKILL.md").to_string_lossy().into_owned(),
        dir_path: dir_path.to_string_lossy().into_owned(),
        link_type: link_type.to_string(),
        install_origin: if link_type == "symlink" {
            "central".to_string()
        } else {
            "standalone".to_string()
        },
        symlink_target: None,
        is_central: false,
        scanned_at: "2026-01-01T00:00:00Z".to_string(),
        installed_at: None,
        created_at: None,
        updated_at: None,
        repository: None,
        source_path: None,
        is_source_unknown: false,
        source_kind: None,
        source_root: None,
        is_read_only: false,
        conflict_group: None,
        conflict_count: 0,
    }
}

#[test]
fn r4_every_builtin_is_mapped_or_explicitly_unsupported() {
    let builtins = db::builtin_agents();
    let mapped: Vec<&str> = SKILLS_CLI_AGENT_MAP.iter().map(|(id, _)| *id).collect();
    let unsupported: Vec<&str> = SKILLS_CLI_UNSUPPORTED.iter().map(|(id, _)| *id).collect();
    for agent in &builtins {
        let in_map = mapped.contains(&agent.id.as_str());
        let in_unsupported = unsupported.contains(&agent.id.as_str());
        assert!(
            in_map ^ in_unsupported,
            "builtin {} must be mapped or unsupported, not both/neither",
            agent.id
        );
    }
    for id in mapped.iter().chain(unsupported.iter()) {
        assert!(
            builtins.iter().any(|agent| agent.id == *id),
            "table id {id} is not a seed builtin"
        );
    }
    assert_eq!(mapped.len() + unsupported.len(), builtins.len());
}

#[test]
fn ac4_argv_prefix_and_forbidden_tokens() {
    let launcher = fake_launcher();
    let source = parse_skill_source("owner/repo").unwrap();
    let add = build_add_global_argv(
        &launcher,
        &source,
        &["demo-skill".to_string()],
        &["cursor".to_string()],
    );
    assert_npx_prefix(&add, &launcher);
    assert!(add.contains(&"-g".to_string()));
    assert!(add.contains(&"-y".to_string()));
    assert!(add.contains(&"-s".to_string()));
    assert!(add.contains(&"-a".to_string()));
    assert!(add.contains(&"demo-skill".to_string()));
    assert!(add.contains(&"cursor".to_string()));

    let list = build_list_global_argv(&launcher);
    assert_npx_prefix(&list, &launcher);
    assert!(list.contains(&"-g".to_string()));

    let preview = build_preview_argv(&launcher, &source);
    assert_npx_prefix(&preview, &launcher);
    assert!(preview.contains(&"--list".to_string()));

    let remove = build_remove_global_argv(&launcher, "demo-skill");
    assert_npx_prefix(&remove, &launcher);
    assert!(remove.contains(&"--global".to_string()));
    assert!(remove.contains(&"-y".to_string()));
    assert!(
        !remove
            .iter()
            .any(|arg| arg == "--force" || arg == "--keep-links"),
        "unverified remove flags must stay out of argv: {remove:?}"
    );
    assert!(
        !add.iter()
            .any(|arg| arg == "--force" || arg == "--keep-links"),
        "unverified add flags must stay out of argv: {add:?}"
    );

    let version = build_node_version_argv(&launcher);
    assert_eq!(version, vec!["--version".to_string()]);
    assert_ne!(
        launcher.program.file_name().and_then(|name| name.to_str()),
        Some("npx.cmd")
    );
}

#[test]
fn ac4_shorthand_with_skill_reaches_argv() {
    let launcher = fake_launcher();
    let source = parse_skill_source("owner/repo@demo-skill").unwrap();
    assert_eq!(source.as_argv_value(), "owner/repo@demo-skill");
    let argv = build_add_global_argv(
        &launcher,
        &source,
        &["demo-skill".to_string()],
        &["cursor".to_string()],
    );
    assert!(argv.contains(&"owner/repo@demo-skill".to_string()));
}

#[test]
fn ac14_forbidden_source_chars_are_rejected() {
    for raw in [
        "own&er/repo",
        "a|b",
        "x^y",
        "a%b",
        "x!y",
        r#"owner/repo"quote"#,
        "owner/repo;rm",
    ] {
        assert!(
            matches!(parse_skill_source(raw), Err(SkillsCliError::SourceInvalid)),
            "accepted forbidden source {raw}"
        );
    }
    assert!(matches!(
        parse_skill_source("owner/repo"),
        Ok(SkillSource::Shorthand { .. })
    ));
}

#[test]
fn ac14_ipc_message_never_contains_stderr() {
    let planted = "SECRET_STDERR_TOKEN npm ERR!";
    for error in [
        SkillsCliError::CliFailed,
        SkillsCliError::CliUnavailable,
        SkillsCliError::SourceInvalid,
        SkillsCliError::PreviewUnparsed,
        SkillsCliError::Cancelled,
    ] {
        let code = error.ipc_code();
        let message = public_message_for_code(code)
            .unwrap_or("The operation failed. See runtime logs for details.");
        let ipc = IpcError::new(code, message, error.retryable());
        assert!(
            !ipc.message.contains(planted),
            "public message leaked planted stderr for {code}"
        );
        assert!(!ipc.message.contains("npm ERR!"));
        assert!(!error.to_string().contains(planted));
    }
    assert_eq!(
        SkillsCliError::CliFailed.ipc_code(),
        "skills_cli.cli_failed"
    );
    let cli_failed = public_message_for_code("skills_cli.cli_failed").unwrap();
    let cli_unavailable = public_message_for_code("skills_cli.cli_unavailable").unwrap();
    assert_ne!(cli_failed, cli_unavailable);
}

/// `io::Write` adapter appending into a shared test log buffer.
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

fn capture_logs() -> (Arc<Mutex<Vec<u8>>>, tracing::subscriber::DefaultGuard) {
    let logs: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let log_buffer = logs.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(move || SharedLogBuffer(log_buffer.clone()))
        .with_ansi(false)
        .compact()
        .finish();
    (logs, tracing::subscriber::set_default(subscriber))
}

#[tokio::test]
async fn ac7_add_nonzero_exit_warns_without_stderr() {
    let (logs, _guard) = capture_logs();
    let runner = FakeCliRunner::new();
    runner.push_output(CliOutput {
        status_success: false,
        exit_code: Some(1),
        stdout: Vec::new(),
        stderr: b"npm ERR! SECRET_STDERR_TOKEN".to_vec(),
    });
    let temp = TempDir::new().unwrap();
    let error = add_global_with_lock_at(AddGlobalLockRequest {
        lock_path: temp.path().join("cli.lock"),
        runner: &runner,
        launcher: &fake_launcher(),
        source: "owner/repo",
        skill_names: vec!["demo".to_string()],
        skillport_agent_ids: vec!["cursor".to_string()],
        cancel: None,
        timeout: Duration::from_secs(2),
    })
    .await
    .unwrap_err();
    assert!(matches!(error, SkillsCliError::CliFailed));
    assert_eq!(error.ipc_code(), "skills_cli.cli_failed");

    let message = public_message_for_code(error.ipc_code())
        .unwrap_or("The operation failed. See runtime logs for details.");
    let ipc = IpcError::new(error.ipc_code(), message, error.retryable());
    assert_eq!(
        ipc.message,
        "The Skills CLI command did not complete successfully."
    );
    assert!(!ipc.message.contains("SECRET_STDERR_TOKEN"));

    let logged = String::from_utf8(logs.lock().unwrap().clone()).unwrap();
    assert!(
        logged.contains("Skills CLI add command failed"),
        "missing add warn: {logged}"
    );
    assert!(
        logged.contains("operation"),
        "missing operation field: {logged}"
    );
    assert!(
        logged.contains("skills_cli.add_global"),
        "missing operation value: {logged}"
    );
    assert!(
        logged.contains("exit_code"),
        "missing exit_code field: {logged}"
    );
    assert!(!logged.contains("SECRET_STDERR_TOKEN"));
    assert!(!logged.contains("npm ERR!"));
}

#[test]
fn ac7b_start_phase_io_maps_to_cli_unavailable_without_source_display() {
    let (logs, _guard) = capture_logs();
    const PLANTED: &str = r"PLANTED_SPAWN_PATH C:\secret\node.exe";
    let mapped = map_runner_error(RunnerError::Io {
        phase: RunnerPhase::Start,
        source: std::io::Error::new(std::io::ErrorKind::NotFound, PLANTED),
    });
    assert!(matches!(mapped, SkillsCliError::CliUnavailable));
    assert_eq!(mapped.ipc_code(), "skills_cli.cli_unavailable");
    let message = public_message_for_code(mapped.ipc_code()).unwrap();
    let ipc = IpcError::new(mapped.ipc_code(), message, mapped.retryable());
    assert_eq!(ipc.message, "The Skills CLI package could not be executed.");
    assert!(!ipc.message.contains(PLANTED));

    let logged = String::from_utf8(logs.lock().unwrap().clone()).unwrap();
    assert!(
        logged.contains("Skills CLI process failed to start"),
        "missing spawn warn: {logged}"
    );
    assert!(logged.contains("phase"), "missing phase field: {logged}");
    assert!(
        logged.contains("io_kind"),
        "missing io_kind field: {logged}"
    );
    assert!(
        !logged.contains("PLANTED_SPAWN_PATH"),
        "source Display leaked: {logged}"
    );
    assert!(!logged.contains(r"C:\secret"));
}

#[tokio::test]
async fn ac1_doctor_spawns_only_node_version() {
    let runner = FakeCliRunner::new();
    runner.push_ok("v26.7.0\n");
    let report = doctor_with_program(&runner, &fake_launcher().program)
        .await
        .unwrap();
    assert_eq!(report.node_version, "v26.7.0");
    assert_eq!(report.npm_spec, SKILLS_CLI_NPM_SPEC);
    let recorded = runner.recorded();
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0].args, vec!["--version".to_string()]);
    assert!(
        !recorded[0]
            .args
            .iter()
            .any(|arg| arg.contains("skills") || arg.contains("--help") || arg.contains("npx")),
        "doctor must not probe the Skills CLI package: {:?}",
        recorded[0].args
    );
}

#[test]
fn ac8_doctor_reports_missing_node_without_path_mutation() {
    let err = resolve_node_launcher_from_dirs(&[]).unwrap_err();
    assert!(matches!(err, SkillsCliError::NodeMissing));
    let err = resolve_node_program_from_dirs(&[]).unwrap_err();
    assert!(matches!(err, SkillsCliError::NodeMissing));
}

#[test]
fn ac8_doctor_reports_missing_npx_js_without_path_mutation() {
    let temp = TempDir::new().unwrap();
    let node_name = if cfg!(windows) { "node.exe" } else { "node" };
    std::fs::write(temp.path().join(node_name), b"").unwrap();
    let search = [temp.path().to_path_buf()];
    let err = resolve_node_launcher_from_dirs(&search).unwrap_err();
    assert!(matches!(err, SkillsCliError::CliUnavailable));
    let program = resolve_node_program_from_dirs(&search).unwrap();
    assert!(program.ends_with(node_name));
}

#[tokio::test]
async fn ac2_doctor_succeeds_when_npx_js_is_missing() {
    let temp = TempDir::new().unwrap();
    let node_name = if cfg!(windows) { "node.exe" } else { "node" };
    std::fs::write(temp.path().join(node_name), b"").unwrap();
    let search = [temp.path().to_path_buf()];
    assert!(matches!(
        resolve_node_launcher_from_dirs(&search).unwrap_err(),
        SkillsCliError::CliUnavailable
    ));
    let program = resolve_node_program_from_dirs(&search).unwrap();
    let runner = FakeCliRunner::new();
    runner.push_ok("v22.20.0\n");
    let report = doctor_with_program(&runner, &program).await.unwrap();
    assert_eq!(report.node_version, "v22.20.0");
    assert_eq!(report.npm_spec, SKILLS_CLI_NPM_SPEC);
    assert_eq!(runner.recorded().len(), 1);
}

#[tokio::test]
async fn ac5c_doctor_remaps_cli_unavailable_spawn_to_node_missing() {
    let runner = FakeCliRunner::new();
    runner.push_err(SkillsCliError::CliUnavailable);
    let err = doctor_with_program(&runner, &fake_launcher().program)
        .await
        .unwrap_err();
    assert!(matches!(err, SkillsCliError::NodeMissing));
    assert_eq!(err.ipc_code(), "skills_cli.node_missing");
}

#[tokio::test]
async fn ac8_doctor_rejects_old_node() {
    let runner = FakeCliRunner::new();
    runner.push_ok("v18.20.0\n");
    let err = doctor_with_program(&runner, &fake_launcher().program)
        .await
        .unwrap_err();
    assert!(matches!(err, SkillsCliError::NodeTooOld { .. }));
    assert_eq!(runner.recorded().len(), 1);
}

#[tokio::test]
async fn ac3_ac5_empty_selection_refuses_before_spawn() {
    let runner = std::sync::Arc::new(FakeCliRunner::new());
    let tx = SkillsCliTransport::for_local_with_runner(runner.clone());
    let empty_skills = add_global(&tx, "owner/repo", vec![], vec!["cursor".to_string()], None)
        .await
        .unwrap_err();
    assert!(matches!(empty_skills, SkillsCliError::SelectionEmpty));

    let empty_platforms = add_global(&tx, "owner/repo", vec!["demo".to_string()], vec![], None)
        .await
        .unwrap_err();
    assert!(matches!(empty_platforms, SkillsCliError::SelectionEmpty));
    assert!(runner.recorded().is_empty());
}

#[tokio::test]
async fn ac3_install_targets_default_detected_enabled_mapped() {
    let pool = mem_pool().await;
    let temp = TempDir::new().unwrap();
    let agents = db::get_all_agents(&pool).await.unwrap();
    for agent in &agents {
        set_agent_dir(
            &pool,
            &agent.id,
            &temp.path().join("missing").join(&agent.id),
        )
        .await;
    }
    let cursor_dir = temp.path().join("cursor").join("skills");
    let amp_dir = temp.path().join("amp").join("skills");
    let ob1_dir = temp.path().join("ob1").join("skills");
    std::fs::create_dir_all(&cursor_dir).unwrap();
    std::fs::create_dir_all(&amp_dir).unwrap();
    std::fs::create_dir_all(&ob1_dir).unwrap();
    set_agent_dir(&pool, "cursor", &cursor_dir).await;
    set_agent_dir(&pool, "amp", &amp_dir).await;
    set_agent_dir(&pool, "ob1", &ob1_dir).await;
    sqlx::query("UPDATE agents SET is_enabled = 1 WHERE id = 'cursor'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE agents SET is_enabled = 0 WHERE id = 'amp'")
        .execute(&pool)
        .await
        .unwrap();

    let targets = install_targets(&SkillsCliTransport::for_local(), &pool)
        .await
        .unwrap();
    let ids: Vec<&str> = targets.iter().map(|target| target.id.as_str()).collect();
    assert!(ids.contains(&"cursor"));
    assert!(ids.contains(&"amp"));
    assert!(
        !ids.contains(&"ob1"),
        "unsupported builtins must stay hidden"
    );
    let cursor = targets.iter().find(|target| target.id == "cursor").unwrap();
    let amp = targets.iter().find(|target| target.id == "amp").unwrap();
    assert!(cursor.default_selected);
    assert!(!amp.default_selected);
    assert_eq!(cursor.cli_agent, "cursor");
}

#[test]
fn ac2_capability_matrix_opens_inventory_reads_on_remote() {
    let local = SkillsCliTransport::for_local();
    for cap in SkillsCliCapability::ALL {
        assert!(
            local.ensure_capability(*cap).is_ok(),
            "local must allow {cap:?}"
        );
    }

    let runner = std::sync::Arc::new(crate::test_support::FakeRunner::new());
    let tx = SkillsCliTransport::for_tests_remote(crate::targets::ConnectedRemoteTarget::Ssh(
        crate::targets::ConnectedSshTarget::for_tests_with_runner(ssh_config(), runner),
    ));
    let open_on_remote = [
        SkillsCliCapability::Doctor,
        SkillsCliCapability::ListGlobal,
        SkillsCliCapability::InstallTargets,
        SkillsCliCapability::ReadSkillMd,
        SkillsCliCapability::ExportInventory,
        SkillsCliCapability::LinkPlatform,
        SkillsCliCapability::UnlinkPlatform,
        SkillsCliCapability::PreviewRemove,
        SkillsCliCapability::RemoveGlobal,
        SkillsCliCapability::LeftoverScan,
    ];
    for cap in open_on_remote {
        assert!(
            tx.ensure_capability(cap).is_ok(),
            "remote must allow {cap:?}"
        );
        assert!(
            SkillsCliTransport::ensure_capability_for_target(&ssh_target(), cap).is_ok(),
            "remote target gate must allow {cap:?} without connecting"
        );
    }
    assert!(!SkillsCliTransport::uses_local_cli_lock(&ssh_target()));
    assert!(!SkillsCliTransport::uses_local_cli_lock(&wsl_target()));
    assert!(SkillsCliTransport::uses_local_cli_lock(
        &ActiveTarget::Local
    ));
    for cap in SkillsCliCapability::ALL
        .iter()
        .copied()
        .filter(|cap| !open_on_remote.contains(cap))
    {
        assert!(
            matches!(
                tx.ensure_capability(cap),
                Err(SkillsCliError::LocalTargetOnly)
            ),
            "remote must reject {cap:?}"
        );
        assert!(
            matches!(
                SkillsCliTransport::ensure_capability_for_target(&ssh_target(), cap),
                Err(SkillsCliError::LocalTargetOnly)
            ),
            "remote target gate must reject {cap:?} without connecting"
        );
        assert_eq!(tx.write_count(), 0, "zero-write after {cap:?}");
    }
}

fn ssh_config() -> RemoteTargetConfig {
    RemoteTargetConfig {
        id: "ssh-cli-test".to_string(),
        label: "SSH".to_string(),
        host: "example.invalid".to_string(),
        username: "alice".to_string(),
        port: 22,
        auth_method: SshAuthMethod::Key,
        key_path: "~/.ssh/id_ed25519".to_string(),
        credential_key: None,
        protected_password: None,
        password: None,
        remote_home: "/mnt/remote-seam-home".to_string(),
        remote_os: "linux".to_string(),
        symlink_enabled: true,
    }
}

#[test]
fn ac11_origin_classifies_cli_link_central_symlink_and_copy() {
    let temp = TempDir::new().unwrap();
    let universal = temp.path().join("universal");
    let canonical = universal.join("demo-skill");
    std::fs::create_dir_all(&canonical).unwrap();
    let lock_path = temp.path().join(".skill-lock.json");
    std::fs::write(&lock_path, r#"{"version":3,"skills":{"demo-skill":{}}}"#).unwrap();
    let ownership = load_cli_lock_ownership(&lock_path).unwrap();

    let cli_link = temp.path().join("platform").join("cli-link");
    std::fs::create_dir_all(cli_link.parent().unwrap()).unwrap();
    symlink_dir(&canonical, &cli_link);

    let central_target = temp.path().join("central").join("demo-skill");
    std::fs::create_dir_all(&central_target).unwrap();
    let central_link = temp.path().join("platform").join("central-link");
    symlink_dir(&central_target, &central_link);

    let copy_dir = temp.path().join("platform").join("copy");
    std::fs::create_dir_all(&copy_dir).unwrap();

    assert_eq!(
        classify_local_path_origin(&cli_link, &universal, &ownership),
        LinkOrigin::SkillsCli
    );
    assert_eq!(
        classify_local_path_origin(&canonical, &universal, &ownership),
        LinkOrigin::SkillsCli
    );
    assert_eq!(
        classify_local_path_origin(&central_link, &universal, &ownership),
        LinkOrigin::Other
    );
    assert_eq!(
        classify_local_path_origin(&copy_dir, &universal, &ownership),
        LinkOrigin::Other
    );

    let mut skills = vec![
        skill_for_agent(&cli_link, "symlink"),
        skill_for_agent(&central_link, "symlink"),
        skill_for_agent(&copy_dir, "copy"),
    ];
    annotate_platform_install_origins_with(&ownership, &universal, &mut skills);
    assert_eq!(skills[0].install_origin, "skills_cli");
    assert_eq!(skills[1].install_origin, "central");
    assert_eq!(skills[2].install_origin, "standalone");
}

#[tokio::test]
async fn ac12_cancel_add_returns_cancelled_and_records_flag() {
    let runner = FakeCliRunner::new();
    runner.push_ok("installed");
    let cancel = AtomicBool::new(true);
    let temp = TempDir::new().unwrap();
    let lock = temp.path().join("cli.lock");
    let err = add_global_with_lock_at(AddGlobalLockRequest {
        lock_path: lock,
        runner: &runner,
        launcher: &fake_launcher(),
        source: "owner/repo",
        skill_names: vec!["demo".to_string()],
        skillport_agent_ids: vec!["cursor".to_string()],
        cancel: Some(&cancel),
        timeout: Duration::from_secs(2),
    })
    .await
    .unwrap_err();
    assert!(matches!(err, SkillsCliError::Cancelled));
    let recorded = runner.recorded();
    assert_eq!(recorded.len(), 1);
    assert!(recorded[0].cancel_observed);
}

#[tokio::test]
async fn ac13_timeout_and_stdout_cap_without_huge_buffers() {
    let _ = ProcessPolicy::for_tests(Duration::from_millis(1), 8, 8);
    let standard = standard_policy();
    assert_eq!(standard.class, ProcessClass::Standard);
    assert_eq!(standard.stdout_limit, 8 * 1024 * 1024);
    let bulk = bulk_transfer_policy();
    assert_eq!(bulk.class, ProcessClass::BulkTransfer);
    assert_eq!(bulk.stdout_limit, 32 * 1024 * 1024);

    let runner = FakeCliRunner::new();
    runner.push_err(SkillsCliError::Timeout(Duration::from_millis(1)));
    let timeout_err = preview_source_with_launcher(&runner, &fake_launcher(), "owner/repo")
        .await
        .unwrap_err();
    assert!(matches!(timeout_err, SkillsCliError::Timeout(_)));

    runner.push_err(SkillsCliError::OutputLimitExceeded { stream: "stdout" });
    let cap_err = preview_source_with_launcher(&runner, &fake_launcher(), "owner/repo")
        .await
        .unwrap_err();
    assert!(matches!(
        cap_err,
        SkillsCliError::OutputLimitExceeded { stream: "stdout" }
    ));

    let recorded = runner.recorded();
    assert_eq!(recorded[0].class, ProcessClass::Standard);
    assert_eq!(recorded[1].class, ProcessClass::Standard);
    assert!(recorded.iter().all(|run| run.args.join(" ").len() < 4096));
}

#[tokio::test]
async fn ac13_add_uses_bulk_transfer_policy() {
    let runner = FakeCliRunner::new();
    runner.push_ok("ok");
    let temp = TempDir::new().unwrap();
    add_global_with_lock_at(AddGlobalLockRequest {
        lock_path: temp.path().join("cli.lock"),
        runner: &runner,
        launcher: &fake_launcher(),
        source: "owner/repo",
        skill_names: vec!["demo".to_string()],
        skillport_agent_ids: vec!["cursor".to_string()],
        cancel: None,
        timeout: Duration::from_secs(2),
    })
    .await
    .unwrap();
    let recorded = runner.recorded();
    assert_eq!(recorded[0].class, ProcessClass::BulkTransfer);
    assert_eq!(recorded[0].stdout_limit, 32 * 1024 * 1024);
    assert!(recorded[0].deadline >= Duration::from_secs(60));
}

#[tokio::test]
async fn ac15_isolated_lock_contends_with_cli_add() {
    let temp = TempDir::new().unwrap();
    let lock = temp.path().join("cli.lock");
    let _holder = acquire_central_mutation_guard_at(
        lock.clone(),
        "hold isolated lock",
        Duration::from_secs(5),
    )
    .await
    .unwrap();
    let runner = FakeCliRunner::new();
    let err = add_global_with_lock_at(AddGlobalLockRequest {
        lock_path: lock,
        runner: &runner,
        launcher: &fake_launcher(),
        source: "owner/repo",
        skill_names: vec!["demo".to_string()],
        skillport_agent_ids: vec!["cursor".to_string()],
        cancel: None,
        timeout: Duration::ZERO,
    })
    .await
    .unwrap_err();
    assert!(matches!(err, SkillsCliError::Busy));
    assert!(runner.recorded().is_empty());
}

#[tokio::test]
async fn ac15_default_lock_holder_blocks_leftover_apply_and_target_guard() {
    let pool = mem_pool().await;
    let temp = TempDir::new().unwrap();
    let cursor_dir = temp.path().join("cursor");
    let leftover = cursor_dir.join("gone-skill");
    std::fs::create_dir_all(&leftover).unwrap();
    set_agent_dir(&pool, "cursor", &cursor_dir).await;

    let _holder = acquire_target_mutation_guard(
        &ActiveTarget::Local,
        "hold default lock",
        Duration::from_secs(5),
    )
    .await
    .unwrap();

    set_leftover_guard_timeout(Some(Duration::ZERO));
    let mut result = SkillUpdateApplyResult::default();
    apply_remove_deleted_platform_copies_step(
        &pool,
        &ActiveTarget::Local,
        vec![DeletedPlatformCopyRemoval {
            agent_id: "cursor".to_string(),
            skill_id: "gone-skill".to_string(),
            paths: vec![leftover.to_string_lossy().into_owned()],
        }],
        &mut result,
        None,
        None,
    )
    .await;
    set_leftover_guard_timeout(None);

    assert_eq!(result.failures[0].phase.as_deref(), Some("mutation_lock"));
    assert!(leftover.exists());

    let guard_err = acquire_target_mutation_guard(
        &ActiveTarget::Local,
        "install_skill contention probe",
        Duration::ZERO,
    )
    .await
    .unwrap_err();
    assert!(
        matches!(
            guard_err,
            crate::services::central_mutation::CentralMutationError::Busy { .. }
        ),
        "{guard_err:?}"
    );
}

#[test]
fn ac4_copy_without_canonical_is_listed_with_agents() {
    let ownership = super::lock::parse_lock_content(
        r#"{"version":3,"skills":{"demo":{"source":"owner/repo","sourceUrl":"https://github.com/owner/repo","sourceType":"github"}}}"#,
    );
    let temp = TempDir::new().unwrap();
    let canonical_root = temp.path().join("universal");
    std::fs::create_dir_all(&canonical_root).unwrap();
    let cursor_dir = temp.path().join("cursor");
    let copy = cursor_dir.join("demo");
    std::fs::create_dir_all(&copy).unwrap();
    let skills = super::inventory::project_global_inventory(
        &ownership,
        &canonical_root,
        &[super::inventory::InventoryPlatform::for_test(
            "cursor", "Cursor", cursor_dir,
        )],
    );
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].install_kind, SkillsCliInstallKind::Copy);
    assert_eq!(skills[0].path.as_deref(), Some(copy.to_str().unwrap()));
    assert_eq!(skills[0].agents, vec!["Cursor"]);
    assert_eq!(
        skills[0].source_type_bucket,
        SkillsCliSourceTypeBucket::Github
    );
}

#[test]
fn ac5_lock_name_without_directories_is_missing_not_empty_inventory() {
    let ownership = super::lock::parse_lock_content(r#"{"version":3,"skills":{"ghost":{}}}"#);
    let temp = TempDir::new().unwrap();
    let canonical_root = temp.path().join("universal");
    std::fs::create_dir_all(&canonical_root).unwrap();
    let skills = super::inventory::project_global_inventory(&ownership, &canonical_root, &[]);
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "ghost");
    assert_eq!(skills[0].install_kind, SkillsCliInstallKind::Missing);
    assert!(skills[0].path.is_none());
    assert!(skills[0].agents.is_empty());
}

#[test]
fn ac6_unknown_source_type_maps_to_unknown_bucket() {
    let ownership = super::lock::parse_lock_content(
        r#"{"version":3,"skills":{"demo":{"sourceType":"not-a-real-type"}}}"#,
    );
    let entry = ownership.entry("demo").unwrap();
    assert_eq!(entry.source_type.as_deref(), Some("not-a-real-type"));
    assert_eq!(
        super::inventory::source_type_bucket(entry.source_type.as_deref()),
        SkillsCliSourceTypeBucket::Unknown
    );
}

#[tokio::test]
async fn ac4_list_global_at_reads_lock_copy_without_spawn() {
    let pool = mem_pool().await;
    let temp = TempDir::new().unwrap();
    let canonical_root = temp.path().join("universal");
    std::fs::create_dir_all(&canonical_root).unwrap();
    let cursor_dir = temp.path().join("cursor-skills");
    let copy = cursor_dir.join("demo");
    std::fs::create_dir_all(&copy).unwrap();
    set_agent_dir(&pool, "cursor", &cursor_dir).await;
    let lock_path = temp.path().join(".skill-lock.json");
    std::fs::write(
        &lock_path,
        r#"{"version":3,"skills":{"demo":{"sourceType":"github"}}}"#,
    )
    .unwrap();
    let snapshot = list_global_at(&pool, &canonical_root, &lock_path)
        .await
        .unwrap();
    assert_eq!(snapshot.skills.len(), 1);
    assert_eq!(snapshot.skills[0].install_kind, SkillsCliInstallKind::Copy);
    assert!(snapshot.skills[0]
        .agents
        .iter()
        .any(|name| name == "Cursor"));
    assert_eq!(snapshot.lock_path, lock_path.to_string_lossy().into_owned());
}

#[test]
fn ac15_resolves_npx_js_from_sibling_npm_layout() {
    let temp = TempDir::new().unwrap();
    let node_dir = temp.path().join("node");
    let npx_js = temp.path().join("npm/node_modules/npm/bin/npx-cli.js");
    std::fs::create_dir_all(npx_js.parent().unwrap()).unwrap();
    std::fs::write(&npx_js, b"").unwrap();
    std::fs::create_dir_all(&node_dir).unwrap();
    let node_bin = node_dir.join(if cfg!(windows) { "node.exe" } else { "node" });
    std::fs::write(&node_bin, b"").unwrap();
    let launcher = resolve_node_launcher_from_dirs(&[node_dir]).unwrap();
    assert!(launcher.npx_js.ends_with("npx-cli.js"));
}

#[test]
fn ac15_missing_npx_js_public_message_omits_candidate_paths() {
    let temp = TempDir::new().unwrap();
    let node_dir = temp.path().join("node");
    std::fs::create_dir_all(&node_dir).unwrap();
    std::fs::write(
        node_dir.join(if cfg!(windows) { "node.exe" } else { "node" }),
        b"",
    )
    .unwrap();
    let err = resolve_node_launcher_from_dirs(&[node_dir]).unwrap_err();
    assert!(matches!(err, SkillsCliError::CliUnavailable));
    let planted = temp.path().to_string_lossy().into_owned();
    let message = public_message_for_code(err.ipc_code())
        .unwrap_or("The operation failed. See runtime logs for details.");
    assert!(!message.contains(&planted), "{message}");
    assert!(!err.to_string().contains(&planted), "{err}");
}

fn remote_tx(runner: std::sync::Arc<crate::test_support::FakeRunner>) -> SkillsCliTransport {
    SkillsCliTransport::for_tests_remote(crate::targets::ConnectedRemoteTarget::Ssh(
        crate::targets::ConnectedSshTarget::for_tests_with_runner(ssh_config(), runner),
    ))
}

#[test]
fn skills_cli_business_logic_does_not_match_active_target() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/services/skills_cli");
    let mut files = Vec::new();
    collect_rs_files(&root, &mut files);
    for path in files {
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        if file_name == "transport.rs" || file_name == "tests.rs" || file_name == "mutate_tests.rs"
        {
            continue;
        }
        let source = std::fs::read_to_string(&path).unwrap();
        let mut skip_test_mod = false;
        let mut brace_depth = 0usize;
        for (index, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            if skip_test_mod {
                let opens = trimmed.bytes().filter(|byte| *byte == b'{').count();
                let closes = trimmed.bytes().filter(|byte| *byte == b'}').count();
                if opens == 0 && closes == 0 {
                    continue;
                }
                brace_depth += opens;
                brace_depth = brace_depth.saturating_sub(closes);
                if brace_depth == 0 {
                    skip_test_mod = false;
                }
                continue;
            }
            if trimmed.starts_with("#[cfg(test)]") {
                skip_test_mod = true;
                brace_depth = 0;
                continue;
            }
            if trimmed.starts_with("//") {
                continue;
            }
            assert!(
                !trimmed.contains("ActiveTarget::Ssh") && !trimmed.contains("ActiveTarget::Wsl"),
                "{}:{} matches ActiveTarget outside transport.rs",
                path.display(),
                index + 1
            );
            assert!(
                !trimmed.contains("\".agents\""),
                "{}:{} hard-codes the Universal agents directory",
                path.display(),
                index + 1
            );
        }
    }
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

#[test]
fn remote_paths_follow_injected_home_not_local_home() {
    let local_home = crate::paths::skills_cli_local_home()
        .to_string_lossy()
        .replace('\\', "/");
    let runner = std::sync::Arc::new(crate::test_support::FakeRunner::new());
    let tx = remote_tx(runner);
    let paths = tx.paths();
    assert!(
        paths.canonical_root().starts_with("/mnt/remote-seam-home"),
        "{}",
        paths.canonical_root()
    );
    assert!(
        paths.lock_path().starts_with("/mnt/remote-seam-home"),
        "{}",
        paths.lock_path()
    );
    assert!(
        !local_home.is_empty() && !paths.canonical_root().contains(&local_home),
        "canonical {} leaked local home {local_home}",
        paths.canonical_root()
    );
    assert!(
        !paths.lock_path().contains(&local_home),
        "lock {} leaked local home {local_home}",
        paths.lock_path()
    );
}

#[tokio::test]
async fn remote_doctor_round_trips_are_constant_and_map_node_errors() {
    for platforms in [&["cursor"][..], &["a", "b", "c", "d", "e", "f"][..]] {
        let runner = std::sync::Arc::new(crate::test_support::FakeRunner::new());
        runner.push_success("XDG=\nHOME=/mnt/remote-seam-home\nNODEV=v22.20.0\n");
        let tx = remote_tx(runner.clone());
        let report = tx.doctor_ignoring_platforms(platforms).await.unwrap();
        assert_eq!(report.node_version, "v22.20.0");
        assert_eq!(report.npm_spec, SKILLS_CLI_NPM_SPEC);
        let calls = runner.calls();
        assert_eq!(calls.len(), 1, "platform count {}", platforms.len());
        let stdin = String::from_utf8_lossy(calls[0].stdin.as_deref().unwrap_or(&[]));
        assert!(!stdin.contains("skills --help"), "{stdin}");
        assert!(stdin.contains("NODEV"), "{stdin}");
    }

    let missing = std::sync::Arc::new(crate::test_support::FakeRunner::new());
    missing.push_success("XDG=\nHOME=/mnt/remote-seam-home\nNODEV=\n");
    let missing_err = remote_tx(missing).doctor().await.unwrap_err();
    assert!(matches!(missing_err, SkillsCliError::NodeMissing));
    assert_eq!(missing_err.ipc_code(), "skills_cli.node_missing");

    let old = std::sync::Arc::new(crate::test_support::FakeRunner::new());
    old.push_success("XDG=\nHOME=/mnt/remote-seam-home\nNODEV=v18.20.0\n");
    let old_err = remote_tx(old).doctor().await.unwrap_err();
    assert!(matches!(old_err, SkillsCliError::NodeTooOld { .. }));
    assert_eq!(old_err.ipc_code(), "skills_cli.node_missing");

    let xdg = std::sync::Arc::new(crate::test_support::FakeRunner::new());
    xdg.push_success("XDG=/var/xdg-state\nHOME=/mnt/remote-seam-home\nNODEV=v22.20.0\n");
    let tx = remote_tx(xdg);
    tx.doctor().await.unwrap();
    assert_eq!(
        tx.paths().lock_path(),
        "/var/xdg-state/skills/.skill-lock.json"
    );
}

#[tokio::test]
async fn remote_doctor_stderr_sentinel_stays_out_of_ipc_and_logs() {
    const SENTINEL: &str = "SKILLPORT_STDERR_SENTINEL_9f3a2c";
    let (logs, _guard) = capture_logs();
    let runner = std::sync::Arc::new(crate::test_support::FakeRunner::new());
    runner.push_output(1, "", SENTINEL);
    let err = remote_tx(runner).doctor().await.unwrap_err();
    assert_eq!(err.ipc_code(), "skills_cli.node_missing");
    let message = public_message_for_code(err.ipc_code()).unwrap();
    let ipc = IpcError::new(err.ipc_code(), message, err.retryable());
    assert!(!ipc.message.contains(SENTINEL));
    assert!(!format!("{err}").contains(SENTINEL));
    assert!(!format!("{err:?}").contains(SENTINEL));
    let serialized = serde_json::to_string(&ipc).unwrap();
    assert!(!serialized.contains(SENTINEL));
    let logged = String::from_utf8(logs.lock().unwrap().clone()).unwrap();
    assert!(!logged.contains(SENTINEL), "{logged}");
}

#[tokio::test]
async fn remote_home_mismatch_warns_without_path_values() {
    let (logs, _guard) = capture_logs();
    let runner = std::sync::Arc::new(crate::test_support::FakeRunner::new());
    runner.push_success("XDG=\nHOME=/other-probed-home\nNODEV=v22.20.0\n");
    remote_tx(runner).doctor().await.unwrap();
    let logged = String::from_utf8(logs.lock().unwrap().clone()).unwrap();
    assert!(
        logged.contains("Skills CLI remote HOME does not match configured remote_home"),
        "{logged}"
    );
    assert!(!logged.contains("/other-probed-home"), "{logged}");
    assert!(!logged.contains("/mnt/remote-seam-home"), "{logged}");
}

async fn four_platform_pool(home: &str) -> crate::db::DbPool {
    let pool = mem_pool_with_home(home).await;
    sqlx::query("DELETE FROM agents WHERE id NOT IN ('cursor', 'amp', 'zed', 'claude-code')")
        .execute(&pool)
        .await
        .unwrap();
    pool
}

async fn agent_dir(pool: &crate::db::DbPool, id: &str) -> String {
    sqlx::query_scalar("SELECT global_skills_dir FROM agents WHERE id = ?")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

fn lock_json(names: &[&str]) -> String {
    let skills = names
        .iter()
        .map(|name| format!("\"{name}\":{{}}"))
        .collect::<Vec<_>>()
        .join(",");
    format!(r#"{{"version":3,"skills":{{{skills}}}}}"#)
}

fn assert_no_cli_spawn(runner: &crate::test_support::FakeRunner) {
    for call in runner.calls().iter() {
        let hay = format!("{} {}", call.program, call.args.join(" "));
        assert!(!hay.contains("npx-cli"), "{hay}");
        assert!(!hay.contains("skills@"), "{hay}");
        assert!(!hay.contains("build_list_global"), "{hay}");
    }
}

async fn remote_list_call_count(skill_count: usize) -> usize {
    let names: Vec<String> = (0..skill_count)
        .map(|index| format!("skill-{index}"))
        .collect();
    let name_refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let runner = std::sync::Arc::new(crate::test_support::FakeRunner::new());
    runner.push_success(&lock_json(&name_refs));
    runner.push_success("");
    let tx = remote_tx(runner.clone());
    let pool = four_platform_pool("/mnt/remote-seam-home").await;
    list_global(&tx, &pool).await.unwrap();
    assert_no_cli_spawn(runner.as_ref());
    let calls = runner.calls();
    let count = calls.len();
    if skill_count > 0 {
        assert_eq!(count, 2, "lock read + one probe_paths");
        let probe_argv = calls[1].args.join(" ");
        let probe_stdin =
            String::from_utf8_lossy(calls[1].stdin.as_deref().unwrap_or(&[])).into_owned();
        assert!(probe_stdin.contains("<<'SKILLPORT_PATHS'"), "{probe_stdin}");
        assert!(probe_argv.contains("sh -s --"), "{probe_argv}");
        for name in &names {
            let skill_path = format!("/mnt/remote-seam-home/.agents/skills/{name}");
            assert!(
                probe_stdin.contains(&skill_path),
                "missing inlined path {skill_path}"
            );
            assert!(
                !probe_argv.contains(&skill_path),
                "path leaked into argv: {probe_argv}"
            );
        }
    }
    drop(calls);
    count
}

#[tokio::test]
async fn remote_list_round_trips_are_constant_and_do_not_spawn_cli() {
    let three = remote_list_call_count(3).await;
    let thirty = remote_list_call_count(30).await;
    assert_eq!(three, thirty);
    assert_eq!(three, 2);
}

#[tokio::test]
async fn remote_missing_and_empty_lock_return_empty_skills_with_paths() {
    let runner = std::sync::Arc::new(crate::test_support::FakeRunner::new());
    runner.push_output(1, "", "");
    let missing = list_global(
        &remote_tx(runner),
        &four_platform_pool("/mnt/remote-seam-home").await,
    )
    .await
    .unwrap();
    assert!(missing.skills.is_empty());
    assert!(missing.canonical_root.starts_with("/mnt/remote-seam-home"));
    assert!(missing.lock_path.starts_with("/mnt/remote-seam-home"));

    let empty = std::sync::Arc::new(crate::test_support::FakeRunner::new());
    empty.push_success(r#"{"version":3,"skills":{}}"#);
    let snapshot = list_global(
        &remote_tx(empty),
        &four_platform_pool("/mnt/remote-seam-home").await,
    )
    .await
    .unwrap();
    assert!(snapshot.skills.is_empty());
    assert!(!snapshot.canonical_root.is_empty());
    assert!(!snapshot.lock_path.is_empty());
}

#[tokio::test]
async fn remote_list_uses_remote_detection_not_local_home() {
    let pool = four_platform_pool("/mnt/remote-seam-home").await;
    let cursor_dir = agent_dir(&pool, "cursor").await;
    let claude_dir = agent_dir(&pool, "claude-code").await;
    let zed_dir = agent_dir(&pool, "zed").await;
    let canonical = "/mnt/remote-seam-home/.agents/skills/demo";
    // cursor/amp share the Universal root; claude-code is a distinct dir so it
    // can be absent on the remote while cursor stays detected.
    let mut probe = String::new();
    probe.push_str(&format!("{canonical}\tdir\t\n"));
    probe.push_str(&format!("{cursor_dir}/demo\tdir\t\n"));
    probe.push_str(&format!("{cursor_dir}\tdir\t\n"));
    probe.push_str(&format!("{claude_dir}\tabsent\t\n"));
    probe.push_str(&format!("{zed_dir}\tdir\t\n"));
    probe.push_str(&format!("{zed_dir}/demo\tabsent\t\n"));
    let runner = std::sync::Arc::new(crate::test_support::FakeRunner::new());
    runner.push_success(&lock_json(&["demo"]));
    runner.push_success(&probe);
    let tx = remote_tx(runner.clone());
    let snapshot = list_global(&tx, &pool).await.unwrap();
    let skill = &snapshot.skills[0];
    let claude = skill
        .placements
        .iter()
        .find(|placement| placement.agent_id == "claude-code")
        .unwrap();
    assert_eq!(claude.state, SkillsCliPlacementState::Unavailable);
    assert_eq!(claude.reason_code.as_deref(), Some("platform_not_detected"));
    let zed = skill
        .placements
        .iter()
        .find(|placement| placement.agent_id == "zed")
        .unwrap();
    assert_ne!(zed.reason_code.as_deref(), Some("platform_not_detected"));
    let local_home = crate::paths::resolve_home_dir()
        .to_string_lossy()
        .replace('\\', "/");
    let stdin = {
        let calls = runner.calls();
        String::from_utf8_lossy(calls[1].stdin.as_deref().unwrap_or(&[])).into_owned()
    };
    assert!(stdin.contains("<<'SKILLPORT_PATHS'"), "{stdin}");
    for line in stdin.lines() {
        if line.starts_with('/') || line.starts_with("/mnt/") {
            assert!(
                line.starts_with("/mnt/remote-seam-home"),
                "path not under remote home: {line}"
            );
            if !local_home.is_empty() {
                assert!(!line.contains(&local_home), "leaked local home in {line}");
            }
        }
    }
    assert_no_cli_spawn(runner.as_ref());
}

#[test]
fn remote_windows_os_uses_junction_kind() {
    let mut config = ssh_config();
    config.remote_os = "windows".to_string();
    let runner = std::sync::Arc::new(crate::test_support::FakeRunner::new());
    let tx = SkillsCliTransport::for_tests_remote(crate::targets::ConnectedRemoteTarget::Ssh(
        crate::targets::ConnectedSshTarget::for_tests_with_runner(config, runner),
    ));
    assert_eq!(
        tx.managed_link_kind(),
        SkillsCliManagedLinkKind::WindowsJunction
    );
    assert_eq!(
        remote_tx(std::sync::Arc::new(crate::test_support::FakeRunner::new())).managed_link_kind(),
        SkillsCliManagedLinkKind::Symlink
    );
}

#[test]
fn remote_unavailable_and_timeout_are_distinct() {
    let timeout = SkillsCliTransport::map_remote_error_for_tests(TargetsError::ProcessTimedOut {
        transport: "ssh",
        class: "standard",
        timeout_ms: 10_000,
    });
    assert_eq!(timeout.ipc_code(), "skills_cli.timeout");
    let unavailable =
        SkillsCliTransport::map_remote_error_for_tests(TargetsError::RemoteCommandFailed {
            status: exit_status(255),
            detail: "permission denied".to_string(),
        });
    assert_eq!(unavailable.ipc_code(), "skills_cli.remote_unavailable");
    let connect = SkillsCliTransport::map_connect_error_for_tests(TargetsError::io(
        "Failed to start ssh",
        std::io::Error::other("connection refused"),
    ));
    assert_eq!(connect.ipc_code(), "skills_cli.remote_unavailable");
    let public = public_message_for_code("skills_cli.remote_unavailable").unwrap();
    assert!(!public.contains("example"));
    assert!(!public.contains('/'));
    assert!(!public.contains("permission denied"));
}

#[tokio::test]
async fn remote_list_timeout_and_stderr_sentinel_stay_out_of_ipc() {
    const SENTINEL: &str = "SKILLPORT_STDERR_SENTINEL_inventory_7e1";
    let timeout_runner = std::sync::Arc::new(crate::test_support::FakeRunner::new());
    timeout_runner.push_timeout();
    let timeout = list_global(
        &remote_tx(timeout_runner),
        &four_platform_pool("/mnt/remote-seam-home").await,
    )
    .await
    .unwrap_err();
    assert_eq!(timeout.ipc_code(), "skills_cli.timeout");

    let (logs, _guard) = capture_logs();
    let runner = std::sync::Arc::new(crate::test_support::FakeRunner::new());
    runner.push_success(&lock_json(&["demo"]));
    runner.push_output(1, "", SENTINEL);
    let err = list_global(
        &remote_tx(runner),
        &four_platform_pool("/mnt/remote-seam-home").await,
    )
    .await
    .unwrap_err();
    assert_eq!(err.ipc_code(), "skills_cli.remote_unavailable");
    let message = public_message_for_code(err.ipc_code()).unwrap();
    let ipc = IpcError::new(err.ipc_code(), message, err.retryable());
    assert!(!ipc.message.contains(SENTINEL));
    assert!(!format!("{err}").contains(SENTINEL));
    assert!(!format!("{err:?}").contains(SENTINEL));
    let serialized = serde_json::to_string(&ipc).unwrap();
    assert!(!serialized.contains(SENTINEL));
    let logged = String::from_utf8(logs.lock().unwrap().clone()).unwrap();
    assert!(!logged.contains(SENTINEL), "{logged}");
}
