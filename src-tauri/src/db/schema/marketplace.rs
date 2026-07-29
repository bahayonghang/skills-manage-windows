//! Marketplace schema：远端源、缓存技能、AI 解释 + 8 条 ALTER 列迁移。
//!
//! - `skill_registries`：远端源 metadata（GitHub repo / 镜像）
//! - `marketplace_skills`：缓存的远端技能列表，FK → registries(id)
//! - `skill_explanations`：AI 解释缓存，按 (skill_id, lang) 双主键
//!
//! 老库迁移：早期版本 `skill_registries` / `marketplace_skills` 字段较少，
//! 通过 [`super::super::migrations::ensure_column`] 幂等增补。

use super::super::migrations::versions::v1::ensure_column;
use sqlx::SqliteConnection;

pub(super) async fn init(connection: &mut SqliteConnection) -> Result<(), sqlx::Error> {
    // skill_registries — remote skill sources (marketplace).
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS skill_registries (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL,
            source_type TEXT NOT NULL,
            url         TEXT NOT NULL,
            is_builtin  BOOLEAN NOT NULL DEFAULT 0,
            is_enabled  BOOLEAN NOT NULL DEFAULT 1,
            last_synced TEXT,
            last_attempted_sync TEXT,
            last_sync_status TEXT NOT NULL DEFAULT 'never',
            last_sync_error TEXT,
            cache_updated_at TEXT,
            cache_expires_at TEXT,
            etag TEXT,
            last_modified TEXT,
            created_at  TEXT NOT NULL
        )",
    )
    .execute(&mut *connection)
    .await?;

    // marketplace_skills — cached remote skill metadata.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS marketplace_skills (
            id           TEXT PRIMARY KEY,
            registry_id  TEXT NOT NULL,
            name         TEXT NOT NULL,
            description  TEXT,
            download_url TEXT NOT NULL,
            is_installed BOOLEAN NOT NULL DEFAULT 0,
            synced_at    TEXT NOT NULL,
            cache_updated_at TEXT,
            FOREIGN KEY (registry_id) REFERENCES skill_registries(id)
        )",
    )
    .execute(&mut *connection)
    .await?;

    // skill_explanations — cached AI-generated skill explanations.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS skill_explanations (
            skill_id    TEXT NOT NULL,
            explanation TEXT NOT NULL,
            lang        TEXT NOT NULL DEFAULT 'zh',
            model       TEXT,
            created_at  TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at  TEXT NOT NULL DEFAULT (datetime('now')),
            PRIMARY KEY (skill_id, lang)
        )",
    )
    .execute(&mut *connection)
    .await?;

    // 老库 skill_registries / marketplace_skills 缺列：逐列幂等增补。
    let alter_specs: &[(&str, &str, &str)] = &[
        (
            "skill_registries",
            "last_attempted_sync",
            "ALTER TABLE skill_registries ADD COLUMN last_attempted_sync TEXT",
        ),
        (
            "skill_registries",
            "last_sync_status",
            "ALTER TABLE skill_registries ADD COLUMN last_sync_status TEXT NOT NULL DEFAULT 'never'",
        ),
        (
            "skill_registries",
            "last_sync_error",
            "ALTER TABLE skill_registries ADD COLUMN last_sync_error TEXT",
        ),
        (
            "skill_registries",
            "cache_updated_at",
            "ALTER TABLE skill_registries ADD COLUMN cache_updated_at TEXT",
        ),
        (
            "skill_registries",
            "cache_expires_at",
            "ALTER TABLE skill_registries ADD COLUMN cache_expires_at TEXT",
        ),
        (
            "skill_registries",
            "etag",
            "ALTER TABLE skill_registries ADD COLUMN etag TEXT",
        ),
        (
            "skill_registries",
            "last_modified",
            "ALTER TABLE skill_registries ADD COLUMN last_modified TEXT",
        ),
        (
            "marketplace_skills",
            "cache_updated_at",
            "ALTER TABLE marketplace_skills ADD COLUMN cache_updated_at TEXT",
        ),
    ];
    for (table, column, alter_sql) in alter_specs {
        ensure_column(connection, table, column, alter_sql).await?;
    }

    Ok(())
}
