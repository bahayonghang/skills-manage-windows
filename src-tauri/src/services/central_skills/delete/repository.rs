//! Repository-level deletion orchestration: previews and deletes that operate
//! on every skill belonging to a skill repository (local and remote targets).

use std::collections::{HashMap, HashSet};

use crate::db::{self, DbPool, SkillRepository, SkillRepositoryWithStats};
use crate::targets::{ActiveTarget, RemoteTargetConfig};

use super::super::common::unique_agent_ids;
use super::super::error::CentralSkillsError;
use super::super::types::{
    BatchDeleteCentralSkillRequest, DeleteSkillRepositoryPreview, DeleteSkillRepositoryResult,
};
use super::{
    delete_central_skills_impl, delete_central_skills_remote_impl,
    preview_delete_central_skills_impl, preview_delete_central_skills_ssh_impl,
};

async fn get_deletable_repository_with_skill_ids(
    pool: &DbPool,
    repository_id: &str,
) -> Result<(SkillRepository, Vec<String>), CentralSkillsError> {
    let repository = db::get_skill_repository_by_id(pool, repository_id)
        .await
        .map_err(CentralSkillsError::Other)? // TODO(C3): typed repos passthrough
        .ok_or_else(|| CentralSkillsError::RepositoryNotFound(repository_id.to_string()))?;
    if repository.id == db::LOCAL_UNKNOWN_REPOSITORY_ID || repository.is_unknown {
        return Err(CentralSkillsError::UnknownRepositoryUndeletable);
    }

    let skill_ids = db::get_central_skill_ids_by_repository(pool, &repository.id)
        .await
        .map_err(CentralSkillsError::Other)?; // TODO(C3): typed repos passthrough
    Ok((repository, skill_ids))
}

fn repository_with_stats(
    repository: SkillRepository,
    skill_count: usize,
) -> SkillRepositoryWithStats {
    SkillRepositoryWithStats {
        repository,
        skill_count: skill_count as i64,
        unknown_skill_count: 0,
    }
}

fn build_repository_delete_requests(
    repository_id: &str,
    skill_ids: &[String],
    requests: &[BatchDeleteCentralSkillRequest],
) -> Result<Vec<BatchDeleteCentralSkillRequest>, CentralSkillsError> {
    let valid_skill_ids: HashSet<&str> = skill_ids.iter().map(String::as_str).collect();
    let mut remove_agents_by_skill: HashMap<String, Vec<String>> = HashMap::new();

    for request in requests {
        if !valid_skill_ids.contains(request.skill_id.as_str()) {
            return Err(CentralSkillsError::SkillNotInRepository {
                skill_id: request.skill_id.clone(),
                repository_id: repository_id.to_string(),
            });
        }

        let entry = remove_agents_by_skill
            .entry(request.skill_id.clone())
            .or_default();
        for agent_id in &request.remove_agent_ids {
            if !entry.contains(agent_id) {
                entry.push(agent_id.clone());
            }
        }
    }

    Ok(skill_ids
        .iter()
        .map(|skill_id| BatchDeleteCentralSkillRequest {
            skill_id: skill_id.clone(),
            remove_agent_ids: remove_agents_by_skill
                .remove(skill_id)
                .map(unique_agent_ids)
                .unwrap_or_default(),
        })
        .collect())
}

pub async fn preview_delete_skill_repository_impl(
    pool: &DbPool,
    repository_id: &str,
) -> Result<DeleteSkillRepositoryPreview, CentralSkillsError> {
    let (repository, skill_ids) =
        get_deletable_repository_with_skill_ids(pool, repository_id).await?;
    let delete_preview = preview_delete_central_skills_impl(pool, &skill_ids).await?;

    Ok(DeleteSkillRepositoryPreview {
        repository: repository_with_stats(repository, skill_ids.len()),
        delete_preview,
    })
}

pub async fn preview_delete_skill_repository_ssh_impl(
    pool: &DbPool,
    repository_id: &str,
) -> Result<DeleteSkillRepositoryPreview, CentralSkillsError> {
    let (repository, skill_ids) =
        get_deletable_repository_with_skill_ids(pool, repository_id).await?;
    let delete_preview = preview_delete_central_skills_ssh_impl(pool, &skill_ids).await?;

    Ok(DeleteSkillRepositoryPreview {
        repository: repository_with_stats(repository, skill_ids.len()),
        delete_preview,
    })
}

pub async fn delete_skill_repository_impl(
    pool: &DbPool,
    repository_id: &str,
    requests: &[BatchDeleteCentralSkillRequest],
) -> Result<DeleteSkillRepositoryResult, CentralSkillsError> {
    let (repository, skill_ids) =
        get_deletable_repository_with_skill_ids(pool, repository_id).await?;
    let delete_requests = build_repository_delete_requests(&repository.id, &skill_ids, requests)?;
    let delete_result = delete_central_skills_impl(pool, &delete_requests).await?;
    let deleted_repository = if delete_result.failed.is_empty() {
        if skill_ids.is_empty() {
            db::delete_empty_skill_repository(pool, &repository.id)
                .await
                .map_err(CentralSkillsError::Other)? // TODO(C3): typed repos passthrough
        } else {
            db::prune_empty_skill_repositories(pool)
                .await
                .map_err(CentralSkillsError::Other)?; // TODO(C3): typed repos passthrough
            db::get_skill_repository_by_id(pool, &repository.id)
                .await
                .map_err(CentralSkillsError::Other)? // TODO(C3): typed repos passthrough
                .is_none()
        }
    } else {
        false
    };

    Ok(DeleteSkillRepositoryResult {
        repository,
        deleted_repository,
        delete_result,
    })
}

pub async fn delete_skill_repository_remote_impl(
    pool: &DbPool,
    active_target: &ActiveTarget,
    repository_id: &str,
    requests: &[BatchDeleteCentralSkillRequest],
) -> Result<DeleteSkillRepositoryResult, CentralSkillsError> {
    let (repository, skill_ids) =
        get_deletable_repository_with_skill_ids(pool, repository_id).await?;
    let delete_requests = build_repository_delete_requests(&repository.id, &skill_ids, requests)?;
    let delete_result =
        delete_central_skills_remote_impl(pool, active_target, &delete_requests).await?;
    let deleted_repository = if delete_result.failed.is_empty() {
        if skill_ids.is_empty() {
            db::delete_empty_skill_repository(pool, &repository.id)
                .await
                .map_err(CentralSkillsError::Other)? // TODO(C3): typed repos passthrough
        } else {
            db::prune_empty_skill_repositories(pool)
                .await
                .map_err(CentralSkillsError::Other)?; // TODO(C3): typed repos passthrough
            db::get_skill_repository_by_id(pool, &repository.id)
                .await
                .map_err(CentralSkillsError::Other)? // TODO(C3): typed repos passthrough
                .is_none()
        }
    } else {
        false
    };

    Ok(DeleteSkillRepositoryResult {
        repository,
        deleted_repository,
        delete_result,
    })
}

pub async fn delete_skill_repository_ssh_impl(
    pool: &DbPool,
    target: &RemoteTargetConfig,
    repository_id: &str,
    requests: &[BatchDeleteCentralSkillRequest],
) -> Result<DeleteSkillRepositoryResult, CentralSkillsError> {
    let active_target = ActiveTarget::Ssh(Box::new(target.clone()));
    delete_skill_repository_remote_impl(pool, &active_target, repository_id, requests).await
}
