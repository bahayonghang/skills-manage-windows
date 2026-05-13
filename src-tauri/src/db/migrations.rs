//! Schema migration helpers — extracted from `db/legacy.rs` per task_plan.md
//! Phase 2b / T2.2.
//!
//! 仅承载「在已存在的 schema 上做增量演进」的工具函数。每个新增列必须走
//! [`ensure_column`]：先 `PRAGMA table_info` 检测列是否已存在，缺则 `ALTER
//! TABLE`。schema/* 子模块在自家 `init()` 中就近调用，避免把所有 ALTER 集中到
//! `init_database` 末尾时打乱「建表 → 该表迁移」的局部顺序。
//!
//! 本模块对 `crate::db` 内部可见（`mod migrations;` 私有），不对外暴露。

use sqlx::Row;

use super::types::DbPool;

/// 若 `column` 在 `table` 上不存在，则执行 `alter_sql`。
///
/// 用途：兼容老版本 db 文件升级到当前 schema。`alter_sql` 应当是完整的
/// `ALTER TABLE <table> ADD COLUMN <column> <type> [DEFAULT ...]`。
///
/// 幂等：若列已存在直接返回 `Ok(())`，不会重复执行。
pub(crate) async fn ensure_column(
    pool: &DbPool,
    table: &str,
    column: &str,
    alter_sql: &str,
) -> Result<(), String> {
    let pragma = format!("PRAGMA table_info({table})");
    let rows = sqlx::query(&pragma)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;

    let has_column = rows
        .iter()
        .any(|row| row.get::<String, _>("name") == column);

    if !has_column {
        sqlx::query(alter_sql)
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}
