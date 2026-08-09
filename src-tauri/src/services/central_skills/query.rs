use std::collections::HashSet;

#[cfg(test)]
use std::cmp::Ordering;

use crate::db::{self, DbPool, SkillForAgent};

use super::common::{
    append_missing_agents, claude_conflict_metadata, installation_details, shared_root_agent_ids,
    skill_dir_path, skill_filesystem_timestamps,
};
use super::error::CentralSkillsError;
use super::types::{CentralSkillsPage, CentralSkillsPageRequest, SkillDetail, SkillWithLinks};

async fn get_observation_detail(
    pool: &DbPool,
    skill_id: &str,
    agent_id: &str,
    row_id: Option<&str>,
) -> Result<Option<SkillDetail>, CentralSkillsError> {
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
                return Err(CentralSkillsError::SourceRowNotFound {
                    row_id: row_id.to_string(),
                    skill_id: skill_id.to_string(),
                })
            }
        },
        None if matches.len() == 1 => matches.into_iter().next().expect("single match"),
        None => {
            return Err(CentralSkillsError::MultipleSourceRows(skill_id.to_string()));
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
        uid: manageable_skill.as_ref().map(|skill| skill.uid.clone()),
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
) -> Result<SkillDetail, CentralSkillsError> {
    if let Some(agent_id) = agent_id {
        if let Some(detail) = get_observation_detail(pool, skill_id, agent_id, row_id).await? {
            return Ok(detail);
        }
    }

    let skill = db::get_skill_by_id(pool, skill_id)
        .await?
        .ok_or_else(|| CentralSkillsError::SkillNotFound(skill_id.to_string()))?;

    let row_id = skill.id.clone();
    let dir_path = skill_dir_path(&skill);
    let installations = installation_details(db::get_skill_installations(pool, skill_id).await?);
    let collections = db::get_skill_collections(pool, skill_id).await?;
    let repository_assignment = db::get_skill_repository_assignment(pool, skill_id).await?;
    let tags = db::get_skill_tags_for_skill(pool, skill_id).await?;

    Ok(SkillDetail {
        row_id,
        id: skill.id,
        uid: Some(skill.uid),
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
) -> Result<Vec<SkillForAgent>, CentralSkillsError> {
    Ok(db::get_skills_for_agent(pool, agent_id).await?)
}

pub async fn get_central_skills_impl(
    pool: &DbPool,
) -> Result<Vec<SkillWithLinks>, CentralSkillsError> {
    let skills = db::get_central_skills(pool).await?;
    let agents = db::get_all_agents(pool).await?;
    skills_with_links_from_rows(pool, skills, &agents, TimestampAuthority::Filesystem).await
}

#[derive(Clone, Copy)]
enum TimestampAuthority {
    Filesystem,
    Persisted,
}

async fn skills_with_links_from_rows(
    pool: &DbPool,
    skills: Vec<db::Skill>,
    agents: &[db::Agent],
    timestamp_authority: TimestampAuthority,
) -> Result<Vec<SkillWithLinks>, CentralSkillsError> {
    if skills.is_empty() {
        return Ok(Vec::new());
    }

    let shared_root_agents = shared_root_agent_ids(agents);
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
        let (created_at, updated_at) = match timestamp_authority {
            TimestampAuthority::Filesystem => skill_filesystem_timestamps(&skill),
            TimestampAuthority::Persisted => crate::skill_time::skill_persisted_timestamps(&skill),
        };
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
            uid: skill.uid,
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

pub async fn resolve_skill_ref_impl(
    pool: &DbPool,
    reference: &str,
) -> Result<db::Skill, CentralSkillsError> {
    if let Some(skill) = db::get_skill_by_uid(pool, reference)
        .await?
        .filter(|skill| skill.is_central)
    {
        return Ok(skill);
    }
    if let Some(skill) = db::get_skill_by_id(pool, reference)
        .await?
        .filter(|skill| skill.is_central)
    {
        return Ok(skill);
    }

    let mut matches = db::get_central_skills_by_exact_name(pool, reference).await?;
    match matches.len() {
        0 => Err(CentralSkillsError::SkillNotFound(reference.to_string())),
        1 => Ok(matches.remove(0)),
        _ => Err(CentralSkillsError::AmbiguousSkillReference(
            reference.to_string(),
        )),
    }
}

pub async fn get_central_skills_page_impl(
    pool: &DbPool,
    request: CentralSkillsPageRequest,
) -> Result<CentralSkillsPage, CentralSkillsError> {
    get_central_skills_page_with_observer_inner(pool, request, |_| {}).await
}

#[cfg(test)]
pub(super) async fn get_central_skills_page_with_observer<F>(
    pool: &DbPool,
    request: CentralSkillsPageRequest,
    observer: F,
) -> Result<CentralSkillsPage, CentralSkillsError>
where
    F: FnOnce(&[db::Skill]),
{
    get_central_skills_page_with_observer_inner(pool, request, observer).await
}

async fn get_central_skills_page_with_observer_inner<F>(
    pool: &DbPool,
    request: CentralSkillsPageRequest,
    observer: F,
) -> Result<CentralSkillsPage, CentralSkillsError>
where
    F: FnOnce(&[db::Skill]),
{
    let mut filter = normalize_central_skills_page_request(request)?;
    let agents = db::get_all_agents(pool).await?;
    filter.has_shared_root_agent = !shared_root_agent_ids(&agents).is_empty();
    let (rows, total) = db::get_central_skills_page(pool, &filter).await?;
    observer(&rows);
    let items =
        skills_with_links_from_rows(pool, rows, &agents, TimestampAuthority::Persisted).await?;
    Ok(CentralSkillsPage { items, total })
}

const MAX_PAGE_FILTER_VALUES: usize = 100;

fn normalize_central_skills_page_request(
    request: CentralSkillsPageRequest,
) -> Result<db::CentralSkillPageQuery, CentralSkillsError> {
    let query = request
        .query
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase());
    let mut sources = normalized_filter_values("source", request.source)?;
    let include_unassigned = sources.iter().any(|value| value == "unassigned");
    sources.retain(|value| value != "unassigned");
    let mut tags = normalized_filter_values("tags", request.tags)?;
    let include_uncategorized = tags.iter().any(|value| value == db::UNCATEGORIZED_TAG_ID);
    tags.retain(|value| value != db::UNCATEGORIZED_TAG_ID);
    let install = match request.install_state.as_deref() {
        Some("linked" | "installed") => db::CentralSkillInstallFilter::Linked,
        Some("unlinked" | "not_installed" | "notInstalled") => {
            db::CentralSkillInstallFilter::Unlinked
        }
        _ => db::CentralSkillInstallFilter::All,
    };
    let (sort, descending) = parse_page_sort(request.sort.as_deref());

    Ok(db::CentralSkillPageQuery {
        query,
        sources,
        include_unassigned,
        tags,
        include_uncategorized,
        install,
        has_shared_root_agent: false,
        sort,
        descending,
        limit: request.limit.unwrap_or(100).clamp(1, 500),
        offset: request.offset.unwrap_or(0).max(0),
    })
}

fn normalized_filter_values(
    field: &'static str,
    values: Vec<String>,
) -> Result<Vec<String>, CentralSkillsError> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();
    for value in values {
        let value = value.trim();
        if value.is_empty() || value == "all" || !seen.insert(value.to_string()) {
            continue;
        }
        normalized.push(value.to_string());
        if normalized.len() > MAX_PAGE_FILTER_VALUES {
            return Err(CentralSkillsError::PageFilterValuesExceeded {
                field,
                limit: MAX_PAGE_FILTER_VALUES,
            });
        }
    }
    Ok(normalized)
}

fn parse_page_sort(sort: Option<&str>) -> (db::CentralSkillPageSort, bool) {
    let Some((field, direction)) = sort.and_then(|value| value.split_once(':')) else {
        return (db::CentralSkillPageSort::Name, false);
    };
    let field = match field {
        "name" => db::CentralSkillPageSort::Name,
        "createdAt" | "created_at" => db::CentralSkillPageSort::CreatedAt,
        "updatedAt" | "updated_at" => db::CentralSkillPageSort::UpdatedAt,
        _ => return (db::CentralSkillPageSort::Name, false),
    };
    let descending = match direction {
        "asc" => false,
        "desc" => true,
        _ => return (db::CentralSkillPageSort::Name, false),
    };
    (field, descending)
}

#[cfg(test)]
fn matches_page_query(skill: &SkillWithLinks, query: Option<&str>) -> bool {
    let Some(query) = query else {
        return true;
    };
    skill.name.to_ascii_lowercase().contains(query)
        || skill
            .description
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase()
            .contains(query)
        || skill.id.to_ascii_lowercase().contains(query)
}

#[cfg(test)]
fn matches_page_source(skill: &SkillWithLinks, filter: &db::CentralSkillPageQuery) -> bool {
    if filter.sources.is_empty() && !filter.include_unassigned {
        return true;
    }

    let repo_id = skill
        .repository
        .as_ref()
        .map(|repository| repository.id.as_str());
    let is_unassigned = skill.is_source_unknown
        || skill
            .repository
            .as_ref()
            .map(|repository| repository.is_unknown)
            .unwrap_or(false);

    (filter.include_unassigned && is_unassigned)
        || repo_id.is_some_and(|repo_id| filter.sources.iter().any(|value| value == repo_id))
}

#[cfg(test)]
fn matches_page_tags(skill: &SkillWithLinks, filter: &db::CentralSkillPageQuery) -> bool {
    if filter.tags.is_empty() && !filter.include_uncategorized {
        return true;
    }

    (filter.include_uncategorized
        && (skill.tags.is_empty()
            || skill
                .tags
                .iter()
                .all(|tag| tag.id == db::UNCATEGORIZED_TAG_ID)))
        || skill
            .tags
            .iter()
            .any(|tag| filter.tags.iter().any(|value| value == &tag.id))
}

#[cfg(test)]
fn matches_page_install_state(
    skill: &SkillWithLinks,
    state: db::CentralSkillInstallFilter,
) -> bool {
    match state {
        db::CentralSkillInstallFilter::Linked => !skill.linked_agents.is_empty(),
        db::CentralSkillInstallFilter::Unlinked => skill.linked_agents.is_empty(),
        db::CentralSkillInstallFilter::All => true,
    }
}

#[cfg(test)]
fn sort_central_skill_page_items(
    items: &mut [SkillWithLinks],
    sort: db::CentralSkillPageSort,
    descending: bool,
) {
    items.sort_by(|left, right| {
        let ordering = match sort {
            db::CentralSkillPageSort::CreatedAt => left
                .created_at
                .cmp(&right.created_at)
                .then_with(|| {
                    left.name
                        .to_ascii_lowercase()
                        .cmp(&right.name.to_ascii_lowercase())
                })
                .then_with(|| left.name.cmp(&right.name))
                .then_with(|| left.id.cmp(&right.id)),
            db::CentralSkillPageSort::UpdatedAt => left
                .updated_at
                .cmp(&right.updated_at)
                .then_with(|| {
                    left.name
                        .to_ascii_lowercase()
                        .cmp(&right.name.to_ascii_lowercase())
                })
                .then_with(|| left.name.cmp(&right.name))
                .then_with(|| left.id.cmp(&right.id)),
            db::CentralSkillPageSort::Name => left
                .name
                .to_ascii_lowercase()
                .cmp(&right.name.to_ascii_lowercase())
                .then_with(|| left.name.cmp(&right.name))
                .then_with(|| left.id.cmp(&right.id)),
        };
        match (descending, ordering) {
            (true, Ordering::Less) => Ordering::Greater,
            (true, Ordering::Greater) => Ordering::Less,
            _ => ordering,
        }
    });
}

#[cfg(test)]
pub(super) async fn get_central_skills_page_reference_impl(
    pool: &DbPool,
    request: CentralSkillsPageRequest,
) -> Result<CentralSkillsPage, CentralSkillsError> {
    let mut filter = normalize_central_skills_page_request(request)?;
    let agents = db::get_all_agents(pool).await?;
    filter.has_shared_root_agent = !shared_root_agent_ids(&agents).is_empty();
    let skills = db::get_central_skills(pool).await?;
    let mut items =
        skills_with_links_from_rows(pool, skills, &agents, TimestampAuthority::Persisted).await?;
    items.retain(|skill| {
        matches_page_query(skill, filter.query.as_deref())
            && matches_page_source(skill, &filter)
            && matches_page_tags(skill, &filter)
            && matches_page_install_state(skill, filter.install)
    });
    sort_central_skill_page_items(&mut items, filter.sort, filter.descending);

    let total = items.len();
    let offset = usize::try_from(filter.offset).unwrap_or(usize::MAX);
    let limit = usize::try_from(filter.limit).unwrap_or(0);
    let items = items.into_iter().skip(offset).take(limit).collect();
    Ok(CentralSkillsPage { items, total })
}
