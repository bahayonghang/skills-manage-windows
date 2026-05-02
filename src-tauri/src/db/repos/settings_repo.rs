//! `settings` table CRUD — Phase 2c.

use std::collections::HashMap;

use sqlx::Row;

use crate::db::types::DbPool;

/// Get a setting value by key.
pub async fn get_setting(pool: &DbPool, key: &str) -> Result<Option<String>, String> {
    let row = sqlx::query("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(row.map(|r| r.get::<String, _>("value")))
}

/// Get a batch of setting values by key. Missing keys are present with `None`.
pub async fn get_settings(
    pool: &DbPool,
    keys: &[String],
) -> Result<HashMap<String, Option<String>>, String> {
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

    let rows = query.fetch_all(pool).await.map_err(|e| e.to_string())?;
    for row in rows {
        let key: String = row.try_get("key").map_err(|e| e.to_string())?;
        let value: String = row.try_get("value").map_err(|e| e.to_string())?;
        result.insert(key, Some(value));
    }

    Ok(result)
}

/// Set (upsert) a setting value.
pub async fn set_setting(pool: &DbPool, key: &str, value: &str) -> Result<(), String> {
    sqlx::query("INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)")
        .bind(key)
        .bind(value)
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Set (upsert) a batch of settings in a single transaction.
pub async fn set_settings(pool: &DbPool, values: &HashMap<String, String>) -> Result<(), String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    for (key, value) in values {
        sqlx::query("INSERT OR REPLACE INTO settings (key, value) VALUES (?, ?)")
            .bind(key)
            .bind(value)
            .execute(&mut *tx)
            .await
            .map_err(|e| e.to_string())?;
    }

    tx.commit().await.map_err(|e| e.to_string())
}
