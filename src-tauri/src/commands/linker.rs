//! Tauri IPC shells for skill install / uninstall operations.
//!
//! Business logic lives in `crate::services::installation::*` (the
//! `install_skill` / `uninstall_skill` orchestration over the
//! `InstallTransport` seam, batch dispatch, project-scoped install). This
//! module is just a thin IPC layer that:
//!
//! 1. Translates `State<AppState>` + arguments into service calls.
//! 2. Wraps every call with an `OperationLogEvent` recorder.
//!
//! Down-stream callers (commands/collections.rs, commands/central_updates.rs)
//! still see the same
//! types and helpers under `commands::linker::*` because of the `pub use`
//! bridge near the top of this file.

use std::time::Instant;

use serde_json::json;
use tauri::State;

use crate::operation_log::{
    record_operation_log_best_effort, target_context_from_active_target, OperationLogEvent,
};
use crate::services::installation::{self, InstallOutcome, InstallTransport};
use crate::AppState;

// Re-export the public surface so existing call-sites under `super::linker::*`
// or `crate::commands::linker::*` (collections / central_updates)
// keep compiling without changes.
pub use crate::services::installation::{
    batch_install_central_skills_impl, batch_uninstall_skills_from_agent_impl, copy_dir_all,
    create_symlink, install_skill, make_relative_path, symlink_target_path, uninstall_skill,
    BatchInstallResult, BatchUninstallSkillFailure, BatchUninstallSkillRequest,
    BatchUninstallSkillResult, BatchUninstallSkillSuccess, CentralBatchInstallFailure,
    CentralBatchInstallResult, CentralBatchInstallSkipped, CentralBatchInstallSuccess,
    FailedInstall, InstallResult, SkippedInstall,
};

/// Tauri command: install a skill to a single agent via relative symlink.
#[tauri::command]
pub async fn install_skill_to_agent(
    state: State<'_, AppState>,
    skill_id: String,
    agent_id: String,
    method: Option<String>,
) -> Result<InstallResult, String> {
    let request_context = state.resolve_target_context().await?;
    let active_target = request_context.target().clone();
    let target_context = target_context_from_active_target(&active_target);
    let pool = request_context.db().clone();
    let method = method.as_deref().unwrap_or("auto");
    let started_at = Instant::now();
    let result = match InstallTransport::for_target(&active_target).await {
        Ok(transport) => {
            installation::install_skill(&pool, &transport, &skill_id, &agent_id, method)
                .await
                .map(InstallOutcome::into_install_result)
        }
        Err(error) => Err(error),
    };
    let result = result.map_err(|e| e.to_string());
    let status = if result.is_ok() {
        "succeeded"
    } else {
        "failed"
    };
    let mut event = OperationLogEvent::new(
        "install",
        "skill.install",
        status,
        if result.is_ok() {
            format!("Installed skill {} to {}", skill_id, agent_id)
        } else {
            format!("Failed to install skill {} to {}", skill_id, agent_id)
        },
    )
    .subject("skill", &skill_id, &skill_id)
    .details(json!({
        "skillId": skill_id,
        "agentId": agent_id,
        "method": method,
        "targetPath": result.as_ref().ok().map(|install| install.symlink_path.clone()),
    }))
    .duration_ms(started_at.elapsed().as_millis() as i64);
    if let Err(error) = &result {
        event = event.error(error);
    }
    record_operation_log_best_effort(&state.db, target_context, event).await;
    result
}

/// Tauri command: remove a skill's symlink from an agent.
#[tauri::command]
pub async fn uninstall_skill_from_agent(
    state: State<'_, AppState>,
    skill_id: String,
    agent_id: String,
    row_id: Option<String>,
) -> Result<(), String> {
    let request_context = state.resolve_target_context().await?;
    let active_target = request_context.target().clone();
    let target_context = target_context_from_active_target(&active_target);
    let pool = request_context.db().clone();
    let started_at = Instant::now();
    let result = match InstallTransport::for_target(&active_target).await {
        Ok(transport) => {
            installation::uninstall_skill(
                &pool,
                &transport,
                &skill_id,
                &agent_id,
                row_id.as_deref(),
            )
            .await
        }
        Err(error) => Err(error),
    };
    let result = result.map_err(|e| e.to_string());
    let status = if result.is_ok() {
        "succeeded"
    } else {
        "failed"
    };
    let mut event = OperationLogEvent::new(
        "install",
        "skill.uninstall",
        status,
        if result.is_ok() {
            format!("Uninstalled skill {} from {}", skill_id, agent_id)
        } else {
            format!("Failed to uninstall skill {} from {}", skill_id, agent_id)
        },
    )
    .subject("skill", &skill_id, &skill_id)
    .details(json!({
        "skillId": skill_id,
        "agentId": agent_id,
        "rowId": row_id,
    }))
    .duration_ms(started_at.elapsed().as_millis() as i64);
    if let Err(error) = &result {
        event = event.error(error);
    }
    record_operation_log_best_effort(&state.db, target_context, event).await;
    result
}

/// Tauri command: remove multiple skills from one agent.
#[tauri::command]
pub async fn batch_uninstall_skills_from_agent(
    state: State<'_, AppState>,
    agent_id: String,
    requests: Vec<BatchUninstallSkillRequest>,
) -> Result<BatchUninstallSkillResult, String> {
    let request_context = state.resolve_target_context().await?;
    let active_target = request_context.target().clone();
    let target_context = target_context_from_active_target(&active_target);
    let pool = request_context.db().clone();
    let started_at = Instant::now();
    let result = match InstallTransport::for_target(&active_target).await {
        Ok(transport) => {
            installation::batch_uninstall_skills_from_agent_impl(
                &pool, &transport, &agent_id, requests,
            )
            .await
        }
        Err(error) => {
            let error = error.to_string();
            BatchUninstallSkillResult {
                succeeded: Vec::new(),
                failed: requests_to_failures(requests, &error),
            }
        }
    };
    let status =
        installation::batch_operation_status(result.succeeded.len(), 0, result.failed.len());
    record_operation_log_best_effort(
        &state.db,
        target_context,
        OperationLogEvent::new(
            "install",
            "skill.batch_uninstall",
            status,
            format!(
                "Uninstalled {} skill(s) from {}, {} failed",
                result.succeeded.len(),
                agent_id,
                result.failed.len()
            ),
        )
        .subject("agent", &agent_id, &agent_id)
        .details(json!({
            "agentId": agent_id,
            "succeeded": &result.succeeded,
            "failed": &result.failed,
        }))
        .duration_ms(started_at.elapsed().as_millis() as i64),
    )
    .await;
    Ok(result)
}

fn requests_to_failures(
    requests: Vec<BatchUninstallSkillRequest>,
    error: &str,
) -> Vec<BatchUninstallSkillFailure> {
    requests
        .into_iter()
        .map(|request| BatchUninstallSkillFailure {
            skill_id: request.skill_id,
            row_id: request.row_id,
            error: error.to_string(),
        })
        .collect()
}

/// Tauri command: install a skill to multiple agents in one call.
///
/// `method` must be either `"symlink"` (default, creates a relative symlink) or
/// `"copy"` (copies the skill directory). Each agent install is attempted
/// independently; failures are collected in the `failed` list rather than
/// short-circuiting the entire batch.
#[tauri::command]
pub async fn batch_install_to_agents(
    state: State<'_, AppState>,
    skill_id: String,
    agent_ids: Vec<String>,
    method: Option<String>,
) -> Result<BatchInstallResult, String> {
    let method = method.as_deref().unwrap_or("auto");
    let request_context = state.resolve_target_context().await?;
    let active_target = request_context.target().clone();
    let target_context = target_context_from_active_target(&active_target);
    let pool = request_context.db().clone();
    let started_at = Instant::now();
    let mut succeeded = Vec::new();
    let mut skipped = Vec::new();
    let mut failed = Vec::new();
    if agent_ids.is_empty() {
        record_operation_log_best_effort(
            &state.db,
            target_context,
            OperationLogEvent::new(
                "install",
                "skill.batch_install",
                "succeeded",
                format!("No target agents selected for skill {}", skill_id),
            )
            .subject("skill", &skill_id, &skill_id)
            .details(json!({
                "skillId": skill_id,
                "agentIds": agent_ids,
                "method": method,
                "succeeded": &succeeded,
                "skipped": &skipped,
                "failed": &failed,
            }))
            .duration_ms(started_at.elapsed().as_millis() as i64),
        )
        .await;
        return Ok(BatchInstallResult {
            succeeded,
            skipped,
            failed,
        });
    }
    let transport = match InstallTransport::for_target(&active_target).await {
        Ok(transport) => transport,
        Err(error) => {
            let error = error.to_string();
            failed.extend(agent_ids.iter().map(|agent_id| FailedInstall {
                agent_id: agent_id.clone(),
                error: error.clone(),
            }));
            record_operation_log_best_effort(
                &state.db,
                target_context,
                OperationLogEvent::new(
                    "install",
                    "skill.batch_install",
                    "failed",
                    format!("Failed to install skill {} to selected agents", skill_id),
                )
                .subject("skill", &skill_id, &skill_id)
                .error(&error)
                .details(json!({
                    "skillId": skill_id,
                    "agentIds": agent_ids,
                    "method": method,
                    "succeeded": &succeeded,
                    "skipped": &skipped,
                    "failed": &failed,
                }))
                .duration_ms(started_at.elapsed().as_millis() as i64),
            )
            .await;
            return Ok(BatchInstallResult {
                succeeded,
                skipped,
                failed,
            });
        }
    };

    for agent_id in &agent_ids {
        match installation::install_skill(&pool, &transport, &skill_id, agent_id, method).await {
            Ok(InstallOutcome::Installed(_)) => succeeded.push(agent_id.clone()),
            Ok(InstallOutcome::Skipped(item)) => skipped.push(item),
            Err(e) => failed.push(FailedInstall {
                agent_id: agent_id.clone(),
                error: e.to_string(),
            }),
        }
    }

    let status = installation::batch_operation_status(succeeded.len(), skipped.len(), failed.len());
    record_operation_log_best_effort(
        &state.db,
        target_context,
        OperationLogEvent::new(
            "install",
            "skill.batch_install",
            status,
            format!(
                "Installed skill {} to {} agent(s), {} failed",
                skill_id,
                succeeded.len(),
                failed.len()
            ),
        )
        .subject("skill", &skill_id, &skill_id)
        .details(json!({
            "skillId": skill_id,
            "agentIds": agent_ids,
            "method": method,
            "succeeded": &succeeded,
            "skipped": &skipped,
            "failed": &failed,
        }))
        .duration_ms(started_at.elapsed().as_millis() as i64),
    )
    .await;

    Ok(BatchInstallResult {
        succeeded,
        skipped,
        failed,
    })
}

/// Tauri command: install multiple Central skills to multiple platform or project targets.
#[tauri::command]
pub async fn batch_install_central_skills(
    state: State<'_, AppState>,
    skill_ids: Vec<String>,
    agent_ids: Vec<String>,
    method: Option<String>,
    project_path: Option<String>,
) -> Result<CentralBatchInstallResult, String> {
    let method = method.as_deref().unwrap_or("auto");
    let request_context = state.resolve_target_context().await?;
    let active_target = request_context.target().clone();
    let target_context = target_context_from_active_target(&active_target);
    let started_at = Instant::now();
    let project_path = project_path
        .map(|path| path.trim().to_string())
        .filter(|path| !path.is_empty());
    let pool = request_context.db().clone();

    let skill_ids = installation::dedupe_ordered(skill_ids);
    let agent_ids = installation::dedupe_ordered(agent_ids);
    if skill_ids.is_empty() || agent_ids.is_empty() {
        record_operation_log_best_effort(
            &state.db,
            target_context,
            OperationLogEvent::new(
                "install",
                "central.batch_install",
                "succeeded",
                "No Central skills or target agents selected",
            )
            .subject("batch", "central.batch_install", "Central batch install")
            .details(json!({
                "skillIds": &skill_ids,
                "agentIds": &agent_ids,
                "method": method,
                "projectPath": &project_path,
                "succeeded": [],
                "skipped": [],
                "failed": [],
            }))
            .duration_ms(started_at.elapsed().as_millis() as i64),
        )
        .await;
        return Ok(CentralBatchInstallResult {
            succeeded: Vec::new(),
            skipped: Vec::new(),
            failed: Vec::new(),
        });
    }

    let transport = match InstallTransport::for_target(&active_target).await {
        Ok(transport) => transport,
        Err(error) => {
            let error = error.to_string();
            let mut failed = Vec::new();
            for skill_id in &skill_ids {
                for agent_id in &agent_ids {
                    failed.push(CentralBatchInstallFailure {
                        skill_id: skill_id.clone(),
                        agent_id: agent_id.clone(),
                        error: error.clone(),
                    });
                }
            }
            record_operation_log_best_effort(
                &state.db,
                target_context,
                OperationLogEvent::new(
                    "install",
                    "central.batch_install",
                    "failed",
                    "Failed to batch install Central skills",
                )
                .subject("batch", "central.batch_install", "Central batch install")
                .error(&error)
                .details(json!({
                    "skillIds": &skill_ids,
                    "agentIds": &agent_ids,
                    "method": method,
                    "projectPath": &project_path,
                    "succeeded": [],
                    "skipped": [],
                    "failed": &failed,
                }))
                .duration_ms(started_at.elapsed().as_millis() as i64),
            )
            .await;
            return Ok(CentralBatchInstallResult {
                succeeded: Vec::new(),
                skipped: Vec::new(),
                failed,
            });
        }
    };

    let batch_result = installation::batch_install_central_skills_impl(
        &pool,
        &transport,
        skill_ids.clone(),
        agent_ids.clone(),
        method,
        project_path.as_deref(),
    )
    .await;
    let status = installation::batch_operation_status(
        batch_result.succeeded.len(),
        batch_result.skipped.len(),
        batch_result.failed.len(),
    );
    record_operation_log_best_effort(
        &state.db,
        target_context,
        OperationLogEvent::new(
            "install",
            "central.batch_install",
            status,
            format!(
                "Installed {} Central skill target(s), {} failed",
                batch_result.succeeded.len() + batch_result.skipped.len(),
                batch_result.failed.len()
            ),
        )
        .subject("batch", "central.batch_install", "Central batch install")
        .details(json!({
            "skillIds": &skill_ids,
            "agentIds": &agent_ids,
            "method": method,
            "projectPath": &project_path,
            "succeeded": &batch_result.succeeded,
            "skipped": &batch_result.skipped,
            "failed": &batch_result.failed,
        }))
        .duration_ms(started_at.elapsed().as_millis() as i64),
    )
    .await;
    Ok(batch_result)
}
