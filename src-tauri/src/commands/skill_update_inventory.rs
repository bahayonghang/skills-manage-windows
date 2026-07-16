//! Tauri IPC shells for the Skill Update Inventory (Update Center panel).
//!
//! Business logic lives in `crate::services::central_updates::inventory`.
//! This module keeps the existing command names and payload shapes stable
//! while translating `State<AppState>` into service inputs.

use serde::Serialize;
use serde_json::{json, Value};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

use crate::operation_log::{
    with_operation_log, OperationLogEvent, OperationLogTargetContext, OperationSpec,
};
use crate::services::central_updates::inventory::{
    apply_skill_update_decisions_impl, clear_skill_update_inventory_impl,
    force_mirror_central_repositories_impl, force_update_central_skills_impl,
    get_skill_update_inventory_impl_scoped, refresh_skill_update_inventory_impl,
    scan_deleted_platform_copies_with_pool, scan_platform_duplicate_skills_with_pool,
    DeletedPlatformCopyGroup, ForceRepositoryMirrorRequest, ForceRepositoryMirrorResult,
    ForceSkillUpdateRequest, ForceSkillUpdateResult, PlatformDuplicateGroup, SkillRefreshScope,
    SkillRefreshScopeKind, SkillUpdateApplyResult, SkillUpdateDecisions, SkillUpdateInventory,
};
use crate::services::central_updates::{
    CentralFs, SnapshotCachePolicy, SnapshotProgressEvent, SnapshotProgressReporter,
    SnapshotProgressStatus,
};
use crate::services::github_import;
use crate::targets::ActiveTarget;
use crate::AppState;

const UPDATE_INVENTORY_PROGRESS_EVENT: &str = "central://skill-update-inventory-progress";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SkillUpdateInventoryProgressPayload {
    operation_id: String,
    status: &'static str,
    total: usize,
    completed: usize,
    repository_key: Option<String>,
    repository_name: Option<String>,
}

#[derive(Debug)]
struct UpdateCommandError(String);

impl UpdateCommandError {
    fn into_inner(self) -> String {
        self.0
    }
}

impl From<String> for UpdateCommandError {
    fn from(error: String) -> Self {
        Self(error)
    }
}

impl std::fmt::Display for UpdateCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Update Center action failed")
    }
}

#[tauri::command]
pub async fn refresh_skill_update_inventory(
    app: AppHandle,
    state: State<'_, AppState>,
    scope: SkillRefreshScope,
    operation_id: String,
) -> Result<SkillUpdateInventory, String> {
    let active_target = state.active_target().await?;
    let target_context = update_target_context(&active_target);
    let request_details = refresh_request_details(&scope);
    let progress_app = Arc::new(app);
    let progress: SnapshotProgressReporter = Arc::new(move |event: SnapshotProgressEvent| {
        let payload = SkillUpdateInventoryProgressPayload {
            operation_id: operation_id.clone(),
            status: match event.status {
                SnapshotProgressStatus::Started => "started",
                SnapshotProgressStatus::RepositoryStarted => "repository_started",
                SnapshotProgressStatus::RepositoryCompleted => "repository_completed",
                SnapshotProgressStatus::RepositoryFailed => "repository_failed",
                SnapshotProgressStatus::Finalizing => "finalizing",
            },
            total: event.total,
            completed: event.completed,
            repository_key: event.repository_key,
            repository_name: event.repository_name,
        };
        let _ = progress_app.emit(UPDATE_INVENTORY_PROGRESS_EVENT, payload);
    });
    with_operation_log(
        &state,
        update_operation_spec(
            target_context,
            "update_center.refresh",
            "Refreshed skill update inventory",
            "Failed to refresh skill update inventory",
            request_details,
            refresh_result_details,
        ),
        || async {
            let pool = state.active_db().await?;
            let fs = CentralFs::from_active_target(active_target)
                .await
                .map_err(|e| e.to_string())?;
            let auth = github_import::github_direct_auth_from_secret_store(
                &state.db,
                state.secrets.as_ref(),
            )
            .await
            .map_err(|e| e.to_string())?;
            let client = github_import::github_client().map_err(|e| e.to_string())?;
            refresh_skill_update_inventory_impl(
                &pool,
                &fs,
                auth.as_deref(),
                &client,
                &state.central_update_snapshots,
                scope,
                Some(progress),
            )
            .await
            .map_err(|e| UpdateCommandError(e.to_string()))
        },
    )
    .await
    .map_err(UpdateCommandError::into_inner)
}

#[tauri::command]
pub async fn get_skill_update_inventory(
    state: State<'_, AppState>,
    scope: Option<SkillRefreshScope>,
) -> Result<SkillUpdateInventory, String> {
    let pool = state.active_db().await?;
    get_skill_update_inventory_impl_scoped(&pool, scope)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn clear_skill_update_inventory(
    state: State<'_, AppState>,
    scope: Option<SkillRefreshScope>,
) -> Result<(), String> {
    let pool = state.active_db().await?;
    clear_skill_update_inventory_impl(&pool, scope)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn apply_skill_update_decisions(
    app: AppHandle,
    state: State<'_, AppState>,
    decisions: SkillUpdateDecisions,
) -> Result<SkillUpdateApplyResult, String> {
    let active_target = state.active_target().await?;
    let target_context = update_target_context(&active_target);
    let request_details = apply_request_details(&decisions);
    with_operation_log(
        &state,
        update_operation_spec(
            target_context,
            "update_center.apply",
            "Applied skill update decisions",
            "Failed to apply skill update decisions",
            request_details,
            apply_result_details,
        ),
        || async {
            let pool = state.active_db().await?;
            let fs = CentralFs::from_active_target(active_target.clone())
                .await
                .map_err(|e| e.to_string())?;
            let auth = github_import::github_direct_auth_from_secret_store(
                &state.db,
                state.secrets.as_ref(),
            )
            .await
            .map_err(|e| e.to_string())?;
            let client = github_import::github_client().map_err(|e| e.to_string())?;
            apply_skill_update_decisions_impl(
                Some(&app),
                &pool,
                &active_target,
                &fs,
                &state.central_update_cancel,
                auth.as_deref(),
                &client,
                &state.central_update_snapshots,
                decisions,
            )
            .await
            .map_err(|e| UpdateCommandError(e.to_string()))
        },
    )
    .await
    .map_err(UpdateCommandError::into_inner)
}

#[tauri::command]
pub async fn force_update_central_skills(
    state: State<'_, AppState>,
    request: ForceSkillUpdateRequest,
) -> Result<ForceSkillUpdateResult, String> {
    let active_target = state.active_target().await?;
    let target_context = update_target_context(&active_target);
    let request_details = json!({
        "requestedSkills": request.skill_ids.len(),
        "refreshCopies": request.refresh_copy_installations,
    });
    with_operation_log(
        &state,
        update_operation_spec(
            target_context,
            "update_center.force_update",
            "Force-updated Central skills",
            "Failed to force-update Central skills",
            request_details,
            |result: &ForceSkillUpdateResult| {
                json!({
                    "overwritten": result.overwritten.len(),
                    "skipped": result.skipped.len(),
                    "failed": result.failed.len(),
                })
            },
        ),
        || async {
            let pool = state.active_db().await?;
            let fs = CentralFs::from_active_target(active_target)
                .await
                .map_err(|e| e.to_string())?;
            let auth = github_import::github_direct_auth_from_secret_store(
                &state.db,
                state.secrets.as_ref(),
            )
            .await
            .map_err(|e| e.to_string())?;
            let client = github_import::github_client().map_err(|e| e.to_string())?;
            force_update_central_skills_impl(
                &pool,
                &fs,
                auth.as_deref(),
                &client,
                &state.central_update_snapshots,
                SnapshotCachePolicy::Bypass,
                request,
            )
            .await
            .map_err(|e| UpdateCommandError(e.to_string()))
        },
    )
    .await
    .map_err(UpdateCommandError::into_inner)
}

#[tauri::command]
pub async fn force_mirror_central_repositories(
    app: AppHandle,
    state: State<'_, AppState>,
    request: ForceRepositoryMirrorRequest,
) -> Result<ForceRepositoryMirrorResult, String> {
    let active_target = state.active_target().await?;
    let target_context = update_target_context(&active_target);
    let request_details = json!({
        "requestedRepositories": request.repository_ids.len(),
        "deleteMissing": request.delete_missing,
        "importAdded": request.import_added,
        "overwriteTracked": request.overwrite_tracked,
    });
    with_operation_log(
        &state,
        update_operation_spec(
            target_context,
            "update_center.force_mirror",
            "Force-mirrored Central repositories",
            "Failed to force-mirror Central repositories",
            request_details,
            |result: &ForceRepositoryMirrorResult| {
                json!({
                    "overwritten": result.overwritten.len(),
                    "imported": result.imported.len(),
                    "deleted": result.deleted.succeeded.len(),
                    "deleteFailures": result.deleted.failed.len(),
                    "skipped": result.skipped.len(),
                    "failedRepositories": result.failed_repositories.len(),
                    "failedItems": result.failed_items.len(),
                })
            },
        ),
        || async {
            let pool = state.active_db().await?;
            let fs = CentralFs::from_active_target(active_target.clone())
                .await
                .map_err(|e| e.to_string())?;
            let auth = github_import::github_direct_auth_from_secret_store(
                &state.db,
                state.secrets.as_ref(),
            )
            .await
            .map_err(|e| e.to_string())?;
            let client = github_import::github_client().map_err(|e| e.to_string())?;
            force_mirror_central_repositories_impl(
                Some(&app),
                &pool,
                &active_target,
                &fs,
                auth.as_deref(),
                &client,
                &state.central_update_snapshots,
                SnapshotCachePolicy::Bypass,
                request,
            )
            .await
            .map_err(|e| UpdateCommandError(e.to_string()))
        },
    )
    .await
    .map_err(UpdateCommandError::into_inner)
}

fn update_operation_event(
    action: &str,
    status: &str,
    summary: &str,
    details: Value,
    duration_ms: i64,
) -> OperationLogEvent {
    OperationLogEvent::new("update_center", action, status, summary)
        .details(details)
        .duration_ms(duration_ms)
}

fn update_target_context(active_target: &ActiveTarget) -> OperationLogTargetContext {
    let kind = match active_target {
        ActiveTarget::Local => "local",
        ActiveTarget::Ssh(_) => "ssh",
        ActiveTarget::Wsl(_) => "wsl",
    };
    OperationLogTargetContext {
        kind: kind.to_string(),
        id: kind.to_string(),
        label: None,
    }
}

fn update_operation_spec<'a, R, ResultDetails>(
    target_context: OperationLogTargetContext,
    action: &'static str,
    success_summary: &'static str,
    failure_summary: &'static str,
    request_details: Value,
    result_details: ResultDetails,
) -> OperationSpec<'a, R, UpdateCommandError>
where
    ResultDetails: FnOnce(&R) -> Value + Send + 'a,
{
    let failure_details = request_details.clone();
    OperationSpec::new(
        target_context,
        move |result, duration_ms| {
            update_operation_event(
                action,
                "succeeded",
                success_summary,
                merge_details(request_details, result_details(result)),
                duration_ms,
            )
        },
        move |_: &UpdateCommandError, duration_ms| {
            update_operation_event(
                action,
                "failed",
                failure_summary,
                failure_details,
                duration_ms,
            )
        },
    )
}

fn merge_details(mut base: Value, extra: Value) -> Value {
    if let (Some(base), Some(extra)) = (base.as_object_mut(), extra.as_object()) {
        base.extend(extra.clone());
    }
    base
}

fn refresh_request_details(scope: &SkillRefreshScope) -> Value {
    let kind = match scope.kind {
        SkillRefreshScopeKind::All => "all",
        SkillRefreshScopeKind::Skills => "skills",
        SkillRefreshScopeKind::Repositories => "repositories",
        SkillRefreshScopeKind::Platform => "platform",
    };
    json!({
        "scopeKind": kind,
        "requestedSkills": scope.skill_ids.as_ref().map_or(0, Vec::len),
        "requestedRepositories": scope.repository_ids.as_ref().map_or(0, Vec::len),
        "requestedAgents": scope.agent_ids.as_ref().map_or(0, Vec::len),
    })
}

fn refresh_result_details(result: &SkillUpdateInventory) -> Value {
    json!({
        "updatable": result.updatable.len(),
        "remoteAdded": result.remote_added.len(),
        "remoteMissing": result.remote_missing.len(),
        "failedRepositories": result.failed_repositories.len(),
    })
}

fn apply_request_details(decisions: &SkillUpdateDecisions) -> Value {
    json!({
        "updates": decisions.updates.len(),
        "keepMissing": decisions.keep_missing.len(),
        "deleteMissing": decisions.delete_missing.len(),
        "importAdditions": decisions.import_additions.len(),
        "skipAdditions": decisions.skip_additions.len(),
        "unskipAdditions": decisions.unskip_additions.len(),
        "removePlatformDuplicates": decisions.remove_platform_duplicates.len(),
        "removeDeletedCopies": decisions.remove_deleted_platform_copies.len(),
    })
}

fn apply_result_details(result: &SkillUpdateApplyResult) -> Value {
    json!({
        "updated": result.updated_skill_ids.len(),
        "keptMissing": result.kept_missing_skill_ids.len(),
        "deleted": result.deleted_skill_ids.len(),
        "imported": result.imported_skill_ids.len(),
        "failures": result.failures.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_operation_event_records_duration_and_redacts_details() {
        let event = update_operation_event(
            "update_center.refresh",
            "succeeded",
            "Refreshed skill update inventory",
            json!({ "skills": 10, "token": "secret" }),
            42,
        );

        assert_eq!(event.duration_ms, Some(42));
        let details: Value = serde_json::from_str(event.details_json.as_deref().unwrap()).unwrap();
        assert_eq!(details["skills"], 10);
        assert_eq!(details["token"], "[redacted]");
    }

    #[test]
    fn update_command_error_hides_sensitive_details_from_operation_log_display() {
        let original = "ssh host secret.example failed at /home/alice/private".to_string();
        let error = UpdateCommandError(original.clone());

        assert_eq!(error.to_string(), "Update Center action failed");
        assert_eq!(error.into_inner(), original);
    }

    #[test]
    fn update_target_context_records_kind_without_remote_identity() {
        let target = ActiveTarget::Wsl(Box::new(crate::targets::WslTargetConfig {
            id: "private-target-id".to_string(),
            label: "alice@example.internal".to_string(),
            distribution: "Ubuntu".to_string(),
            remote_home: "/home/alice".to_string(),
            remote_os: "linux".to_string(),
            symlink_enabled: true,
        }));

        let context = update_target_context(&target);

        assert_eq!(context.kind, "wsl");
        assert_eq!(context.id, "wsl");
        assert!(context.label.is_none());
    }
}

#[tauri::command]
pub async fn scan_platform_duplicate_skills(
    state: State<'_, AppState>,
    agent_ids: Option<Vec<String>>,
) -> Result<Vec<PlatformDuplicateGroup>, String> {
    let pool = state.active_db().await?;
    scan_platform_duplicate_skills_with_pool(&pool, agent_ids)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn scan_deleted_platform_copies(
    state: State<'_, AppState>,
    agent_ids: Option<Vec<String>>,
) -> Result<Vec<DeletedPlatformCopyGroup>, String> {
    let pool = state.active_db().await?;
    scan_deleted_platform_copies_with_pool(&pool, agent_ids)
        .await
        .map_err(|e| e.to_string())
}
