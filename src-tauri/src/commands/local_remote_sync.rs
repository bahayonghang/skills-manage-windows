//! Tauri command shell for syncing the current local repo and local Central
//! skills to a selected SSH/WSL target.

use serde_json::json;
use tauri::State;

use crate::operation_log::{
    record_operation_log_best_effort, target_context_from_active_target, OperationLogEvent,
};
use crate::services::local_remote_sync::{
    apply_local_remote_sync_impl, preview_local_remote_sync_impl, LocalRemoteSyncApplyRequest,
    LocalRemoteSyncApplyResult, LocalRemoteSyncFailure, LocalRemoteSyncPreview,
    LocalRemoteSyncPreviewRequest,
};
use crate::services::scanner::scan_remote_skills_impl;
use crate::targets::ActiveTarget;
use crate::AppState;

#[tauri::command]
pub async fn preview_local_remote_sync(
    state: State<'_, AppState>,
    request: LocalRemoteSyncPreviewRequest,
) -> Result<LocalRemoteSyncPreview, String> {
    let active_target = selected_remote_target(&state, &request.target_id).await?;
    preview_local_remote_sync_impl(active_target, request.repo_path).await
}

#[tauri::command]
pub async fn apply_local_remote_sync(
    state: State<'_, AppState>,
    request: LocalRemoteSyncApplyRequest,
) -> Result<LocalRemoteSyncApplyResult, String> {
    let active_target = selected_remote_target(&state, &request.target_id).await?;
    let target_context = target_context_from_active_target(&active_target);
    let mut result = apply_local_remote_sync_impl(active_target.clone(), request.repo_path).await;
    if let Ok(value) = &mut result {
        refresh_synced_target_cache(&state, &active_target, value).await;
    }

    match &result {
        Ok(value) => {
            let status = if value.failed.is_empty() {
                "succeeded"
            } else if value.synced_repo.is_none() && value.synced_skills.is_empty() {
                "failed"
            } else {
                "partial"
            };
            record_operation_log_best_effort(
                &state.db,
                target_context,
                OperationLogEvent::new(
                    "target",
                    "local_remote_sync.apply",
                    status,
                    format!(
                        "Synced local repo and {} skill(s) to {}",
                        value.synced_skills.len(),
                        value.target_label
                    ),
                )
                .subject("target", &value.target_id, &value.target_label)
                .details(json!({
                    "syncedRepo": value.synced_repo.as_ref().map(|item| &item.remote_path),
                    "syncedSkills": value.synced_skills.iter().map(|item| &item.id).collect::<Vec<_>>(),
                    "skippedSkills": value.skipped_skills.iter().map(|item| &item.id).collect::<Vec<_>>(),
                    "failed": value.failed,
                })),
            )
            .await;
        }
        Err(error) => {
            record_operation_log_best_effort(
                &state.db,
                target_context,
                OperationLogEvent::new(
                    "target",
                    "local_remote_sync.apply",
                    "failed",
                    "Failed to sync local repo and skills to remote target",
                )
                .subject("target", active_target.id(), active_target.label())
                .error(error),
            )
            .await;
        }
    }

    result
}

async fn selected_remote_target(
    state: &State<'_, AppState>,
    target_id: &str,
) -> Result<ActiveTarget, String> {
    if target_id.trim().is_empty() || target_id == "local" {
        return Err("Select an SSH or WSL target before syncing.".to_string());
    }

    let target = state.targets.target_by_id(&state.db, target_id).await?;

    match target {
        ActiveTarget::Local => Err("Select an SSH or WSL target before syncing.".to_string()),
        ActiveTarget::Ssh(_) | ActiveTarget::Wsl(_) => Ok(target),
    }
}

async fn refresh_synced_target_cache(
    state: &State<'_, AppState>,
    active_target: &ActiveTarget,
    result: &mut LocalRemoteSyncApplyResult,
) {
    let pool = match active_target {
        ActiveTarget::Local => return,
        ActiveTarget::Ssh(target) => state.targets.remote_db(target).await,
        ActiveTarget::Wsl(target) => {
            state
                .targets
                .remote_db_for(&target.id, &target.remote_home)
                .await
        }
    };

    match pool {
        Ok(pool) => {
            if let Err(error) = scan_remote_skills_impl(&pool, active_target).await {
                push_refresh_failure(active_target, result, error.to_string());
            }
        }
        Err(error) => push_refresh_failure(active_target, result, error),
    }
}

fn push_refresh_failure(
    active_target: &ActiveTarget,
    result: &mut LocalRemoteSyncApplyResult,
    error: String,
) {
    result.failed.push(LocalRemoteSyncFailure {
        id: "target-cache-refresh".to_string(),
        label: "Target cache refresh".to_string(),
        target_path: active_target.remote_home().unwrap_or("").to_string(),
        error,
    });
}
