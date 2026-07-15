//! Core schema：技能本体 + 安装关联 + agent 观察记录 + agent 注册表。
//!
//! 这些表是其他模块的基础（collections / metadata / discovery / marketplace 都
//! 引用其中至少一张表的 PK），必须在调度链上最先初始化。
//!
//! 内含 `skill_installations.created_at` 老库迁移：检测列缺失 → ALTER TABLE +
//! 回填 `datetime('now')`。SQLite 在某些构建（Apple-modified）不支持非常量
//! DEFAULT 表达式，故拆成两步。

use sqlx::Row;
use uuid::Uuid;

use crate::db::migrations::ensure_column;
use crate::db::DbPool;

pub(super) async fn init(pool: &DbPool) -> Result<(), sqlx::Error> {
    // skills：中央技能 / 平台技能的元数据合表。
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS skills (
            id             TEXT PRIMARY KEY,
            uid            TEXT,
            name           TEXT NOT NULL,
            description    TEXT,
            file_path      TEXT NOT NULL,
            canonical_path TEXT,
            is_central     BOOLEAN NOT NULL DEFAULT 0,
            source         TEXT,
            content        TEXT,
            scanned_at     TEXT NOT NULL,
            fs_created_at  TEXT,
            fs_updated_at  TEXT
        )",
    )
    .execute(pool)
    .await?;

    ensure_column(
        pool,
        "skills",
        "uid",
        "ALTER TABLE skills ADD COLUMN uid TEXT",
    )
    .await?;

    let mut transaction = pool.begin().await?;
    let missing_uid_rows = sqlx::query("SELECT id FROM skills WHERE uid IS NULL OR TRIM(uid) = ''")
        .fetch_all(&mut *transaction)
        .await?;
    for row in missing_uid_rows {
        let id = row.try_get::<String, _>("id")?;
        sqlx::query("UPDATE skills SET uid = ? WHERE id = ?")
            .bind(Uuid::new_v4().to_string())
            .bind(id)
            .execute(&mut *transaction)
            .await?;
    }
    transaction.commit().await?;

    sqlx::query("CREATE UNIQUE INDEX IF NOT EXISTS idx_skills_uid ON skills(uid)")
        .execute(pool)
        .await?;

    let invalid_uid_count =
        sqlx::query("SELECT COUNT(*) AS count FROM skills WHERE uid IS NULL OR TRIM(uid) = ''")
            .fetch_one(pool)
            .await?
            .try_get::<i64, _>("count")?;
    if invalid_uid_count != 0 {
        return Err(sqlx::Error::InvalidArgument(
            "skills.uid backfill left empty identities".to_string(),
        ));
    }

    // skill_installations：(skill_id, agent_id) 唯一安装关系。
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS skill_installations (
            skill_id       TEXT NOT NULL,
            agent_id       TEXT NOT NULL,
            installed_path TEXT NOT NULL,
            link_type      TEXT NOT NULL CHECK (link_type IN ('native', 'symlink', 'copy', 'writable')),
            symlink_target TEXT,
            created_at     TEXT NOT NULL DEFAULT (datetime('now')),
            PRIMARY KEY (skill_id, agent_id)
        )",
    )
    .execute(pool)
    .await
    ?;

    // Phase 7: `(agent_id, skill_id)` covers the hot
    // `WHERE agent_id = ?` lookup plus the subsequent join back to `skills`.
    // Drop the older single-column index on upgrade because the composite
    // prefix subsumes it.
    sqlx::query("DROP INDEX IF EXISTS idx_skill_installations_agent_id")
        .execute(pool)
        .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_skill_installations_agent_skill_id
         ON skill_installations(agent_id, skill_id)",
    )
    .execute(pool)
    .await?;

    // agent_skill_observations：每次 agent 扫描到技能时的事实记录。
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS agent_skill_observations (
            row_id         TEXT PRIMARY KEY,
            agent_id       TEXT NOT NULL,
            skill_id       TEXT NOT NULL,
            name           TEXT NOT NULL,
            description    TEXT,
            file_path      TEXT NOT NULL,
            dir_path       TEXT NOT NULL,
            source_kind    TEXT NOT NULL,
            source_root    TEXT NOT NULL,
            link_type      TEXT NOT NULL CHECK (link_type IN ('native', 'symlink', 'copy', 'writable')),
            symlink_target TEXT,
            is_read_only   BOOLEAN NOT NULL DEFAULT 0,
            scanned_at     TEXT NOT NULL,
            fs_created_at  TEXT,
            fs_updated_at  TEXT
        )",
    )
    .execute(pool)
    .await
    ?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_agent_skill_observations_agent_id
         ON agent_skill_observations(agent_id)",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_agent_skill_observations_agent_name_dir
         ON agent_skill_observations(agent_id, name, dir_path)",
    )
    .execute(pool)
    .await?;

    ensure_column(
        pool,
        "skills",
        "fs_created_at",
        "ALTER TABLE skills ADD COLUMN fs_created_at TEXT",
    )
    .await?;
    ensure_column(
        pool,
        "skills",
        "fs_updated_at",
        "ALTER TABLE skills ADD COLUMN fs_updated_at TEXT",
    )
    .await?;
    ensure_column(
        pool,
        "agent_skill_observations",
        "fs_created_at",
        "ALTER TABLE agent_skill_observations ADD COLUMN fs_created_at TEXT",
    )
    .await?;
    ensure_column(
        pool,
        "agent_skill_observations",
        "fs_updated_at",
        "ALTER TABLE agent_skill_observations ADD COLUMN fs_updated_at TEXT",
    )
    .await?;

    // 增量迁移：老 db 的 skill_installations 缺 created_at。两步走避免 SQLite
    // 在某些构建（如 Apple-modified SQLite）不支持非常量 DEFAULT 表达式：
    //   1) ALTER TABLE ADD COLUMN created_at TEXT （可空，无 DEFAULT）
    //   2) UPDATE … SET created_at = datetime('now') WHERE created_at IS NULL
    // 新行始终由应用显式写入 created_at，迁移后不再需要 DEFAULT。
    let columns = sqlx::query("PRAGMA table_info(skill_installations)")
        .fetch_all(pool)
        .await?;

    let has_created_at = columns.iter().any(|row| {
        row.try_get::<String, _>("name")
            .map(|name| name == "created_at")
            .unwrap_or(false)
    });

    if !has_created_at {
        sqlx::query("ALTER TABLE skill_installations ADD COLUMN created_at TEXT")
            .execute(pool)
            .await?;

        sqlx::query(
            "UPDATE skill_installations SET created_at = datetime('now') WHERE created_at IS NULL",
        )
        .execute(pool)
        .await?;
    }

    // agents：内置与自定义 agent 元数据。
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS agents (
            id                 TEXT PRIMARY KEY,
            display_name       TEXT NOT NULL,
            category           TEXT NOT NULL,
            global_skills_dir  TEXT NOT NULL,
            project_skills_dir TEXT,
            icon_name          TEXT,
            is_detected        BOOLEAN NOT NULL DEFAULT 0,
            is_builtin         BOOLEAN NOT NULL DEFAULT 1,
            is_enabled         BOOLEAN NOT NULL DEFAULT 1
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_skills_is_central
         ON skills(is_central)",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_skills_is_central_name
         ON skills(is_central, name)",
    )
    .execute(pool)
    .await?;

    Ok(())
}
