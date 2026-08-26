//! Tauri IPC shells for Central skill update checks, updates, and
//! repository-level sync.
//!
//! Business logic lives in `crate::services::central_updates`. This module
//! keeps the existing command names, payload shapes, and deprecation notes
//! stable while translating `State<AppState>` into service inputs.

use tauri::{AppHandle, State};

use crate::db::SkillUpdateState;
use crate::observability::{
    CommandLogPolicy, OperationBatchId, OperationContext, OperationDefinition, OperationTarget,
    OperationTargetKind, ReviewedDiagnostic, ReviewedFailure, SafeDetailKey, SafeOperationResult,
};
use crate::services::central_updates::{
    apply_central_repository_sync_impl, check_central_repository_sync_impl,
    check_central_skill_updates_impl, get_central_skill_update_states_impl,
    keep_remote_missing_central_skills_impl, update_central_skills_impl, CentralFs,
    CentralRepositorySyncApplyResult, CentralRepositorySyncDecisions, CentralRepositorySyncPreview,
    CentralSkillUpdateResult,
};
use crate::services::github_import;
use crate::AppState;

fn operation_definition(command: &'static str) -> OperationDefinition {
    match crate::ipc_registry::command_policy(command)
        .expect("Central update command must be registered")
        .policy
    {
        CommandLogPolicy::Operation(definition) => definition,
        _ => unreachable!("Central update command must have an operation policy"),
    }
}

fn reviewed_failure(definition: OperationDefinition) -> ReviewedFailure {
    ReviewedFailure::new(ReviewedDiagnostic::unexpected(definition))
}

fn audit_target(target: &crate::targets::ActiveTarget) -> OperationTarget {
    match target {
        crate::targets::ActiveTarget::Local => OperationTarget::local(),
        crate::targets::ActiveTarget::Ssh(target) => {
            OperationTarget::new(OperationTargetKind::Ssh, &target.id)
        }
        crate::targets::ActiveTarget::Wsl(target) => {
            OperationTarget::new(OperationTargetKind::Wsl, &target.id)
        }
    }
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn get_central_skill_update_states(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<Vec<SkillUpdateState>> {
    crate::ipc_boundary!(
        "get_central_skill_update_states",
        async move {
            let pool = state.active_db().await?;
            get_central_skill_update_states_impl(&pool)
                .await
                .map_err(|e| e.to_string())
        }
        .await
    )
}

#[tauri::command]
#[deprecated(
    note = "Use refresh_skill_update_inventory + apply_skill_update_decisions instead. See plans/update-mechanism-overhaul-plan.md."
)]
pub async fn check_central_skill_updates(
    app: AppHandle,
    state: State<'_, AppState>,
    job_id: String,
    skill_ids: Option<Vec<String>>,
) -> crate::ipc_error::IpcResult<Vec<SkillUpdateState>> {
    crate::ipc_boundary!(
        "check_central_skill_updates",
        async move {
            let request_context = state.resolve_target_context().await?;
            let pool = request_context.db().clone();
            let active_target = request_context.target().clone();
            let definition = operation_definition("check_central_skill_updates");
            let batch_id = OperationBatchId::parse(&job_id).unwrap_or_default();
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(audit_target(&active_target)).batch(batch_id),
                |states: &Vec<SkillUpdateState>| {
                    SafeOperationResult::succeeded("Central skill update check completed.")
                        .count(SafeDetailKey::AffectedCount, states.len() as u64)
                },
                || async {
                    let lease = state
                        .central_update_jobs
                        .acquire(&job_id)
                        .map_err(|_| reviewed_failure(definition))?;
                    let fs = CentralFs::from_active_target(active_target)
                        .await
                        .map_err(|_| reviewed_failure(definition))?;
                    let auth = github_import::github_direct_auth_from_secret_store(
                        &state.db,
                        state.secrets.as_ref(),
                    )
                    .await
                    .map_err(|_| reviewed_failure(definition))?;
                    let client =
                        github_import::github_client().map_err(|_| reviewed_failure(definition))?;
                    check_central_skill_updates_impl(
                        Some(&app),
                        lease.job_id(),
                        &pool,
                        &fs,
                        lease.cancel_flag(),
                        auth.as_deref(),
                        &client,
                        &state.central_update_snapshots,
                        skill_ids,
                    )
                    .await
                    .map_err(|_| reviewed_failure(definition))
                },
            )
            .await
        }
        .await
    )
}

#[tauri::command]
#[deprecated(
    note = "Use refresh_skill_update_inventory with scope=Repositories instead. See plans/update-mechanism-overhaul-plan.md."
)]
pub async fn check_central_repository_sync(
    app: AppHandle,
    state: State<'_, AppState>,
    job_id: String,
    repository_ids: Vec<String>,
    skill_ids: Option<Vec<String>>,
) -> crate::ipc_error::IpcResult<CentralRepositorySyncPreview> {
    crate::ipc_boundary!(
        "check_central_repository_sync",
        async move {
            let request_context = state.resolve_target_context().await?;
            let pool = request_context.db().clone();
            let active_target = request_context.target().clone();
            let definition = operation_definition("check_central_repository_sync");
            let batch_id = OperationBatchId::parse(&job_id).unwrap_or_default();
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(audit_target(&active_target)).batch(batch_id),
                |preview: &CentralRepositorySyncPreview| {
                    let result = if preview.failed_repositories.is_empty() {
                        SafeOperationResult::succeeded("Central repository check completed.")
                    } else {
                        SafeOperationResult::partial(
                            "Central repository check completed with failures.",
                        )
                    };
                    result
                        .count(
                            SafeDetailKey::AffectedCount,
                            preview.repositories.len() as u64,
                        )
                        .count(
                            SafeDetailKey::FailedCount,
                            preview.failed_repositories.len() as u64,
                        )
                },
                || async {
                    let lease = state
                        .central_update_jobs
                        .acquire(&job_id)
                        .map_err(|_| reviewed_failure(definition))?;
                    let fs = CentralFs::from_active_target(active_target)
                        .await
                        .map_err(|_| reviewed_failure(definition))?;
                    let auth = github_import::github_direct_auth_from_secret_store(
                        &state.db,
                        state.secrets.as_ref(),
                    )
                    .await
                    .map_err(|_| reviewed_failure(definition))?;
                    let client =
                        github_import::github_client().map_err(|_| reviewed_failure(definition))?;
                    check_central_repository_sync_impl(
                        Some(&app),
                        lease.job_id(),
                        &pool,
                        &fs,
                        lease.cancel_flag(),
                        auth.as_deref(),
                        &client,
                        &state.central_update_snapshots,
                        repository_ids,
                        skill_ids,
                    )
                    .await
                    .map_err(|_| reviewed_failure(definition))
                },
            )
            .await
        }
        .await
    )
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
#[deprecated(
    note = "Use apply_skill_update_decisions instead. See plans/update-mechanism-overhaul-plan.md."
)]
pub async fn apply_central_repository_sync(
    app: AppHandle,
    state: State<'_, AppState>,
    decisions: CentralRepositorySyncDecisions,
) -> crate::ipc_error::IpcResult<CentralRepositorySyncApplyResult> {
    crate::ipc_boundary!(
        "apply_central_repository_sync",
        async move {
            let request_context = state.resolve_target_context().await?;
            let pool = request_context.db().clone();
            let active_target = request_context.target().clone();
            let definition = operation_definition("apply_central_repository_sync");
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(audit_target(&active_target)),
                |result: &CentralRepositorySyncApplyResult| {
                    let safe_result = if result.failed_repositories.is_empty() {
                        SafeOperationResult::succeeded("Central repository changes applied.")
                    } else {
                        SafeOperationResult::partial(
                            "Central repository changes completed with failures.",
                        )
                    };
                    safe_result
                        .count(
                            SafeDetailKey::SucceededCount,
                            result.import_results.len() as u64,
                        )
                        .count(
                            SafeDetailKey::FailedCount,
                            result.failed_repositories.len() as u64,
                        )
                },
                || async {
                    let auth = github_import::github_direct_auth_from_secret_store(
                        &state.db,
                        state.secrets.as_ref(),
                    )
                    .await
                    .map_err(|_| reviewed_failure(definition))?;
                    apply_central_repository_sync_impl(
                        Some(&app),
                        &pool,
                        &active_target,
                        auth.as_deref(),
                        decisions,
                    )
                    .await
                    .map_err(|_| reviewed_failure(definition))
                },
            )
            .await
        }
        .await
    )
}

#[tauri::command]
#[deprecated(
    note = "Use apply_skill_update_decisions with `updates` field instead. See plans/update-mechanism-overhaul-plan.md."
)]
pub async fn update_central_skills(
    app: AppHandle,
    state: State<'_, AppState>,
    job_id: String,
    skill_ids: Vec<String>,
) -> crate::ipc_error::IpcResult<CentralSkillUpdateResult> {
    crate::ipc_boundary!(
        "update_central_skills",
        async move {
            let request_context = state.resolve_target_context().await?;
            let pool = request_context.db().clone();
            let active_target = request_context.target().clone();
            let definition = operation_definition("update_central_skills");
            let batch_id = OperationBatchId::parse(&job_id).unwrap_or_default();
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(audit_target(&active_target)).batch(batch_id),
                |result: &CentralSkillUpdateResult| {
                    let safe_result = if result.failed.is_empty() {
                        SafeOperationResult::succeeded("Central skills updated.")
                    } else {
                        SafeOperationResult::partial(
                            "Central skill update completed with failures.",
                        )
                    };
                    safe_result
                        .count(SafeDetailKey::SucceededCount, result.succeeded.len() as u64)
                        .count(SafeDetailKey::FailedCount, result.failed.len() as u64)
                        .count(SafeDetailKey::SkippedCount, result.skipped.len() as u64)
                },
                || async {
                    let lease = state
                        .central_update_jobs
                        .acquire(&job_id)
                        .map_err(|_| reviewed_failure(definition))?;
                    let fs = CentralFs::from_active_target(active_target)
                        .await
                        .map_err(|_| reviewed_failure(definition))?;
                    let auth = github_import::github_direct_auth_from_secret_store(
                        &state.db,
                        state.secrets.as_ref(),
                    )
                    .await
                    .map_err(|_| reviewed_failure(definition))?;
                    let client =
                        github_import::github_client().map_err(|_| reviewed_failure(definition))?;
                    update_central_skills_impl(
                        Some(&app),
                        lease.job_id(),
                        &pool,
                        &fs,
                        lease.cancel_flag(),
                        auth.as_deref(),
                        &client,
                        &state.central_update_snapshots,
                        skill_ids,
                    )
                    .await
                    .map_err(|_| reviewed_failure(definition))
                },
            )
            .await
        }
        .await
    )
}

#[tauri::command]
pub async fn cancel_central_skill_updates(
    state: State<'_, AppState>,
    job_id: String,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary!(
        "cancel_central_skill_updates",
        async move {
            let definition = operation_definition("cancel_central_skill_updates");
            let batch_id = OperationBatchId::parse(&job_id).unwrap_or_default();
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(OperationTarget::local()).batch(batch_id),
                |_| SafeOperationResult::succeeded("Central update cancellation requested."),
                || async {
                    state
                        .central_update_jobs
                        .cancel(&job_id)
                        .map_err(|_| reviewed_failure(definition))?;
                    Ok::<(), ReviewedFailure>(())
                },
            )
            .await
        }
        .await
    )
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
#[deprecated(
    note = "Use apply_skill_update_decisions with `keep_missing` field instead. See plans/update-mechanism-overhaul-plan.md."
)]
pub async fn keep_remote_missing_central_skills(
    state: State<'_, AppState>,
    skill_ids: Vec<String>,
) -> crate::ipc_error::IpcResult<Vec<String>> {
    crate::ipc_boundary!(
        "keep_remote_missing_central_skills",
        async move {
            let request_context = state.resolve_target_context().await?;
            let pool = request_context.db().clone();
            let definition = operation_definition("keep_remote_missing_central_skills");
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(audit_target(request_context.target())),
                |skills: &Vec<String>| {
                    SafeOperationResult::succeeded("Remote-missing Central skills retained.")
                        .count(SafeDetailKey::AffectedCount, skills.len() as u64)
                },
                || async {
                    keep_remote_missing_central_skills_impl(&pool, &skill_ids)
                        .await
                        .map_err(|_| reviewed_failure(definition))
                },
            )
            .await
        }
        .await
    )
}
