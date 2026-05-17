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
use crate::db::repos::repositories_repo::{
    get_skill_repository_assignments_for_skills, prune_empty_skill_repositories,
};
use crate::db::types::{
    AgentSkillObservation, DbPool, Skill, SkillRepository, SkillRepositoryAssignment,
};
use crate::skill_time::skill_filesystem_timestamps;

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
    let observations = get_agent_skill_observations(pool, agent_id).await?;
    if agent_id == "claude-code" && !observations.is_empty() {
        return Ok(observations.into_iter().map(observation_to_skill).collect());
    }

    let mut skills = sqlx::query_as::<_, Skill>(
        "SELECT s.* FROM skills s
         JOIN skill_installations si ON s.id = si.skill_id
         WHERE si.agent_id = ?",
    )
    .bind(agent_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    skills.extend(observations.into_iter().map(observation_to_skill));
    Ok(skills)
}

/// A skill enriched with the installation-specific fields for a given agent.
///
/// Returned by `get_skills_for_agent`. The extra fields come from the
/// `skill_installations` row and allow the frontend `SkillCard` to display
/// the correct source indicator without a second round-trip.
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Scan timestamp from the central skill row or observation row.
    pub scanned_at: String,
    /// Installation timestamp for writable platform rows.
    pub installed_at: Option<String>,
    /// Filesystem-created timestamp when available; otherwise `scanned_at`.
    pub created_at: Option<String>,
    /// Filesystem-modified timestamp when available; otherwise `scanned_at`.
    pub updated_at: Option<String>,
    /// Central repository assignment for writable rows, when known.
    pub repository: Option<SkillRepository>,
    pub source_path: Option<String>,
    pub is_source_unknown: bool,
    pub source_kind: Option<String>,
    pub source_root: Option<String>,
    pub is_read_only: bool,
    pub conflict_group: Option<String>,
    pub conflict_count: i64,
}

#[derive(Debug, FromRow)]
struct InstalledSkillForAgentRow {
    id: String,
    name: String,
    description: Option<String>,
    file_path: String,
    canonical_path: Option<String>,
    is_central: bool,
    source: Option<String>,
    scanned_at: String,
    dir_path: String,
    link_type: String,
    symlink_target: Option<String>,
    installed_at: String,
    repository_id: Option<String>,
    repository_name: Option<String>,
    repository_source_type: Option<String>,
    repository_owner: Option<String>,
    repository_repo: Option<String>,
    repository_branch: Option<String>,
    repository_url: Option<String>,
    repository_is_unknown: Option<bool>,
    repository_created_at: Option<String>,
    repository_updated_at: Option<String>,
    source_path: Option<String>,
}

/// Retrieve skills installed for a given agent, enriched with installation
/// metadata (`dir_path`, `link_type`, `symlink_target`) required by the
/// platform-view skill cards.
pub async fn get_skills_for_agent(
    pool: &DbPool,
    agent_id: &str,
) -> Result<Vec<SkillForAgent>, String> {
    let observations = get_agent_skill_observations(pool, agent_id).await?;
    if agent_id == "claude-code" && !observations.is_empty() {
        let mut skills = observations_to_skills_for_agent(pool, observations).await?;
        apply_conflict_metadata(agent_id, &mut skills);
        return Ok(skills);
    }

    let rows = sqlx::query_as::<_, InstalledSkillForAgentRow>(
        "SELECT s.id,
                s.name,
                s.description,
                s.file_path,
                s.canonical_path,
                s.source,
                s.scanned_at,
                si.installed_path AS dir_path,
                si.link_type,
                si.symlink_target,
                si.created_at AS installed_at,
                s.is_central,
                r.id AS repository_id,
                r.name AS repository_name,
                r.source_type AS repository_source_type,
                r.owner AS repository_owner,
                r.repo AS repository_repo,
                r.branch AS repository_branch,
                r.url AS repository_url,
                r.is_unknown AS repository_is_unknown,
                r.created_at AS repository_created_at,
                r.updated_at AS repository_updated_at,
                m.source_path AS source_path
         FROM skills s
         JOIN skill_installations si ON s.id = si.skill_id
         LEFT JOIN skill_repository_members m ON s.id = m.skill_id
         LEFT JOIN skill_repositories r ON r.id = m.repository_id
         WHERE si.agent_id = ?",
    )
    .bind(agent_id)
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut skills = rows
        .into_iter()
        .map(installed_row_to_skill_for_agent)
        .collect::<Result<Vec<_>, _>>()?;
    skills.extend(observations_to_skills_for_agent(pool, observations).await?);
    apply_conflict_metadata(agent_id, &mut skills);
    Ok(skills)
}

fn installed_row_to_skill_for_agent(row: InstalledSkillForAgentRow) -> Result<SkillForAgent, String> {
    let skill = Skill {
        id: row.id.clone(),
        name: row.name.clone(),
        description: row.description.clone(),
        file_path: row.file_path.clone(),
        canonical_path: row.canonical_path.clone(),
        is_central: row.is_central,
        source: row.source.clone(),
        content: None,
        scanned_at: row.scanned_at.clone(),
    };
    let (created_at, updated_at) = skill_filesystem_timestamps(&skill);
    let repository = repository_from_installed_row(&row)?;
    let is_source_unknown = repository
        .as_ref()
        .map(|repository| repository.is_unknown)
        .unwrap_or(true);

    Ok(SkillForAgent {
        id: row.id.clone(),
        row_id: row.id,
        name: row.name,
        description: row.description,
        file_path: row.file_path,
        dir_path: row.dir_path,
        link_type: row.link_type,
        symlink_target: row.symlink_target,
        is_central: row.is_central,
        scanned_at: row.scanned_at,
        installed_at: Some(row.installed_at),
        created_at: Some(created_at),
        updated_at: Some(updated_at),
        repository,
        source_path: row.source_path,
        is_source_unknown,
        source_kind: None,
        source_root: None,
        is_read_only: false,
        conflict_group: None,
        conflict_count: 0,
    })
}

fn repository_from_installed_row(
    row: &InstalledSkillForAgentRow,
) -> Result<Option<SkillRepository>, String> {
    let Some(id) = row.repository_id.clone() else {
        return Ok(None);
    };

    Ok(Some(SkillRepository {
        id,
        name: row
            .repository_name
            .clone()
            .ok_or_else(|| "Repository row missing name".to_string())?,
        source_type: row
            .repository_source_type
            .clone()
            .ok_or_else(|| "Repository row missing source_type".to_string())?,
        owner: row.repository_owner.clone(),
        repo: row.repository_repo.clone(),
        branch: row.repository_branch.clone(),
        url: row.repository_url.clone(),
        is_unknown: row.repository_is_unknown.unwrap_or(true),
        created_at: row
            .repository_created_at
            .clone()
            .ok_or_else(|| "Repository row missing created_at".to_string())?,
        updated_at: row
            .repository_updated_at
            .clone()
            .ok_or_else(|| "Repository row missing updated_at".to_string())?,
    }))
}

async fn observations_to_skills_for_agent(
    pool: &DbPool,
    observations: Vec<AgentSkillObservation>,
) -> Result<Vec<SkillForAgent>, String> {
    if observations.is_empty() {
        return Ok(Vec::new());
    }

    let writable_skill_ids = observations
        .iter()
        .filter(|observation| !observation.is_read_only)
        .map(|observation| observation.skill_id.clone())
        .collect::<Vec<_>>();
    let repository_assignments =
        get_skill_repository_assignments_for_skills(pool, &writable_skill_ids).await?;
    let skills_by_id = get_skills_by_ids(pool, &writable_skill_ids).await?;

    observations
        .into_iter()
        .map(|observation| {
            let skill = skills_by_id.get(&observation.skill_id);
            let (created_at, updated_at) = skill
                .map(skill_filesystem_timestamps)
                .unwrap_or_else(|| (observation.scanned_at.clone(), observation.scanned_at.clone()));
            let repository_assignment = if observation.is_read_only {
                None
            } else {
                repository_assignments.get(&observation.skill_id).cloned()
            };
            Ok(observation_to_skill_for_agent(
                observation,
                repository_assignment,
                created_at,
                updated_at,
            ))
        })
        .collect()
}

fn observation_to_skill_for_agent(
    observation: AgentSkillObservation,
    repository_assignment: Option<SkillRepositoryAssignment>,
    created_at: String,
    updated_at: String,
) -> SkillForAgent {
    let repository = repository_assignment
        .as_ref()
        .map(|assignment| assignment.repository.clone());
    let source_path = repository_assignment
        .as_ref()
        .and_then(|assignment| assignment.source_path.clone());
    let is_source_unknown = repository_assignment
        .as_ref()
        .map(|assignment| assignment.is_source_unknown)
        .unwrap_or(true);

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
        scanned_at: observation.scanned_at.clone(),
        installed_at: None,
        created_at: Some(created_at),
        updated_at: Some(updated_at),
        repository,
        source_path,
        is_source_unknown,
        source_kind: Some(observation.source_kind),
        source_root: Some(observation.source_root),
        is_read_only: observation.is_read_only,
        conflict_group: None,
        conflict_count: 0,
    }
}

fn apply_conflict_metadata(agent_id: &str, skills: &mut [SkillForAgent]) {
    let mut conflict_counts = HashMap::new();
    for skill in skills.iter() {
        *conflict_counts.entry(skill.id.clone()).or_insert(0_i64) += 1;
    }

    for skill in skills.iter_mut() {
        let conflict_count = conflict_counts.get(&skill.id).copied().unwrap_or(0);
        if conflict_count > 1 {
            skill.conflict_group = Some(conflict_group(agent_id, &skill.id));
            skill.conflict_count = conflict_count;
        }
    }
}

fn conflict_group(agent_id: &str, skill_id: &str) -> String {
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

/// Retrieve Central Skills by ID in caller-provided order.
pub async fn get_central_skills_by_ids(
    pool: &DbPool,
    skill_ids: &[String],
) -> Result<Vec<Skill>, String> {
    if skill_ids.is_empty() {
        return Ok(Vec::new());
    }

    let placeholders = skill_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!("SELECT * FROM skills WHERE is_central = 1 AND id IN ({placeholders})");
    let mut query = sqlx::query_as::<_, Skill>(&sql);
    for skill_id in skill_ids {
        query = query.bind(skill_id);
    }

    let skills = query.fetch_all(pool).await.map_err(|e| e.to_string())?;
    let mut by_id = skills
        .into_iter()
        .map(|skill| (skill.id.clone(), skill))
        .collect::<HashMap<_, _>>();
    Ok(skill_ids
        .iter()
        .filter_map(|skill_id| by_id.remove(skill_id))
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
