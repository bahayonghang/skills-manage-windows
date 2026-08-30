//! Shared Local/SSH/WSL Central-skill delete batch orchestration.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::db::{self, DbPool};
use crate::services::central_updates::CentralFs;
use crate::targets::{connect_remote_target, ActiveTarget, RemoteTargetConfig};

use super::super::common::unique_agent_ids;
use super::super::error::CentralSkillsError;
use super::super::types::{
    BatchDeleteCentralSkillRequest, BatchDeleteCentralSkillResult, BatchDeleteCentralSkillSuccess,
    DeleteCentralSkillResult, FailedCentralSkillDelete,
};
use super::{delete_central_skill_local_under_guard, delete_central_skill_remote_under_guard};

pub(super) struct CentralSkillDeleteItemError {
    pub(super) phase: &'static str,
    pub(super) error: CentralSkillsError,
}

impl CentralSkillDeleteItemError {
    fn new(phase: &'static str, error: impl Into<CentralSkillsError>) -> Self {
        Self {
            phase,
            error: error.into(),
        }
    }
}

pub(super) struct CentralSkillDeleteItemOutcome {
    pub(super) skill_id: String,
    pub(super) result: Result<DeleteCentralSkillResult, CentralSkillDeleteItemError>,
}

fn deduplicate_delete_requests(
    requests: &[BatchDeleteCentralSkillRequest],
) -> Vec<BatchDeleteCentralSkillRequest> {
    let mut ordered_requests: Vec<BatchDeleteCentralSkillRequest> = Vec::new();
    for request in requests {
        if let Some(existing) = ordered_requests
            .iter_mut()
            .find(|existing| existing.skill_id == request.skill_id)
        {
            for agent_id in &request.remove_agent_ids {
                if !existing.remove_agent_ids.contains(agent_id) {
                    existing.remove_agent_ids.push(agent_id.clone());
                }
            }
            existing.force |= request.force;
        } else {
            ordered_requests.push(BatchDeleteCentralSkillRequest {
                skill_id: request.skill_id.clone(),
                remove_agent_ids: unique_agent_ids(request.remove_agent_ids.clone()),
                force: request.force,
            });
        }
    }
    ordered_requests
}

async fn recover_selected_pending_operations_under_guard(
    pool: &DbPool,
    active_target: &ActiveTarget,
    remote: Option<&Arc<crate::targets::ConnectedRemoteTarget>>,
    skill_ids: &[String],
) -> Result<HashMap<String, CentralSkillDeleteItemError>, CentralSkillsError> {
    let selected = skill_ids.iter().map(String::as_str).collect::<HashSet<_>>();
    let target_kind = match active_target.kind() {
        crate::targets::TargetKind::Local => "local",
        crate::targets::TargetKind::Ssh => "ssh",
        crate::targets::TargetKind::Wsl => "wsl",
    };
    let mut failures = HashMap::new();
    for row in db::list_pending_fs_db_operations(pool, active_target.id()).await? {
        if !selected.contains(row.skill_id.as_str()) {
            continue;
        }
        let recovery = match row.operation_kind.as_str() {
            "central_delete" => {
                crate::services::central_operation::recover_pending_delete_operation_with_transport(
                    pool,
                    active_target.id(),
                    target_kind,
                    remote.map(Arc::as_ref),
                    &row,
                )
                .await
                .map_err(CentralSkillsError::from)
            }
            "central_update" => {
                let fs = match remote {
                    Some(connection) => CentralFs::Remote(Arc::clone(connection)),
                    None => CentralFs::Local,
                };
                crate::services::central_updates::recover_pending_update_operation(pool, &fs, &row)
                    .await
                    .map_err(|error| CentralSkillsError::UpdateRecovery {
                        error_code: error.stable_error_code(),
                        error_category: error.diagnostic_category(),
                    })
            }
            _ => continue,
        };
        if let Err(error) = recovery {
            failures
                .entry(row.skill_id)
                .or_insert_with(|| CentralSkillDeleteItemError::new("recovery", error));
        }
    }
    Ok(failures)
}

pub(super) async fn delete_central_skills_for_target(
    pool: &DbPool,
    active_target: &ActiveTarget,
    requests: &[BatchDeleteCentralSkillRequest],
    batch_id: Option<&str>,
) -> Result<Vec<CentralSkillDeleteItemOutcome>, CentralSkillsError> {
    let ordered_requests = deduplicate_delete_requests(requests);
    if ordered_requests.is_empty() {
        return Ok(Vec::new());
    }

    let _mutation_guard = crate::services::central_mutation::acquire_target_mutation_guard(
        active_target,
        "delete Central skill",
        crate::services::central_mutation::DEFAULT_CENTRAL_MUTATION_TIMEOUT,
    )
    .await?;
    let remote = if active_target.is_remote_like() {
        Some(Arc::new(
            connect_remote_target(active_target).await.map_err(|_| {
                CentralSkillsError::Remote("Failed to connect to the selected target.".to_string())
            })?,
        ))
    } else {
        None
    };
    delete_central_skills_under_guard(
        pool,
        active_target,
        ordered_requests,
        batch_id,
        remote.as_ref(),
    )
    .await
}

pub(super) async fn delete_central_skills_under_guard(
    pool: &DbPool,
    active_target: &ActiveTarget,
    ordered_requests: Vec<BatchDeleteCentralSkillRequest>,
    batch_id: Option<&str>,
    remote: Option<&Arc<crate::targets::ConnectedRemoteTarget>>,
) -> Result<Vec<CentralSkillDeleteItemOutcome>, CentralSkillsError> {
    let selected_skill_ids = ordered_requests
        .iter()
        .map(|request| request.skill_id.clone())
        .collect::<Vec<_>>();
    let mut force_failures = HashMap::new();
    let mut skip_recovery = HashSet::new();
    for request in &ordered_requests {
        if !request.force {
            continue;
        }
        match crate::services::central_operation::force_abandon_prepared_delete_under_guard(
            pool,
            active_target,
            &request.skill_id,
            remote.map(Arc::as_ref),
        )
        .await
        {
            Ok(crate::services::central_operation::ForceAbandonDecision::Blocked) => {
                skip_recovery.insert(request.skill_id.clone());
                force_failures.insert(
                    request.skill_id.clone(),
                    CentralSkillDeleteItemError::new(
                        "recovery",
                        CentralSkillsError::ForceDeleteBlocked,
                    ),
                );
            }
            Ok(_) => {}
            Err(error) => {
                skip_recovery.insert(request.skill_id.clone());
                force_failures.insert(
                    request.skill_id.clone(),
                    CentralSkillDeleteItemError::new("recovery", CentralSkillsError::from(error)),
                );
            }
        }
    }
    let recover_skill_ids = selected_skill_ids
        .iter()
        .filter(|skill_id| !skip_recovery.contains(*skill_id))
        .cloned()
        .collect::<Vec<_>>();
    let mut recovery_failures = recover_selected_pending_operations_under_guard(
        pool,
        active_target,
        remote,
        &recover_skill_ids,
    )
    .await?;
    recovery_failures.extend(force_failures);

    let mut outcomes = Vec::with_capacity(ordered_requests.len());
    for request in ordered_requests {
        let skill_id = request.skill_id;
        let result = if let Some(error) = recovery_failures.remove(&skill_id) {
            Err(error)
        } else if let Some(connection) = remote {
            delete_central_skill_remote_under_guard(
                pool,
                active_target,
                connection.as_ref(),
                &skill_id,
                &request.remove_agent_ids,
                batch_id,
            )
            .await
            .map_err(|error| CentralSkillDeleteItemError::new("prepare", error))
        } else {
            delete_central_skill_local_under_guard(
                pool,
                &skill_id,
                &request.remove_agent_ids,
                batch_id,
            )
            .await
            .map_err(|error| CentralSkillDeleteItemError::new("prepare", error))
        };
        outcomes.push(CentralSkillDeleteItemOutcome { skill_id, result });
    }
    Ok(outcomes)
}

#[cfg(test)]
pub(crate) async fn delete_central_skills_for_target_with_connection_for_tests(
    pool: &DbPool,
    active_target: &ActiveTarget,
    connection: Arc<crate::targets::ConnectedRemoteTarget>,
    requests: &[BatchDeleteCentralSkillRequest],
    batch_id: Option<&str>,
) -> Result<BatchDeleteCentralSkillResult, CentralSkillsError> {
    if connection.target_id() != active_target.id()
        || connection.active_target().kind() != active_target.kind()
    {
        return Err(CentralSkillsError::Remote(
            "Selected target connection identity mismatch.".to_string(),
        ));
    }
    let ordered_requests = deduplicate_delete_requests(requests);
    if ordered_requests.is_empty() {
        return Ok(BatchDeleteCentralSkillResult {
            succeeded: Vec::new(),
            failed: Vec::new(),
        });
    }
    let _mutation_guard = crate::services::central_mutation::acquire_target_mutation_guard(
        active_target,
        "delete Central skill",
        crate::services::central_mutation::DEFAULT_CENTRAL_MUTATION_TIMEOUT,
    )
    .await?;
    let outcomes = delete_central_skills_under_guard(
        pool,
        active_target,
        ordered_requests,
        batch_id,
        Some(&connection),
    )
    .await?;
    Ok(public_delete_batch_result(outcomes))
}

pub(super) fn public_delete_batch_result(
    outcomes: Vec<CentralSkillDeleteItemOutcome>,
) -> BatchDeleteCentralSkillResult {
    let mut succeeded = Vec::new();
    let mut failed = Vec::new();
    for outcome in outcomes {
        match outcome.result {
            Ok(result) => succeeded.push(BatchDeleteCentralSkillSuccess {
                skill_id: outcome.skill_id,
                removed_central_path: result.removed_central_path,
                removed_agent_ids: result.removed_agent_ids,
                retained_agent_ids: result.retained_agent_ids,
            }),
            Err(error) => failed.push(FailedCentralSkillDelete::from_error(
                outcome.skill_id,
                error.phase,
                &error.error,
            )),
        }
    }
    BatchDeleteCentralSkillResult { succeeded, failed }
}

pub async fn delete_central_skills_remote_impl(
    pool: &DbPool,
    active_target: &ActiveTarget,
    requests: &[BatchDeleteCentralSkillRequest],
) -> Result<BatchDeleteCentralSkillResult, CentralSkillsError> {
    let batch_id = uuid::Uuid::new_v4().to_string();
    let outcomes =
        delete_central_skills_for_target(pool, active_target, requests, Some(&batch_id)).await?;
    Ok(public_delete_batch_result(outcomes))
}

pub async fn delete_central_skills_ssh_impl(
    pool: &DbPool,
    target: &RemoteTargetConfig,
    requests: &[BatchDeleteCentralSkillRequest],
) -> Result<BatchDeleteCentralSkillResult, CentralSkillsError> {
    let active_target = ActiveTarget::Ssh(Box::new(target.clone()));
    delete_central_skills_remote_impl(pool, &active_target, requests).await
}

pub async fn delete_central_skills_impl(
    pool: &DbPool,
    requests: &[BatchDeleteCentralSkillRequest],
) -> Result<BatchDeleteCentralSkillResult, CentralSkillsError> {
    let batch_id = uuid::Uuid::new_v4().to_string();
    let outcomes =
        delete_central_skills_for_target(pool, &ActiveTarget::Local, requests, Some(&batch_id))
            .await?;
    Ok(public_delete_batch_result(outcomes))
}
