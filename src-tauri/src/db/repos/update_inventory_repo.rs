//! `skill_update_inventory_*` table CRUD.
//!
//! Refresh owns these rows. Apply may prune affected entries after successful
//! decisions, while `skill_update_states` remains the installed baseline.

use crate::db::types::{DbPool, SkillUpdateInventoryEntry, SkillUpdateInventoryRun};

pub async fn replace_skill_update_inventory(
    pool: &DbPool,
    run: &SkillUpdateInventoryRun,
    entries: &[SkillUpdateInventoryEntry],
) -> Result<(), String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;

    sqlx::query("DELETE FROM skill_update_inventory_entries WHERE inventory_id = ?")
        .bind(&run.inventory_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

    sqlx::query(
        "INSERT INTO skill_update_inventory_runs
         (inventory_id, scope_kind, mode, skill_ids_json, repository_ids_json,
          agent_ids_json, cache_policy, generated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(inventory_id) DO UPDATE SET
           scope_kind          = excluded.scope_kind,
           mode                = excluded.mode,
           skill_ids_json      = excluded.skill_ids_json,
           repository_ids_json = excluded.repository_ids_json,
           agent_ids_json      = excluded.agent_ids_json,
           cache_policy        = excluded.cache_policy,
           generated_at        = excluded.generated_at",
    )
    .bind(&run.inventory_id)
    .bind(&run.scope_kind)
    .bind(&run.mode)
    .bind(&run.skill_ids_json)
    .bind(&run.repository_ids_json)
    .bind(&run.agent_ids_json)
    .bind(&run.cache_policy)
    .bind(&run.generated_at)
    .execute(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    for entry in entries {
        sqlx::query(
            "INSERT INTO skill_update_inventory_entries
             (inventory_id, bucket, entity_key, skill_id, skill_name, repository_id,
              source_type, source_url, ref_name, source_path, agent_id, local_hash,
              baseline_hash, remote_hash, local_version, remote_version, cache_policy,
              cache_hit, snapshot_fetched_at, generated_at, payload_json, error)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&entry.inventory_id)
        .bind(&entry.bucket)
        .bind(&entry.entity_key)
        .bind(&entry.skill_id)
        .bind(&entry.skill_name)
        .bind(&entry.repository_id)
        .bind(&entry.source_type)
        .bind(&entry.source_url)
        .bind(&entry.ref_name)
        .bind(&entry.source_path)
        .bind(&entry.agent_id)
        .bind(&entry.local_hash)
        .bind(&entry.baseline_hash)
        .bind(&entry.remote_hash)
        .bind(&entry.local_version)
        .bind(&entry.remote_version)
        .bind(&entry.cache_policy)
        .bind(entry.cache_hit)
        .bind(&entry.snapshot_fetched_at)
        .bind(&entry.generated_at)
        .bind(&entry.payload_json)
        .bind(&entry.error)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    }

    tx.commit().await.map_err(|e| e.to_string())
}

pub async fn list_skill_update_inventory_entries(
    pool: &DbPool,
    inventory_id: &str,
) -> Result<Vec<SkillUpdateInventoryEntry>, String> {
    sqlx::query_as::<_, SkillUpdateInventoryEntry>(
        "SELECT *
         FROM skill_update_inventory_entries
         WHERE inventory_id = ?
         ORDER BY bucket, generated_at DESC, entity_key",
    )
    .bind(inventory_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())
}

pub async fn clear_skill_update_inventory_run(
    pool: &DbPool,
    inventory_id: &str,
) -> Result<(), String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM skill_update_inventory_entries WHERE inventory_id = ?")
        .bind(inventory_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM skill_update_inventory_runs WHERE inventory_id = ?")
        .bind(inventory_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())
}

pub async fn clear_all_skill_update_inventory(pool: &DbPool) -> Result<(), String> {
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM skill_update_inventory_entries")
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM skill_update_inventory_runs")
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
    tx.commit().await.map_err(|e| e.to_string())
}

pub async fn delete_skill_update_inventory_entries_for_skills(
    pool: &DbPool,
    skill_ids: &[String],
) -> Result<(), String> {
    if skill_ids.is_empty() {
        return Ok(());
    }
    let placeholders = skill_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "DELETE FROM skill_update_inventory_entries WHERE skill_id IN ({})",
        placeholders
    );
    let mut query = sqlx::query(&sql);
    for skill_id in skill_ids {
        query = query.bind(skill_id);
    }
    query
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub async fn delete_skill_update_inventory_entries_for_repositories(
    pool: &DbPool,
    repository_ids: &[String],
) -> Result<(), String> {
    if repository_ids.is_empty() {
        return Ok(());
    }
    let placeholders = repository_ids
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "DELETE FROM skill_update_inventory_entries WHERE repository_id IN ({})",
        placeholders
    );
    let mut query = sqlx::query(&sql);
    for repository_id in repository_ids {
        query = query.bind(repository_id);
    }
    query
        .execute(pool)
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
}
