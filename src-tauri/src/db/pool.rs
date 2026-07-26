//! SQLite connection-pool construction.

use std::path::Path;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::Row;

use super::types::DbPool;

fn pool_options() -> SqlitePoolOptions {
    SqlitePoolOptions::new().after_connect(|connection, _metadata| {
        Box::pin(async move {
            sqlx::query("PRAGMA foreign_keys = ON")
                .execute(&mut *connection)
                .await?;
            let enabled = sqlx::query("PRAGMA foreign_keys")
                .fetch_one(&mut *connection)
                .await?
                .try_get::<i64, _>(0)?;
            if enabled != 1 {
                return Err(sqlx::Error::InvalidArgument(
                    "SQLite foreign key enforcement could not be enabled".to_string(),
                ));
            }
            Ok(())
        })
    })
}

pub(crate) async fn create_pool(db_path: &Path) -> Result<DbPool, sqlx::Error> {
    let options = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal);

    pool_options()
        .max_connections(5)
        .connect_with(options)
        .await
}

#[cfg(test)]
pub(crate) async fn create_memory_pool() -> Result<DbPool, sqlx::Error> {
    pool_options().connect(":memory:").await
}

#[cfg(test)]
pub(crate) async fn create_memory_pool_single_conn() -> Result<DbPool, sqlx::Error> {
    pool_options().max_connections(1).connect(":memory:").await
}
