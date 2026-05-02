//! `collections` and `collection_skills` CRUD — Phase 2c.

use sqlx::Row;
use uuid::Uuid;

use crate::db::types::{Collection, DbPool, Skill};
use crate::db::util::now_rfc3339;

/// Total number of collections (used for dashboard stats).
pub async fn get_collection_count(pool: &DbPool) -> Result<usize, String> {
    let row = sqlx::query("SELECT COUNT(*) AS cnt FROM collections")
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;
    let count: i64 = row.try_get("cnt").map_err(|e| e.to_string())?;
    Ok(count.max(0) as usize)
}

/// Create a new collection and return it.
pub async fn create_collection(
    pool: &DbPool,
    name: &str,
    description: Option<&str>,
) -> Result<Collection, String> {
    let id = Uuid::new_v4().to_string();
    let now = now_rfc3339();

    sqlx::query(
        "INSERT INTO collections (id, name, description, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(name)
    .bind(description)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;

    get_collection_by_id(pool, &id)
        .await?
        .ok_or_else(|| "Failed to retrieve newly created collection".to_string())
}

/// Retrieve all collections.
pub async fn get_all_collections(pool: &DbPool) -> Result<Vec<Collection>, String> {
    sqlx::query_as::<_, Collection>("SELECT * FROM collections ORDER BY created_at")
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())
}

/// Retrieve a single collection by ID.
pub async fn get_collection_by_id(
    pool: &DbPool,
    collection_id: &str,
) -> Result<Option<Collection>, String> {
    sqlx::query_as::<_, Collection>("SELECT * FROM collections WHERE id = ?")
        .bind(collection_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())
}

/// Update a collection's name/description.
pub async fn update_collection(
    pool: &DbPool,
    collection_id: &str,
    name: &str,
    description: Option<&str>,
) -> Result<(), String> {
    let now = now_rfc3339();
    sqlx::query("UPDATE collections SET name = ?, description = ?, updated_at = ? WHERE id = ?")
        .bind(name)
        .bind(description)
        .bind(&now)
        .bind(collection_id)
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Delete a collection and its skill membership rows.
pub async fn delete_collection(pool: &DbPool, collection_id: &str) -> Result<(), String> {
    sqlx::query("DELETE FROM collection_skills WHERE collection_id = ?")
        .bind(collection_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM collections WHERE id = ?")
        .bind(collection_id)
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Add a skill to a collection.
pub async fn add_skill_to_collection(
    pool: &DbPool,
    collection_id: &str,
    skill_id: &str,
) -> Result<(), String> {
    let now = now_rfc3339();
    sqlx::query(
        "INSERT OR IGNORE INTO collection_skills (collection_id, skill_id, added_at)
         VALUES (?, ?, ?)",
    )
    .bind(collection_id)
    .bind(skill_id)
    .bind(&now)
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(|e| e.to_string())
}

/// Remove a skill from a collection.
pub async fn remove_skill_from_collection(
    pool: &DbPool,
    collection_id: &str,
    skill_id: &str,
) -> Result<(), String> {
    sqlx::query("DELETE FROM collection_skills WHERE collection_id = ? AND skill_id = ?")
        .bind(collection_id)
        .bind(skill_id)
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Retrieve all skills in a given collection.
pub async fn get_collection_skills(
    pool: &DbPool,
    collection_id: &str,
) -> Result<Vec<Skill>, String> {
    sqlx::query_as::<_, Skill>(
        "SELECT s.* FROM skills s
         JOIN collection_skills cs ON s.id = cs.skill_id
         WHERE cs.collection_id = ?
         ORDER BY s.name",
    )
    .bind(collection_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())
}

/// Retrieve all collections containing a given skill.
pub async fn get_skill_collections(
    pool: &DbPool,
    skill_id: &str,
) -> Result<Vec<Collection>, String> {
    sqlx::query_as::<_, Collection>(
        "SELECT c.* FROM collections c
         JOIN collection_skills cs ON c.id = cs.collection_id
         WHERE cs.skill_id = ?
         ORDER BY c.name",
    )
    .bind(skill_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())
}
