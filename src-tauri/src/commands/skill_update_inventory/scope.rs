use std::collections::HashSet;

use crate::db::{self, DbPool};

use super::{SkillRefreshScope, SkillRefreshScopeKind};

pub(crate) enum PendingAdditionScope {
    All,
    Repositories(HashSet<String>),
    SkillIds(HashSet<String>),
    None,
}

pub(crate) struct InventoryScopeFilter {
    pub(crate) agent_ids: Option<Vec<String>>,
    pub(crate) pending: PendingAdditionScope,
}

impl InventoryScopeFilter {
    pub(crate) async fn from_scope(
        _pool: &DbPool,
        scope: Option<SkillRefreshScope>,
    ) -> Result<Self, String> {
        let Some(scope) = scope else {
            return Ok(Self {
                agent_ids: None,
                pending: PendingAdditionScope::All,
            });
        };

        match scope.kind {
            SkillRefreshScopeKind::All => Ok(Self {
                agent_ids: None,
                pending: PendingAdditionScope::All,
            }),
            SkillRefreshScopeKind::Skills => {
                let skill_ids = normalize_ids(scope.skill_ids.unwrap_or_default())
                    .into_iter()
                    .collect::<HashSet<_>>();
                let pending = if skill_ids.is_empty() {
                    PendingAdditionScope::None
                } else {
                    PendingAdditionScope::SkillIds(skill_ids.clone())
                };
                Ok(Self {
                    agent_ids: None,
                    pending,
                })
            }
            SkillRefreshScopeKind::Repositories => {
                let repository_ids = normalize_ids(scope.repository_ids.unwrap_or_default());
                let repository_ids = repository_ids.into_iter().collect::<HashSet<_>>();
                let pending = if repository_ids.is_empty() {
                    PendingAdditionScope::None
                } else {
                    PendingAdditionScope::Repositories(repository_ids)
                };
                Ok(Self {
                    agent_ids: None,
                    pending,
                })
            }
            SkillRefreshScopeKind::Platform => {
                let agent_ids = normalize_ids(scope.agent_ids.unwrap_or_default());
                Ok(Self {
                    agent_ids: Some(agent_ids),
                    pending: PendingAdditionScope::None,
                })
            }
        }
    }
}

pub(crate) async fn central_skill_ids_for_agents(
    pool: &DbPool,
    agent_ids: &[String],
) -> Result<Vec<String>, String> {
    if agent_ids.is_empty() {
        return Ok(Vec::new());
    }

    let central_skill_ids = db::get_central_skills(pool)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|skill| skill.id)
        .collect::<HashSet<_>>();
    let mut ids = HashSet::new();

    for agent_id in agent_ids {
        for observation in db::get_agent_skill_observations(pool, agent_id)
            .await
            .map_err(|e| e.to_string())?
        {
            if central_skill_ids.contains(&observation.skill_id) {
                ids.insert(observation.skill_id);
            }
        }

        let installed_skill_ids = sqlx::query_scalar::<_, String>(
            "SELECT si.skill_id
             FROM skill_installations si
             JOIN skills s ON s.id = si.skill_id AND s.is_central = 1
             WHERE si.agent_id = ?",
        )
        .bind(agent_id)
        .fetch_all(pool)
        .await
        .map_err(|e| e.to_string())?;
        ids.extend(installed_skill_ids);
    }

    let mut ids = ids.into_iter().collect::<Vec<_>>();
    ids.sort();
    Ok(ids)
}

pub(crate) fn normalize_ids(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            continue;
        }
        if seen.insert(trimmed.to_string()) {
            normalized.push(trimmed.to_string());
        }
    }
    normalized
}

pub(crate) fn normalize_optional_id_set(values: Option<&[String]>) -> Option<HashSet<String>> {
    values
        .map(|ids| {
            normalize_ids(ids.to_vec())
                .into_iter()
                .collect::<HashSet<_>>()
        })
        .filter(|ids| !ids.is_empty())
}
