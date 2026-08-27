//! Remote link / unlink / remove / leftover mutation coverage (AC1–AC9).

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use sha2::{Digest, Sha256};
use tempfile::TempDir;

use super::error::SkillsCliError;
use super::link::{link_platform, link_platforms_batch, unlink_platform, unlink_platforms_batch};
use super::remote_scripts::{
    is_skillport_canonical_backup_path, remote_mutation_command_budget,
    SKILLS_CLI_REMOTE_MUTATION_CHUNK_SIZE, SKILLS_CLI_REMOTE_MUTATION_PROBE_OVERHEAD,
};
use super::remove::{preview_remove_global, recover_pending_via_transport, remove_global};
use super::{SkillsCliPlacementState, SkillsCliTransport};
use crate::ipc_error::public_message_for_code;
use crate::services::central_mutation::acquire_central_mutation_guard_at;
use crate::targets::{
    ActiveTarget, ConnectedRemoteTarget, ConnectedSshTarget, RemoteTargetConfig, SshAuthMethod,
};
use crate::test_support::{mem_pool_with_home, FakeRunner};

const SENTINEL: &str = "SENTINEL_TOKEN_SKILLS_CLI_STDERR";
const HOME: &str = "/mnt/remote-seam-home";

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

fn windows_tx(runner: Arc<FakeRunner>) -> SkillsCliTransport {
    let mut config = ssh_config();
    config.remote_os = "windows".to_string();
    SkillsCliTransport::for_tests_remote(ConnectedRemoteTarget::Ssh(
        ConnectedSshTarget::for_tests_with_runner(config, runner),
    ))
}

fn lock_json(names: &[&str]) -> String {
    let skills = names
        .iter()
        .map(|name| format!("\"{name}\":{{}}"))
        .collect::<Vec<_>>()
        .join(",");
    format!(r#"{{"version":3,"skills":{{{skills}}}}}"#)
}

fn probe_line(path: &str, kind: &str, target: &str) -> String {
    format!("{path}\t{kind}\t{target}\n")
}

fn stdin_of(runner: &FakeRunner, index: usize) -> String {
    let calls = runner.calls();
    String::from_utf8_lossy(calls[index].stdin.as_deref().unwrap_or(&[])).into_owned()
}

fn call_count(runner: &FakeRunner) -> usize {
    runner.calls().len()
}

fn mutation_script_count(runner: &FakeRunner) -> usize {
    runner
        .calls()
        .iter()
        .filter(|call| {
            let stdin = String::from_utf8_lossy(call.stdin.as_deref().unwrap_or(&[]));
            stdin.contains("SKILLPORT_PATHS")
                || stdin.contains("SKILLPORT_VERIFIED_LINK_REMOVE")
                || stdin.contains("ln -s")
                || stdin.contains("mklink /J")
        })
        .count()
}

async fn four_platform_pool() -> crate::db::DbPool {
    let pool = mem_pool_with_home(HOME).await;
    sqlx::query("DELETE FROM agents WHERE id NOT IN ('cursor', 'amp', 'zed', 'claude-code')")
        .execute(&pool)
        .await
        .unwrap();
    // cursor/amp share the Universal root with canonical, so slot == canonical.
    // Link/unlink Missing tests must use zed or claude-code (distinct dirs).
    sqlx::query(
        "UPDATE agents SET is_enabled = 1 WHERE id IN ('cursor', 'amp', 'zed', 'claude-code')",
    )
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

fn wipe_skill_recovery(skill: &str) {
    let dir = std::env::temp_dir()
        .join("skillport-skills-cli-remove-recovery")
        .join("ssh-cli-test");
    let _ = std::fs::remove_file(dir.join(format!("{skill}.json")));
}

fn assert_no_sentinel(hay: &str) {
    assert!(!hay.contains(SENTINEL), "{hay}");
}

#[tokio::test]
async fn ac1_remote_unix_create_classifies_as_managed_link() {
    wipe_skill_recovery("ac1u");
    let pool = four_platform_pool().await;
    let zed = agent_dir(&pool, "zed").await;
    let canonical = format!("{HOME}/.agents/skills/ac1u");
    let slot = format!("{zed}/ac1u");
    let runner = Arc::new(FakeRunner::new());
    runner.push_success(&lock_json(&["ac1u"]));
    runner.push_success(&format!(
        "{}{}",
        probe_line(&canonical, "dir", ""),
        probe_line(&slot, "absent", "")
    ));
    runner.push_success("");
    runner.push_success(&probe_line(&slot, "link", &canonical));
    let tx = remote_tx(runner.clone());
    let placement = link_platform(&tx, &pool, "ac1u", "zed", None)
        .await
        .unwrap();
    assert_eq!(placement.state, SkillsCliPlacementState::ManagedLink);
    assert!(stdin_of(runner.as_ref(), 2).contains("ln -s"));
    assert!(!stdin_of(runner.as_ref(), 2).contains("cp "));
}

#[tokio::test]
async fn ac1_remote_windows_create_uses_mklink_unverified() {
    // UNVERIFIED: live Windows junction via remote sh / cmd.exe //c mklink /J.
    wipe_skill_recovery("ac1w");
    let pool = four_platform_pool().await;
    let zed = agent_dir(&pool, "zed").await;
    let canonical = format!("{HOME}/.agents/skills/ac1w");
    let slot = format!("{zed}/ac1w");
    let runner = Arc::new(FakeRunner::new());
    runner.push_success(&lock_json(&["ac1w"]));
    runner.push_success(&format!(
        "{}{}",
        probe_line(&canonical, "dir", ""),
        probe_line(&slot, "absent", "")
    ));
    runner.push_success("");
    runner.push_success(&probe_line(&slot, "link", &canonical));
    let tx = windows_tx(runner.clone());
    let placement = link_platform(&tx, &pool, "ac1w", "zed", None)
        .await
        .unwrap();
    assert_eq!(placement.state, SkillsCliPlacementState::ManagedLink);
    assert!(stdin_of(runner.as_ref(), 2).contains("mklink /J"));
    assert!(!stdin_of(runner.as_ref(), 2).contains("cp "));
}

async fn assert_zero_write_reject(state_probe: &str, link: bool, expected: SkillsCliError) {
    wipe_skill_recovery("demo");
    let pool = four_platform_pool().await;
    let runner = Arc::new(FakeRunner::new());
    runner.push_success(&lock_json(&["demo"]));
    runner.push_success(state_probe);
    let tx = remote_tx(runner.clone());
    let error = if link {
        link_platform(&tx, &pool, "demo", "zed", None)
            .await
            .unwrap_err()
    } else {
        unlink_platform(&tx, &pool, "demo", "zed", None)
            .await
            .unwrap_err()
    };
    assert_eq!(error.ipc_code(), expected.ipc_code());
    assert_eq!(tx.write_count(), 0);
    assert_eq!(mutation_script_count(runner.as_ref()), 1);
}

#[tokio::test]
async fn ac2_direct_copy_conflict_unavailable_are_zero_write() {
    let pool = four_platform_pool().await;
    let zed = agent_dir(&pool, "zed").await;
    let canonical = format!("{HOME}/.agents/skills/demo");
    let slot = format!("{zed}/demo");
    let direct = format!(
        "{}{}",
        probe_line(&canonical, "dir", ""),
        probe_line(&slot, "dir", "")
    );
    let conflict = format!(
        "{}{}",
        probe_line(&canonical, "dir", ""),
        probe_line(&slot, "link", "/other")
    );
    let unavailable = format!(
        "{}{}",
        probe_line(&canonical, "absent", ""),
        probe_line(&slot, "absent", "")
    );
    drop(pool);
    assert_zero_write_reject(&direct, true, SkillsCliError::DirectCopyNotToggleable).await;
    assert_zero_write_reject(&direct, false, SkillsCliError::DirectCopyNotToggleable).await;
    assert_zero_write_reject(&conflict, true, SkillsCliError::PlacementConflict).await;
    assert_zero_write_reject(&conflict, false, SkillsCliError::PlacementConflict).await;
    assert_zero_write_reject(&unavailable, true, SkillsCliError::PlacementUnavailable).await;
    assert_zero_write_reject(&unavailable, false, SkillsCliError::PlacementUnavailable).await;
}

#[tokio::test]
async fn ac3_ordinary_directory_batch_unlink_is_skipped_without_delete() {
    wipe_skill_recovery("demo");
    let pool = four_platform_pool().await;
    let zed = agent_dir(&pool, "zed").await;
    let canonical = format!("{HOME}/.agents/skills/demo");
    let slot = format!("{zed}/demo");
    let runner = Arc::new(FakeRunner::new());
    runner.push_success(&format!(
        "{}{}",
        probe_line(&canonical, "dir", ""),
        probe_line(&slot, "dir", "")
    ));
    let tx = remote_tx(runner.clone());
    let outcome =
        unlink_platforms_batch(&tx, &pool, &[("demo".to_string(), "zed".to_string())], None)
            .await
            .unwrap();
    assert_eq!(outcome.skipped.len(), 1);
    assert!(outcome.succeeded.is_empty());
    assert_eq!(tx.write_count(), 0);
    for index in 0..call_count(runner.as_ref()) {
        let stdin = stdin_of(runner.as_ref(), index);
        assert!(!stdin.contains("SKILLPORT_VERIFIED_LINK_REMOVE"), "{stdin}");
        assert!(!stdin.contains("rm -rf"), "{stdin}");
    }
}

#[test]
fn ac3b_rm_rf_only_on_backup_paths_and_never_remove_install() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/services/skills_cli");
    let mut files = Vec::new();
    collect_rs(&root, &mut files);
    for path in files {
        let source = std::fs::read_to_string(&path).unwrap();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name == "remote_scripts.rs" || name == "mutate_tests.rs" || name == "tests.rs" {
            continue;
        }
        assert!(
            !source.contains("InstallTransport::remove_install")
                && !source.contains(".remove_install("),
            "{} must not reuse InstallTransport::remove_install",
            path.display()
        );
        for (index, line) in source.lines().enumerate() {
            if line.contains("rm -rf") && !line.contains('"') {
                assert!(
                    line.contains("skillport")
                        || source.contains("is_skillport_canonical_backup_path")
                        || source.contains("SkillPort-generated"),
                    "{}:{} uses rm -rf outside the backup boundary: {line}",
                    path.display(),
                    index + 1
                );
            }
        }
    }
    assert!(is_skillport_canonical_backup_path(
        "/root/.skillport-remove-abcd"
    ));
    assert!(!is_skillport_canonical_backup_path("/root/demo"));
}

fn collect_rs(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

#[tokio::test]
async fn ac3c_verify_failure_rolls_back_placeholder() {
    wipe_skill_recovery("ac3c");
    let pool = four_platform_pool().await;
    let zed = agent_dir(&pool, "zed").await;
    let canonical = format!("{HOME}/.agents/skills/ac3c");
    let slot = format!("{zed}/ac3c");
    let runner = Arc::new(FakeRunner::new());
    runner.push_success(&lock_json(&["ac3c"]));
    runner.push_success(&format!(
        "{}{}",
        probe_line(&canonical, "dir", ""),
        probe_line(&slot, "absent", "")
    ));
    runner.push_success("");
    runner.push_success(&probe_line(&slot, "dir", ""));
    runner.push_success(&format!("{slot}\tskipped_not_link\n"));
    let tx = remote_tx(runner.clone());
    let error = link_platform(&tx, &pool, "ac3c", "zed", None)
        .await
        .unwrap_err();
    assert_eq!(error.ipc_code(), "skills_cli.placement_unavailable");
    let remove_stdin = stdin_of(runner.as_ref(), 4);
    assert!(remove_stdin.contains("SKILLPORT_VERIFIED_LINK_REMOVE"));
    assert!(!remove_stdin.contains("rm -rf"));
}

#[tokio::test]
async fn ac4_remote_remove_keeps_direct_copy_and_conflict_is_zero_write() {
    wipe_skill_recovery("ac4");
    let pool = four_platform_pool().await;
    let zed = agent_dir(&pool, "zed").await;
    let claude = agent_dir(&pool, "claude-code").await;
    let canonical = format!("{HOME}/.agents/skills/ac4");
    let managed = format!("{zed}/ac4");
    let copy = format!("{claude}/ac4");
    let runner = Arc::new(FakeRunner::new());
    runner.push_success(&lock_json(&["ac4"]));
    let mut probe = String::new();
    probe.push_str(&probe_line(&canonical, "dir", ""));
    probe.push_str(&probe_line(&managed, "link", &canonical));
    probe.push_str(&probe_line(&copy, "dir", ""));
    runner.push_success(&probe);
    let lock = lock_json(&["ac4"]);
    runner.push_success(&lock);
    runner.push_success("");
    runner.push_success(&format!("{managed}\tremoved\n"));
    runner.push_success(&lock);
    for _ in 0..8 {
        runner.push_success("");
    }
    let tx = remote_tx(runner.clone());
    let result = remove_global(&tx, &pool, "ac4", None).await.unwrap();
    assert!(result.removed_canonical);
    assert!(result
        .removed_managed_agent_ids
        .contains(&"zed".to_string()));
    assert!(result
        .retained_direct_copy_agent_ids
        .contains(&"claude-code".to_string()));
    let joined: String = (0..call_count(runner.as_ref()))
        .map(|index| stdin_of(runner.as_ref(), index))
        .collect();
    assert!(joined.contains("SKILLPORT_VERIFIED_LINK_REMOVE"));
    assert!(joined.contains(&managed));
    for index in 0..call_count(runner.as_ref()) {
        let stdin = stdin_of(runner.as_ref(), index);
        if stdin.contains("SKILLPORT_VERIFIED_LINK_REMOVE") {
            assert!(
                !stdin.contains(&copy),
                "direct copy must not enter the verified-remove path list: {stdin}"
            );
        }
    }
    assert!(!joined.contains("remove_install"));

    wipe_skill_recovery("ac4");
    let conflict_runner = Arc::new(FakeRunner::new());
    conflict_runner.push_success(&lock_json(&["ac4"]));
    conflict_runner.push_success(&format!(
        "{}{}",
        probe_line(&canonical, "dir", ""),
        probe_line(&managed, "file", "")
    ));
    let conflict_tx = remote_tx(conflict_runner.clone());
    let error = remove_global(&conflict_tx, &pool, "ac4", None)
        .await
        .unwrap_err();
    assert_eq!(error.ipc_code(), "skills_cli.placement_conflict");
    assert_eq!(conflict_tx.write_count(), 0);
}

#[tokio::test]
async fn ac5_remote_recovery_converges_from_three_phases() {
    wipe_skill_recovery("ac5");
    let lock = lock_json(&["ac5"]);
    let fingerprint = format!("{:x}", Sha256::digest(lock.as_bytes()));
    let phases = ["prepared", "staged", "metadata_committed"];
    for phase in phases {
        let runner = Arc::new(FakeRunner::new());
        match phase {
            "prepared" => {
                runner.push_output(0, "", "");
                runner.push_output(1, "", "");
            }
            "staged" => {
                runner.push_output(1, "", "");
                runner.push_output(0, "", "");
                runner.push_success("");
                runner.push_success(&lock);
            }
            _ => {
                runner.push_output(0, "", "");
                runner.push_success("");
            }
        }
        let tx = remote_tx(runner);
        let dir = super::remove_recovery_dir_for_transport(&tx);
        std::fs::create_dir_all(&dir).unwrap();
        let manifest = serde_json::json!({
            "version": 1,
            "operation_id": "op-ac5",
            "skill_name": "ac5",
            "phase": phase,
            "lock_fingerprint": fingerprint,
            "lock_path": format!("{HOME}/.agents/.skill-lock.json"),
            "canonical_path": format!("{HOME}/.agents/skills/ac5"),
            "canonical_backup_path": format!("{HOME}/.agents/skills/.skillport-remove-op-ac5"),
            "managed_links": []
        });
        std::fs::write(dir.join("ac5.json"), serde_json::to_vec(&manifest).unwrap()).unwrap();
        recover_pending_via_transport(&tx, "ac5")
            .await
            .expect(phase);
        assert!(
            !dir.join("ac5.json").exists(),
            "recovery must consume the {phase} manifest"
        );
    }
}

#[tokio::test]
async fn ac6_remote_and_local_guards_are_independent_file_paths() {
    let temp = TempDir::new().unwrap();
    let remote_lock = temp.path().join("central-mutation-ssh-demo.lock");
    let local_lock = temp.path().join("central-mutation.lock");
    let _remote = acquire_central_mutation_guard_at(
        remote_lock.clone(),
        "remote skills cli",
        Duration::from_secs(2),
    )
    .await
    .unwrap();
    let local = acquire_central_mutation_guard_at(
        local_lock,
        "local skills cli",
        Duration::from_millis(200),
    )
    .await;
    assert!(
        local.is_ok(),
        "holding a remote file lock must not block Local"
    );
    let busy =
        acquire_central_mutation_guard_at(remote_lock, "second remote", Duration::from_millis(50))
            .await;
    assert!(busy.is_err());

    let digest = format!("{:x}", Sha256::digest(b"ssh-cli-test"));
    let production = crate::paths::central_mutation_lock_path()
        .parent()
        .unwrap()
        .join(format!("central-mutation-ssh-{digest}.lock"));
    assert!(production
        .file_name()
        .unwrap()
        .to_string_lossy()
        .starts_with("central-mutation-ssh-"));
}

#[tokio::test]
async fn ac8_partial_batch_unlink_keeps_earlier_chunk() {
    let pool = four_platform_pool().await;
    let cursor = agent_dir(&pool, "cursor").await;
    let k = SKILLS_CLI_REMOTE_MUTATION_CHUNK_SIZE;
    let n = k + 1;
    let mut items = Vec::new();
    let mut probe = String::new();
    for index in 0..n {
        let name = format!("skill-{index}");
        items.push((name.clone(), "cursor".to_string()));
        let canonical = format!("{HOME}/.agents/skills/{name}");
        let slot = format!("{cursor}/{name}");
        probe.push_str(&probe_line(&canonical, "dir", ""));
        probe.push_str(&probe_line(&slot, "link", &canonical));
    }
    let runner = Arc::new(FakeRunner::new());
    runner.push_success(&probe);
    let mut first = String::new();
    for index in 0..k {
        first.push_str(&format!("{cursor}/skill-{index}\tremoved\n"));
    }
    runner.push_success(&first);
    runner.push_output(1, "", SENTINEL);
    let tx = remote_tx(runner.clone());
    let outcome = unlink_platforms_batch(&tx, &pool, &items, None)
        .await
        .unwrap();
    assert_eq!(outcome.succeeded.len(), k);
    assert_eq!(outcome.failed.len(), 1);
    assert_eq!(
        mutation_script_count(runner.as_ref()),
        remote_mutation_command_budget(n)
    );
}

#[tokio::test]
async fn ac8_partial_batch_link_keeps_earlier_chunk() {
    let pool = four_platform_pool().await;
    let zed = agent_dir(&pool, "zed").await;
    let k = SKILLS_CLI_REMOTE_MUTATION_CHUNK_SIZE;
    let n = k + 1;
    let mut items = Vec::new();
    let mut probe = String::new();
    for index in 0..n {
        let name = format!("link-{index}");
        items.push((name.clone(), "zed".to_string()));
        let canonical = format!("{HOME}/.agents/skills/{name}");
        let slot = format!("{zed}/{name}");
        probe.push_str(&probe_line(&canonical, "dir", ""));
        probe.push_str(&probe_line(&slot, "absent", ""));
    }
    let runner = Arc::new(FakeRunner::new());
    runner.push_success(&probe);
    runner.push_success("");
    runner.push_output(1, "", SENTINEL);
    let tx = remote_tx(runner.clone());
    let outcome = link_platforms_batch(&tx, &pool, &items, None)
        .await
        .unwrap();
    assert_eq!(outcome.succeeded.len(), k);
    assert_eq!(outcome.failed.len(), 1);
}

#[tokio::test]
async fn ac8b_batch_command_budget_matches_named_constant() {
    let pool = four_platform_pool().await;
    let cursor = agent_dir(&pool, "cursor").await;
    let k = SKILLS_CLI_REMOTE_MUTATION_CHUNK_SIZE;
    for n in [1usize, k, k + 1, 4 * k] {
        let mut items = Vec::new();
        let mut probe = String::new();
        for index in 0..n {
            let name = format!("skill-{index}");
            items.push((name.clone(), "cursor".to_string()));
            let canonical = format!("{HOME}/.agents/skills/{name}");
            let slot = format!("{cursor}/{name}");
            probe.push_str(&probe_line(&canonical, "dir", ""));
            probe.push_str(&probe_line(&slot, "link", &canonical));
        }
        let runner = Arc::new(FakeRunner::new());
        runner.push_success(&probe);
        let chunks = n.div_ceil(k);
        for _ in 0..chunks {
            runner.push_success("removed\n");
        }
        let tx = remote_tx(runner.clone());
        unlink_platforms_batch(&tx, &pool, &items, None)
            .await
            .unwrap();
        assert_eq!(
            mutation_script_count(runner.as_ref()),
            remote_mutation_command_budget(n),
            "n={n}"
        );
        assert_eq!(
            remote_mutation_command_budget(1),
            remote_mutation_command_budget(k)
        );
        assert_eq!(SKILLS_CLI_REMOTE_MUTATION_PROBE_OVERHEAD, 1);
    }

    let six_pool = mem_pool_with_home(HOME).await;
    let n = k;
    let mut items = Vec::new();
    let mut probe = String::new();
    let cursor = agent_dir(&six_pool, "cursor").await;
    for index in 0..n {
        let name = format!("skill-{index}");
        items.push((name.clone(), "cursor".to_string()));
        let canonical = format!("{HOME}/.agents/skills/{name}");
        let slot = format!("{cursor}/{name}");
        probe.push_str(&probe_line(&canonical, "dir", ""));
        probe.push_str(&probe_line(&slot, "link", &canonical));
    }
    let runner = Arc::new(FakeRunner::new());
    runner.push_success(&probe);
    for _ in 0..n.div_ceil(k) {
        runner.push_success("removed\n");
    }
    let tx = remote_tx(runner.clone());
    unlink_platforms_batch(&tx, &six_pool, &items, None)
        .await
        .unwrap();
    assert_eq!(
        mutation_script_count(runner.as_ref()),
        remote_mutation_command_budget(n)
    );
}

#[tokio::test]
async fn ac9_remote_stderr_sentinel_stays_out_of_ipc_message() {
    wipe_skill_recovery("ac9");
    let pool = four_platform_pool().await;
    let zed = agent_dir(&pool, "zed").await;
    let canonical = format!("{HOME}/.agents/skills/ac9");
    let slot = format!("{zed}/ac9");
    let runner = Arc::new(FakeRunner::new());
    runner.push_success(&lock_json(&["ac9"]));
    runner.push_success(&format!(
        "{}{}",
        probe_line(&canonical, "dir", ""),
        probe_line(&slot, "absent", "")
    ));
    runner.push_output(1, "", SENTINEL);
    runner.push_success(&format!("{slot}\tabsent\n"));
    let tx = remote_tx(runner);
    let error = link_platform(&tx, &pool, "ac9", "zed", None)
        .await
        .unwrap_err();
    assert_no_sentinel(&error.to_string());
    assert_no_sentinel(error.ipc_code());
    let public = public_message_for_code(error.ipc_code()).unwrap_or("");
    assert_no_sentinel(public);
}

#[test]
fn ac7_remote_leftover_scan_does_not_call_local_lock_loader_on_remote_branch() {
    let source = include_str!("../central_updates/inventory/scan.rs");
    assert!(source.contains("scan_deleted_platform_copies_for_target"));
    assert!(source.contains("load_lock_from_transport"));
    let remote_fn = source
        .split("pub(crate) async fn scan_deleted_platform_copies_for_target")
        .nth(1)
        .unwrap();
    let body = remote_fn
        .split("pub(crate) async fn scan_deleted_platform_copies_with_ownership")
        .next()
        .unwrap();
    assert!(
        !body.contains("load_local_cli_lock_ownership"),
        "remote leftover must not read this machine's lock"
    );
}

#[test]
fn preview_remove_is_open_on_remote_capability_gate() {
    use super::SkillsCliCapability;
    assert!(SkillsCliTransport::ensure_capability_for_target(
        &ActiveTarget::Ssh(Box::new(ssh_config())),
        SkillsCliCapability::PreviewRemove,
    )
    .is_ok());
}

#[tokio::test]
async fn preview_remove_conflict_is_zero_write() {
    wipe_skill_recovery("demo");
    let pool = four_platform_pool().await;
    let cursor = agent_dir(&pool, "cursor").await;
    let canonical = format!("{HOME}/.agents/skills/demo");
    let slot = format!("{cursor}/demo");
    let runner = Arc::new(FakeRunner::new());
    runner.push_success(&lock_json(&["demo"]));
    runner.push_success(&format!(
        "{}{}",
        probe_line(&canonical, "dir", ""),
        probe_line(&slot, "file", "")
    ));
    let tx = remote_tx(runner);
    let plan = preview_remove_global(&tx, &pool, "demo").await.unwrap();
    assert!(!plan.conflicts.is_empty());
    assert!(!plan.confirmable);
    assert_eq!(tx.write_count(), 0);
}
