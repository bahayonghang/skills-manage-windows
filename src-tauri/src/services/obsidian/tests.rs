#![cfg(test)]
//! Obsidian 域首批 service 层测试：源模式导入（symlink / copy）主路径。
//!
//! fixture 全部来自 `crate::test_support`（mem_pool + set_agent_dir +
//! write_skill_md），作为「新域首条测试成本骤降」的 harness 验收演示。
//! vault 扫描侧的纯函数测试见 `query.rs` 尾部内联 mod。

use tempfile::TempDir;

use crate::db;
use crate::test_support::{mem_pool, set_agent_dir, write_skill_md};

use super::{import_obsidian_skill_to_platform_impl, ObsidianError};

/// 建一个池 + 把 claude-code 平台目录重定向到 tmp 下，返回 (pool, agent_dir)。
async fn setup_platform(tmp: &TempDir) -> (crate::db::DbPool, std::path::PathBuf) {
    let agent_dir = tmp.path().join("claude-skills");
    std::fs::create_dir_all(&agent_dir).unwrap();
    let pool = mem_pool().await;
    set_agent_dir(&pool, "claude-code", &agent_dir).await;
    (pool, agent_dir)
}

fn vault_skill(tmp: &TempDir, id: &str) -> std::path::PathBuf {
    write_skill_md(
        &tmp.path().join("vault/.skills").join(id),
        id,
        Some("From vault"),
    )
}

#[tokio::test]
async fn platform_import_symlink_creates_link_and_rows() {
    let tmp = TempDir::new().unwrap();
    let (pool, agent_dir) = setup_platform(&tmp).await;
    let source = vault_skill(&tmp, "alpha");

    let result = import_obsidian_skill_to_platform_impl(
        &pool,
        source.to_str().unwrap(),
        "claude-code",
        Some("symlink"),
    )
    .await
    .unwrap();

    assert_eq!(result.skill_id, "alpha");
    assert_eq!(result.target, "claude-code");

    let link = agent_dir.join("alpha");
    let meta = std::fs::symlink_metadata(&link).unwrap();
    assert!(
        meta.file_type().is_symlink(),
        "import should create a symlink"
    );

    let skill = db::get_skill_by_id(&pool, "alpha").await.unwrap().unwrap();
    assert!(!skill.is_central);
    assert_eq!(skill.source.as_deref(), Some("symlink"));
    assert_eq!(skill.description.as_deref(), Some("From vault"));

    let installs = db::get_skill_installations(&pool, "alpha").await.unwrap();
    assert_eq!(installs.len(), 1);
    assert_eq!(installs[0].agent_id, "claude-code");
    assert_eq!(installs[0].link_type, "symlink");
    assert_eq!(installs[0].symlink_target.as_deref(), source.to_str());
}

#[tokio::test]
async fn platform_import_copy_copies_dir() {
    let tmp = TempDir::new().unwrap();
    let (pool, agent_dir) = setup_platform(&tmp).await;
    let source = vault_skill(&tmp, "alpha");

    import_obsidian_skill_to_platform_impl(
        &pool,
        source.to_str().unwrap(),
        "claude-code",
        Some("copy"),
    )
    .await
    .unwrap();

    let target = agent_dir.join("alpha");
    let meta = std::fs::symlink_metadata(&target).unwrap();
    assert!(
        meta.file_type().is_dir(),
        "copy import should materialize a dir"
    );
    assert!(target.join("SKILL.md").exists());

    let skill = db::get_skill_by_id(&pool, "alpha").await.unwrap().unwrap();
    assert_eq!(skill.source.as_deref(), Some("copy"));

    let installs = db::get_skill_installations(&pool, "alpha").await.unwrap();
    assert_eq!(installs.len(), 1);
    assert_eq!(installs[0].link_type, "copy");
    assert!(installs[0].symlink_target.is_none());
}

#[tokio::test]
async fn platform_import_defaults_to_symlink() {
    let tmp = TempDir::new().unwrap();
    let (pool, agent_dir) = setup_platform(&tmp).await;
    let source = vault_skill(&tmp, "alpha");

    import_obsidian_skill_to_platform_impl(&pool, source.to_str().unwrap(), "claude-code", None)
        .await
        .unwrap();

    let meta = std::fs::symlink_metadata(agent_dir.join("alpha")).unwrap();
    assert!(
        meta.file_type().is_symlink(),
        "method 缺省应按 symlink 处理"
    );
}

#[tokio::test]
async fn platform_import_rejects_existing_target() {
    let tmp = TempDir::new().unwrap();
    let (pool, agent_dir) = setup_platform(&tmp).await;
    let source = vault_skill(&tmp, "alpha");
    std::fs::create_dir_all(agent_dir.join("alpha")).unwrap();

    let err = import_obsidian_skill_to_platform_impl(
        &pool,
        source.to_str().unwrap(),
        "claude-code",
        Some("copy"),
    )
    .await
    .unwrap_err();

    assert!(matches!(err, ObsidianError::SkillExistsInAgent { .. }));
}

#[tokio::test]
async fn platform_import_unknown_agent_is_typed() {
    let tmp = TempDir::new().unwrap();
    let (pool, _agent_dir) = setup_platform(&tmp).await;
    let source = vault_skill(&tmp, "alpha");

    let err = import_obsidian_skill_to_platform_impl(
        &pool,
        source.to_str().unwrap(),
        "no-such-agent",
        Some("copy"),
    )
    .await
    .unwrap_err();

    assert!(matches!(err, ObsidianError::AgentNotFound(id) if id == "no-such-agent"));
}

#[tokio::test]
async fn platform_import_rejects_unknown_method() {
    let tmp = TempDir::new().unwrap();
    let (pool, _agent_dir) = setup_platform(&tmp).await;
    let source = vault_skill(&tmp, "alpha");

    let err = import_obsidian_skill_to_platform_impl(
        &pool,
        source.to_str().unwrap(),
        "claude-code",
        Some("hardlink"),
    )
    .await
    .unwrap_err();

    assert!(matches!(err, ObsidianError::UnsupportedInstallMethod(m) if m == "hardlink"));
}

/// 路径给不出目录名（根路径）→ 类型化错误而非 panic。
#[tokio::test]
async fn platform_import_root_path_has_no_dir_name() {
    let tmp = TempDir::new().unwrap();
    let (pool, _agent_dir) = setup_platform(&tmp).await;

    let err = import_obsidian_skill_to_platform_impl(&pool, "/", "claude-code", Some("copy"))
        .await
        .unwrap_err();

    assert!(matches!(err, ObsidianError::SkillDirNameUnavailable));
}
