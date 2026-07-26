//! Projects schema：手动注册的项目 + 项目下技能安装记录。
//!
//! 替代旧 `discovered_skills` 全盘扫描模型：项目作为一等实体，由用户手动 add；
//! `project_skill_installations` 记录每个项目下各 agent 目录中已落盘的 skill。
//!
//! 旧 Discover 表结构的清理在 `schema/discovery.rs` 里完成（drop discovered_skills
//! 与其索引）；这里只清剩下的 `settings.discover_scan_roots_config` 一行。

use super::super::migrations::versions::v1::ensure_column;
use sqlx::SqliteConnection;

pub(super) async fn init(connection: &mut SqliteConnection) -> Result<(), sqlx::Error> {
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
    .execute(&mut *connection)
    .await?;

    // project_skill_installations：项目下某个 agent 目录下登记的 skill 安装。
    // 复合主键覆盖「同一项目同一 skill 装到不同 agent 目录」的合法场景；
    // ON DELETE CASCADE 让删项目时自动清掉 psi 残留。
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS project_skill_installations (
            project_id      TEXT NOT NULL,
            skill_id        TEXT NOT NULL,
            name            TEXT NOT NULL DEFAULT '',
            description     TEXT,
            file_path       TEXT NOT NULL DEFAULT '',
            source_origin   TEXT NOT NULL DEFAULT 'project',
            agent_id        TEXT NOT NULL,
            installed_path  TEXT NOT NULL,
            link_type       TEXT NOT NULL,
            symlink_target  TEXT,
            created_at      TEXT NOT NULL,
            PRIMARY KEY (project_id, skill_id, agent_id),
            FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
        )",
    )
    .execute(&mut *connection)
    .await?;

    let alter_specs: &[(&str, &str, &str)] = &[
        (
            "project_skill_installations",
            "name",
            "ALTER TABLE project_skill_installations ADD COLUMN name TEXT NOT NULL DEFAULT ''",
        ),
        (
            "project_skill_installations",
            "description",
            "ALTER TABLE project_skill_installations ADD COLUMN description TEXT",
        ),
        (
            "project_skill_installations",
            "file_path",
            "ALTER TABLE project_skill_installations ADD COLUMN file_path TEXT NOT NULL DEFAULT ''",
        ),
        (
            "project_skill_installations",
            "source_origin",
            "ALTER TABLE project_skill_installations ADD COLUMN source_origin TEXT NOT NULL DEFAULT 'project'",
        ),
    ];
    for (table, column, alter_sql) in alter_specs {
        ensure_column(connection, table, column, alter_sql).await?;
    }

    repair_extended_project_paths(connection).await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_psi_project
         ON project_skill_installations(project_id)",
    )
    .execute(&mut *connection)
    .await?;

    sqlx::query("DELETE FROM settings WHERE key = 'discover_scan_roots_config'")
        .execute(&mut *connection)
        .await?;

    Ok(())
}

async fn repair_extended_project_paths(
    connection: &mut SqliteConnection,
) -> Result<(), sqlx::Error> {
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT id, path FROM projects WHERE path LIKE '//?/%'",
    )
    .fetch_all(&mut *connection)
    .await?;

    for (id, path) in rows {
        let cleaned = crate::paths::normalize_stored_path(&path);
        if cleaned != path {
            sqlx::query("UPDATE projects SET path = ? WHERE id = ?")
                .bind(cleaned)
                .bind(id)
                .execute(&mut *connection)
                .await?;
        }
    }

    Ok(())
}
