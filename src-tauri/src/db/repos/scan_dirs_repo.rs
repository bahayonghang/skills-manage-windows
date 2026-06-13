//! `scan_directories` table CRUD — Phase 2c.
//!
//! Built-in scan directory seeding lives in `db/legacy.rs` and runs from
//! `init_database_with_agents`. This file owns runtime CRUD invoked from
//! `commands/settings.rs` / `commands/discover.rs`.

use chrono::Utc;
use sqlx::Row;

use crate::db::types::{DbPool, ScanDirectory};

/// Retrieve all scan directories.
pub async fn get_scan_directories(pool: &DbPool) -> Result<Vec<ScanDirectory>, sqlx::Error> {
    sqlx::query_as::<_, ScanDirectory>(
        "SELECT * FROM scan_directories ORDER BY is_builtin DESC, added_at",
    )
    .fetch_all(pool)
    .await
}

/// Add a new scan directory entry (non-builtin by default).
pub async fn add_scan_directory(
    pool: &DbPool,
    path: &str,
    label: Option<&str>,
) -> Result<ScanDirectory, sqlx::Error> {
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO scan_directories (path, label, is_active, is_builtin, added_at)
         VALUES (?, ?, 1, 0, ?)",
    )
    .bind(path)
    .bind(label)
    .bind(&now)
    .execute(pool)
    .await?;

    sqlx::query_as::<_, ScanDirectory>("SELECT * FROM scan_directories WHERE path = ?")
        .bind(path)
        .fetch_one(pool)
        .await
}

/// Remove a scan directory. Returns an error if the directory is builtin.
pub async fn remove_scan_directory(pool: &DbPool, path: &str) -> Result<(), sqlx::Error> {
    let row = sqlx::query("SELECT is_builtin FROM scan_directories WHERE path = ?")
        .bind(path)
        .fetch_optional(pool)
        .await?;

    match row {
        None => Err(sqlx::Error::InvalidArgument(format!(
            "Scan directory '{}' not found",
            path
        ))),
        Some(r) => {
            let is_builtin: bool = r.try_get("is_builtin")?;
            if is_builtin {
                return Err(sqlx::Error::InvalidArgument(format!(
                    "Cannot remove built-in scan directory '{}'",
                    path
                )));
            }
            sqlx::query("DELETE FROM scan_directories WHERE path = ?")
                .bind(path)
                .execute(pool)
                .await
                .map(|_| ())
        }
    }
}

/// Toggle the `is_active` flag on a scan directory.
pub async fn toggle_scan_directory(
    pool: &DbPool,
    path: &str,
    is_active: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE scan_directories SET is_active = ? WHERE path = ?")
        .bind(is_active)
        .bind(path)
        .execute(pool)
        .await
        .map(|_| ())
}
