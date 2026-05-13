//! `skills` table CRUD — Phase 2c.
//!
//! The Central Skills view (`is_central = 1`) is the system of record. Platform
//! installations are tracked in `skill_installations` (see `installations_repo`)
//! and observations of agent-side files live in `agent_skill_observations`
//! (see `observations_repo`).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::db::repos::observations_repo::get_agent_skill_observations;
use crate::db::repos::repositories_repo::prune_empty_skill_repositories;
use crate::db::types::{AgentSkillObservation, DbPool, Skill};

/// Insert or update a skill record.
///
/// Uses `ON CONFLICT DO UPDATE` to preserve the private Central record if a
/// platform scan later observes the same skill id in an agent directory.
/// Once a skill is flagged as central it must never be downgraded to non-central
/// or have its canonical file path overwritten by a platform copy.
pub async fn upsert_skill(pool: &DbPool, skill: &Skill) -> Result<(), String> {
    sqlx::query(
        "INSERT INTO skills
         (id, name, description, file_path, canonical_path, is_central, source, content, scanned_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET
           name           = CASE
                              WHEN skills.is_central = 1 AND excluded.is_central = 0 THEN skills.name
                              ELSE excluded.name
                            END,
           description    = CASE
                              WHEN skills.is_central = 1 AND excluded.is_central = 0 THEN skills.description
                              ELSE excluded.description
                            END,
           file_path      = CASE
                              WHEN skills.is_central = 1 AND excluded.is_central = 0 THEN skills.file_path
                              ELSE excluded.file_path
                            END,
           canonical_path = CASE
                              WHEN skills.is_central = 1 AND excluded.is_central = 0 THEN skills.canonical_path
                              ELSE COALESCE(excluded.canonical_path, skills.canonical_path)
                            END,
           is_central     = MAX(skills.is_central, excluded.is_central),
           source         = CASE
                              WHEN skills.is_central = 1 AND excluded.is_central = 0 THEN skills.source
                              ELSE excluded.source
                            END,
           content        = CASE
                              WHEN skills.is_central = 1 AND excluded.is_central = 0 THEN skills.content
                              ELSE excluded.content
                            END,
           scanned_at     = excluded.scanned_at",
    )
    .bind(&skill.id)
    .bind(&skill.name)
    .bind(&skill.description)
    .bind(&skill.file_path)
    .bind(&skill.canonical_path)
    .bind(skill.is_central)
    .bind(&skill.source)
    .bind(&skill.content)
    .bind(&skill.scanned_at)
    .execute(pool)
    .await
    .map(|_| ())
    .map_err(|e| e.to_string())
}

fn observation_to_skill(observation: AgentSkillObservation) -> Skill {
    Skill {
        id: observation.skill_id,
        name: observation.name,
        description: observation.description,
        file_path: observation.file_path,
        canonical_path: None,
        is_central: false,
        source: Some(observation.link_type),
        content: None,
        scanned_at: observation.scanned_at,
    }
}

/// Retrieve all skills installed for a given agent.
pub async fn get_skills_by_agent(pool: &DbPool, agent_id: &str) -> Result<Vec<Skill>, String> {
    if agent_id == "claude-code" {
        let observations = get_agent_skill_observations(pool, agent_id).await?;
        if !observations.is_empty() {
            return Ok(observations.into_iter().map(observation_to_skill).collect());
        }
    }

    sqlx::query_as::<_, Skill>(
        "SELECT s.* FROM skills s
         JOIN skill_installations si ON s.id = si.skill_id
         WHERE si.agent_id = ?",
    )
    .bind(agent_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())
}

/// A skill enriched with the installation-specific fields for a given agent.
///
/// Returned by `get_skills_for_agent`. The extra fields come from the
/// `skill_installations` row and allow the frontend `SkillCard` to display
/// the correct source indicator without a second round-trip.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SkillForAgent {
    pub id: String,
    /// Stable row identity for source-specific detail routing.
    pub row_id: String,
    pub name: String,
    pub description: Option<String>,
    /// Absolute path to the `SKILL.md` file.
    pub file_path: String,
    /// Absolute path to the skill directory as installed for this agent
    /// (i.e., `skill_installations.installed_path`).
    pub dir_path: String,
    /// How the skill is linked: "symlink", "copy", or "native".
    pub link_type: String,
    /// Symlink target path, if `link_type` is "symlink".
    pub symlink_target: Option<String>,
    pub is_central: bool,
    pub source_kind: Option<String>,
    pub source_root: Option<String>,
    pub is_read_only: bool,
    pub conflict_group: Option<String>,
    pub conflict_count: i64,
}

/// Retrieve skills installed for a given agent, enriched with installation
/// metadata (`dir_path`, `link_type`, `symlink_target`) required by the
/// platform-view skill cards.
pub async fn get_skills_for_agent(
    pool: &DbPool,
    agent_id: &str,
) -> Result<Vec<SkillForAgent>, String> {
    if agent_id == "claude-code" {
        let observations = get_agent_skill_observations(pool, agent_id).await?;
        if !observations.is_empty() {
            let mut conflict_counts = HashMap::new();
            for observation in &observations {
                *conflict_counts
                    .entry(observation.skill_id.clone())
                    .or_insert(0_i64) += 1;
            }

            return Ok(observations
                .into_iter()
                .map(|observation| {
                    let conflict_count = conflict_counts
                        .get(&observation.skill_id)
                        .copied()
                        .unwrap_or(0);
                    let mut skill = observation_to_skill_for_agent(observation);
                    if conflict_count > 1 {
                        skill.conflict_group = Some(claude_conflict_group(agent_id, &skill.id));
                        skill.conflict_count = conflict_count;
                    }
                    skill
                })
                .collect());
        }
    }

    sqlx::query_as::<_, SkillForAgent>(
        "SELECT s.id,
                s.id AS row_id,
                s.name,
                s.description,
                s.file_path,
                si.installed_path AS dir_path,
                si.link_type,
                si.symlink_target,
                s.is_central,
                NULL AS source_kind,
                NULL AS source_root,
                0 AS is_read_only,
                NULL AS conflict_group,
                0 AS conflict_count
         FROM skills s
         JOIN skill_installations si ON s.id = si.skill_id
         WHERE si.agent_id = ?",
    )
    .bind(agent_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())
}

fn observation_to_skill_for_agent(observation: AgentSkillObservation) -> SkillForAgent {
    SkillForAgent {
        id: observation.skill_id,
        row_id: observation.row_id,
        name: observation.name,
        description: observation.description,
        file_path: observation.file_path,
        dir_path: observation.dir_path,
        link_type: observation.link_type,
        symlink_target: observation.symlink_target,
        is_central: false,
        source_kind: Some(observation.source_kind),
        source_root: Some(observation.source_root),
        is_read_only: observation.is_read_only,
        conflict_group: None,
        conflict_count: 0,
    }
}

fn claude_conflict_group(agent_id: &str, skill_id: &str) -> String {
    format!("{agent_id}::{skill_id}")
}

/// Retrieve all Central Skills (`is_central = true`).
pub async fn get_central_skills(pool: &DbPool) -> Result<Vec<Skill>, String> {
    sqlx::query_as::<_, Skill>("SELECT * FROM skills WHERE is_central = 1")
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())
}

/// Retrieve a skill by its ID.
pub async fn get_skill_by_id(pool: &DbPool, skill_id: &str) -> Result<Option<Skill>, String> {
    sqlx::query_as::<_, Skill>("SELECT * FROM skills WHERE id = ?")
        .bind(skill_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())
}

/// Retrieve multiple skills by ID in one round-trip.
pub async fn get_skills_by_ids(
    pool: &DbPool,
    skill_ids: &[String],
) -> Result<HashMap<String, Skill>, String> {
    if skill_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let placeholders = skill_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!("SELECT * FROM skills WHERE id IN ({placeholders})");
    let mut query = sqlx::query_as::<_, Skill>(&sql);
    for skill_id in skill_ids {
        query = query.bind(skill_id);
    }

    let skills = query.fetch_all(pool).await.map_err(|e| e.to_string())?;
    Ok(skills
        .into_iter()
        .map(|skill| (skill.id.clone(), skill))
        .collect())
}

/// Delete a skill and all its installation records.
pub async fn delete_skill(pool: &DbPool, skill_id: &str) -> Result<(), String> {
    sqlx::query("DELETE FROM skill_update_states WHERE skill_id = ?")
        .bind(skill_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM skill_repository_members WHERE skill_id = ?")
        .bind(skill_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM collection_skills WHERE skill_id = ?")
        .bind(skill_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM skill_tag_links WHERE skill_id = ?")
        .bind(skill_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM skill_ai_tag_reviews WHERE skill_id = ?")
        .bind(skill_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM skill_explanations WHERE skill_id = ?")
        .bind(skill_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM skill_installations WHERE skill_id = ?")
        .bind(skill_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    sqlx::query("DELETE FROM skills WHERE id = ?")
        .bind(skill_id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;
    prune_empty_skill_repositories(pool).await?;
    Ok(())
}

/// Delete skills whose IDs are NOT in `found_skill_ids`. Also cascades to
/// remove any orphaned `skill_installations` rows for those skills.
///
/// This is the global reconciliation step run after a full scan to purge rows
/// for skills that no longer exist on disk in any scanned scope.
///
/// Pass an empty slice to delete ALL skills (used only when every scanned
/// directory is empty or missing).
pub async fn delete_skills_not_in_scope(
    pool: &DbPool,
    found_skill_ids: &[String],
) -> Result<(), String> {
    if found_skill_ids.is_empty() {
        // Nothing found — delete all installation records first, then all skills.
        sqlx::query("DELETE FROM skill_update_states")
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        sqlx::query("DELETE FROM skill_repository_members")
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        sqlx::query("DELETE FROM skill_tag_links")
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        sqlx::query("DELETE FROM skill_installations")
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        sqlx::query("DELETE FROM skills")
            .execute(pool)
            .await
            .map_err(|e| e.to_string())?;
        prune_empty_skill_repositories(pool).await?;
        return Ok(());
    }

    let placeholders = found_skill_ids
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(",");

    // Cascade: remove installation rows for skills that are no longer on disk.
    let repo_sql = format!(
        "DELETE FROM skill_repository_members WHERE skill_id NOT IN ({})",
        placeholders
    );
    let mut repo_q = sqlx::query(&repo_sql);
    for id in found_skill_ids {
        repo_q = repo_q.bind(id.as_str());
    }
    repo_q.execute(pool).await.map_err(|e| e.to_string())?;

    let update_state_sql = format!(
        "DELETE FROM skill_update_states WHERE skill_id NOT IN ({})",
        placeholders
    );
    let mut update_state_q = sqlx::query(&update_state_sql);
    for id in found_skill_ids {
        update_state_q = update_state_q.bind(id.as_str());
    }
    update_state_q
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    let tag_sql = format!(
        "DELETE FROM skill_tag_links WHERE skill_id NOT IN ({})",
        placeholders
    );
    let mut tag_q = sqlx::query(&tag_sql);
    for id in found_skill_ids {
        tag_q = tag_q.bind(id.as_str());
    }
    tag_q.execute(pool).await.map_err(|e| e.to_string())?;

    let install_sql = format!(
        "DELETE FROM skill_installations WHERE skill_id NOT IN ({})",
        placeholders
    );
    let mut q = sqlx::query(&install_sql);
    for id in found_skill_ids {
        q = q.bind(id.as_str());
    }
    q.execute(pool).await.map_err(|e| e.to_string())?;

    // Remove the stale skills themselves.
    let skill_sql = format!("DELETE FROM skills WHERE id NOT IN ({})", placeholders);
    let mut q2 = sqlx::query(&skill_sql);
    for id in found_skill_ids {
        q2 = q2.bind(id.as_str());
    }
    q2.execute(pool).await.map_err(|e| e.to_string())?;
    prune_empty_skill_repositories(pool).await?;
    Ok(())
}
