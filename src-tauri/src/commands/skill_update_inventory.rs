//! Tauri IPC shells for the Skill Update Inventory (Update Center panel).
//!
//! Business logic lives in `crate::services::central_updates::inventory`.
//! This module keeps the existing command names and payload shapes stable
//! while translating `State<AppState>` into service inputs.

use serde_json::{json, Value};
use tauri::{AppHandle, State};

use crate::operation_log::{
    target_context_from_active_target, with_operation_log, OperationLogEvent,
    OperationLogTargetContext, OperationSpec,
};
use crate::services::central_updates::inventory::{
    apply_skill_update_decisions_impl, clear_skill_update_inventory_impl,
    force_mirror_central_repositories_impl, force_update_central_skills_impl,
    get_skill_update_inventory_impl_scoped, refresh_skill_update_inventory_impl,
    retry_failed_repositories_impl, scan_deleted_platform_copies_with_pool,
    scan_platform_duplicate_skills_with_pool, DeletedPlatformCopyGroup,
    ForceRepositoryMirrorRequest, ForceRepositoryMirrorResult, ForceSkillUpdateRequest,
    ForceSkillUpdateResult, PlatformDuplicateGroup, SkillRefreshMode, SkillRefreshScope,
    SkillUpdateApplyResult, SkillUpdateDecisions, SkillUpdateInventory,
};
use crate::services::central_updates::{CentralFs, CentralUpdatesError, SnapshotCachePolicy};
use crate::services::github_import;
use crate::AppState;

#[path = "skill_update_inventory_apply_log.rs"]
mod apply_log;
use apply_log::apply_operation_spec;

#[path = "skill_update_inventory_refresh_log.rs"]
mod refresh_log;
use refresh_log::{
    inventory_progress_reporter, refresh_request_details, refresh_result_details,
    retry_refresh_result_details, retry_request_details,
};
#[cfg(test)]
use refresh_log::{
    refresh_failure_diagnostics, REFRESH_FAILURE_CATEGORY_FALLBACK, REFRESH_FAILURE_CODE_FALLBACK,
    REFRESH_RUNTIME_ACTION, RETRY_RUNTIME_ACTION,
};

/// Legacy string-error call sites (apply / force update / force mirror) have no
/// typed domain error to classify, so they report this fixed family instead of
/// leaking the stringified cause.
const UNCLASSIFIED_CATEGORY: &str = "central_updates.unclassified";

#[derive(Debug)]
struct UpdateCommandError {
    ipc_error: String,
    error_code: Option<&'static str>,
    phase: Option<&'static str>,
    category: &'static str,
}

impl UpdateCommandError {
    fn into_inner(self) -> String {
        self.ipc_error
    }

    fn from_central_updates(error: CentralUpdatesError) -> Self {
        let (error_code, phase) = error
            .reviewed_operation_failure()
            .map_or((None, None), |(code, phase)| (Some(code), Some(phase)));
        Self {
            ipc_error: error.to_ipc_error(),
            error_code,
            phase,
            category: error.diagnostic_category(),
        }
    }

    /// Operation Log payload. `errorCategory` is always present so a failure
    /// without a reviewed IPC code is still attributable; `errorCode`/`phase`
    /// are added when the domain classified the failure. Every value is a
    /// `&'static str` literal.
    fn operation_details(&self) -> Value {
        let mut details = json!({ "errorCategory": self.category });
        if let (Some(error_code), Some(phase)) = (self.error_code, self.phase) {
            details["errorCode"] = json!(error_code);
            details["phase"] = json!(phase);
        }
        details
    }
}

impl From<String> for UpdateCommandError {
    fn from(error: String) -> Self {
        Self {
            ipc_error: error,
            error_code: None,
            phase: None,
            category: UNCLASSIFIED_CATEGORY,
        }
    }
}

impl std::fmt::Display for UpdateCommandError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Update Center action failed")
    }
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn refresh_skill_update_inventory(
    app: AppHandle,
    state: State<'_, AppState>,
    scope: SkillRefreshScope,
    operation_id: String,
) -> crate::ipc_error::IpcResult<SkillUpdateInventory> {
    crate::ipc_boundary!(
        async move {
            let request_context = state.resolve_target_context().await?;
            let active_target = request_context.target().clone();
            let pool = request_context.db().clone();
            let target_context = target_context_from_active_target(&active_target);
            // Only the Local target consults this machine's Skills CLI lock
            // when excluding CLI-owned skills from leftover buckets.
            let cli_lock_protect = crate::services::skills_cli::is_local_target(&active_target);
            let request_details = refresh_request_details(&scope);
            let progress = inventory_progress_reporter(app, operation_id);
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
                        cli_lock_protect,
                    )
                    .await
                    .map_err(UpdateCommandError::from_central_updates)
                },
            )
            .await
            .map_err(UpdateCommandError::into_inner)
        }
        .await
    )
}

/// Re-check only the given repositories and merge the result into the
/// inventory stored for `scope`. `mode_override` lets a failed row that needs a
/// keep-or-delete decision be re-checked in incremental mode without changing
/// the panel's own mode.
#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn retry_failed_update_repositories(
    app: AppHandle,
    state: State<'_, AppState>,
    scope: SkillRefreshScope,
    repository_ids: Vec<String>,
    mode_override: Option<SkillRefreshMode>,
    operation_id: String,
) -> crate::ipc_error::IpcResult<SkillUpdateInventory> {
    crate::ipc_boundary!(
        async move {
            let request_context = state.resolve_target_context().await?;
            let active_target = request_context.target().clone();
            let pool = request_context.db().clone();
            let target_context = target_context_from_active_target(&active_target);
            let cli_lock_protect = crate::services::skills_cli::is_local_target(&active_target);
            let request_details = retry_request_details(&scope, &repository_ids, mode_override);
            let progress = inventory_progress_reporter(app, operation_id);
            with_operation_log(
                &state,
                update_operation_spec(
                    target_context,
                    "update_center.retry_repositories",
                    "Retried failed update repositories",
                    "Failed to retry update repositories",
                    request_details,
                    retry_refresh_result_details,
                ),
                || async {
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
                    retry_failed_repositories_impl(
                        &pool,
                        &fs,
                        auth.as_deref(),
                        &client,
                        &state.central_update_snapshots,
                        scope,
                        repository_ids,
                        mode_override,
                        Some(progress),
                        cli_lock_protect,
                    )
                    .await
                    .map_err(UpdateCommandError::from_central_updates)
                },
            )
            .await
            .map_err(UpdateCommandError::into_inner)
        }
        .await
    )
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn get_skill_update_inventory(
    state: State<'_, AppState>,
    scope: Option<SkillRefreshScope>,
) -> crate::ipc_error::IpcResult<SkillUpdateInventory> {
    crate::ipc_boundary!(
        async move {
            let request_context = state.resolve_target_context().await?;
            let active_target = request_context.target().clone();
            let pool = request_context.db().clone();
            let cli_lock_protect = crate::services::skills_cli::is_local_target(&active_target);
            get_skill_update_inventory_impl_scoped(&pool, scope, cli_lock_protect)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn clear_skill_update_inventory(
    state: State<'_, AppState>,
    scope: Option<SkillRefreshScope>,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary!(
        async move {
            let pool = state.active_db().await?;
            clear_skill_update_inventory_impl(&pool, scope)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
pub async fn apply_skill_update_decisions(
    app: AppHandle,
    state: State<'_, AppState>,
    job_id: String,
    decisions: SkillUpdateDecisions,
) -> crate::ipc_error::IpcResult<SkillUpdateApplyResult> {
    crate::ipc_boundary!(
        async move {
            let lease = state
                .central_update_jobs
                .acquire(&job_id)
                .map_err(|e| e.to_string())?;
            let request_context = state.resolve_target_context().await?;
            let active_target = request_context.target().clone();
            let pool = request_context.db().clone();
            let target_context = target_context_from_active_target(&active_target);
            let request_details = apply_request_details(&decisions);
            with_operation_log(
                &state,
                apply_operation_spec(target_context, request_details),
                || async {
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
                        lease.job_id(),
                        &pool,
                        &active_target,
                        &fs,
                        lease.cancel_flag(),
                        auth.as_deref(),
                        &client,
                        &state.central_update_snapshots,
                        decisions,
                    )
                    .await
                    .map_err(|e| UpdateCommandError::from(e.to_string()))
                },
            )
            .await
            .map_err(UpdateCommandError::into_inner)
        }
        .await
    )
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn force_update_central_skills(
    state: State<'_, AppState>,
    request: ForceSkillUpdateRequest,
) -> crate::ipc_error::IpcResult<ForceSkillUpdateResult> {
    crate::ipc_boundary!(
        async move {
            let request_context = state.resolve_target_context().await?;
            let active_target = request_context.target().clone();
            let pool = request_context.db().clone();
            let target_context = target_context_from_active_target(&active_target);
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
                    .map_err(|e| UpdateCommandError::from(e.to_string()))
                },
            )
            .await
            .map_err(UpdateCommandError::into_inner)
        }
        .await
    )
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn force_mirror_central_repositories(
    app: AppHandle,
    state: State<'_, AppState>,
    request: ForceRepositoryMirrorRequest,
) -> crate::ipc_error::IpcResult<ForceRepositoryMirrorResult> {
    crate::ipc_boundary!(
        async move {
            let request_context = state.resolve_target_context().await?;
            let active_target = request_context.target().clone();
            let pool = request_context.db().clone();
            let target_context = target_context_from_active_target(&active_target);
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
                    .map_err(|e| UpdateCommandError::from(e.to_string()))
                },
            )
            .await
            .map_err(UpdateCommandError::into_inner)
        }
        .await
    )
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
        move |error: &UpdateCommandError, duration_ms| {
            // Runtime Log counterpart of the Operation Log row. Without this the
            // "See runtime logs for details" fallback message points at a file
            // that only ever contained the frontend's own generic re-log.
            tracing::error!(
                target: "skillport::update_center",
                action,
                error_code = error.error_code.unwrap_or("none"),
                error_category = error.category,
                phase = error.phase.unwrap_or("none"),
                duration_ms,
                "Update Center action failed"
            );
            update_operation_event(
                action,
                "failed",
                failure_summary,
                merge_details(failure_details, error.operation_details()),
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
        let error = UpdateCommandError::from(original.clone());

        assert_eq!(error.to_string(), "Update Center action failed");
        assert_eq!(error.into_inner(), original);
    }

    #[test]
    fn archive_redirect_error_keeps_only_static_ipc_and_operation_details() {
        let error = UpdateCommandError::from_central_updates(CentralUpdatesError::GithubImport(
            crate::services::github_import::GithubImportError::ArchiveRedirectRejected,
        ));

        assert_eq!(error.to_string(), "Update Center action failed");
        assert_eq!(
            error.operation_details(),
            json!({
                "errorCode": "github_import.archive_redirect_rejected",
                "errorCategory": "github_import.archive_redirect_rejected",
                "phase": "repository_snapshot",
            })
        );
        assert_eq!(
            error.into_inner(),
            "github_import.archive_redirect_rejected:GitHub repository archive redirect was rejected."
        );
    }

    #[test]
    fn github_transport_failure_reaches_the_operation_log_with_a_stable_code() {
        let error = UpdateCommandError::from_central_updates(CentralUpdatesError::GithubImport(
            crate::services::github_import::GithubImportError::Http(
                "Failed to download GitHub repository archive: HTTP 301 https://secret/path"
                    .to_string(),
            ),
        ));

        assert_eq!(
            error.operation_details(),
            json!({
                "errorCode": "github_import.transport_failed",
                "errorCategory": "github_import.transport_failed",
                "phase": "repository_snapshot",
            })
        );
        let details = serde_json::to_string(&error.operation_details()).expect("serialize");
        assert!(!details.contains("secret"));
        assert!(!details.contains("301"));
    }

    #[test]
    fn refresh_result_details_bounds_static_failure_items_and_retry_diagnostics() {
        assert_eq!(REFRESH_RUNTIME_ACTION, "update_center.refresh");
        assert_eq!(RETRY_RUNTIME_ACTION, "update_center.retry_repositories");
        assert_ne!(REFRESH_RUNTIME_ACTION, RETRY_RUNTIME_ACTION);
        let failed_repositories = (0..51)
            .map(|index| {
                json!({
                    "repositoryId": format!("safe-repository-{index:02}"),
                    "error": "token=secret https://example.invalid owner/private ref=secret C:\\Users\\private response body HTTP 503 reqwest",
                    "errorCode": if index % 2 == 0 {
                        "github_import.transport_failed"
                    } else {
                        "github_import.response_invalid"
                    },
                    "diagnosticCategory": if index % 2 == 0 {
                        "github_import.archive_timeout"
                    } else {
                        "github_import.archive_integrity"
                    },
                    "retry": "retryable"
                })
            })
            .collect::<Vec<_>>();
        let inventory: SkillUpdateInventory = serde_json::from_value(json!({
            "updatable": [],
            "remoteAdded": [],
            "remoteMissing": [],
            "unsupported": [],
            "platformDuplicates": [],
            "deletedPlatformCopies": [],
            "orphans": [],
            "failedRepositories": failed_repositories,
            "generatedAt": "2026-08-09T00:00:00Z",
            "snapshotRetryAttempted": 3,
            "snapshotRetryRecovered": 2
        }))
        .unwrap();

        let diagnostics = refresh_failure_diagnostics(&inventory);
        assert_eq!(
            diagnostics.failure_codes,
            vec![
                "github_import.response_invalid".to_string(),
                "github_import.transport_failed".to_string()
            ]
        );
        assert_eq!(
            diagnostics.failure_categories,
            vec![
                "github_import.archive_integrity".to_string(),
                "github_import.archive_timeout".to_string()
            ]
        );
        assert_eq!(diagnostics.failure_items.len(), 50);
        assert_eq!(diagnostics.failure_items_truncated, 1);
        assert_eq!(diagnostics.retry_attempted, 3);
        assert_eq!(diagnostics.retry_recovered, 2);
        assert_eq!(
            diagnostics.failure_items[0],
            json!({
                "repositoryId": "safe-repository-00",
                "errorCode": "github_import.transport_failed",
                "errorCategory": "github_import.archive_timeout"
            })
        );
        let details = serde_json::to_string(&refresh_result_details(&inventory)).unwrap();
        for secret in [
            "token=secret",
            "example.invalid",
            "owner/private",
            "ref=secret",
            "Users\\private",
            "response body",
            "503",
            "reqwest",
        ] {
            assert!(!details.contains(secret), "leaked {secret}");
        }

        let hostile: SkillUpdateInventory = serde_json::from_value(json!({
            "updatable": [],
            "remoteAdded": [],
            "remoteMissing": [],
            "platformDuplicates": [],
            "orphans": [],
            "failedRepositories": [{
                "repositoryId": "https://example.invalid/owner/private?token=secret",
                "error": "raw response body",
                "errorCode": "github_import.transport_failed/HTTP503",
                "diagnosticCategory": "github_import.archive_timeout token=secret",
                "retry": "retryable"
            }],
            "generatedAt": "2026-08-09T00:00:00Z"
        }))
        .unwrap();
        assert_eq!(
            refresh_failure_diagnostics(&hostile).failure_items[0],
            json!({
                "repositoryId": "batch",
                "errorCode": REFRESH_FAILURE_CODE_FALLBACK,
                "errorCategory": REFRESH_FAILURE_CATEGORY_FALLBACK,
            })
        );
    }

    #[test]
    fn unclassified_failures_still_record_a_category_instead_of_nothing() {
        let error = UpdateCommandError::from_central_updates(CentralUpdatesError::Remote(
            "ssh host secret.example failed at /home/alice/private".to_string(),
        ));

        assert_eq!(
            error.operation_details(),
            json!({ "errorCategory": "central_updates.remote" })
        );
        let details = serde_json::to_string(&error.operation_details()).expect("serialize");
        assert!(!details.contains("secret.example"));
        assert!(!details.contains("alice"));
    }

    #[test]
    fn update_target_context_records_real_remote_identity() {
        let target = crate::targets::ActiveTarget::Wsl(Box::new(crate::targets::WslTargetConfig {
            id: "private-target-id".to_string(),
            label: "alice@example.internal".to_string(),
            distribution: "Ubuntu".to_string(),
            remote_home: "/home/alice".to_string(),
            remote_os: "linux".to_string(),
            symlink_enabled: true,
        }));

        let context = target_context_from_active_target(&target);

        assert_eq!(context.kind, "wsl");
        assert_eq!(context.id, "private-target-id");
        assert_eq!(context.label.as_deref(), Some("alice@example.internal"));
    }
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn scan_platform_duplicate_skills(
    state: State<'_, AppState>,
    agent_ids: Option<Vec<String>>,
) -> crate::ipc_error::IpcResult<Vec<PlatformDuplicateGroup>> {
    crate::ipc_boundary!(
        async move {
            let pool = state.active_db().await?;
            scan_platform_duplicate_skills_with_pool(&pool, agent_ids)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn scan_deleted_platform_copies(
    state: State<'_, AppState>,
    agent_ids: Option<Vec<String>>,
) -> crate::ipc_error::IpcResult<Vec<DeletedPlatformCopyGroup>> {
    crate::ipc_boundary!(
        async move {
            let request_context = state.resolve_target_context().await?;
            let active_target = request_context.target().clone();
            let pool = request_context.db().clone();
            // CLI lock protection applies only to the Local target; remote
            // scans never consult this machine's lock file.
            let cli_lock_protect = crate::services::skills_cli::is_local_target(&active_target);
            scan_deleted_platform_copies_with_pool(&pool, agent_ids, cli_lock_protect)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}
