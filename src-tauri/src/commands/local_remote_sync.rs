//! Tauri command shell for syncing the current local repo and local Central
//! skills to a selected SSH/WSL target.

use tauri::State;

use crate::observability::{
    CommandLogPolicy, OperationContext, OperationDefinition, OperationTarget, OperationTargetKind,
    ReviewedDiagnostic, ReviewedFailure, SafeDetailKey, SafeOperationResult,
};
use crate::services::local_remote_sync::{
    apply_local_remote_sync_impl, preview_local_remote_sync_impl, LocalRemoteSyncApplyRequest,
    LocalRemoteSyncApplyResult, LocalRemoteSyncFailure, LocalRemoteSyncPreview,
    LocalRemoteSyncPreviewRequest,
};
use crate::services::scanner::scan_remote_skills_impl;
use crate::targets::ActiveTarget;
use crate::AppState;

fn operation_definition() -> OperationDefinition {
    match crate::ipc_registry::command_policy("apply_local_remote_sync")
        .expect("sync command must be registered")
        .policy
    {
        CommandLogPolicy::Operation(definition) => definition,
        _ => unreachable!("sync command must have an operation policy"),
    }
}

fn audit_target(target: &ActiveTarget) -> (OperationTargetKind, OperationTarget) {
    match target {
        ActiveTarget::Local => (OperationTargetKind::Local, OperationTarget::local()),
        ActiveTarget::Ssh(target) => (
            OperationTargetKind::Ssh,
            OperationTarget::new(OperationTargetKind::Ssh, &target.id),
        ),
        ActiveTarget::Wsl(target) => (
            OperationTargetKind::Wsl,
            OperationTarget::new(OperationTargetKind::Wsl, &target.id),
        ),
    }
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn preview_local_remote_sync(
    state: State<'_, AppState>,
    request: LocalRemoteSyncPreviewRequest,
) -> crate::ipc_error::IpcResult<LocalRemoteSyncPreview> {
    let target_kind = if request.target_id.starts_with("ssh-") {
        OperationTargetKind::Ssh
    } else if request.target_id.starts_with("wsl-") {
        OperationTargetKind::Wsl
    } else {
        OperationTargetKind::Local
    };
    crate::ipc_boundary!(
        "preview_local_remote_sync",
        target_kind = target_kind,
        async move {
            let active_target = selected_remote_target(&state, &request.target_id).await?;
            preview_local_remote_sync_impl(active_target, request.repo_path)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn apply_local_remote_sync(
    state: State<'_, AppState>,
    request: LocalRemoteSyncApplyRequest,
) -> crate::ipc_error::IpcResult<LocalRemoteSyncApplyResult> {
    let target_kind = if request.target_id.starts_with("ssh-") {
        OperationTargetKind::Ssh
    } else if request.target_id.starts_with("wsl-") {
        OperationTargetKind::Wsl
    } else {
        OperationTargetKind::Local
    };
    crate::ipc_boundary!(
        "apply_local_remote_sync",
        target_kind = target_kind,
        async move {
            let active_target = selected_remote_target(&state, &request.target_id).await?;
            let (_, audit_target) = audit_target(&active_target);
            let definition = operation_definition();
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(audit_target),
                |value: &LocalRemoteSyncApplyResult| {
                    let result = if value.failed.is_empty() {
                        SafeOperationResult::succeeded("Local and remote skills synchronized.")
                    } else {
                        SafeOperationResult::partial(
                            "Local and remote synchronization completed with failures.",
                        )
                    };
                    result
                        .count(
                            SafeDetailKey::SucceededCount,
                            value.synced_skills.len() as u64,
                        )
                        .count(
                            SafeDetailKey::SkippedCount,
                            value.skipped_skills.len() as u64,
                        )
                        .count(SafeDetailKey::FailedCount, value.failed.len() as u64)
                },
                || async {
                    let mut value =
                        apply_local_remote_sync_impl(active_target.clone(), request.repo_path)
                            .await
                            .map_err(|_| {
                                ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition))
                            })?;
                    refresh_synced_target_cache(&state, &active_target, &mut value).await;
                    Ok::<_, ReviewedFailure>(value)
                },
            )
            .await
        }
        .await,
    )
}

async fn selected_remote_target(
    state: &State<'_, AppState>,
    target_id: &str,
) -> Result<ActiveTarget, String> {
    if target_id.trim().is_empty() || target_id == "local" {
        return Err("Select an SSH or WSL target before syncing.".to_string());
    }

    let target = state
        .targets
        .target_by_id(&state.db, target_id)
        .await
        .map_err(|e| e.to_string())?;

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
        Err(error) => push_refresh_failure(active_target, result, error.to_string()),
    }
}

fn push_refresh_failure(
    active_target: &ActiveTarget,
    result: &mut LocalRemoteSyncApplyResult,
    _error: String,
) {
    result.failed.push(LocalRemoteSyncFailure {
        id: "target-cache-refresh".to_string(),
        label: "Target cache refresh".to_string(),
        target_path: active_target.remote_home().unwrap_or("").to_string(),
        error: "Target cache refresh failed.".to_string(),
    });
}
