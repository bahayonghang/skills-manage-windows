//! Projects schema：手动注册的项目 + 项目下技能安装记录。
//!
//! 替代旧 `discovered_skills` 全盘扫描模型：项目作为一等实体，由用户手动 add；
//! `project_skill_installations` 记录每个项目下各 agent 目录中已落盘的 skill。
//!
//! 同时清理旧 Discover 残留：
//! - 清空 `discovered_skills` 表（保留表结构以兼容回退）
//! - 删除 `settings.discover_scan_roots_config` 行

use crate::db::DbPool;

pub(super) async fn init(pool: &DbPool) -> Result<(), String> {
    // projects：用户手动 add 的项目根目录。
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS projects (
            id              TEXT PRIMARY KEY,
            path            TEXT NOT NULL UNIQUE,
            name            TEXT NOT NULL,
            pinned          BOOLEAN NOT NULL DEFAULT 0,
            added_at        TEXT NOT NULL,
            last_scanned_at TEXT
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    // project_skill_installations：项目下某个 agent 目录下登记的 skill 安装。
    // 复合主键覆盖「同一项目同一 skill 装到不同 agent 目录」的合法场景；
    // ON DELETE CASCADE 让删项目时自动清掉 psi 残留。
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS project_skill_installations (
            project_id      TEXT NOT NULL,
            skill_id        TEXT NOT NULL,
            agent_id        TEXT NOT NULL,
            installed_path  TEXT NOT NULL,
            link_type       TEXT NOT NULL,
            symlink_target  TEXT,
            created_at      TEXT NOT NULL,
            PRIMARY KEY (project_id, skill_id, agent_id),
            FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_psi_project
         ON project_skill_installations(project_id)",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    // 旧 Discover 残留清理：
    // - discovered_skills 表只清空数据，保留表本身（防止回退老版本时 schema 校验挂）。
    // - settings 里旧的扫描根配置直接删，没有兼容价值。
    sqlx::query("DELETE FROM discovered_skills")
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    sqlx::query("DELETE FROM settings WHERE key = 'discover_scan_roots_config'")
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}
