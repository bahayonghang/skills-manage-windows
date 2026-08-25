//! Table-driven Skills CLI service tests. Node/npx never leave this process:
//! every spawn goes through [`FakeCliRunner`].

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use async_trait::async_trait;
use tempfile::TempDir;

use super::argv::{
    build_add_global_argv, build_list_global_argv, build_node_version_argv, build_preview_argv,
    build_remove_global_argv, parse_skill_source, resolve_node_launcher_from_dirs, NodeLauncher,
    SkillSource, SKILLS_CLI_NPM_SPEC,
};
use super::error::SkillsCliError;
use super::lock::{
    annotate_platform_install_origins_with, classify_local_path_origin, load_cli_lock_ownership,
    LinkOrigin,
};
use super::runner::{
    bulk_transfer_policy, standard_policy, CliOutput, RunnerRequest, SkillsCliRunner,
};
use super::{
    add_global, add_global_with_lock_at, doctor_with_launcher, ensure_local_target,
    install_targets, list_global_at, preview_source_with_launcher, AddGlobalLockRequest,
    SkillsCliInstallKind, SkillsCliSourceTypeBucket, SKILLS_CLI_AGENT_MAP, SKILLS_CLI_UNSUPPORTED,
};
use crate::db::{self, SkillForAgent};
use crate::ipc_error::{public_message_for_code, IpcError};
use crate::paths::central_mutation_lock_path;
use crate::services::central_mutation::{
    acquire_central_mutation_guard_at, acquire_target_mutation_guard,
};
use crate::services::central_updates::inventory::{
    apply_remove_deleted_platform_copies_step, set_leftover_guard_timeout,
    DeletedPlatformCopyRemoval, SkillUpdateApplyResult,
};
use crate::targets::{
    ActiveTarget, ProcessClass, ProcessPolicy, RemoteTargetConfig, SshAuthMethod, WslTargetConfig,
};
use crate::test_support::{mem_pool, set_agent_dir, symlink_dir};

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
}

#[test]
fn ac8_doctor_reports_missing_node_without_path_mutation() {
    let err = resolve_node_launcher_from_dirs(&[]).unwrap_err();
    assert!(matches!(err, SkillsCliError::NodeMissing));
}

#[test]
fn ac8_doctor_reports_missing_npx_js_without_path_mutation() {
    let temp = TempDir::new().unwrap();
    let node_name = if cfg!(windows) { "node.exe" } else { "node" };
    std::fs::write(temp.path().join(node_name), b"").unwrap();
    let err = resolve_node_launcher_from_dirs(&[temp.path().to_path_buf()]).unwrap_err();
    assert!(matches!(err, SkillsCliError::CliUnavailable));
}

#[tokio::test]
async fn ac8_doctor_rejects_old_node() {
    let runner = FakeCliRunner::new();
    runner.push_ok("v18.20.0\n");
    let err = doctor_with_launcher(&runner, &fake_launcher())
        .await
        .unwrap_err();
    assert!(matches!(err, SkillsCliError::NodeTooOld { .. }));
    assert_eq!(runner.recorded().len(), 1);
}

#[tokio::test]
async fn ac3_ac5_empty_selection_refuses_before_spawn() {
    let runner = FakeCliRunner::new();
    let empty_skills = add_global(
        &runner,
        "owner/repo",
        vec![],
        vec!["cursor".to_string()],
        None,
    )
    .await
    .unwrap_err();
    assert!(matches!(empty_skills, SkillsCliError::SelectionEmpty));

    let empty_platforms = add_global(
        &runner,
        "owner/repo",
        vec!["demo".to_string()],
        vec![],
        None,
    )
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

    let targets = install_targets(&pool).await.unwrap();
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
fn ac2_non_local_ipc_rejects_without_spawn() {
    assert!(matches!(
        ensure_local_target(&ssh_target()),
        Err(SkillsCliError::LocalTargetOnly)
    ));
    assert!(matches!(
        ensure_local_target(&wsl_target()),
        Err(SkillsCliError::LocalTargetOnly)
    ));
    assert!(ensure_local_target(&ActiveTarget::Local).is_ok());
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

    let _holder = acquire_central_mutation_guard_at(
        central_mutation_lock_path(),
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
        &[super::inventory::InventoryPlatform {
            display_name: "Cursor".to_string(),
            global_skills_dir: cursor_dir,
        }],
    );
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].install_kind, SkillsCliInstallKind::Copy);
    assert_eq!(skills[0].path.as_deref(), Some(copy.to_str().unwrap()));
    assert_eq!(skills[0].agents, vec!["Cursor"]);
    assert_eq!(skills[0].source_type_bucket, SkillsCliSourceTypeBucket::Github);
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
    assert!(snapshot.skills[0].agents.iter().any(|name| name == "Cursor"));
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
