//! Remote force-remove / relative-symlink classification coverage.

use std::sync::Arc;

use super::link::unlink_platform;
use super::mutate_tests::{
    agent_dir, call_count, four_platform_pool, lock_json, probe_line, remote_tx, stdin_of,
    wipe_skill_recovery, HOME,
};
use super::remove::{preview_remove_global, remove_global};
use crate::test_support::FakeRunner;

#[tokio::test]
async fn official_relative_symlink_is_managed_and_confirmable() {
    wipe_skill_recovery("ask-matt");
    let pool = four_platform_pool().await;
    let claude = agent_dir(&pool, "claude-code").await;
    let zed = agent_dir(&pool, "zed").await;
    let canonical = format!("{HOME}/.agents/skills/ask-matt");
    let relative_slot = format!("{claude}/ask-matt");
    let copy = format!("{zed}/ask-matt");
    let runner = Arc::new(FakeRunner::new());
    runner.push_success(&lock_json(&["ask-matt"]));
    runner.push_success(&format!(
        "{}{}{}",
        probe_line(&canonical, "dir", ""),
        probe_line(&relative_slot, "link", "../../.agents/skills/ask-matt"),
        probe_line(&copy, "dir", ""),
    ));
    let tx = remote_tx(runner);
    let plan = preview_remove_global(&tx, &pool, "ask-matt").await.unwrap();
    assert!(plan.confirmable);
    assert!(plan.conflicts.is_empty());
    assert!(
        plan.managed_placements
            .iter()
            .any(|item| item.agent_id == "claude-code"),
        "{:?}",
        plan.managed_placements
    );
    assert!(
        plan.retained_direct_copies
            .iter()
            .any(|item| item.agent_id == "zed"),
        "{:?}",
        plan.retained_direct_copies
    );
}

#[tokio::test]
async fn force_remove_unlinks_wrong_target_and_keeps_direct_copy() {
    wipe_skill_recovery("ask-matt");
    let pool = four_platform_pool().await;
    let claude = agent_dir(&pool, "claude-code").await;
    let zed = agent_dir(&pool, "zed").await;
    let canonical = format!("{HOME}/.agents/skills/ask-matt");
    let conflict_slot = format!("{claude}/ask-matt");
    let copy = format!("{zed}/ask-matt");
    let foreign = format!("{HOME}/.skillsmanage/skills/ask-matt");
    let runner = Arc::new(FakeRunner::new());
    runner.push_success(&lock_json(&["ask-matt"]));
    let mut probe = String::new();
    probe.push_str(&probe_line(&canonical, "dir", ""));
    probe.push_str(&probe_line(&conflict_slot, "link", &foreign));
    probe.push_str(&probe_line(&copy, "dir", ""));
    runner.push_success(&probe);
    let lock = lock_json(&["ask-matt"]);
    runner.push_success(&lock);
    runner.push_success("");
    runner.push_success(&format!("{conflict_slot}\tremoved\n"));
    runner.push_success(&lock);
    for _ in 0..8 {
        runner.push_success("");
    }
    let tx = remote_tx(runner.clone());
    let result = remove_global(&tx, &pool, "ask-matt", true, None)
        .await
        .unwrap();
    assert!(result.removed_canonical);
    assert!(result
        .removed_managed_agent_ids
        .contains(&"claude-code".to_string()));
    assert!(result
        .retained_direct_copy_agent_ids
        .contains(&"zed".to_string()));
    let joined: String = (0..call_count(runner.as_ref()))
        .map(|index| stdin_of(runner.as_ref(), index))
        .collect();
    assert!(joined.contains("SKILLPORT_VERIFIED_LINK_REMOVE"));
    assert!(joined.contains(&conflict_slot));
    for index in 0..call_count(runner.as_ref()) {
        let stdin = stdin_of(runner.as_ref(), index);
        if stdin.contains("SKILLPORT_VERIFIED_LINK_REMOVE") {
            assert!(
                !stdin.contains(&copy),
                "direct copy must not enter the verified-remove path list: {stdin}"
            );
            assert!(
                !stdin.contains(&foreign),
                "force unlink must not follow the Central target: {stdin}"
            );
        }
        if stdin.contains("rm -rf") {
            assert!(
                stdin.contains(".skillport-remove-"),
                "rm -rf must stay on SkillPort backup paths: {stdin}"
            );
            assert!(!stdin.contains(&copy), "{stdin}");
            assert!(!stdin.contains(&foreign), "{stdin}");
        }
    }
}

#[tokio::test]
async fn force_unlink_skips_ordinary_directory() {
    wipe_skill_recovery("copy-slot");
    let pool = four_platform_pool().await;
    let zed = agent_dir(&pool, "zed").await;
    let canonical = format!("{HOME}/.agents/skills/copy-slot");
    let slot = format!("{zed}/copy-slot");
    let runner = Arc::new(FakeRunner::new());
    runner.push_success(&lock_json(&["copy-slot"]));
    runner.push_success(&format!(
        "{}{}",
        probe_line(&canonical, "dir", ""),
        probe_line(&slot, "dir", "")
    ));
    let tx = remote_tx(runner.clone());
    let error = unlink_platform(&tx, &pool, "copy-slot", "zed", true, None)
        .await
        .unwrap_err();
    assert_eq!(error.ipc_code(), "skills_cli.direct_copy_not_toggleable");
    assert_eq!(tx.write_count(), 0);
    let joined: String = (0..call_count(runner.as_ref()))
        .map(|index| stdin_of(runner.as_ref(), index))
        .collect();
    assert!(!joined.contains("SKILLPORT_VERIFIED_LINK_REMOVE"));
    assert!(!joined.contains("rm -rf"));
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

#[tokio::test]
async fn preview_remove_copy_mode_without_canonical_is_confirmable() {
    wipe_skill_recovery("demo");
    let pool = four_platform_pool().await;
    let zed = agent_dir(&pool, "zed").await;
    let canonical = format!("{HOME}/.agents/skills/demo");
    let slot = format!("{zed}/demo");
    let runner = Arc::new(FakeRunner::new());
    runner.push_success(&lock_json(&["demo"]));
    runner.push_success(&format!(
        "{}{}",
        probe_line(&canonical, "absent", ""),
        probe_line(&slot, "dir", "")
    ));
    let tx = remote_tx(runner);
    let plan = preview_remove_global(&tx, &pool, "demo").await.unwrap();
    assert!(plan.confirmable);
    assert!(!plan.owned_canonical);
    assert!(plan.conflicts.is_empty());
    assert!(plan.managed_placements.is_empty());
    assert_eq!(plan.retained_direct_copies[0].agent_id, "zed");
    assert_eq!(tx.write_count(), 0);
}
