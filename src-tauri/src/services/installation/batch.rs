//! Batch install orchestration: cartesian product of skill_ids x agent_ids,
//! dispatched to the platform install path or to project install when a
//! `project_path` is given.

use std::collections::HashSet;
use std::path::Path;

use crate::db::DbPool;

use super::native::install_central_skill_to_agent_outcome_by_method;
use super::project::install_central_skill_to_project_outcome_impl;
use super::types::{
    BatchUninstallSkillFailure, BatchUninstallSkillRequest, BatchUninstallSkillResult,
    BatchUninstallSkillSuccess, CentralBatchInstallFailure, CentralBatchInstallResult,
    CentralBatchInstallSkipped, CentralBatchInstallSuccess, InstallOutcome,
};

/// Drop empty strings and collapse duplicates while preserving first-occurrence
/// order. Used to normalize batch IPC inputs.
pub(crate) fn dedupe_ordered(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();
    for value in values {
        if value.is_empty() {
            continue;
        }
        if seen.insert(value.clone()) {
            deduped.push(value);
        }
    }
    deduped
}

/// Summarize a batch outcome as one of "succeeded" / "failed" / "partial".
/// Used by the IPC layer for operation-log records.
pub(crate) fn batch_operation_status(
    success_count: usize,
    skipped_count: usize,
    failure_count: usize,
) -> &'static str {
    match (success_count + skipped_count, failure_count) {
        (_, 0) => "succeeded",
        (0, _) => "failed",
        _ => "partial",
    }
}

pub async fn batch_install_central_skills_impl(
    pool: &DbPool,
    skill_ids: Vec<String>,
    agent_ids: Vec<String>,
    method: &str,
    project_path: Option<&Path>,
) -> CentralBatchInstallResult {
    let skill_ids = dedupe_ordered(skill_ids);
    let agent_ids = dedupe_ordered(agent_ids);
    let mut succeeded = Vec::new();
    let mut skipped = Vec::new();
    let mut failed = Vec::new();

    for skill_id in &skill_ids {
        for agent_id in &agent_ids {
            let install_result = if let Some(project_path) = project_path {
                install_central_skill_to_project_outcome_impl(
                    pool,
                    skill_id,
                    agent_id,
                    project_path,
                    method,
                )
                .await
            } else {
                install_central_skill_to_agent_outcome_by_method(pool, skill_id, agent_id, method)
                    .await
            };

            match install_result {
                Ok(InstallOutcome::Installed(result)) => {
                    succeeded.push(CentralBatchInstallSuccess {
                        skill_id: skill_id.clone(),
                        agent_id: agent_id.clone(),
                        target_path: result.symlink_path,
                    })
                }
                Ok(InstallOutcome::Skipped(item)) => skipped.push(CentralBatchInstallSkipped {
                    skill_id: skill_id.clone(),
                    agent_id: item.agent_id,
                    target_path: item.target_path,
                    reason: item.reason,
                }),
                Err(error) => failed.push(CentralBatchInstallFailure {
                    skill_id: skill_id.clone(),
                    agent_id: agent_id.clone(),
                    error,
                }),
            }
        }
    }

    CentralBatchInstallResult {
        succeeded,
        skipped,
        failed,
    }
}

pub async fn batch_uninstall_skills_from_agent_impl(
    pool: &DbPool,
    agent_id: &str,
    requests: Vec<BatchUninstallSkillRequest>,
) -> BatchUninstallSkillResult {
    let mut succeeded = Vec::new();
    let mut failed = Vec::new();

    for request in requests {
        if request.skill_id.is_empty() {
            failed.push(BatchUninstallSkillFailure {
                skill_id: request.skill_id,
                row_id: request.row_id,
                error: "skill_id is required".to_string(),
            });
            continue;
        }

        match super::native::uninstall_skill_from_agent_with_row_impl(
            pool,
            &request.skill_id,
            agent_id,
            request.row_id.as_deref(),
        )
        .await
        {
            Ok(()) => succeeded.push(BatchUninstallSkillSuccess {
                skill_id: request.skill_id,
                row_id: request.row_id,
            }),
            Err(error) => failed.push(BatchUninstallSkillFailure {
                skill_id: request.skill_id,
                row_id: request.row_id,
                error,
            }),
        }
    }

    BatchUninstallSkillResult { succeeded, failed }
}
