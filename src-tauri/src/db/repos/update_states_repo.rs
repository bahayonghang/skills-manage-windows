//! `skill_update_states` table CRUD — Phase 2c.

use crate::db::types::{DbPool, SkillUpdateState};

pub async fn get_skill_update_states(pool: &DbPool) -> Result<Vec<SkillUpdateState>, String> {
    sqlx::query_as::<_, SkillUpdateState>(
        "SELECT * FROM skill_update_states ORDER BY last_checked_at DESC, skill_id",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())
}

pub async fn get_skill_update_states_for_skills(
    pool: &DbPool,
    skill_ids: &[String],
) -> Result<Vec<SkillUpdateState>, String> {
    if skill_ids.is_empty() {
        return Ok(Vec::new());
    }

    let placeholders = skill_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT * FROM skill_update_states WHERE skill_id IN ({})",
        placeholders
    );
    let mut query = sqlx::query_as::<_, SkillUpdateState>(&sql);
    for skill_id in skill_ids {
        query = query.bind(skill_id);
    }

    query.fetch_all(pool).await.map_err(|e| e.to_string())
}

pub async fn upsert_skill_update_state(
    pool: &DbPool,
    state: &SkillUpdateState,
) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO skill_update_states
         (skill_id, source_type, source_url, ref_name, source_path, last_remote_hash,
          latest_remote_hash, last_checked_at, last_updated_at, status, error)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(skill_id) DO UPDATE SET
           source_type        = excluded.source_type,
           source_url         = excluded.source_url,
           ref_name           = excluded.ref_name,
           source_path        = excluded.source_path,
           last_remote_hash   = COALESCE(excluded.last_remote_hash, skill_update_states.last_remote_hash),
           latest_remote_hash = excluded.latest_remote_hash,
           last_checked_at    = excluded.last_checked_at,
           last_updated_at    = COALESCE(excluded.last_updated_at, skill_update_states.last_updated_at),
           status             = excluded.status,
           error              = excluded.error",
    )
    .bind(&state.skill_id)
    .bind(&state.source_type)
    .bind(&state.source_url)
    .bind(&state.ref_name)
    .bind(&state.source_path)
    .bind(&state.last_remote_hash)
    .bind(&state.latest_remote_hash)
    .bind(&state.last_checked_at)
    .bind(&state.last_updated_at)
    .bind(&state.status)
    .bind(&state.error)
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(|e| e.to_string())
}
