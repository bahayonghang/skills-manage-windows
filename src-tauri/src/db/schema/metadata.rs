//! Metadata schema：仓库归属、远端更新状态、本地标签 / AI 标签建议。
//!
//! 表分组：
//! - `skill_repositories` / `skill_repository_members`：技能按源仓库分组
//! - `skill_update_states`：远端版本号、抓取状态、错误
//! - `skill_tags` / `skill_tag_links`：本地分类标签（含手动 / AI 双 source）
//! - `skill_ai_tag_reviews`：待审核的 AI 标签建议

use crate::db::migrations::ensure_column;
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
            pinned      BOOLEAN NOT NULL DEFAULT 0,
            is_unknown  BOOLEAN NOT NULL DEFAULT 0,
            created_at  TEXT NOT NULL,
            updated_at  TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    ensure_column(
        pool,
        "skill_repositories",
        "pinned",
        "ALTER TABLE skill_repositories ADD COLUMN pinned BOOLEAN NOT NULL DEFAULT 0",
    )
    .await?;

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

    // Phase 7: `(repository_id, skill_id)` makes repository → member scans
    // covering and replaces the older single-column index.
    sqlx::query("DROP INDEX IF EXISTS idx_skill_repository_members_repository_id")
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_skill_repository_members_repository_skill_id
         ON skill_repository_members(repository_id, skill_id)",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS skill_repository_sync_skips (
            repository_id TEXT NOT NULL,
            source_path   TEXT NOT NULL,
            skill_id      TEXT NOT NULL,
            skill_name    TEXT NOT NULL,
            created_at    TEXT NOT NULL,
            updated_at    TEXT NOT NULL,
            last_seen_at  TEXT NOT NULL,
            PRIMARY KEY (repository_id, source_path)
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_skill_repository_sync_skips_repository_seen
         ON skill_repository_sync_skips(repository_id, last_seen_at DESC)",
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

    sqlx::query("DROP INDEX IF EXISTS idx_skill_update_states_status")
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_skill_update_states_checked_skill
         ON skill_update_states(last_checked_at DESC, skill_id)",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_skill_update_states_status_skill
         ON skill_update_states(status, skill_id)",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    // skill_tag_groups — 标签分组（M3）。一级，不允许嵌套（D4）。tag.group_id 是
    // skill_tag_groups.id 的 nullable 引用；group 被删除时由 commands 把成员的
    // group_id 置 NULL，不在 DB 层做级联（保证 tag 本身不丢）。
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS skill_tag_groups (
            id          TEXT PRIMARY KEY,
            name        TEXT NOT NULL,
            color       TEXT,
            sort_order  INTEGER NOT NULL DEFAULT 0,
            is_builtin  BOOLEAN NOT NULL DEFAULT 0,
            created_at  TEXT NOT NULL,
            updated_at  TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_skill_tag_groups_sort_order
         ON skill_tag_groups(sort_order)",
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

    // M3 增量列：tag 隶属哪个 group。旧 db 文件升级时通过 ensure_column 安全加列。
    ensure_column(
        pool,
        "skill_tags",
        "group_id",
        "ALTER TABLE skill_tags ADD COLUMN group_id TEXT",
    )
    .await?;

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

    sqlx::query("DROP INDEX IF EXISTS idx_skill_ai_tag_reviews_status")
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_skill_ai_tag_reviews_status_updated_skill_tag
         ON skill_ai_tag_reviews(status, updated_at DESC, skill_id, tag_id)",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    /*
     * ========================================================================
     * 步骤 N（Update Mechanism Overhaul P2）：新数据模型
     * ========================================================================
     * 目标：
     * 1) `skill_repositories.last_synced_at`：repo 级 sync 时间戳，refresh 写入
     * 2) `skill_repository_pending_additions`：refresh 发现的远端新增 skill 候选
     */
    ensure_column(
        pool,
        "skill_repositories",
        "last_synced_at",
        "ALTER TABLE skill_repositories ADD COLUMN last_synced_at TEXT",
    )
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS skill_repository_pending_additions (
            repository_id              TEXT NOT NULL,
            source_path                TEXT NOT NULL,
            skill_id                   TEXT NOT NULL,
            skill_name                 TEXT NOT NULL,
            conflict_existing_skill_id TEXT,
            discovered_at              TEXT NOT NULL,
            PRIMARY KEY (repository_id, source_path)
        )",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_skill_repository_pending_additions_repo
         ON skill_repository_pending_additions(repository_id, discovered_at DESC)",
    )
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}
