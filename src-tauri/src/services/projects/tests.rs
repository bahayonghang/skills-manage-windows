//! 项目级 skill 管理：add / scan / reconcile 三条核心路径的覆盖。
//!
//! 不覆盖 install/uninstall——那部分在阶段 3 落地，配套测试一起写。

use sqlx::SqlitePool;
use std::path::Path;
use tempfile::TempDir;

use crate::db::{self, DbPool};

use super::crud::{
    add_project_impl, get_project_skills_impl, list_projects_impl, normalize_project_path,
    project_id_from_path, rename_project_impl, rescan_project_impl, set_project_pinned_impl,
};

async fn setup_test_db() -> DbPool {
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    db::init_database(&pool).await.unwrap();
    pool
}

fn write_skill_md(dir: &Path, name: &str, description: Option<&str>) {
    std::fs::create_dir_all(dir).unwrap();
    let body = match description {
        Some(d) => format!("---\nname: {name}\ndescription: {d}\n---\n\n# {name}\n"),
        None => format!("---\nname: {name}\n---\n\n# {name}\n"),
    };
    std::fs::write(dir.join("SKILL.md"), body).unwrap();
}

#[test]
fn project_id_is_stable_for_same_path() {
    let path = "/tmp/whatever/project";
    let a = project_id_from_path(path);
    let b = project_id_from_path(path);
    assert_eq!(a, b);
    assert_eq!(a.len(), 16, "id should be 16 hex chars");
}

#[test]
fn project_id_differs_across_paths() {
    let a = project_id_from_path("/tmp/a/project");
    let b = project_id_from_path("/tmp/b/project");
    assert_ne!(a, b);
}

#[test]
fn normalize_strips_trailing_slash_and_unifies_separators() {
    let n = normalize_project_path(r"C:\Users\lyh\code\foo\");
    assert!(!n.ends_with('/'));
    assert!(!n.contains('\\'));
}

#[tokio::test]
async fn add_project_returns_existing_for_duplicate_path() {
    let tmp = TempDir::new().unwrap();
    let pool = setup_test_db().await;

    let first = add_project_impl(&pool, tmp.path().to_str().unwrap())
        .await
        .unwrap();
    let second = add_project_impl(&pool, tmp.path().to_str().unwrap())
        .await
        .unwrap();

    assert_eq!(first.id, second.id);
    assert_eq!(first.path, second.path);
}

#[tokio::test]
async fn add_project_rejects_nonexistent_path() {
    let pool = setup_test_db().await;
    let result = add_project_impl(&pool, "/this/path/should/not/exist/zzz").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn rescan_finds_skills_in_enabled_agent_dirs() {
    let tmp = TempDir::new().unwrap();
    let pool = setup_test_db().await;

    // 写一个 claude-code 项目级 skill（默认 enabled）。
    let claude_skill = tmp.path().join(".claude/skills/brainstorming");
    write_skill_md(&claude_skill, "brainstorming", Some("Test skill"));

    let project = add_project_impl(&pool, tmp.path().to_str().unwrap())
        .await
        .unwrap();
    let count = rescan_project_impl(&pool, &project.id).await.unwrap();
    assert_eq!(count, 1, "expected exactly one project skill scanned");

    let skills = get_project_skills_impl(&pool, &project.id).await.unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].agent_id, "claude-code");
    assert_eq!(skills[0].skill_id, "brainstorming");
    assert_eq!(skills[0].link_type, "copy", "regular dir should be 'copy'");
}

#[tokio::test]
async fn rescan_skips_disabled_agents() {
    let tmp = TempDir::new().unwrap();
    let pool = setup_test_db().await;

    // 准备一个非默认启用的 agent（cursor 默认不在 DEFAULT_ENABLED_PLATFORM_IDS）
    // 的 skill。
    let cursor_skill = tmp.path().join(".cursor/skills/foo");
    write_skill_md(&cursor_skill, "foo", None);

    let project = add_project_impl(&pool, tmp.path().to_str().unwrap())
        .await
        .unwrap();
    let count = rescan_project_impl(&pool, &project.id).await.unwrap();
    assert_eq!(count, 0, "disabled agent's dir should be ignored");
}

#[tokio::test]
async fn rescan_reconciles_psi_after_disk_removal() {
    let tmp = TempDir::new().unwrap();
    let pool = setup_test_db().await;

    let claude_skill = tmp.path().join(".claude/skills/will-be-removed");
    write_skill_md(&claude_skill, "will-be-removed", None);

    let project = add_project_impl(&pool, tmp.path().to_str().unwrap())
        .await
        .unwrap();
    assert_eq!(
        rescan_project_impl(&pool, &project.id).await.unwrap(),
        1
    );

    // 磁盘移除后再次扫描，psi 应被清空。
    std::fs::remove_dir_all(&claude_skill).unwrap();
    assert_eq!(
        rescan_project_impl(&pool, &project.id).await.unwrap(),
        0
    );
    let skills = get_project_skills_impl(&pool, &project.id).await.unwrap();
    assert!(skills.is_empty(), "psi should be reconciled after removal");
}

#[tokio::test]
async fn list_projects_orders_pinned_first() {
    let tmp_a = TempDir::new().unwrap();
    let tmp_b = TempDir::new().unwrap();
    let pool = setup_test_db().await;

    let a = add_project_impl(&pool, tmp_a.path().to_str().unwrap())
        .await
        .unwrap();
    let _b = add_project_impl(&pool, tmp_b.path().to_str().unwrap())
        .await
        .unwrap();

    set_project_pinned_impl(&pool, &a.id, true).await.unwrap();

    let list = list_projects_impl(&pool).await.unwrap();
    assert!(list[0].pinned, "pinned project should sort first");
    assert_eq!(list[0].id, a.id);
}

#[tokio::test]
async fn rename_project_updates_name() {
    let tmp = TempDir::new().unwrap();
    let pool = setup_test_db().await;
    let p = add_project_impl(&pool, tmp.path().to_str().unwrap())
        .await
        .unwrap();

    rename_project_impl(&pool, &p.id, "my-fancy-name")
        .await
        .unwrap();

    let refreshed = db::get_project_by_id(&pool, &p.id).await.unwrap().unwrap();
    assert_eq!(refreshed.name, "my-fancy-name");
}

#[tokio::test]
async fn rename_project_rejects_empty_name() {
    let tmp = TempDir::new().unwrap();
    let pool = setup_test_db().await;
    let p = add_project_impl(&pool, tmp.path().to_str().unwrap())
        .await
        .unwrap();

    let result = rename_project_impl(&pool, &p.id, "   ").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn cleanup_removes_old_discover_settings() {
    // schema init 应当清空 discovered_skills 表和 discover_scan_roots_config 设置。
    let pool = setup_test_db().await;

    // 模拟旧 settings 残留：先写一行，再二次 init，应被清除。
    sqlx::query("INSERT INTO settings(key, value) VALUES ('discover_scan_roots_config', '{}')")
        .execute(&pool)
        .await
        .unwrap();
    db::init_database(&pool).await.unwrap();

    let row =
        sqlx::query_scalar::<_, Option<String>>(
            "SELECT value FROM settings WHERE key = 'discover_scan_roots_config'",
        )
        .fetch_optional(&pool)
        .await
        .unwrap();
    assert!(row.is_none(), "old discover scan roots config should be gone");
}

#[tokio::test]
#[cfg(unix)]
async fn rescan_detects_symlinked_skills() {
    use std::os::unix::fs::symlink;

    let tmp = TempDir::new().unwrap();
    let pool = setup_test_db().await;

    // 源 skill 目录
    let canonical = tmp.path().join(".source/brain");
    write_skill_md(&canonical, "brain", None);

    // 项目目录下用 symlink 指向它
    let project_root = tmp.path().join("proj");
    std::fs::create_dir_all(project_root.join(".claude/skills")).unwrap();
    let link = project_root.join(".claude/skills/brain");
    symlink(&canonical, &link).unwrap();

    let project = add_project_impl(&pool, project_root.to_str().unwrap())
        .await
        .unwrap();
    rescan_project_impl(&pool, &project.id).await.unwrap();

    let skills = get_project_skills_impl(&pool, &project.id).await.unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(
        skills[0].link_type, "symlink",
        "symlinked entry should be tagged as symlink"
    );
    assert!(skills[0].symlink_target.is_some());
}
