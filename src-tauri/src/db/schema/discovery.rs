//! Discovery schema：扫描目录配置 + 项目级技能发现表。
//!
//! - `scan_directories`：扫描目录注册表，含 `is_builtin` 区分内置 / 用户添加
//! - `discovered_skills`：项目级（非 Central）SKILL.md 发现结果，按 project +
//!   platform 检索

use crate::db::DbPool;

pub(super) async fn init(pool: &DbPool) -> Result<(), String> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS scan_directories (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            path       TEXT NOT NULL UNIQUE,
            label      TEXT,
            is_active  BOOLEAN NOT NULL DEFAULT 1,
            is_builtin BOOLEAN NOT NULL DEFAULT 0,
            added_at   TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    // discovered_skills — skills found in project-level directories during a
    // "discover project skills" scan.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS discovered_skills (
            id             TEXT PRIMARY KEY,
            name           TEXT NOT NULL,
            description    TEXT,
            file_path      TEXT NOT NULL,
            dir_path       TEXT NOT NULL,
            project_path   TEXT NOT NULL,
            project_name   TEXT NOT NULL,
            platform_id    TEXT NOT NULL,
            discovered_at  TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_discovered_skills_project_path
         ON discovered_skills(project_path)",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_discovered_skills_platform_id
         ON discovered_skills(platform_id)",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}
