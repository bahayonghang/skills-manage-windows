//! SQLite connection pool factory — extracted from `db/legacy.rs` per task_plan.md
//! Phase 2 / T2.1.
//!
//! `create_pool` 是冷启动路径上唯一的池构造函数；启用 WAL 模式以支持读写并发。
//! 与 `init_database` 的关系：先 `create_pool` 拿到池，再 `init_database` 建表。
//! 两者均同步 await，但本身不阻塞 Tauri setup 块（详见 lib.rs Phase 1 调整）。

use std::str::FromStr;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};

use super::types::DbPool;

/// Create a production SQLite pool for the given file path with WAL mode enabled.
///
/// 行为：
/// - `create_if_missing(true)`：首次启动允许在目标路径创建空 db 文件
/// - `journal_mode=WAL`：写入不阻塞读取，配合 `init_database` 内的 `PRAGMA` 一致
/// - `max_connections=5`：保守上限，对桌面端单用户场景足够，避免 fd 占用过多
pub async fn create_pool(db_path: &str) -> Result<DbPool, sqlx::Error> {
    let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", db_path))?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal);

    SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(opts)
        .await
}
