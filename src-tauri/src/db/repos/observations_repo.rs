//! `agent_skill_observations` table CRUD — Phase 2c.

use crate::db::types::{AgentSkillObservation, DbPool};

pub async fn upsert_agent_skill_observation(
    pool: &DbPool,
    observation: &AgentSkillObservation,
) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO agent_skill_observations
         (row_id, agent_id, skill_id, name, description, file_path, dir_path,
          source_kind, source_root, link_type, symlink_target, is_read_only, scanned_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(row_id) DO UPDATE SET
           agent_id       = excluded.agent_id,
           skill_id       = excluded.skill_id,
           name           = excluded.name,
           description    = excluded.description,
           file_path      = excluded.file_path,
           dir_path       = excluded.dir_path,
           source_kind    = excluded.source_kind,
           source_root    = excluded.source_root,
           link_type      = excluded.link_type,
           symlink_target = excluded.symlink_target,
           is_read_only   = excluded.is_read_only,
           scanned_at     = excluded.scanned_at",
    )
    .bind(&observation.row_id)
    .bind(&observation.agent_id)
    .bind(&observation.skill_id)
    .bind(&observation.name)
    .bind(&observation.description)
    .bind(&observation.file_path)
    .bind(&observation.dir_path)
    .bind(&observation.source_kind)
    .bind(&observation.source_root)
    .bind(&observation.link_type)
    .bind(&observation.symlink_target)
    .bind(observation.is_read_only)
    .bind(&observation.scanned_at)
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(|e| e.to_string())
}

pub async fn get_agent_skill_observations(
    pool: &DbPool,
    agent_id: &str,
) -> Result<Vec<AgentSkillObservation>, String> {
    sqlx::query_as::<_, AgentSkillObservation>(
        "SELECT * FROM agent_skill_observations
         WHERE agent_id = ?
         ORDER BY name, dir_path",
    )
    .bind(agent_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())
}

pub async fn get_agent_skill_observation_by_row_id(
    pool: &DbPool,
    row_id: &str,
) -> Result<Option<AgentSkillObservation>, String> {
    sqlx::query_as::<_, AgentSkillObservation>(
        "SELECT * FROM agent_skill_observations
         WHERE row_id = ?",
    )
    .bind(row_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| e.to_string())
}

pub async fn delete_agent_skill_observation(pool: &DbPool, row_id: &str) -> Result<(), String> {
    sqlx::query("DELETE FROM agent_skill_observations WHERE row_id = ?")
        .bind(row_id)
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub async fn delete_stale_agent_skill_observations(
    pool: &DbPool,
    agent_id: &str,
    found_row_ids: &[String],
) -> Result<(), String> {
    if found_row_ids.is_empty() {
        return sqlx::query("DELETE FROM agent_skill_observations WHERE agent_id = ?")
            .bind(agent_id)
            .execute(pool)
            .await
            .map(|_| ())
            .map_err(|e| e.to_string());
    }

    let placeholders = found_row_ids
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "DELETE FROM agent_skill_observations WHERE agent_id = ? AND row_id NOT IN ({})",
        placeholders
    );

    let mut q = sqlx::query(&sql).bind(agent_id);
    for row_id in found_row_ids {
        q = q.bind(row_id.as_str());
    }

    q.execute(pool).await.map(|_| ()).map_err(|e| e.to_string())
}
