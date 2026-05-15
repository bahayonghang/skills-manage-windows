use crate::db::{self, DbPool, SkillForAgent};

use super::common::{
    append_missing_agents, claude_conflict_metadata, installation_details, shared_root_agent_ids,
    skill_dir_path, skill_filesystem_timestamps,
};
use super::types::{SkillDetail, SkillWithLinks};

async fn get_observation_detail(
    pool: &DbPool,
    skill_id: &str,
    agent_id: &str,
    row_id: Option<&str>,
) -> Result<Option<SkillDetail>, String> {
    let observations = db::get_agent_skill_observations(pool, agent_id).await?;
    if observations.is_empty() {
        return Ok(None);
    }

    let matches: Vec<db::AgentSkillObservation> = observations
        .into_iter()
        .filter(|observation| observation.skill_id == skill_id)
        .collect();

    if matches.is_empty() {
        return Ok(None);
    }

    let observation = match row_id {
        Some(row_id) => match matches
            .into_iter()
            .find(|observation| observation.row_id == row_id)
        {
            Some(observation) => observation,
            None if row_id == skill_id => return Ok(None),
            None => {
                return Err(format!(
                    "Source row '{}' not found for skill '{}'",
                    row_id, skill_id
                ))
            }
        },
        None if matches.len() == 1 => matches.into_iter().next().expect("single match"),
        None => {
            return Err(format!(
                "Multiple source rows found for skill '{}'; row_id is required",
                skill_id,
            ))
        }
    };

    let manageable_skill = db::get_skill_by_id(pool, &observation.skill_id).await?;
    let installations = if observation.is_read_only {
        Vec::new()
    } else {
        installation_details(db::get_skill_installations(pool, &observation.skill_id).await?)
    };
    let collections = if observation.is_read_only {
        Vec::new()
    } else {
        db::get_skill_collections(pool, &observation.skill_id).await?
    };
    let repository_assignment = if observation.is_read_only {
        None
    } else {
        Some(db::get_skill_repository_assignment(pool, &observation.skill_id).await?)
    };
    let tags = if observation.is_read_only {
        Vec::new()
    } else {
        db::get_skill_tags_for_skill(pool, &observation.skill_id).await?
    };
    let agent_rows = db::get_skills_for_agent(pool, agent_id).await?;
    let mut conflict_counts = std::collections::HashMap::new();
    for row in agent_rows {
        *conflict_counts.entry(row.id).or_insert(0_i64) += 1;
    }
    let (conflict_group, conflict_count) =
        claude_conflict_metadata(agent_id, &observation.skill_id, &conflict_counts);

    Ok(Some(SkillDetail {
        row_id: observation.row_id,
        id: observation.skill_id.clone(),
        name: observation.name,
        description: observation.description.or_else(|| {
            manageable_skill
                .as_ref()
                .and_then(|skill| skill.description.clone())
        }),
        file_path: observation.file_path,
        dir_path: observation.dir_path,
        canonical_path: if observation.is_read_only {
            None
        } else {
            manageable_skill
                .as_ref()
                .and_then(|skill| skill.canonical_path.clone())
        },
        is_central: manageable_skill
            .as_ref()
            .map(|skill| skill.is_central)
            .unwrap_or(false),
        source: manageable_skill
            .as_ref()
            .and_then(|skill| skill.source.clone())
            .or_else(|| Some(observation.link_type.clone())),
        scanned_at: observation.scanned_at,
        source_kind: Some(observation.source_kind),
        source_root: Some(observation.source_root),
        is_read_only: observation.is_read_only,
        conflict_group,
        conflict_count,
        installations,
        collections,
        repository: repository_assignment
            .as_ref()
            .map(|assignment| assignment.repository.clone()),
        tags,
        source_path: repository_assignment
            .as_ref()
            .and_then(|assignment| assignment.source_path.clone()),
        is_source_unknown: repository_assignment
            .as_ref()
            .map(|assignment| assignment.is_source_unknown)
            .unwrap_or(true),
    }))
}

pub async fn get_skill_detail_with_row_impl(
    pool: &DbPool,
    skill_id: &str,
    agent_id: Option<&str>,
    row_id: Option<&str>,
) -> Result<SkillDetail, String> {
    if let Some(agent_id) = agent_id {
        if let Some(detail) = get_observation_detail(pool, skill_id, agent_id, row_id).await? {
            return Ok(detail);
        }
    }

    let skill = db::get_skill_by_id(pool, skill_id)
        .await?
        .ok_or_else(|| format!("Skill '{}' not found", skill_id))?;

    let row_id = skill.id.clone();
    let dir_path = skill_dir_path(&skill);
    let installations = installation_details(db::get_skill_installations(pool, skill_id).await?);
    let collections = db::get_skill_collections(pool, skill_id).await?;
    let repository_assignment = db::get_skill_repository_assignment(pool, skill_id).await?;
    let tags = db::get_skill_tags_for_skill(pool, skill_id).await?;

    Ok(SkillDetail {
        row_id,
        id: skill.id,
        name: skill.name,
        description: skill.description,
        file_path: skill.file_path,
        dir_path,
        canonical_path: skill.canonical_path,
        is_central: skill.is_central,
        source: skill.source,
        scanned_at: skill.scanned_at,
        source_kind: None,
        source_root: None,
        is_read_only: false,
        conflict_group: None,
        conflict_count: 0,
        installations,
        collections,
        repository: Some(repository_assignment.repository),
        tags,
        source_path: repository_assignment.source_path,
        is_source_unknown: repository_assignment.is_source_unknown,
    })
}

/// Testable core implementation of `get_skills_by_agent`.
///
/// Returns skills for the given agent enriched with installation metadata
/// (`dir_path`, `link_type`, `symlink_target`) so the frontend `SkillCard`
/// can display the correct source indicator.
pub async fn get_skills_by_agent_impl(
    pool: &DbPool,
    agent_id: &str,
) -> Result<Vec<SkillForAgent>, String> {
    db::get_skills_for_agent(pool, agent_id).await
}

pub async fn get_central_skills_impl(pool: &DbPool) -> Result<Vec<SkillWithLinks>, String> {
    let skills = db::get_central_skills(pool).await?;
    let agents = db::get_all_agents(pool).await?;
    let shared_root_agents = shared_root_agent_ids(&agents);
    let skill_ids = skills
        .iter()
        .map(|skill| skill.id.clone())
        .collect::<Vec<_>>();
    let mut installations_by_skill =
        db::get_skill_installations_for_skills(pool, &skill_ids).await?;
    let mut repository_assignments =
        db::get_skill_repository_assignments_for_skills(pool, &skill_ids).await?;
    let mut tags_by_skill = db::get_skill_tags_for_skills(pool, &skill_ids).await?;
    let unknown_repository = db::get_local_unknown_repository(pool).await?;
    let mut result = Vec::with_capacity(skills.len());
    for skill in skills {
        let installations = installations_by_skill.remove(&skill.id).unwrap_or_default();
        let mut linked_agents: Vec<String> =
            installations.into_iter().map(|i| i.agent_id).collect();
        append_missing_agents(&mut linked_agents, &shared_root_agents);
        let (created_at, updated_at) = skill_filesystem_timestamps(&skill);
        let repository_assignment = repository_assignments.remove(&skill.id).unwrap_or_else(|| {
            db::SkillRepositoryAssignment {
                repository: unknown_repository.clone(),
                source_path: None,
                is_source_unknown: true,
            }
        });
        let tags = tags_by_skill.remove(&skill.id).unwrap_or_default();

        result.push(SkillWithLinks {
            id: skill.id,
            name: skill.name,
            description: skill.description,
            file_path: skill.file_path,
            canonical_path: skill.canonical_path,
            is_central: skill.is_central,
            source: skill.source,
            scanned_at: skill.scanned_at,
            created_at,
            updated_at,
            linked_agents,
            shared_root_agents: shared_root_agents.clone(),
            repository: Some(repository_assignment.repository),
            tags,
            source_path: repository_assignment.source_path,
            is_source_unknown: repository_assignment.is_source_unknown,
        });
    }

    Ok(result)
}
