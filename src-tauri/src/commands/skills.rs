//! Tauri IPC shells for Central Skills and skill detail operations.
//!
//! Business logic lives in `crate::services::central_skills`. This module keeps
//! the existing command names and public type paths stable while translating
//! `State<AppState>` into service inputs and recording operation logs for
//! destructive Central operations.

use std::time::Instant;

use serde_json::json;
use tauri::State;

use crate::operation_log::{
    record_operation_log_best_effort, target_context_from_active_target, OperationLogEvent,
};
use crate::services::central_skills;
use crate::targets::ActiveTarget;
use crate::AppState;

// Re-export the public service surface so existing Rust call-sites that import
// `commands::skills::*` keep compiling while implementation lives in services.
use crate::db::SkillForAgent;
pub use crate::services::central_skills::{
    delete_central_skill_impl, delete_central_skill_ssh_impl, delete_central_skills_impl,
    delete_central_skills_ssh_impl, delete_skill_repository_impl, delete_skill_repository_ssh_impl,
    get_central_skills_impl, get_central_skills_page_impl, get_skill_detail_with_row_impl,
    get_skills_by_agent_impl, list_directory_tree_for_target_impl,
    preview_delete_central_skills_impl, preview_delete_central_skills_ssh_impl,
    preview_delete_skill_repository_impl, preview_delete_skill_repository_ssh_impl,
    preview_reset_unknown_source_skills_impl, reset_unknown_source_skills_impl,
    BatchDeleteCentralSkillPreviewResult, BatchDeleteCentralSkillRequest,
    BatchDeleteCentralSkillResult, BatchDeleteCentralSkillSuccess, CentralSkillsPage,
    CentralSkillsPageRequest, DeleteCentralSkillPreview, DeleteCentralSkillResult,
    DeleteSkillRepositoryPreview, DeleteSkillRepositoryResult, DirectoryTreeEntry,
    FailedCentralSkillDelete, PendingDeleteRecoveryPreview, ResetUnknownSourceSkillsPreview,
    SkillDetail, SkillInstallationDetail, SkillPathAccessContext, SkillWithLinks,
};

/// Tauri command: return all skills installed for a given agent, including
/// installation metadata needed by the platform-view skill cards.
#[tauri::command]
pub async fn get_skills_by_agent(
    state: State<'_, AppState>,
    agent_id: String,
) -> crate::ipc_error::IpcResult<Vec<SkillForAgent>> {
    crate::ipc_boundary!(
        async move {
            let pool = state.active_db().await?;
            central_skills::get_skills_by_agent_impl(&pool, &agent_id)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

/// Tauri command: return all Central Skills with per-platform link status.
#[tauri::command]
pub async fn get_central_skills(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<Vec<SkillWithLinks>> {
    crate::ipc_boundary!(
        async move {
            let pool = state.active_db().await?;
            central_skills::get_central_skills_impl(&pool)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
pub async fn get_central_skills_page(
    state: State<'_, AppState>,
    request: CentralSkillsPageRequest,
) -> crate::ipc_error::IpcResult<CentralSkillsPage> {
    crate::ipc_boundary!(
        async move {
            let pool = state.active_db().await?;
            central_skills::get_central_skills_page_impl(&pool, request)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
pub async fn preview_delete_central_skills(
    state: State<'_, AppState>,
    skill_ids: Vec<String>,
) -> crate::ipc_error::IpcResult<BatchDeleteCentralSkillPreviewResult> {
    crate::ipc_boundary!(
        async move {
            let request_context = state.resolve_target_context().await?;
            let pool = request_context.db().clone();
            match request_context.target() {
                ActiveTarget::Local => {
                    central_skills::preview_delete_central_skills_impl(&pool, &skill_ids).await
                }
                ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => {
                    central_skills::preview_delete_central_skills_ssh_impl(
                        &pool,
                        request_context.target(),
                        &skill_ids,
                    )
                    .await
                }
            }
            .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn delete_central_skill(
    state: State<'_, AppState>,
    skill_id: String,
    remove_agent_ids: Vec<String>,
    force: Option<bool>,
) -> crate::ipc_error::IpcResult<DeleteCentralSkillResult> {
    crate::ipc_boundary!(
        async move {
            let force = force.unwrap_or(false);
            let request_context = state.resolve_target_context().await?;
            let active_target = request_context.target().clone();
            let target_context = target_context_from_active_target(&active_target);
            let pool = request_context.db().clone();
            let started_at = Instant::now();
            let result = match &active_target {
                ActiveTarget::Local => {
                    central_skills::delete_central_skill_impl(
                        &pool,
                        &skill_id,
                        &remove_agent_ids,
                        force,
                    )
                    .await
                }
                ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => {
                    central_skills::delete_central_skill_remote_impl(
                        &pool,
                        &active_target,
                        &skill_id,
                        &remove_agent_ids,
                        force,
                    )
                    .await
                }
            }
            .map_err(delete_command_error);
            let status = if result.is_ok() {
                "succeeded"
            } else {
                "failed"
            };
            let mut event = OperationLogEvent::new(
        "delete",
        "central.delete",
        status,
        if result.is_ok() {
            format!("Deleted Central skill {}", skill_id)
        } else {
            format!("Failed to delete Central skill {}", skill_id)
        },
    )
    .subject("skill", &skill_id, &skill_id)
    .details(json!({
        "skillId": skill_id,
        "removeAgentIds": &remove_agent_ids,
        "force": force,
        "removedAgentIds": result.as_ref().ok().map(|item| item.removed_agent_ids.clone()),
        "retainedAgentIds": result.as_ref().ok().map(|item| item.retained_agent_ids.clone()),
    }))
    .duration_ms(started_at.elapsed().as_millis() as i64);
            if let Err(error) = &result {
                event = event.error(error);
            }
            record_operation_log_best_effort(&state.db, target_context, event).await;
            result
        }
        .await
    )
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn delete_central_skills(
    state: State<'_, AppState>,
    requests: Vec<BatchDeleteCentralSkillRequest>,
) -> crate::ipc_error::IpcResult<BatchDeleteCentralSkillResult> {
    crate::ipc_boundary!(
        async move {
            let request_context = state.resolve_target_context().await?;
            let active_target = request_context.target().clone();
            let target_context = target_context_from_active_target(&active_target);
            let pool = request_context.db().clone();
            let started_at = Instant::now();
            let result = match &active_target {
                ActiveTarget::Local => {
                    central_skills::delete_central_skills_impl(&pool, &requests).await
                }
                ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => {
                    central_skills::delete_central_skills_remote_impl(
                        &pool,
                        &active_target,
                        &requests,
                    )
                    .await
                }
            }
            .map_err(delete_command_error);
            match &result {
                Ok(batch_result) => {
                    let status = match (batch_result.succeeded.len(), batch_result.failed.len()) {
                        (_, 0) => "succeeded",
                        (0, _) => "failed",
                        _ => "partial",
                    };
                    record_operation_log_best_effort(
                        &state.db,
                        target_context,
                        OperationLogEvent::new(
                            "delete",
                            "central.batch_delete",
                            status,
                            format!(
                                "Deleted {} Central skill(s), {} failed",
                                batch_result.succeeded.len(),
                                batch_result.failed.len()
                            ),
                        )
                        .subject("batch", "central.batch_delete", "Central batch delete")
                        .details(json!({
                            "requestCount": requests.len(),
                            "succeeded": &batch_result.succeeded,
                            "failed": &batch_result.failed,
                        }))
                        .duration_ms(started_at.elapsed().as_millis() as i64),
                    )
                    .await;
                }
                Err(error) => {
                    record_operation_log_best_effort(
                        &state.db,
                        target_context,
                        OperationLogEvent::new(
                            "delete",
                            "central.batch_delete",
                            "failed",
                            "Failed to delete Central skills",
                        )
                        .subject("batch", "central.batch_delete", "Central batch delete")
                        .error(error)
                        .details(json!({
                            "requestCount": requests.len(),
                        }))
                        .duration_ms(started_at.elapsed().as_millis() as i64),
                    )
                    .await;
                }
            }
            result
        }
        .await
    )
}

#[tauri::command]
pub async fn preview_reset_unknown_source_skills(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<ResetUnknownSourceSkillsPreview> {
    crate::ipc_boundary!(
        async move {
            let request_context = state.resolve_target_context().await?;
            let pool = request_context.db().clone();
            let active_target = request_context.target().clone();
            central_skills::preview_reset_unknown_source_skills_impl(&pool, &active_target)
                .await
                .map_err(reset_command_error)
        }
        .await
    )
}

#[tauri::command]
pub async fn reset_unknown_source_skills(
    state: State<'_, AppState>,
    skill_ids: Vec<String>,
    remove_copy_agent_ids: Vec<String>,
) -> crate::ipc_error::IpcResult<BatchDeleteCentralSkillResult> {
    crate::ipc_boundary!(
        async move {
            let request_context = state.resolve_target_context().await?;
            let active_target = request_context.target().clone();
            let target_context = target_context_from_active_target(&active_target);
            let pool = request_context.db().clone();
            let started_at = Instant::now();
            let target_kind = match &active_target {
                ActiveTarget::Local => "local",
                ActiveTarget::Ssh(_) => "ssh",
                ActiveTarget::Wsl(_) => "wsl",
            };
            let result = central_skills::reset_unknown_source_skills_impl(
                &pool,
                &active_target,
                &skill_ids,
                &remove_copy_agent_ids,
            )
            .await
            .map_err(reset_command_error);
            match &result {
                Ok(batch_result) => {
                    let attempted = batch_result.succeeded.len() + batch_result.failed.len();
                    let status = match (batch_result.succeeded.len(), batch_result.failed.len()) {
                        (_, 0) => "succeeded",
                        (0, _) => "failed",
                        _ => "partial",
                    };
                    let failed_codes: Vec<String> = batch_result
                        .failed
                        .iter()
                        .filter_map(|item| item.error_code.clone())
                        .collect();
                    record_operation_log_best_effort(
                        &state.db,
                        target_context,
                        OperationLogEvent::new(
                            "delete",
                            "central.reset_unknown_source",
                            status,
                            format!(
                                "Reset {} unknown-source Central skill(s), {} failed",
                                batch_result.succeeded.len(),
                                batch_result.failed.len()
                            ),
                        )
                        .subject(
                            "batch",
                            "central.reset_unknown_source",
                            "Unknown-source Central reset",
                        )
                        .details(json!({
                            "targetKind": target_kind,
                            "attempted": attempted,
                            "succeeded": batch_result.succeeded.len(),
                            "failed": batch_result.failed.len(),
                            "failedCodes": failed_codes,
                        }))
                        .duration_ms(started_at.elapsed().as_millis() as i64),
                    )
                    .await;
                }
                Err(error) => {
                    record_operation_log_best_effort(
                        &state.db,
                        target_context,
                        OperationLogEvent::new(
                            "delete",
                            "central.reset_unknown_source",
                            "failed",
                            "Failed to reset unknown-source Central skills",
                        )
                        .subject(
                            "batch",
                            "central.reset_unknown_source",
                            "Unknown-source Central reset",
                        )
                        .error(error)
                        .details(json!({
                            "targetKind": target_kind,
                        }))
                        .duration_ms(started_at.elapsed().as_millis() as i64),
                    )
                    .await;
                }
            }
            result
        }
        .await
    )
}

#[tauri::command]
pub async fn preview_delete_skill_repository(
    state: State<'_, AppState>,
    repository_id: String,
) -> crate::ipc_error::IpcResult<DeleteSkillRepositoryPreview> {
    crate::ipc_boundary!(
        async move {
            let request_context = state.resolve_target_context().await?;
            let pool = request_context.db().clone();
            match request_context.target() {
                ActiveTarget::Local => {
                    central_skills::preview_delete_skill_repository_impl(&pool, &repository_id)
                        .await
                }
                ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => {
                    central_skills::preview_delete_skill_repository_ssh_impl(
                        &pool,
                        request_context.target(),
                        &repository_id,
                    )
                    .await
                }
            }
            .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn delete_skill_repository(
    state: State<'_, AppState>,
    repository_id: String,
    requests: Vec<BatchDeleteCentralSkillRequest>,
) -> crate::ipc_error::IpcResult<DeleteSkillRepositoryResult> {
    crate::ipc_boundary!(
        async move {
            let request_context = state.resolve_target_context().await?;
            let active_target = request_context.target().clone();
            let target_context = target_context_from_active_target(&active_target);
            let pool = request_context.db().clone();
            let started_at = Instant::now();
            let result = match &active_target {
                ActiveTarget::Local => {
                    central_skills::delete_skill_repository_impl(&pool, &repository_id, &requests)
                        .await
                }
                ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => {
                    central_skills::delete_skill_repository_remote_impl(
                        &pool,
                        &active_target,
                        &repository_id,
                        &requests,
                    )
                    .await
                }
            }
            .map_err(delete_command_error);
            match &result {
                Ok(delete_result) => {
                    let batch_result = &delete_result.delete_result;
                    let status = match (batch_result.succeeded.len(), batch_result.failed.len()) {
                        (_, 0) => "succeeded",
                        (0, _) => "failed",
                        _ => "partial",
                    };
                    record_operation_log_best_effort(
                        &state.db,
                        target_context,
                        OperationLogEvent::new(
                            "delete",
                            "central.delete_repository",
                            status,
                            format!(
                                "Deleted repository {} with {} skill(s), {} failed",
                                delete_result.repository.name,
                                batch_result.succeeded.len(),
                                batch_result.failed.len()
                            ),
                        )
                        .subject(
                            "repository",
                            &delete_result.repository.id,
                            &delete_result.repository.name,
                        )
                        .details(json!({
                            "repositoryId": repository_id,
                            "requestCount": requests.len(),
                            "deletedRepository": delete_result.deleted_repository,
                            "succeeded": &batch_result.succeeded,
                            "failed": &batch_result.failed,
                        }))
                        .duration_ms(started_at.elapsed().as_millis() as i64),
                    )
                    .await;
                }
                Err(error) => {
                    record_operation_log_best_effort(
                        &state.db,
                        target_context,
                        OperationLogEvent::new(
                            "delete",
                            "central.delete_repository",
                            "failed",
                            format!("Failed to delete repository {}", repository_id),
                        )
                        .subject("repository", &repository_id, &repository_id)
                        .error(error)
                        .details(json!({
                            "repositoryId": repository_id,
                            "requestCount": requests.len(),
                        }))
                        .duration_ms(started_at.elapsed().as_millis() as i64),
                    )
                    .await;
                }
            }
            result
        }
        .await
    )
}

/// Tauri command: return detailed information about a skill, including all
/// installation records across agents. Each installation includes `installed_at`
/// (the `created_at` timestamp from the DB, renamed for frontend clarity).
#[tauri::command]
pub async fn get_skill_detail(
    state: State<'_, AppState>,
    skill_id: String,
    agent_id: Option<String>,
    row_id: Option<String>,
) -> crate::ipc_error::IpcResult<SkillDetail> {
    crate::ipc_boundary!(
        async move {
            let pool = state.active_db().await?;
            central_skills::get_skill_detail_with_row_impl(
                &pool,
                &skill_id,
                agent_id.as_deref(),
                row_id.as_deref(),
            )
            .await
            .map_err(|e| e.to_string())
        }
        .await
    )
}

/// Tauri command: read and return the raw content of a skill's `SKILL.md` file.
#[tauri::command]
pub async fn read_skill_content(
    state: State<'_, AppState>,
    skill_id: String,
) -> crate::ipc_error::IpcResult<String> {
    crate::ipc_boundary!(
        async move {
            let request_context = state.resolve_target_context().await?;
            let pool = request_context.db().clone();
            let active_target = request_context.target().clone();
            central_skills::read_skill_content_for_target_impl(&pool, active_target, &skill_id)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
pub async fn read_file_by_path(
    state: State<'_, AppState>,
    path: String,
    skill_id: Option<String>,
    agent_id: Option<String>,
    row_id: Option<String>,
) -> crate::ipc_error::IpcResult<String> {
    crate::ipc_boundary!(
        async move {
            let request_context = state.resolve_target_context().await?;
            let pool = request_context.db().clone();
            let active_target = request_context.target().clone();
            let access = path_access_context(skill_id, agent_id, row_id)?;
            central_skills::read_file_by_path_for_target_impl(&pool, active_target, &path, &access)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
pub async fn open_in_file_manager(
    state: State<'_, AppState>,
    path: String,
    skill_id: Option<String>,
    agent_id: Option<String>,
    row_id: Option<String>,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary!(
        async move {
            let request_context = state.resolve_target_context().await?;
            let pool = request_context.db().clone();
            let active_target = request_context.target().clone();
            let access = path_access_context(skill_id, agent_id, row_id)?;
            central_skills::open_in_file_manager_for_target_impl(
                &pool,
                active_target,
                &path,
                &access,
            )
            .await
            .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
pub async fn list_directory_tree(
    state: State<'_, AppState>,
    path: String,
    skill_id: Option<String>,
    agent_id: Option<String>,
    row_id: Option<String>,
) -> crate::ipc_error::IpcResult<Vec<DirectoryTreeEntry>> {
    crate::ipc_boundary!(
        async move {
            let request_context = state.resolve_target_context().await?;
            let pool = request_context.db().clone();
            let active_target = request_context.target().clone();
            let access = path_access_context(skill_id, agent_id, row_id)?;
            central_skills::list_directory_tree_for_target_impl(
                &pool,
                active_target,
                &path,
                &access,
            )
            .await
            .map_err(|e| e.to_string())
        }
        .await
    )
}

fn delete_command_error(error: central_skills::CentralSkillsError) -> String {
    format!(
        "{}:{}",
        error.stable_delete_error_code(),
        error.public_delete_message()
    )
}

fn reset_command_error(error: central_skills::CentralSkillsError) -> String {
    let code = error.stable_delete_error_code();
    if code == "central_skills.mutation_lock_failed" {
        format!("{code}:{}", error.public_delete_message())
    } else {
        format!("central.reset_failed:{code}")
    }
}

fn path_access_context(
    skill_id: Option<String>,
    agent_id: Option<String>,
    row_id: Option<String>,
) -> Result<SkillPathAccessContext, String> {
    let skill_id = skill_id
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "A skill context is required for file access.".to_string())?;
    Ok(SkillPathAccessContext {
        skill_id,
        agent_id,
        row_id,
    })
}
