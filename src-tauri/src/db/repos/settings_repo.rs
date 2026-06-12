//! `settings` table CRUD — Phase 2c.

use std::collections::HashMap;

use sqlx::Row;

use crate::db::types::DbPool;

/// Get a setting value by key.
pub async fn get_setting(pool: &DbPool, key: &str) -> Result<Option<String>, sqlx::Error> {
    let row = sqlx::query("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await?;

    Ok(row.map(|r| r.get::<String, _>("value")))
}

/// Get a batch of setting values by key. Missing keys are present with `None`.
pub async fn get_settings(
    pool: &DbPool,
    keys: &[String],
) -> Result<HashMap<String, Option<String>>, sqlx::Error> {
    let mut result = keys
        .iter()
        .map(|key| (key.clone(), None))
        .collect::<HashMap<_, _>>();

    if keys.is_empty() {
        return Ok(result);
    }

    let placeholders = keys.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT key, value FROM settings WHERE key IN ({})",
        placeholders
    );
    let mut query = sqlx::query(&sql);
    for key in keys {
        query = query.bind(key);
    }

    let rows = query.fetch_all(pool).await?;
    for row in rows {
        let key: String = row.try_get("key")?;
        let value: String = row.try_get("value")?;
        result.insert(key, Some(value));
    }

    Ok(result)
}

/// Set (upsert) a setting value.
pub async fn set_setting(pool: &DbPool, key: &str, value: &str) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)")
        .bind(key)
        .bind(value)
        .execute(pool)
        .await
        .map(|_| ())
}

/// Set (upsert) a setting value on a best-effort basis: failures are logged
/// via `tracing` instead of propagating, for bookkeeping writes (e.g. scan
/// state markers) that must not abort the surrounding operation.
pub async fn set_setting_best_effort(pool: &DbPool, key: &str, value: &str) {
    if let Err(error) = set_setting(pool, key, value).await {
        tracing::warn!(key, error = %error, "Failed to persist setting (best-effort)");
    }
}

/// Delete a setting value by key.
pub async fn delete_setting(pool: &DbPool, key: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM settings WHERE key = ?")
        .bind(key)
        .execute(pool)
        .await
        .map(|_| ())
}

/// Set (upsert) a batch of settings in a single transaction.
pub async fn set_settings(
    pool: &DbPool,
    values: &HashMap<String, String>,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    for (key, value) in values {
        sqlx::query("INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)")
            .bind(key)
            .bind(value)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await
}
