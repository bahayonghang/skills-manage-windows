//! Metadata schema：仓库归属、远端更新状态、本地标签 / AI 标签建议。
//!
//! 表分组：
//! - `skill_repositories` / `skill_repository_members`：技能按源仓库分组
//! - `skill_update_states`：远端版本号、抓取状态、错误
//! - `skill_tags` / `skill_tag_links`：本地分类标签（含手动 / AI 双 source）
//! - `skill_ai_tag_reviews`：待审核的 AI 标签建议

use crate::db::DbPool;

pub(super) async fn init(pool: &DbPool) -> Result<(), String> {
    // skill_repositories — local metadata for grouping Central skills by source repo.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS skill_repositories (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL,
            source_type TEXT NOT NULL,
            owner       TEXT,
            repo        TEXT,
            branch      TEXT,
            url         TEXT,
            is_unknown  BOOLEAN NOT NULL DEFAULT 0,
            created_at  TEXT NOT NULL,
            updated_at  TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS skill_repository_members (
            skill_id      TEXT PRIMARY KEY,
            repository_id TEXT NOT NULL,
            source_path   TEXT,
            added_at      TEXT NOT NULL,
            updated_at    TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_skill_repository_members_repository_id
         ON skill_repository_members(repository_id)",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS skill_update_states (
            skill_id           TEXT PRIMARY KEY,
            source_type        TEXT NOT NULL,
            source_url         TEXT,
            ref_name           TEXT,
            source_path        TEXT,
            last_remote_hash   TEXT,
            latest_remote_hash TEXT,
            last_checked_at    TEXT,
            last_updated_at    TEXT,
            status             TEXT NOT NULL,
            error              TEXT
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_skill_update_states_status
         ON skill_update_states(status)",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    // skill_tags — local category taxonomy separate from user Collections.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS skill_tags (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL UNIQUE,
            description TEXT,
            color       TEXT,
            is_builtin  BOOLEAN NOT NULL DEFAULT 0,
            created_at  TEXT NOT NULL,
            updated_at  TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS skill_tag_links (
            skill_id    TEXT NOT NULL,
            tag_id      TEXT NOT NULL,
            confidence  REAL,
            reason      TEXT,
            source      TEXT NOT NULL DEFAULT 'manual',
            added_at    TEXT NOT NULL,
            PRIMARY KEY (skill_id, tag_id)
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_skill_tag_links_tag_id
         ON skill_tag_links(tag_id)",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS skill_ai_tag_reviews (
            skill_id     TEXT NOT NULL,
            tag_id       TEXT NOT NULL,
            confidence   REAL NOT NULL,
            reason       TEXT,
            status       TEXT NOT NULL DEFAULT 'pending',
            suggested_at TEXT NOT NULL,
            updated_at   TEXT NOT NULL,
            PRIMARY KEY (skill_id, tag_id)
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_skill_ai_tag_reviews_status
         ON skill_ai_tag_reviews(status)",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}
