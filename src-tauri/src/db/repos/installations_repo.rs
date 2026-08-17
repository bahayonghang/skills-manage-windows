//! `skill_installations` table CRUD — Phase 2c.

use std::collections::HashMap;

use sqlx::Row;

use crate::db::sqlite_batch::SQLITE_IN_QUERY_BATCH_SIZE;
use crate::db::types::{DbPool, LinkType, SkillInstallation};

/// Insert or update a skill installation record.
///
/// On conflict (same skill_id + agent_id), updates the mutable fields
/// (installed_path, link_type, symlink_target) but **preserves the original
/// `created_at`** so the installation timestamp reflects when the skill was
/// first installed, not when it was last re-scanned.
pub async fn upsert_skill_installation(
    pool: &DbPool,
    installation: &SkillInstallation,
) -> Result<(), sqlx::Error> {
    installation
        .link_type
        .parse::<LinkType>()
        .map_err(sqlx::Error::InvalidArgument)?;
    sqlx::query(
        "INSERT INTO skill_installations
         (skill_id, agent_id, installed_path, link_type, symlink_target, created_at)
         VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT(skill_id, agent_id) DO UPDATE SET
           installed_path = excluded.installed_path,
           link_type      = excluded.link_type,
           symlink_target = excluded.symlink_target",
    )
    .bind(&installation.skill_id)
    .bind(&installation.agent_id)
    .bind(&installation.installed_path)
    .bind(&installation.link_type)
    .bind(&installation.symlink_target)
    .bind(&installation.created_at)
    .execute(pool)
    .await
    .map(|_| ())
}

/// Delete an installation record for a specific skill+agent pair.
pub async fn delete_skill_installation(
    pool: &DbPool,
    skill_id: &str,
    agent_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM skill_installations WHERE skill_id = ? AND agent_id = ?")
        .bind(skill_id)
        .bind(agent_id)
        .execute(pool)
        .await
        .map(|_| ())
}

/// Atomically remove one agent installation plus the observation rows for the
/// same on-disk entry. The caller resolves path equivalence before entering the
/// transaction because SQLite string equality is not a safe Windows path
/// comparison (8.3 aliases and case differences may identify the same path).
pub async fn delete_skill_installation_with_observations(
    pool: &DbPool,
    skill_id: &str,
    agent_id: &str,
    observation_row_ids: &[String],
) -> Result<(), sqlx::Error> {
    let mut transaction = pool.begin().await?;
    for row_id in observation_row_ids {
        sqlx::query("DELETE FROM agent_skill_observations WHERE row_id = ? AND agent_id = ?")
            .bind(row_id)
            .bind(agent_id)
            .execute(&mut *transaction)
            .await?;
    }
    sqlx::query("DELETE FROM skill_installations WHERE skill_id = ? AND agent_id = ?")
        .bind(skill_id)
        .bind(agent_id)
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await
}

/// Delete leftover installations and writable observations for one physical
/// remote path. Payload pairs cover stored-path drift; path aliases cover
/// shared-root sibling platforms that were not in the apply payload.
pub async fn delete_leftover_installations_and_observations_for_paths(
    pool: &DbPool,
    path_aliases: &[String],
    payload_pairs: &[(String, String)],
) -> Result<(), sqlx::Error> {
    let mut transaction = pool.begin().await?;
    for (skill_id, agent_id) in payload_pairs {
        sqlx::query(
            "DELETE FROM skill_installations
             WHERE skill_id = ? AND agent_id = ?
               AND skill_id NOT IN (SELECT id FROM skills WHERE is_central = 1)",
        )
        .bind(skill_id)
        .bind(agent_id)
        .execute(&mut *transaction)
        .await?;
    }
    for chunk in path_aliases.chunks(SQLITE_IN_QUERY_BATCH_SIZE) {
        if chunk.is_empty() {
            continue;
        }
        let placeholders = chunk.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let install_sql = format!(
            "DELETE FROM skill_installations
             WHERE installed_path IN ({placeholders})
               AND skill_id NOT IN (SELECT id FROM skills WHERE is_central = 1)"
        );
        let mut install_query = sqlx::query(&install_sql);
        for path in chunk {
            install_query = install_query.bind(path);
        }
        install_query.execute(&mut *transaction).await?;

        let observation_sql = format!(
            "DELETE FROM agent_skill_observations
             WHERE dir_path IN ({placeholders})
               AND is_read_only = 0
               AND source_kind != 'plugin'"
        );
        let mut observation_query = sqlx::query(&observation_sql);
        for path in chunk {
            observation_query = observation_query.bind(path);
        }
        observation_query.execute(&mut *transaction).await?;
    }
    transaction.commit().await
}

/// Remove installation records for a given agent where the skill ID is NOT in
/// `found_skill_ids`. Pass an empty slice to remove ALL installations for the
/// agent (used when the agent's skills directory no longer exists).
pub async fn delete_stale_skill_installations(
    pool: &DbPool,
    agent_id: &str,
    found_skill_ids: &[String],
) -> Result<(), sqlx::Error> {
    if found_skill_ids.is_empty() {
        return sqlx::query("DELETE FROM skill_installations WHERE agent_id = ?")
            .bind(agent_id)
            .execute(pool)
            .await
            .map(|_| ());
    }

    let placeholders = found_skill_ids
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "DELETE FROM skill_installations WHERE agent_id = ? AND skill_id NOT IN ({})",
        placeholders
    );

    let mut q = sqlx::query(&sql).bind(agent_id);
    for id in found_skill_ids {
        q = q.bind(id.as_str());
    }
    q.execute(pool).await.map(|_| ())
}

/// Retrieve all installation records for a given skill.
pub async fn get_skill_installations(
    pool: &DbPool,
    skill_id: &str,
) -> Result<Vec<SkillInstallation>, sqlx::Error> {
    sqlx::query_as::<_, SkillInstallation>("SELECT * FROM skill_installations WHERE skill_id = ?")
        .bind(skill_id)
        .fetch_all(pool)
        .await
}

/// Retrieve all installation records for a batch of skills, grouped by skill_id.
pub async fn get_skill_installations_for_skills(
    pool: &DbPool,
    skill_ids: &[String],
) -> Result<HashMap<String, Vec<SkillInstallation>>, sqlx::Error> {
    if skill_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let mut grouped: HashMap<String, Vec<SkillInstallation>> = HashMap::new();
    for chunk in skill_ids.chunks(SQLITE_IN_QUERY_BATCH_SIZE) {
        let placeholders = chunk.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT * FROM skill_installations WHERE skill_id IN ({})",
            placeholders
        );
        let mut query = sqlx::query_as::<_, SkillInstallation>(&sql);
        for id in chunk {
            query = query.bind(id);
        }

        for row in query.fetch_all(pool).await? {
            grouped.entry(row.skill_id.clone()).or_default().push(row);
        }
    }
    Ok(grouped)
}

/// Aggregate installations per agent_id → count.
pub async fn get_skill_counts_by_agent(
    pool: &DbPool,
) -> Result<HashMap<String, usize>, sqlx::Error> {
    let rows =
        sqlx::query("SELECT agent_id, COUNT(*) AS cnt FROM skill_installations GROUP BY agent_id")
            .fetch_all(pool)
            .await?;

    let mut result = HashMap::with_capacity(rows.len());
    for row in rows {
        let agent_id: String = row.try_get("agent_id")?;
        let cnt: i64 = row.try_get("cnt")?;
        result.insert(agent_id, cnt.max(0) as usize);
    }
    Ok(result)
}
