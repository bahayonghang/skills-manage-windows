//! 项目级 skill 管理：add / scan / reconcile 三条核心路径的覆盖。
//!
//! 不覆盖 install/uninstall——那部分在阶段 3 落地，配套测试一起写。

use sqlx::SqlitePool;
use std::path::Path;
use tempfile::TempDir;

use crate::db::{self, DbPool, Skill};

use super::crud::{
    add_project_impl, get_project_skills_impl, install_skill_to_project_impl, list_projects_impl,
    list_projects_using_skill_impl, normalize_project_path, project_id_from_path,
    rename_project_impl, rescan_project_impl, set_project_pinned_impl,
    uninstall_skill_from_project_impl,
};

use crate::test_support::mem_pool as setup_test_db;
use crate::test_support::write_skill_md;

fn assert_path_equivalent(actual: &str, expected: &Path) {
    assert!(
        crate::paths::paths_equivalent(Path::new(actual), expected),
        "paths differ: actual={actual:?}, expected={expected:?}",
    );
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

#[test]
fn normalize_strips_windows_extended_length_prefixes() {
    assert_eq!(
        normalize_project_path(r"\\?\D:\Documents\Code\demo\"),
        "D:/Documents/Code/demo"
    );
    assert_eq!(
        normalize_project_path("//?/D:/Documents/Code/demo/"),
        "D:/Documents/Code/demo"
    );
    assert_eq!(
        normalize_project_path(r"\\?\UNC\server\share\demo\"),
        "//server/share/demo"
    );
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
    assert_eq!(skills[0].name, "brainstorming");
    assert_eq!(skills[0].description.as_deref(), Some("Test skill"));
    assert_path_equivalent(&skills[0].file_path, &claude_skill.join("SKILL.md"));
    assert_eq!(skills[0].source_origin, "project");
    assert_eq!(skills[0].link_type, "copy", "regular dir should be 'copy'");
}

#[tokio::test]
async fn rescan_finds_universal_skills_from_agents_dir() {
    let tmp = TempDir::new().unwrap();
    let pool = setup_test_db().await;

    let universal_skill = tmp.path().join(".agents/skills/brainstorming");
    write_skill_md(&universal_skill, "brainstorming", Some("Universal skill"));

    let project = add_project_impl(&pool, tmp.path().to_str().unwrap())
        .await
        .unwrap();
    let count = rescan_project_impl(&pool, &project.id).await.unwrap();
    assert_eq!(count, 1, "expected one Universal project skill scanned");

    let skills = get_project_skills_impl(&pool, &project.id).await.unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].agent_id, "codex");
    assert_eq!(skills[0].agent_display_name, "Codex CLI");
    assert_eq!(skills[0].skill_id, "brainstorming");
    assert_eq!(skills[0].description.as_deref(), Some("Universal skill"));
    assert_path_equivalent(&skills[0].installed_path, &universal_skill);
}

#[tokio::test]
async fn rescan_uses_antigravity_as_universal_representative_when_it_is_the_enabled_member() {
    let tmp = TempDir::new().unwrap();
    let pool = setup_test_db().await;

    sqlx::query("UPDATE agents SET is_enabled = CASE WHEN id = 'antigravity' THEN 1 ELSE 0 END")
        .execute(&pool)
        .await
        .unwrap();

    let universal_skill = tmp.path().join(".agents/skills/antigravity-only");
    write_skill_md(
        &universal_skill,
        "antigravity-only",
        Some("Antigravity Universal skill"),
    );

    let project = add_project_impl(&pool, tmp.path().to_str().unwrap())
        .await
        .unwrap();
    let count = rescan_project_impl(&pool, &project.id).await.unwrap();
    assert_eq!(count, 1, "expected one Antigravity Universal project skill");

    let skills = get_project_skills_impl(&pool, &project.id).await.unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].agent_id, "antigravity");
    assert_eq!(skills[0].agent_display_name, "Antigravity");
    assert_eq!(
        skills[0].description.as_deref(),
        Some("Antigravity Universal skill")
    );
    assert_path_equivalent(&skills[0].installed_path, &universal_skill);
}

#[tokio::test]
async fn rescan_uses_antigravity_cli_as_universal_representative_when_it_is_the_enabled_member() {
    let tmp = TempDir::new().unwrap();
    let pool = setup_test_db().await;

    sqlx::query(
        "UPDATE agents SET is_enabled = CASE WHEN id = 'antigravity-cli' THEN 1 ELSE 0 END",
    )
    .execute(&pool)
    .await
    .unwrap();

    let universal_skill = tmp.path().join(".agents/skills/antigravity-cli-only");
    write_skill_md(
        &universal_skill,
        "antigravity-cli-only",
        Some("Antigravity CLI Universal skill"),
    );

    let project = add_project_impl(&pool, tmp.path().to_str().unwrap())
        .await
        .unwrap();
    let count = rescan_project_impl(&pool, &project.id).await.unwrap();
    assert_eq!(
        count, 1,
        "expected one Antigravity CLI Universal project skill"
    );

    let skills = get_project_skills_impl(&pool, &project.id).await.unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].agent_id, "antigravity-cli");
    assert_eq!(skills[0].agent_display_name, "Antigravity CLI");
    assert_eq!(
        skills[0].description.as_deref(),
        Some("Antigravity CLI Universal skill")
    );
    assert_path_equivalent(&skills[0].installed_path, &universal_skill);
}

#[tokio::test]
async fn rescan_prefers_universal_agents_dir_over_legacy_member_paths() {
    let tmp = TempDir::new().unwrap();
    let pool = setup_test_db().await;

    let canonical_skill = tmp.path().join(".agents/skills/duplicate");
    let legacy_skill = tmp.path().join(".codex/skills/duplicate");
    write_skill_md(&canonical_skill, "duplicate", Some("Canonical Universal"));
    write_skill_md(&legacy_skill, "duplicate", Some("Legacy Codex"));

    let project = add_project_impl(&pool, tmp.path().to_str().unwrap())
        .await
        .unwrap();
    let count = rescan_project_impl(&pool, &project.id).await.unwrap();
    assert_eq!(count, 1, "duplicate Universal legacy paths should collapse");

    let skills = get_project_skills_impl(&pool, &project.id).await.unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].agent_id, "codex");
    assert_eq!(skills[0].skill_id, "duplicate");
    assert_eq!(
        skills[0].description.as_deref(),
        Some("Canonical Universal")
    );
    assert_path_equivalent(&skills[0].installed_path, &canonical_skill);
}

#[tokio::test]
async fn project_schema_migration_adds_metadata_columns_with_project_default() {
    // 豁免 test_support::mem_pool：本测试手工搭建 legacy schema 验证迁移，
    // 必须拿到未 init 的裸池。
    let pool = SqlitePool::connect(":memory:").await.unwrap();
    sqlx::query(
        "CREATE TABLE projects (
            id              TEXT PRIMARY KEY,
            path            TEXT NOT NULL UNIQUE,
            name            TEXT NOT NULL,
            pinned          BOOLEAN NOT NULL DEFAULT 0,
            added_at        TEXT NOT NULL,
            last_scanned_at TEXT
        )",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "CREATE TABLE project_skill_installations (
            project_id      TEXT NOT NULL,
            skill_id        TEXT NOT NULL,
            agent_id        TEXT NOT NULL,
            installed_path  TEXT NOT NULL,
            link_type       TEXT NOT NULL,
            symlink_target  TEXT,
            created_at      TEXT NOT NULL,
            PRIMARY KEY (project_id, skill_id, agent_id)
        )",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO projects (id, path, name, pinned, added_at, last_scanned_at)
         VALUES ('p1', '//?/D:/Code/demo', 'demo', 0, 'now', NULL)",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO project_skill_installations
         (project_id, skill_id, agent_id, installed_path, link_type, symlink_target, created_at)
         VALUES ('p1', 'legacy', 'claude-code', 'D:/Code/demo/.claude/skills/legacy', 'copy', NULL, 'now')",
    )
    .execute(&pool)
    .await
    .unwrap();

    db::init_database(&pool).await.unwrap();

    let source_origin: String = sqlx::query_scalar(
        "SELECT source_origin FROM project_skill_installations
         WHERE project_id = 'p1' AND skill_id = 'legacy'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(source_origin, "project");

    let project_path: String = sqlx::query_scalar("SELECT path FROM projects WHERE id = 'p1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(project_path, "D:/Code/demo");
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
    assert_eq!(rescan_project_impl(&pool, &project.id).await.unwrap(), 1);

    // 磁盘移除后再次扫描，psi 应被清空。
    std::fs::remove_dir_all(&claude_skill).unwrap();
    assert_eq!(rescan_project_impl(&pool, &project.id).await.unwrap(), 0);
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
async fn current_schema_reinit_does_not_replay_legacy_cleanup() {
    let pool = setup_test_db().await;

    // Migration 1 owns this legacy cleanup and must not replay on a current DB.
    sqlx::query("INSERT INTO settings(key, value) VALUES ('discover_scan_roots_config', '{}')")
        .execute(&pool)
        .await
        .unwrap();
    db::init_database(&pool).await.unwrap();

    let row = sqlx::query_scalar::<_, Option<String>>(
        "SELECT value FROM settings WHERE key = 'discover_scan_roots_config'",
    )
    .fetch_optional(&pool)
    .await
    .unwrap();
    assert_eq!(row, Some(Some("{}".to_string())));
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

// ─── Stage 3: install / uninstall ────────────────────────────────────────────

/// 准备中央 skill：在指定 canonical_dir 写 SKILL.md，并 upsert 进 skills 表。
async fn seed_central_skill(pool: &DbPool, canonical_dir: &Path, skill_id: &str) {
    crate::test_support::seed_central_skill(pool, canonical_dir, skill_id, "seed").await;
}

#[tokio::test]
async fn install_skill_copy_writes_psi_and_copies_dir() {
    let tmp = TempDir::new().unwrap();
    let pool = setup_test_db().await;

    // 中央 skill
    let canonical = tmp.path().join(".agents/skills/seeded");
    seed_central_skill(&pool, &canonical, "seeded").await;

    // 项目
    let project_root = tmp.path().join("proj");
    std::fs::create_dir_all(&project_root).unwrap();
    let project = add_project_impl(&pool, project_root.to_str().unwrap())
        .await
        .unwrap();

    let psi = install_skill_to_project_impl(&pool, &project.id, "seeded", "claude-code", "copy")
        .await
        .unwrap();

    assert_eq!(psi.link_type, "copy");
    assert!(psi.symlink_target.is_none());
    assert_eq!(psi.name, "seeded");
    assert_eq!(psi.description.as_deref(), Some("seed"));
    assert_path_equivalent(
        &psi.file_path,
        &project_root.join(".claude/skills/seeded/SKILL.md"),
    );
    assert_eq!(psi.source_origin, "central");

    let target = project_root.join(".claude/skills/seeded/SKILL.md");
    assert!(target.exists(), "copy should materialise SKILL.md");
    let meta = std::fs::symlink_metadata(project_root.join(".claude/skills/seeded")).unwrap();
    assert!(
        !meta.file_type().is_symlink(),
        "copy target must be a real directory"
    );

    // psi 行落库
    let row = db::get_project_skill_installation(&pool, &project.id, "seeded", "claude-code")
        .await
        .unwrap();
    assert!(row.is_some(), "psi row must exist after install");
    assert_eq!(row.unwrap().source_origin, "central");
}

#[tokio::test]
async fn rescan_preserves_central_origin_for_symlinked_central_install() {
    let tmp = TempDir::new().unwrap();
    let pool = setup_test_db().await;

    let canonical = tmp.path().join(".agents/skills/central-brain");
    seed_central_skill(&pool, &canonical, "central-brain").await;

    let project_root = tmp.path().join("proj");
    std::fs::create_dir_all(&project_root).unwrap();
    let project = add_project_impl(&pool, project_root.to_str().unwrap())
        .await
        .unwrap();

    let install_result = install_skill_to_project_impl(
        &pool,
        &project.id,
        "central-brain",
        "claude-code",
        "symlink",
    )
    .await;
    if let Err(err) = install_result {
        if err.to_string().to_lowercase().contains("symlink") {
            return;
        }
        panic!("unexpected install error: {err}");
    }

    rescan_project_impl(&pool, &project.id).await.unwrap();
    let skills = get_project_skills_impl(&pool, &project.id).await.unwrap();
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].source_origin, "central");
    assert_eq!(skills[0].name, "central-brain");
    assert_eq!(skills[0].description.as_deref(), Some("seed"));
}

#[tokio::test]
async fn install_skill_symlink_writes_psi_and_creates_link() {
    let tmp = TempDir::new().unwrap();
    let pool = setup_test_db().await;

    let canonical = tmp.path().join(".agents/skills/linker");
    seed_central_skill(&pool, &canonical, "linker").await;

    let project_root = tmp.path().join("proj");
    std::fs::create_dir_all(&project_root).unwrap();
    let project = add_project_impl(&pool, project_root.to_str().unwrap())
        .await
        .unwrap();

    let result =
        install_skill_to_project_impl(&pool, &project.id, "linker", "claude-code", "symlink").await;

    // Windows 非开发者模式下可能创建符号链接失败：把 error 当成测试预期跳过。
    // CI 上 Linux/macOS 应当能直接拿到 Ok。
    let psi = match result {
        Ok(p) => p,
        Err(err) if err.to_string().to_lowercase().contains("symlink") => return,
        Err(err) => panic!("unexpected install error: {err}"),
    };

    assert_eq!(psi.link_type, "symlink");
    assert!(psi.symlink_target.is_some());

    let link_path = project_root.join(".claude/skills/linker");
    let meta = std::fs::symlink_metadata(&link_path).unwrap();
    assert!(meta.file_type().is_symlink(), "target must be a symlink");
}

#[tokio::test]
async fn install_skill_rejects_non_central_skill() {
    let tmp = TempDir::new().unwrap();
    let pool = setup_test_db().await;

    // 写一条非 central 的 skill 记录
    let skill = Skill {
        id: "ghost".to_string(),
        uid: "ghost-uid".to_string(),
        name: "ghost".to_string(),
        description: None,
        file_path: tmp
            .path()
            .join("ghost/SKILL.md")
            .to_string_lossy()
            .into_owned(),
        canonical_path: None,
        is_central: false,
        source: None,
        content: None,
        scanned_at: chrono::Utc::now().to_rfc3339(),
        fs_created_at: None,
        fs_updated_at: None,
    };
    db::upsert_skill(&pool, &skill).await.unwrap();

    let project_root = tmp.path().join("proj");
    std::fs::create_dir_all(&project_root).unwrap();
    let project = add_project_impl(&pool, project_root.to_str().unwrap())
        .await
        .unwrap();

    let result =
        install_skill_to_project_impl(&pool, &project.id, "ghost", "claude-code", "copy").await;
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("not centralized") || err.contains("canonical_path"),
        "expected centralization error, got: {err}"
    );
}

#[tokio::test]
async fn install_skill_rejects_missing_skill() {
    let tmp = TempDir::new().unwrap();
    let pool = setup_test_db().await;

    let project_root = tmp.path().join("proj");
    std::fs::create_dir_all(&project_root).unwrap();
    let project = add_project_impl(&pool, project_root.to_str().unwrap())
        .await
        .unwrap();

    let result =
        install_skill_to_project_impl(&pool, &project.id, "no-such-skill", "claude-code", "copy")
            .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn install_skill_rejects_existing_real_dir_at_target() {
    let tmp = TempDir::new().unwrap();
    let pool = setup_test_db().await;

    let canonical = tmp.path().join(".agents/skills/clash");
    seed_central_skill(&pool, &canonical, "clash").await;

    let project_root = tmp.path().join("proj");
    let target_dir = project_root.join(".claude/skills/clash");
    std::fs::create_dir_all(&target_dir).unwrap();
    std::fs::write(target_dir.join("dummy.txt"), "manual").unwrap();

    let project = add_project_impl(&pool, project_root.to_str().unwrap())
        .await
        .unwrap();

    let result =
        install_skill_to_project_impl(&pool, &project.id, "clash", "claude-code", "copy").await;
    assert!(
        result.is_err(),
        "must refuse to overwrite existing real dir"
    );
}

#[tokio::test]
async fn uninstall_skill_removes_copy_and_psi() {
    let tmp = TempDir::new().unwrap();
    let pool = setup_test_db().await;

    let canonical = tmp.path().join(".agents/skills/dismantle");
    seed_central_skill(&pool, &canonical, "dismantle").await;

    let project_root = tmp.path().join("proj");
    std::fs::create_dir_all(&project_root).unwrap();
    let project = add_project_impl(&pool, project_root.to_str().unwrap())
        .await
        .unwrap();

    install_skill_to_project_impl(&pool, &project.id, "dismantle", "claude-code", "copy")
        .await
        .unwrap();
    let installed_path = project_root.join(".claude/skills/dismantle");
    assert!(installed_path.exists());

    uninstall_skill_from_project_impl(&pool, &project.id, "dismantle", "claude-code")
        .await
        .unwrap();

    assert!(
        !installed_path.exists(),
        "skill dir should be gone after uninstall"
    );
    let row = db::get_project_skill_installation(&pool, &project.id, "dismantle", "claude-code")
        .await
        .unwrap();
    assert!(row.is_none(), "psi row must be cleared after uninstall");
}

#[tokio::test]
async fn uninstall_skill_rejects_unknown_pair() {
    let tmp = TempDir::new().unwrap();
    let pool = setup_test_db().await;

    let project_root = tmp.path().join("proj");
    std::fs::create_dir_all(&project_root).unwrap();
    let project = add_project_impl(&pool, project_root.to_str().unwrap())
        .await
        .unwrap();

    let result =
        uninstall_skill_from_project_impl(&pool, &project.id, "never-installed", "claude-code")
            .await;
    assert!(result.is_err());
}

// ─── Stage 3.8 (deferred): reverse view ──────────────────────────────────────

#[tokio::test]
async fn list_projects_using_skill_returns_each_install() {
    let tmp = TempDir::new().unwrap();
    let pool = setup_test_db().await;

    let canonical = tmp.path().join(".agents/skills/cross");
    seed_central_skill(&pool, &canonical, "cross").await;

    let root_a = tmp.path().join("proj-a");
    let root_b = tmp.path().join("proj-b");
    std::fs::create_dir_all(&root_a).unwrap();
    std::fs::create_dir_all(&root_b).unwrap();
    let proj_a = add_project_impl(&pool, root_a.to_str().unwrap())
        .await
        .unwrap();
    let proj_b = add_project_impl(&pool, root_b.to_str().unwrap())
        .await
        .unwrap();

    install_skill_to_project_impl(&pool, &proj_a.id, "cross", "claude-code", "copy")
        .await
        .unwrap();
    install_skill_to_project_impl(&pool, &proj_b.id, "cross", "claude-code", "copy")
        .await
        .unwrap();

    let rows = list_projects_using_skill_impl(&pool, "cross")
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    let project_ids: std::collections::HashSet<_> =
        rows.iter().map(|r| r.project_id.as_str()).collect();
    assert!(project_ids.contains(proj_a.id.as_str()));
    assert!(project_ids.contains(proj_b.id.as_str()));
    assert!(rows.iter().all(|r| r.agent_display_name == "Claude Code"));
    assert!(rows.iter().all(|r| r.link_type == "copy"));
}

#[tokio::test]
async fn list_projects_using_skill_pinned_first() {
    let tmp = TempDir::new().unwrap();
    let pool = setup_test_db().await;

    let canonical = tmp.path().join(".agents/skills/sorted");
    seed_central_skill(&pool, &canonical, "sorted").await;

    let root_z = tmp.path().join("zproj");
    let root_a = tmp.path().join("aproj");
    std::fs::create_dir_all(&root_z).unwrap();
    std::fs::create_dir_all(&root_a).unwrap();
    let zproj = add_project_impl(&pool, root_z.to_str().unwrap())
        .await
        .unwrap();
    let aproj = add_project_impl(&pool, root_a.to_str().unwrap())
        .await
        .unwrap();

    // 给 z 项目 rename + pin，给 a 项目不 pin
    super::crud::rename_project_impl(&pool, &zproj.id, "Zeta")
        .await
        .unwrap();
    super::crud::rename_project_impl(&pool, &aproj.id, "Alpha")
        .await
        .unwrap();
    set_project_pinned_impl(&pool, &zproj.id, true)
        .await
        .unwrap();

    install_skill_to_project_impl(&pool, &zproj.id, "sorted", "claude-code", "copy")
        .await
        .unwrap();
    install_skill_to_project_impl(&pool, &aproj.id, "sorted", "claude-code", "copy")
        .await
        .unwrap();

    let rows = list_projects_using_skill_impl(&pool, "sorted")
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows[0].project_name, "Zeta",
        "pinned project must come first"
    );
    assert_eq!(rows[1].project_name, "Alpha");
}

#[tokio::test]
async fn list_projects_using_skill_empty_for_unused_skill() {
    let pool = setup_test_db().await;
    let rows = list_projects_using_skill_impl(&pool, "ghost")
        .await
        .unwrap();
    assert!(rows.is_empty());
}
