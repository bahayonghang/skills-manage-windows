//! `discovered_skills` table CRUD — Phase 2c.

use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row};

use crate::db::types::DbPool;

/// A skill discovered in a project-level directory during a full-disk scan.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DiscoveredSkillRow {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub file_path: String,
    pub dir_path: String,
    pub project_path: String,
    pub project_name: String,
    pub platform_id: String,
    pub discovered_at: String,
}

#[derive(Debug, Clone, Copy)]
pub struct DiscoveredSkillInsert<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub file_path: &'a str,
    pub dir_path: &'a str,
    pub project_path: &'a str,
    pub project_name: &'a str,
    pub platform_id: &'a str,
    pub discovered_at: &'a str,
}

/// Total number of distinct discovered skills (cheap count for dashboard).
pub async fn get_discovered_skill_count(pool: &DbPool) -> Result<usize, String> {
    let row = sqlx::query("SELECT COUNT(*) AS cnt FROM discovered_skills")
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;
    let count: i64 = row.try_get("cnt").map_err(|e| e.to_string())?;
    Ok(count.max(0) as usize)
}

/// Insert a discovered skill record.
#[allow(clippy::too_many_arguments)]
pub async fn insert_discovered_skill(
    pool: &DbPool,
    id: &str,
    name: &str,
    description: Option<&str>,
    file_path: &str,
    dir_path: &str,
    project_path: &str,
    project_name: &str,
    platform_id: &str,
    discovered_at: &str,
) -> Result<(), String> {
    sqlx::query(
        "INSERT OR IGNORE INTO discovered_skills
         (id, name, description, file_path, dir_path, project_path, project_name, platform_id, discovered_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(name)
    .bind(description)
    .bind(file_path)
    .bind(dir_path)
    .bind(project_path)
    .bind(project_name)
    .bind(platform_id)
    .bind(discovered_at)
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(|e| e.to_string())
}

/// Bulk insert in a single transaction.
pub async fn insert_discovered_skills(
    pool: &DbPool,
    skills: &[DiscoveredSkillInsert<'_>],
) -> Result<(), String> {
    if skills.is_empty() {
        return Ok(());
    }

    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    for skill in skills {
        sqlx::query(
            "INSERT OR IGNORE INTO discovered_skills
             (id, name, description, file_path, dir_path, project_path, project_name, platform_id, discovered_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(skill.id)
        .bind(skill.name)
        .bind(skill.description)
        .bind(skill.file_path)
        .bind(skill.dir_path)
        .bind(skill.project_path)
        .bind(skill.project_name)
        .bind(skill.platform_id)
        .bind(skill.discovered_at)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    }
    tx.commit().await.map_err(|e| e.to_string())
}

/// Retrieve a discovered skill by its qualified ID.
pub async fn get_discovered_skill_by_id(
    pool: &DbPool,
    id: &str,
) -> Result<Option<DiscoveredSkillRow>, String> {
    sqlx::query_as::<_, DiscoveredSkillRow>("SELECT * FROM discovered_skills WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())
}

/// Retrieve all discovered skills.
pub async fn get_all_discovered_skills(pool: &DbPool) -> Result<Vec<DiscoveredSkillRow>, String> {
    sqlx::query_as::<_, DiscoveredSkillRow>(
        "SELECT * FROM discovered_skills ORDER BY project_name, platform_id, name",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())
}

/// Delete a discovered skill by ID.
pub async fn delete_discovered_skill(pool: &DbPool, id: &str) -> Result<(), String> {
    sqlx::query("DELETE FROM discovered_skills WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Clear all discovered skills.
pub async fn clear_all_discovered_skills(pool: &DbPool) -> Result<(), String> {
    sqlx::query("DELETE FROM discovered_skills")
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

/// Get count of discovered projects (distinct `project_path` values).
pub async fn get_discovered_project_count(pool: &DbPool) -> Result<i64, String> {
    let row = sqlx::query("SELECT COUNT(DISTINCT project_path) AS cnt FROM discovered_skills")
        .fetch_one(pool)
        .await
        .map_err(|e| e.to_string())?;

    row.try_get::<i64, _>("cnt").map_err(|e| e.to_string())
}
