//! Batch install orchestration: cartesian product of skill_ids x agent_ids,
//! dispatched to the platform install path or to project install when a
//! `project_path` is given.

use std::collections::HashSet;
use std::path::Path;

use crate::db::DbPool;

use super::native::install_central_skill_to_agent_by_method;
use super::project::install_central_skill_to_project_impl;
use super::types::{
    CentralBatchInstallFailure, CentralBatchInstallResult, CentralBatchInstallSuccess,
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
pub(crate) fn batch_operation_status(success_count: usize, failure_count: usize) -> &'static str {
    match (success_count, failure_count) {
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
    let mut failed = Vec::new();

    for skill_id in &skill_ids {
        for agent_id in &agent_ids {
            let install_result = if let Some(project_path) = project_path {
                install_central_skill_to_project_impl(
                    pool,
                    skill_id,
                    agent_id,
                    project_path,
                    method,
                )
                .await
            } else {
                install_central_skill_to_agent_by_method(pool, skill_id, agent_id, method).await
            };

            match install_result {
                Ok(result) => succeeded.push(CentralBatchInstallSuccess {
                    skill_id: skill_id.clone(),
                    agent_id: agent_id.clone(),
                    target_path: result.symlink_path,
                }),
                Err(error) => failed.push(CentralBatchInstallFailure {
                    skill_id: skill_id.clone(),
                    agent_id: agent_id.clone(),
                    error,
                }),
            }
        }
    }

    CentralBatchInstallResult { succeeded, failed }
}
