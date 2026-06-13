//! Settings schema：键值设置表 + 操作日志事实表（含 6 条索引）。
//!
//! - `settings`：扫描目录、AI provider、远程目标等用户配置的键值存储
//! - `operation_logs`：本地结构化操作历史，6 条索引覆盖时间、目标、级别、
//!   动作、分类、批次维度的查询路径

use crate::db::DbPool;

pub(super) async fn init(pool: &DbPool) -> Result<(), sqlx::Error> {
    // settings table
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS settings (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    // operation_logs table — local-only structured operation history.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS operation_logs (
            id             TEXT PRIMARY KEY,
            created_at     TEXT NOT NULL,
            level          TEXT NOT NULL,
            target_kind    TEXT NOT NULL,
            target_id      TEXT NOT NULL,
            target_label   TEXT,
            category       TEXT NOT NULL,
            action         TEXT NOT NULL,
            status         TEXT NOT NULL,
            subject_type   TEXT,
            subject_id     TEXT,
            subject_label  TEXT,
            summary        TEXT NOT NULL,
            error_summary  TEXT,
            details_json   TEXT,
            duration_ms    INTEGER,
            batch_id       TEXT
        )",
    )
    .execute(pool)
    .await?;

    for index_sql in [
        "CREATE INDEX IF NOT EXISTS idx_operation_logs_created_at
         ON operation_logs(created_at)",
        "CREATE INDEX IF NOT EXISTS idx_operation_logs_target
         ON operation_logs(target_kind, target_id)",
        "CREATE INDEX IF NOT EXISTS idx_operation_logs_level_status
         ON operation_logs(level, status)",
        "CREATE INDEX IF NOT EXISTS idx_operation_logs_action
         ON operation_logs(action)",
        "CREATE INDEX IF NOT EXISTS idx_operation_logs_category
         ON operation_logs(category)",
        "CREATE INDEX IF NOT EXISTS idx_operation_logs_batch_id
         ON operation_logs(batch_id)",
    ] {
        sqlx::query(index_sql).execute(pool).await?;
    }

    Ok(())
}
