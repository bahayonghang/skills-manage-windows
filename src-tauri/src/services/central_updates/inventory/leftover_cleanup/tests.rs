use std::collections::HashSet;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use chrono::Utc;
use tempfile::TempDir;

use super::super::scan::{
    scan_deleted_platform_copies_with_ownership, scan_deleted_platform_copies_with_pool,
};
use super::super::{DeletedPlatformCopyRemoval, SkillUpdateApplyResult};
use super::{
    apply_remove_deleted_platform_copies_on_connection, apply_remove_deleted_platform_copies_step,
    leftover_remote_chunk_count, parse_remote_leftover_delete_stdout, set_leftover_guard_timeout,
    RemoteLeftoverPathStatus, REMOTE_LEFTOVER_DELETE_CHUNK_SIZE, REMOTE_LEFTOVER_DELETE_SCRIPT,
};
use crate::db::{self, AgentSkillObservation, SkillInstallation, UNIVERSAL_AGENT_IDS};
use crate::targets::{
    ActiveTarget, CommandRunner, ConnectedRemoteTarget, ConnectedSshTarget, ConnectedWslTarget,
    RemoteTargetConfig, RunnerError, SshAuthMethod, WslTargetConfig,
};
use crate::test_support::{mem_pool_with_home, FakeRunner};

const REMOTE_HOME: &str = "/home/alice";
const SHARED_SKILL_ID: &str = "ask-matt";
const SHARED_PATH: &str = "/home/alice/.agents/skills/ask-matt";

fn fake_ssh() -> (Arc<FakeRunner>, ConnectedRemoteTarget) {
    let runner = Arc::new(FakeRunner::new());
    let target = RemoteTargetConfig {
        id: "ssh-leftover-test".to_string(),
        label: "Leftover SSH".to_string(),
        host: "example.invalid".to_string(),
        username: "tester".to_string(),
        port: 22,
        auth_method: SshAuthMethod::Key,
        key_path: "~/.ssh/id_ed25519".to_string(),
        credential_key: None,
        protected_password: None,
        password: None,
        remote_home: REMOTE_HOME.to_string(),
        remote_os: "linux".to_string(),
        symlink_enabled: true,
    };
    let connection = ConnectedSshTarget::for_tests_with_runner(target, runner.clone());
    (runner, ConnectedRemoteTarget::Ssh(connection))
}

fn fake_wsl() -> (Arc<FakeRunner>, ConnectedRemoteTarget) {
    let runner = Arc::new(FakeRunner::new());
    let connection = ConnectedWslTarget::for_tests_with_runner(
        WslTargetConfig {
            id: "wsl-leftover-test".to_string(),
            label: "Leftover WSL".to_string(),
            distribution: "TestDistro".to_string(),
            remote_home: REMOTE_HOME.to_string(),
            remote_os: "linux".to_string(),
            symlink_enabled: true,
        },
        runner.clone(),
    );
    (runner, ConnectedRemoteTarget::Wsl(connection))
}

fn make_observation(agent_id: &str, skill_id: &str, dir_path: &str) -> AgentSkillObservation {
    AgentSkillObservation {
        row_id: format!("{agent_id}::{skill_id}::{dir_path}"),
        agent_id: agent_id.to_string(),
        skill_id: skill_id.to_string(),
        name: skill_id.to_string(),
        description: None,
        file_path: format!("{dir_path}/SKILL.md"),
        dir_path: dir_path.to_string(),
        source_kind: "user".to_string(),
        source_root: dir_path.to_string(),
        link_type: "copy".to_string(),
        symlink_target: None,
        is_read_only: false,
        scanned_at: Utc::now().to_rfc3339(),
        fs_created_at: None,
        fs_updated_at: None,
    }
}

async fn seed_platform_skill(pool: &db::DbPool, skill_id: &str, path: &str) {
    let mut skill = crate::test_support::central_skill_row(skill_id, Path::new(path));
    skill.is_central = false;
    skill.canonical_path = None;
    db::upsert_skill(pool, &skill).await.unwrap();
}

fn copy_installation(skill_id: &str, agent_id: &str, path: &str) -> SkillInstallation {
    SkillInstallation {
        skill_id: skill_id.to_string(),
        agent_id: agent_id.to_string(),
        installed_path: path.to_string(),
        link_type: "copy".to_string(),
        symlink_target: None,
        created_at: Utc::now().to_rfc3339(),
    }
}

fn shared_root_removal(agent_id: &str) -> DeletedPlatformCopyRemoval {
    DeletedPlatformCopyRemoval {
        agent_id: agent_id.to_string(),
        skill_id: SHARED_SKILL_ID.to_string(),
        paths: vec![SHARED_PATH.to_string()],
    }
}

async fn seed_shared_root_leftovers(pool: &db::DbPool, agent_ids: &[&str]) {
    seed_platform_skill(pool, SHARED_SKILL_ID, SHARED_PATH).await;
    for agent_id in agent_ids {
        db::upsert_skill_installation(
            pool,
            &copy_installation(SHARED_SKILL_ID, agent_id, SHARED_PATH),
        )
        .await
        .unwrap();
        db::upsert_agent_skill_observation(
            pool,
            &make_observation(agent_id, SHARED_SKILL_ID, SHARED_PATH),
        )
        .await
        .unwrap();
    }
}

fn recorded_path_hits(call: &crate::test_support::RecordedCommand, path: &str) -> usize {
    let arg_hits = call.args.iter().filter(|arg| arg.contains(path)).count();
    let stdin_hits = call
        .stdin
        .as_deref()
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .map(|stdin| stdin.matches(path).count())
        .unwrap_or(0);
    arg_hits + stdin_hits
}

fn ok_rows(count: usize) -> String {
    (0..count).map(|index| format!("OK\t{index}\n")).collect()
}

#[test]
fn leftover_remote_chunk_size_is_fixed() {
    assert_eq!(REMOTE_LEFTOVER_DELETE_CHUNK_SIZE, 256);
    assert_eq!(leftover_remote_chunk_count(1), 1);
    assert_eq!(leftover_remote_chunk_count(256), 1);
    assert_eq!(leftover_remote_chunk_count(257), 2);
}

#[test]
fn parse_remote_leftover_stdout_accepts_mixed_status_rows() {
    let statuses = parse_remote_leftover_delete_stdout("OK\t0\nMISSING\t1\nERR\t2\n", 3).unwrap();
    assert_eq!(
        statuses,
        vec![
            RemoteLeftoverPathStatus::Ok,
            RemoteLeftoverPathStatus::Missing,
            RemoteLeftoverPathStatus::Err
        ]
    );
}

#[test]
fn parse_remote_leftover_stdout_rejects_malformed_protocol() {
    assert!(parse_remote_leftover_delete_stdout("OK 0\n", 1).is_err());
    assert!(parse_remote_leftover_delete_stdout("OK\t0\n", 2).is_err());
    assert!(parse_remote_leftover_delete_stdout("OK\t0\nOK\t0\n", 1).is_err());
    assert!(parse_remote_leftover_delete_stdout("WAT\t0\n", 1).is_err());
}

#[tokio::test]
async fn shared_universal_root_uses_one_runner_call_and_clears_scan() {
    let pool = mem_pool_with_home(REMOTE_HOME).await;
    seed_shared_root_leftovers(&pool, &UNIVERSAL_AGENT_IDS).await;
    let (runner, connection) = fake_ssh();
    runner.push_success("OK\t0\n");

    let removals = UNIVERSAL_AGENT_IDS
        .iter()
        .map(|agent_id| shared_root_removal(agent_id))
        .collect();
    let mut result = SkillUpdateApplyResult::default();
    apply_remove_deleted_platform_copies_on_connection(
        &pool,
        &connection,
        removals,
        &mut result,
        None,
        None,
    )
    .await;

    assert!(result.failures.is_empty(), "{:?}", result.failures);
    assert_eq!(result.removed_deleted_platform_copy_paths.len(), 10);
    assert!(result
        .removed_deleted_platform_copy_paths
        .iter()
        .all(|path| path == SHARED_PATH));

    {
        let calls = runner.calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(
            calls[0].stdin.as_deref(),
            Some(REMOTE_LEFTOVER_DELETE_SCRIPT.as_bytes())
        );
        assert_eq!(calls[0].policy.class.label(), "bulk_transfer");
        assert_eq!(recorded_path_hits(&calls[0], SHARED_PATH), 1);
    }

    let groups = scan_deleted_platform_copies_with_pool(&pool, None, false)
        .await
        .unwrap();
    assert!(
        groups
            .iter()
            .all(|group| { !group.writable_paths.iter().any(|path| path == SHARED_PATH) }),
        "{groups:?}"
    );
    assert!(db::get_skill_installations(&pool, SHARED_SKILL_ID)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn shared_root_success_clears_sibling_platforms_not_in_payload() {
    let pool = mem_pool_with_home(REMOTE_HOME).await;
    seed_shared_root_leftovers(&pool, &["amp", "cursor"]).await;
    let (runner, connection) = fake_ssh();
    runner.push_success("OK\t0\n");

    let mut result = SkillUpdateApplyResult::default();
    apply_remove_deleted_platform_copies_on_connection(
        &pool,
        &connection,
        vec![shared_root_removal("amp")],
        &mut result,
        None,
        None,
    )
    .await;

    assert!(result.failures.is_empty(), "{:?}", result.failures);
    assert_eq!(runner.calls().len(), 1);
    let groups = scan_deleted_platform_copies_with_pool(
        &pool,
        Some(vec!["amp".to_string(), "cursor".to_string()]),
        false,
    )
    .await
    .unwrap();
    assert!(groups.is_empty(), "{groups:?}");
    assert!(db::get_agent_skill_observations(&pool, "cursor")
        .await
        .unwrap()
        .is_empty());
    assert!(db::get_skill_installations(&pool, SHARED_SKILL_ID)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn wsl_shared_root_matches_ssh_runner_count_and_script() {
    let pool = mem_pool_with_home(REMOTE_HOME).await;
    seed_shared_root_leftovers(&pool, &["amp", "cursor"]).await;
    let (runner, connection) = fake_wsl();
    runner.push_success("OK\t0\n");

    let mut result = SkillUpdateApplyResult::default();
    apply_remove_deleted_platform_copies_on_connection(
        &pool,
        &connection,
        vec![shared_root_removal("amp"), shared_root_removal("cursor")],
        &mut result,
        None,
        None,
    )
    .await;

    assert!(result.failures.is_empty(), "{:?}", result.failures);
    let calls = runner.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(
        calls[0].stdin.as_deref(),
        Some(REMOTE_LEFTOVER_DELETE_SCRIPT.as_bytes())
    );
    assert_eq!(recorded_path_hits(&calls[0], SHARED_PATH), 1);
    assert_eq!(calls[0].policy.class.label(), "bulk_transfer");
}

#[tokio::test]
async fn mixed_remote_paths_keep_partial_success() {
    let pool = mem_pool_with_home(REMOTE_HOME).await;
    let ok_path = "/home/alice/.claude/skills/ok-skill";
    let missing_path = "/home/alice/.claude/skills/missing-skill";
    let err_path = "/home/alice/.claude/skills/err-skill";
    for (skill_id, path) in [
        ("ok-skill", ok_path),
        ("missing-skill", missing_path),
        ("err-skill", err_path),
    ] {
        seed_platform_skill(&pool, skill_id, path).await;
        db::upsert_skill_installation(&pool, &copy_installation(skill_id, "claude-code", path))
            .await
            .unwrap();
        db::upsert_agent_skill_observation(&pool, &make_observation("claude-code", skill_id, path))
            .await
            .unwrap();
    }
    let (runner, connection) = fake_ssh();
    runner.push_success("OK\t0\nMISSING\t1\nERR\t2\n");

    let mut result = SkillUpdateApplyResult::default();
    apply_remove_deleted_platform_copies_on_connection(
        &pool,
        &connection,
        vec![
            DeletedPlatformCopyRemoval {
                agent_id: "claude-code".to_string(),
                skill_id: "ok-skill".to_string(),
                paths: vec![ok_path.to_string()],
            },
            DeletedPlatformCopyRemoval {
                agent_id: "claude-code".to_string(),
                skill_id: "missing-skill".to_string(),
                paths: vec![missing_path.to_string()],
            },
            DeletedPlatformCopyRemoval {
                agent_id: "claude-code".to_string(),
                skill_id: "err-skill".to_string(),
                paths: vec![err_path.to_string()],
            },
        ],
        &mut result,
        None,
        None,
    )
    .await;

    assert_eq!(runner.calls().len(), 1);
    assert_eq!(
        result.removed_deleted_platform_copy_paths,
        vec![ok_path.to_string(), missing_path.to_string()]
    );
    assert_eq!(result.failures.len(), 1);
    assert_eq!(result.failures[0].step, "remove_deleted_platform_copy");
    assert_eq!(result.failures[0].identifier, "claude-code::err-skill");
    assert!(db::get_skill_installations(&pool, "ok-skill")
        .await
        .unwrap()
        .is_empty());
    assert!(db::get_skill_installations(&pool, "missing-skill")
        .await
        .unwrap()
        .is_empty());
    assert_eq!(
        db::get_skill_installations(&pool, "err-skill")
            .await
            .unwrap()
            .len(),
        1
    );
    let leftover_obs = db::get_agent_skill_observations(&pool, "claude-code")
        .await
        .unwrap();
    assert_eq!(leftover_obs.len(), 1);
    assert_eq!(leftover_obs[0].skill_id, "err-skill");
    let groups =
        scan_deleted_platform_copies_with_pool(&pool, Some(vec!["claude-code".to_string()]), false)
            .await
            .unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].skill_id, "err-skill");
}

#[tokio::test]
async fn guard_failure_does_not_start_runner_or_change_db() {
    let pool = mem_pool_with_home(REMOTE_HOME).await;
    seed_shared_root_leftovers(&pool, &["amp"]).await;
    let (runner, connection) = fake_ssh();

    let mut result = SkillUpdateApplyResult::default();
    apply_remove_deleted_platform_copies_on_connection(
        &pool,
        &connection,
        vec![DeletedPlatformCopyRemoval {
            agent_id: "amp".to_string(),
            skill_id: SHARED_SKILL_ID.to_string(),
            paths: vec!["/tmp/not-managed".to_string()],
        }],
        &mut result,
        None,
        None,
    )
    .await;

    assert_eq!(runner.calls().len(), 0);
    assert!(result.removed_deleted_platform_copy_paths.is_empty());
    assert_eq!(result.failures.len(), 1);
    assert_eq!(result.failures[0].step, "remove_deleted_platform_copy");
    assert_eq!(
        db::get_skill_installations(&pool, SHARED_SKILL_ID)
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        db::get_agent_skill_observations(&pool, "amp")
            .await
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn central_skill_reappeared_skips_remote_delete() {
    let pool = mem_pool_with_home(REMOTE_HOME).await;
    seed_shared_root_leftovers(&pool, &["amp"]).await;
    db::upsert_skill(
        &pool,
        &crate::test_support::central_skill_row(
            SHARED_SKILL_ID,
            Path::new("/home/alice/.skillsmanage/skills/ask-matt"),
        ),
    )
    .await
    .unwrap();
    let (runner, connection) = fake_ssh();

    let mut result = SkillUpdateApplyResult::default();
    apply_remove_deleted_platform_copies_on_connection(
        &pool,
        &connection,
        vec![shared_root_removal("amp")],
        &mut result,
        None,
        None,
    )
    .await;

    assert_eq!(runner.calls().len(), 0);
    assert!(result.removed_deleted_platform_copy_paths.is_empty());
    assert_eq!(result.failures.len(), 1);
    assert_eq!(
        db::get_skill_installations(&pool, SHARED_SKILL_ID)
            .await
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn cancel_before_remote_leftover_script_starts_no_runner() {
    let pool = mem_pool_with_home(REMOTE_HOME).await;
    seed_shared_root_leftovers(&pool, &["amp"]).await;
    let (runner, connection) = fake_ssh();
    let cancel = AtomicBool::new(true);

    let mut result = SkillUpdateApplyResult::default();
    apply_remove_deleted_platform_copies_on_connection(
        &pool,
        &connection,
        vec![shared_root_removal("amp")],
        &mut result,
        None,
        Some(&cancel),
    )
    .await;

    assert_eq!(runner.calls().len(), 0);
    assert!(result.removed_deleted_platform_copy_paths.is_empty());
    assert_eq!(result.failures.len(), 1);
    assert_eq!(
        result.failures[0].error_code.as_deref(),
        Some("operation.cancelled")
    );
}

#[tokio::test]
async fn two_hundred_fifty_seven_paths_use_two_fixed_chunks() {
    let pool = mem_pool_with_home(REMOTE_HOME).await;
    let removals = (0..257)
        .map(|index| {
            let skill_id = format!("skill-{index:03}");
            let path = format!("/home/alice/.claude/skills/{skill_id}");
            DeletedPlatformCopyRemoval {
                agent_id: "claude-code".to_string(),
                skill_id,
                paths: vec![path],
            }
        })
        .collect::<Vec<_>>();
    let (runner, connection) = fake_ssh();
    runner.push_success(&ok_rows(256));
    runner.push_success(&ok_rows(1));

    let mut result = SkillUpdateApplyResult::default();
    apply_remove_deleted_platform_copies_on_connection(
        &pool,
        &connection,
        removals,
        &mut result,
        None,
        None,
    )
    .await;

    assert!(result.failures.is_empty(), "{:?}", result.failures);
    assert_eq!(result.removed_deleted_platform_copy_paths.len(), 257);
    assert_eq!(runner.calls().len(), leftover_remote_chunk_count(257));
    {
        let calls = runner.calls();
        for call in calls.iter() {
            assert_eq!(
                call.stdin.as_deref(),
                Some(REMOTE_LEFTOVER_DELETE_SCRIPT.as_bytes())
            );
            let stdin = std::str::from_utf8(call.stdin.as_deref().unwrap()).unwrap();
            assert!(!stdin.contains("/home/alice"));
            assert_eq!(call.policy.class.label(), "bulk_transfer");
        }
    }
}

#[tokio::test]
async fn local_central_reappeared_keeps_platform_copy() {
    let pool = crate::test_support::mem_pool().await;
    let temp = TempDir::new().unwrap();
    let central_dir = temp.path().join("central");
    let cursor_dir = temp.path().join("cursor");
    let cursor_skill_dir = cursor_dir.join("kept-skill");
    std::fs::create_dir_all(&central_dir).unwrap();
    std::fs::create_dir_all(&cursor_skill_dir).unwrap();
    std::fs::write(cursor_skill_dir.join("SKILL.md"), b"---\nname: Kept\n---").unwrap();
    crate::test_support::set_agent_dir(&pool, "central", &central_dir).await;
    crate::test_support::set_agent_dir(&pool, "cursor", &cursor_dir).await;
    crate::test_support::seed_central_skill(
        &pool,
        &central_dir.join("kept-skill"),
        "kept-skill",
        "still present",
    )
    .await;
    db::upsert_skill_installation(
        &pool,
        &copy_installation("kept-skill", "cursor", &cursor_skill_dir.to_string_lossy()),
    )
    .await
    .unwrap();

    let mut result = SkillUpdateApplyResult::default();
    apply_remove_deleted_platform_copies_step(
        &pool,
        &ActiveTarget::Local,
        vec![DeletedPlatformCopyRemoval {
            agent_id: "cursor".to_string(),
            skill_id: "kept-skill".to_string(),
            paths: vec![cursor_skill_dir.to_string_lossy().into_owned()],
        }],
        &mut result,
        None,
        None,
    )
    .await;

    assert!(result.removed_deleted_platform_copy_paths.is_empty());
    assert_eq!(result.failures.len(), 1);
    assert!(cursor_skill_dir.exists());
    assert_eq!(
        db::get_skill_installations(&pool, "kept-skill")
            .await
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn allowed_agent_filter_still_rejects_without_runner() {
    let pool = mem_pool_with_home(REMOTE_HOME).await;
    seed_shared_root_leftovers(&pool, &["amp"]).await;
    let (runner, connection) = fake_ssh();
    let allowed = HashSet::from(["cursor".to_string()]);

    let mut result = SkillUpdateApplyResult::default();
    apply_remove_deleted_platform_copies_on_connection(
        &pool,
        &connection,
        vec![shared_root_removal("amp")],
        &mut result,
        Some(&allowed),
        None,
    )
    .await;

    assert_eq!(runner.calls().len(), 0);
    assert_eq!(result.failures[0].identifier, "amp::ask-matt");
}

#[tokio::test]
async fn platform_root_and_traversal_never_start_runner() {
    let pool = mem_pool_with_home(REMOTE_HOME).await;
    seed_shared_root_leftovers(&pool, &["amp"]).await;
    let (runner, connection) = fake_ssh();

    let mut result = SkillUpdateApplyResult::default();
    apply_remove_deleted_platform_copies_on_connection(
        &pool,
        &connection,
        vec![
            DeletedPlatformCopyRemoval {
                agent_id: "amp".to_string(),
                skill_id: String::new(),
                paths: vec!["/home/alice/.agents/skills".to_string()],
            },
            DeletedPlatformCopyRemoval {
                agent_id: "amp".to_string(),
                skill_id: "..".to_string(),
                paths: vec!["/home/alice/.agents/skills/..".to_string()],
            },
            DeletedPlatformCopyRemoval {
                agent_id: "amp".to_string(),
                skill_id: SHARED_SKILL_ID.to_string(),
                paths: vec!["/home/alice/.agents/skills".to_string()],
            },
        ],
        &mut result,
        None,
        None,
    )
    .await;

    assert_eq!(runner.calls().len(), 0);
    assert!(result.removed_deleted_platform_copy_paths.is_empty());
    assert_eq!(result.failures.len(), 3);
    assert_eq!(
        db::get_skill_installations(&pool, SHARED_SKILL_ID)
            .await
            .unwrap()
            .len(),
        1
    );
}

struct CancellingRunner {
    inner: FakeRunner,
    cancel: Arc<AtomicBool>,
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

#[tokio::test]
async fn cancel_after_first_chunk_does_not_start_next_chunk() {
    let pool = mem_pool_with_home(REMOTE_HOME).await;
    let removals = (0..257)
        .map(|index| {
            let skill_id = format!("skill-{index:03}");
            let path = format!("/home/alice/.claude/skills/{skill_id}");
            DeletedPlatformCopyRemoval {
                agent_id: "claude-code".to_string(),
                skill_id,
                paths: vec![path],
            }
        })
        .collect::<Vec<_>>();
    let cancel = Arc::new(AtomicBool::new(false));
    let runner = Arc::new(CancellingRunner {
        inner: FakeRunner::new(),
        cancel: cancel.clone(),
    });
    runner.inner.push_success(&ok_rows(256));
    let target = RemoteTargetConfig {
        id: "ssh-leftover-cancel-test".to_string(),
        label: "Leftover SSH".to_string(),
        host: "example.invalid".to_string(),
        username: "tester".to_string(),
        port: 22,
        auth_method: SshAuthMethod::Key,
        key_path: "~/.ssh/id_ed25519".to_string(),
        credential_key: None,
        protected_password: None,
        password: None,
        remote_home: REMOTE_HOME.to_string(),
        remote_os: "linux".to_string(),
        symlink_enabled: true,
    };
    let connection = ConnectedRemoteTarget::Ssh(ConnectedSshTarget::for_tests_with_runner(
        target,
        runner.clone(),
    ));

    let mut result = SkillUpdateApplyResult::default();
    apply_remove_deleted_platform_copies_on_connection(
        &pool,
        &connection,
        removals,
        &mut result,
        None,
        Some(cancel.as_ref()),
    )
    .await;

    assert_eq!(runner.inner.calls().len(), 1);
    assert_eq!(result.removed_deleted_platform_copy_paths.len(), 256);
    assert_eq!(result.failures.len(), 1);
    assert_eq!(
        result.failures[0].error_code.as_deref(),
        Some("operation.cancelled")
    );
}

async fn seed_leftover_observation(pool: &db::DbPool, agent_id: &str, skill_id: &str, path: &Path) {
    db::upsert_agent_skill_observation(
        pool,
        &make_observation(agent_id, skill_id, &path.to_string_lossy()),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn ac9_ac10_lock_owned_canonical_is_excluded_unlocked_copy_still_listed() {
    let pool = crate::test_support::mem_pool().await;
    let temp = TempDir::new().unwrap();
    let universal = temp.path().join("universal");
    let locked = universal.join("locked-skill");
    let unlocked = universal.join("unlocked-skill");
    std::fs::create_dir_all(&locked).unwrap();
    std::fs::create_dir_all(&unlocked).unwrap();
    crate::test_support::set_agent_dir(&pool, "cursor", &universal).await;
    seed_leftover_observation(&pool, "cursor", "locked-skill", &locked).await;
    seed_leftover_observation(&pool, "cursor", "unlocked-skill", &unlocked).await;

    let lock_path = temp.path().join(".skill-lock.json");
    std::fs::write(&lock_path, r#"{"version":3,"skills":{"locked-skill":{}}}"#).unwrap();
    let ownership = crate::services::skills_cli::load_cli_lock_ownership(&lock_path).unwrap();

    let protected = scan_deleted_platform_copies_with_ownership(
        &pool,
        Some(vec!["cursor".to_string()]),
        Some(&ownership),
        &universal,
    )
    .await
    .unwrap();
    assert_eq!(protected.len(), 1);
    assert_eq!(protected[0].skill_id, "unlocked-skill");
    assert_eq!(
        protected[0].writable_paths,
        vec![unlocked.to_string_lossy().into_owned()]
    );

    let remote = scan_deleted_platform_copies_with_ownership(
        &pool,
        Some(vec!["cursor".to_string()]),
        None,
        &universal,
    )
    .await
    .unwrap();
    let remote_ids: Vec<&str> = remote.iter().map(|group| group.skill_id.as_str()).collect();
    assert!(remote_ids.contains(&"locked-skill"));
    assert!(remote_ids.contains(&"unlocked-skill"));
}

#[tokio::test]
async fn ac16_lock_named_mapped_copy_is_excluded_sibling_copy_still_listed() {
    let pool = crate::test_support::mem_pool().await;
    let temp = TempDir::new().unwrap();
    let universal = temp.path().join("universal");
    std::fs::create_dir_all(&universal).unwrap();
    let cursor_dir = temp.path().join("cursor-skills");
    let copy = cursor_dir.join("demo");
    let sibling = cursor_dir.join("other");
    std::fs::create_dir_all(&copy).unwrap();
    std::fs::create_dir_all(&sibling).unwrap();
    crate::test_support::set_agent_dir(&pool, "cursor", &cursor_dir).await;
    seed_leftover_observation(&pool, "cursor", "demo", &copy).await;
    seed_leftover_observation(&pool, "cursor", "other", &sibling).await;

    let lock_path = temp.path().join(".skill-lock.json");
    std::fs::write(&lock_path, r#"{"version":3,"skills":{"demo":{}}}"#).unwrap();
    let ownership = crate::services::skills_cli::load_cli_lock_ownership(&lock_path).unwrap();

    let protected = scan_deleted_platform_copies_with_ownership(
        &pool,
        Some(vec!["cursor".to_string()]),
        Some(&ownership),
        &universal,
    )
    .await
    .unwrap();
    assert_eq!(protected.len(), 1);
    assert_eq!(protected[0].skill_id, "other");

    let remote = scan_deleted_platform_copies_with_ownership(
        &pool,
        Some(vec!["cursor".to_string()]),
        None,
        &universal,
    )
    .await
    .unwrap();
    let remote_ids: Vec<&str> = remote.iter().map(|group| group.skill_id.as_str()).collect();
    assert!(remote_ids.contains(&"demo"));
    assert!(remote_ids.contains(&"other"));
}

#[tokio::test]
async fn ac2_remote_scan_does_not_use_local_lock_exclusion() {
    let pool = crate::test_support::mem_pool().await;
    let temp = TempDir::new().unwrap();
    let universal = temp.path().join("universal");
    let locked = universal.join("locked-skill");
    std::fs::create_dir_all(&locked).unwrap();
    crate::test_support::set_agent_dir(&pool, "cursor", &universal).await;
    seed_leftover_observation(&pool, "cursor", "locked-skill", &locked).await;

    let groups =
        scan_deleted_platform_copies_with_pool(&pool, Some(vec!["cursor".to_string()]), false)
            .await
            .unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].skill_id, "locked-skill");
}

#[tokio::test]
async fn ac15_leftover_local_apply_is_busy_while_default_lock_held() {
    let pool = crate::test_support::mem_pool().await;
    let temp = TempDir::new().unwrap();
    let cursor_dir = temp.path().join("cursor");
    let leftover = cursor_dir.join("gone-skill");
    std::fs::create_dir_all(&leftover).unwrap();
    crate::test_support::set_agent_dir(&pool, "cursor", &cursor_dir).await;

    let _holder = crate::services::central_mutation::acquire_central_mutation_guard_at(
        crate::paths::central_mutation_lock_path(),
        "hold leftover lock",
        std::time::Duration::from_secs(5),
    )
    .await
    .unwrap();

    set_leftover_guard_timeout(Some(std::time::Duration::ZERO));
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

    assert!(result.removed_deleted_platform_copy_paths.is_empty());
    assert_eq!(result.failures[0].phase.as_deref(), Some("mutation_lock"));
    assert!(leftover.exists());
}
