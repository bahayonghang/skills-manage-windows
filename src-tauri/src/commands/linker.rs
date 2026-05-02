//! Tauri IPC shells for skill install / uninstall operations.
//!
//! Business logic lives in `crate::services::installation::*` (filesystem
//! primitives, auto-centralize, native install/uninstall, SSH install /
//! uninstall, project-scoped install, batch dispatch). This module is just
//! a thin IPC layer that:
//!
//! 1. Translates `State<AppState>` + arguments into service calls.
//! 2. Wraps every call with an `OperationLogEvent` recorder.
//!
//! Down-stream callers (commands/collections.rs, commands/discover.rs,
//! commands/central_updates.rs, central_migration.rs) still see the same
//! types and helpers under `commands::linker::*` because of the `pub use`
//! bridge near the top of this file.

use std::path::PathBuf;
use std::time::Instant;

use serde_json::json;
use tauri::State;

use crate::commands::logs::{
    record_operation_log_best_effort, target_context_from_active_target, OperationLogEvent,
};
use crate::services::installation;
use crate::targets::{connect_ssh_target, ActiveTarget};
use crate::AppState;

// Re-export the public surface so existing call-sites under `super::linker::*`
// or `crate::commands::linker::*` (collections / discover / central_migration
// / central_updates) keep compiling without changes.
pub use crate::services::installation::{
    batch_install_central_skills_impl, copy_dir_all, create_symlink,
    install_skill_to_agent_auto_impl, install_skill_to_agent_copy_impl,
    install_skill_to_agent_impl, install_skill_to_agent_ssh_impl, make_relative_path,
    symlink_target_path, uninstall_skill_from_agent_impl, uninstall_skill_from_agent_ssh_impl,
    BatchInstallResult, CentralBatchInstallFailure, CentralBatchInstallResult,
    CentralBatchInstallSuccess, FailedInstall, InstallResult,
};

/// Tauri command: install a skill to a single agent via relative symlink.
#[tauri::command]
pub async fn install_skill_to_agent(
    state: State<'_, AppState>,
    skill_id: String,
    agent_id: String,
    method: Option<String>,
) -> Result<InstallResult, String> {
    let active_target = state.active_target().await?;
    let target_context = target_context_from_active_target(&active_target);
    let pool = state.active_db().await?;
    let method = method.as_deref().unwrap_or("auto");
    let started_at = Instant::now();
    let result = match active_target {
        ActiveTarget::Local => match method {
            "copy" => {
                installation::install_skill_to_agent_copy_impl(&pool, &skill_id, &agent_id).await
            }
            "symlink" => {
                installation::install_skill_to_agent_impl(&pool, &skill_id, &agent_id).await
            }
            _ => installation::install_skill_to_agent_auto_impl(&pool, &skill_id, &agent_id).await,
        },
        ActiveTarget::Ssh(target) => {
            let remote_method = if method == "symlink" {
                "symlink"
            } else {
                "copy"
            };
            installation::install_skill_to_agent_ssh_impl(
                &pool,
                &target,
                &skill_id,
                &agent_id,
                remote_method,
            )
            .await
        }
    };
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
) -> Result<(), String> {
    let active_target = state.active_target().await?;
    let target_context = target_context_from_active_target(&active_target);
    let pool = state.active_db().await?;
    let started_at = Instant::now();
    let result = match active_target {
        ActiveTarget::Local => {
            installation::uninstall_skill_from_agent_impl(&pool, &skill_id, &agent_id).await
        }
        ActiveTarget::Ssh(target) => {
            installation::uninstall_skill_from_agent_ssh_impl(&pool, &target, &skill_id, &agent_id)
                .await
        }
    };
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
    }))
    .duration_ms(started_at.elapsed().as_millis() as i64);
    if let Err(error) = &result {
        event = event.error(error);
    }
    record_operation_log_best_effort(&state.db, target_context, event).await;
    result
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
    let active_target = state.active_target().await?;
    let target_context = target_context_from_active_target(&active_target);
    let pool = state.active_db().await?;
    let started_at = Instant::now();
    let mut succeeded = Vec::new();
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
                "failed": &failed,
            }))
            .duration_ms(started_at.elapsed().as_millis() as i64),
        )
        .await;
        return Ok(BatchInstallResult { succeeded, failed });
    }
    let ssh_connection = match &active_target {
        ActiveTarget::Ssh(target) => match connect_ssh_target(target).await {
            Ok(connection) => Some(connection),
            Err(error) => {
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
                        "failed": &failed,
                    }))
                    .duration_ms(started_at.elapsed().as_millis() as i64),
                )
                .await;
                return Ok(BatchInstallResult { succeeded, failed });
            }
        },
        ActiveTarget::Local => None,
    };

    for agent_id in &agent_ids {
        let install_result = match &active_target {
            ActiveTarget::Local => match method {
                "copy" => {
                    installation::install_skill_to_agent_copy_impl(&pool, &skill_id, agent_id).await
                }
                "symlink" => {
                    installation::install_skill_to_agent_impl(&pool, &skill_id, agent_id).await
                }
                _ => {
                    installation::install_skill_to_agent_auto_impl(&pool, &skill_id, agent_id).await
                }
            },
            ActiveTarget::Ssh(target) => {
                let remote_method = if method == "symlink" {
                    "symlink"
                } else {
                    "copy"
                };
                let connection = ssh_connection
                    .as_ref()
                    .ok_or_else(|| "SSH connection was not initialized".to_string())?;
                installation::install_skill_to_agent_ssh_with_connection(
                    &pool,
                    connection,
                    target,
                    &skill_id,
                    agent_id,
                    remote_method,
                )
                .await
            }
        };
        match install_result {
            Ok(_) => succeeded.push(agent_id.clone()),
            Err(e) => failed.push(FailedInstall {
                agent_id: agent_id.clone(),
                error: e,
            }),
        }
    }

    let status = installation::batch_operation_status(succeeded.len(), failed.len());
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
            "failed": &failed,
        }))
        .duration_ms(started_at.elapsed().as_millis() as i64),
    )
    .await;

    Ok(BatchInstallResult { succeeded, failed })
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
    let active_target = state.active_target().await?;
    let target_context = target_context_from_active_target(&active_target);
    let started_at = Instant::now();
    if matches!(&active_target, ActiveTarget::Ssh(_)) && project_path.is_some() {
        let error = "Remote project install is not supported in this version.".to_string();
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
            }))
            .duration_ms(started_at.elapsed().as_millis() as i64),
        )
        .await;
        return Err(error);
    }
    let pool = state.active_db().await?;
    if let ActiveTarget::Ssh(target) = active_target {
        let remote_method = if method == "symlink" {
            "symlink"
        } else {
            "copy"
        };
        let mut succeeded = Vec::new();
        let mut failed = Vec::new();
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
                    "succeeded": &succeeded,
                    "failed": &failed,
                }))
                .duration_ms(started_at.elapsed().as_millis() as i64),
            )
            .await;
            return Ok(CentralBatchInstallResult { succeeded, failed });
        }
        let connection = match connect_ssh_target(&target).await {
            Ok(connection) => connection,
            Err(error) => {
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
                        "succeeded": &succeeded,
                        "failed": &failed,
                    }))
                    .duration_ms(started_at.elapsed().as_millis() as i64),
                )
                .await;
                return Ok(CentralBatchInstallResult { succeeded, failed });
            }
        };
        for skill_id in skill_ids {
            for agent_id in &agent_ids {
                match installation::install_skill_to_agent_ssh_with_connection(
                    &pool,
                    &connection,
                    &target,
                    &skill_id,
                    agent_id,
                    remote_method,
                )
                .await
                {
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
        let status = installation::batch_operation_status(succeeded.len(), failed.len());
        record_operation_log_best_effort(
            &state.db,
            target_context,
            OperationLogEvent::new(
                "install",
                "central.batch_install",
                status,
                format!(
                    "Installed {} Central skill target(s), {} failed",
                    succeeded.len(),
                    failed.len()
                ),
            )
            .subject("batch", "central.batch_install", "Central batch install")
            .details(json!({
                "agentIds": &agent_ids,
                "method": method,
                "succeeded": &succeeded,
                "failed": &failed,
            }))
            .duration_ms(started_at.elapsed().as_millis() as i64),
        )
        .await;
        return Ok(CentralBatchInstallResult { succeeded, failed });
    }

    let project_path_buf = project_path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(PathBuf::from);

    let batch_result = installation::batch_install_central_skills_impl(
        &pool,
        skill_ids,
        agent_ids,
        method,
        project_path_buf.as_deref(),
    )
    .await;
    let status = installation::batch_operation_status(
        batch_result.succeeded.len(),
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
                batch_result.succeeded.len(),
                batch_result.failed.len()
            ),
        )
        .subject("batch", "central.batch_install", "Central batch install")
        .details(json!({
            "method": method,
            "projectPath": project_path_buf.as_ref().map(|path| path.display().to_string()),
            "succeeded": &batch_result.succeeded,
            "failed": &batch_result.failed,
        }))
        .duration_ms(started_at.elapsed().as_millis() as i64),
    )
    .await;
    Ok(batch_result)
}
