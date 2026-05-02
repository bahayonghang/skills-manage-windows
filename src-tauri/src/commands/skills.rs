//! Tauri IPC shells for Central Skills and skill detail operations.
//!
//! Business logic lives in `crate::services::central_skills`. This module keeps
//! the existing command names and public type paths stable while translating
//! `State<AppState>` into service inputs and recording operation logs for
//! destructive Central operations.

use std::time::Instant;

use serde_json::json;
use tauri::State;

use crate::commands::logs::{
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
    get_central_skills_impl, get_skill_detail_with_row_impl, get_skills_by_agent_impl,
    preview_delete_central_skills_impl, preview_delete_central_skills_ssh_impl,
    preview_delete_skill_repository_impl, preview_delete_skill_repository_ssh_impl,
    BatchDeleteCentralSkillPreviewResult, BatchDeleteCentralSkillRequest,
    BatchDeleteCentralSkillResult, BatchDeleteCentralSkillSuccess, DeleteCentralSkillPreview,
    DeleteCentralSkillResult, DeleteSkillRepositoryPreview, DeleteSkillRepositoryResult,
    FailedCentralSkillDelete, SkillDetail, SkillInstallationDetail, SkillWithLinks,
};

/// Tauri command: return all skills installed for a given agent, including
/// installation metadata needed by the platform-view skill cards.
#[tauri::command]
pub async fn get_skills_by_agent(
    state: State<'_, AppState>,
    agent_id: String,
) -> Result<Vec<SkillForAgent>, String> {
    let pool = state.active_db().await?;
    central_skills::get_skills_by_agent_impl(&pool, &agent_id).await
}

/// Tauri command: return all Central Skills with per-platform link status.
#[tauri::command]
pub async fn get_central_skills(state: State<'_, AppState>) -> Result<Vec<SkillWithLinks>, String> {
    let pool = state.active_db().await?;
    central_skills::get_central_skills_impl(&pool).await
}

#[tauri::command]
pub async fn preview_delete_central_skills(
    state: State<'_, AppState>,
    skill_ids: Vec<String>,
) -> Result<BatchDeleteCentralSkillPreviewResult, String> {
    let pool = state.active_db().await?;
    match state.active_target().await? {
        ActiveTarget::Local => {
            central_skills::preview_delete_central_skills_impl(&pool, &skill_ids).await
        }
        ActiveTarget::Ssh(_) => {
            central_skills::preview_delete_central_skills_ssh_impl(&pool, &skill_ids).await
        }
    }
}

#[tauri::command]
pub async fn delete_central_skill(
    state: State<'_, AppState>,
    skill_id: String,
    remove_agent_ids: Vec<String>,
) -> Result<DeleteCentralSkillResult, String> {
    let active_target = state.active_target().await?;
    let target_context = target_context_from_active_target(&active_target);
    let pool = state.active_db().await?;
    let started_at = Instant::now();
    let result = match active_target {
        ActiveTarget::Local => {
            central_skills::delete_central_skill_impl(&pool, &skill_id, &remove_agent_ids).await
        }
        ActiveTarget::Ssh(target) => {
            central_skills::delete_central_skill_ssh_impl(
                &pool,
                &target,
                &skill_id,
                &remove_agent_ids,
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

#[tauri::command]
pub async fn delete_central_skills(
    state: State<'_, AppState>,
    requests: Vec<BatchDeleteCentralSkillRequest>,
) -> Result<BatchDeleteCentralSkillResult, String> {
    let active_target = state.active_target().await?;
    let target_context = target_context_from_active_target(&active_target);
    let pool = state.active_db().await?;
    let started_at = Instant::now();
    let result = match active_target {
        ActiveTarget::Local => central_skills::delete_central_skills_impl(&pool, &requests).await,
        ActiveTarget::Ssh(target) => {
            central_skills::delete_central_skills_ssh_impl(&pool, &target, &requests).await
        }
    };
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

#[tauri::command]
pub async fn preview_delete_skill_repository(
    state: State<'_, AppState>,
    repository_id: String,
) -> Result<DeleteSkillRepositoryPreview, String> {
    let pool = state.active_db().await?;
    match state.active_target().await? {
        ActiveTarget::Local => {
            central_skills::preview_delete_skill_repository_impl(&pool, &repository_id).await
        }
        ActiveTarget::Ssh(_) => {
            central_skills::preview_delete_skill_repository_ssh_impl(&pool, &repository_id).await
        }
    }
}

#[tauri::command]
pub async fn delete_skill_repository(
    state: State<'_, AppState>,
    repository_id: String,
    requests: Vec<BatchDeleteCentralSkillRequest>,
) -> Result<DeleteSkillRepositoryResult, String> {
    let active_target = state.active_target().await?;
    let target_context = target_context_from_active_target(&active_target);
    let pool = state.active_db().await?;
    let started_at = Instant::now();
    let result = match active_target {
        ActiveTarget::Local => {
            central_skills::delete_skill_repository_impl(&pool, &repository_id, &requests).await
        }
        ActiveTarget::Ssh(target) => {
            central_skills::delete_skill_repository_ssh_impl(
                &pool,
                &target,
                &repository_id,
                &requests,
            )
            .await
        }
    };
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

/// Tauri command: return detailed information about a skill, including all
/// installation records across agents. Each installation includes `installed_at`
/// (the `created_at` timestamp from the DB, renamed for frontend clarity).
#[tauri::command]
pub async fn get_skill_detail(
    state: State<'_, AppState>,
    skill_id: String,
    agent_id: Option<String>,
    row_id: Option<String>,
) -> Result<SkillDetail, String> {
    let pool = state.active_db().await?;
    central_skills::get_skill_detail_with_row_impl(
        &pool,
        &skill_id,
        agent_id.as_deref(),
        row_id.as_deref(),
    )
    .await
}

/// Tauri command: read and return the raw content of a skill's `SKILL.md` file.
#[tauri::command]
pub async fn read_skill_content(
    state: State<'_, AppState>,
    skill_id: String,
) -> Result<String, String> {
    let pool = state.active_db().await?;
    let active_target = state.active_target().await?;
    central_skills::read_skill_content_for_target_impl(&pool, active_target, &skill_id).await
}

#[tauri::command]
pub async fn read_file_by_path(state: State<'_, AppState>, path: String) -> Result<String, String> {
    let active_target = state.active_target().await?;
    central_skills::read_file_by_path_for_target_impl(active_target, &path).await
}

#[tauri::command]
pub async fn open_in_file_manager(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let active_target = state.active_target().await?;
    central_skills::open_in_file_manager_for_target_impl(active_target, &path)
}
