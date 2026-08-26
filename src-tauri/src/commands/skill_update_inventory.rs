//! Tauri IPC shells for the Skill Update Inventory (Update Center panel).
//!
//! Business logic lives in `crate::services::central_updates::inventory`.
//! This module keeps the existing command names and payload shapes stable
//! while translating `State<AppState>` into service inputs.

use tauri::{AppHandle, State};

use crate::ipc_error::{public_message_for_code, IpcError, REVIEWED_IPC_ERROR_CODES};
use crate::observability::{
    CommandLogPolicy, OperationBatchId, OperationContext, OperationDefinition, OperationPhase,
    OperationTarget, OperationTargetKind, ReviewedDiagnostic, ReviewedFailure, SafeDetailKey,
    SafeOperationResult,
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
use crate::targets::ActiveTarget;
use crate::AppState;

#[path = "skill_update_inventory_refresh_log.rs"]
#[allow(dead_code)]
mod refresh_log;
use refresh_log::inventory_progress_reporter;

fn operation_definition(command: &'static str) -> OperationDefinition {
    match crate::ipc_registry::command_policy(command)
        .expect("Update Center command must be registered")
        .policy
    {
        CommandLogPolicy::Operation(definition) => definition,
        _ => unreachable!("Update Center mutation must have an operation policy"),
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

fn reviewed_failure(
    error: IpcError,
    category: &'static str,
    phase: OperationPhase,
) -> ReviewedFailure {
    let code = REVIEWED_IPC_ERROR_CODES
        .iter()
        .copied()
        .find(|code| *code == error.safe_code())
        .unwrap_or("internal.unexpected");
    let message = public_message_for_code(code)
        .unwrap_or("The operation failed. See runtime logs for details.");
    ReviewedFailure::new(ReviewedDiagnostic::new(
        code,
        category,
        phase,
        message,
        error.retryable,
    ))
}

fn unexpected_failure(
    definition: OperationDefinition,
    error: impl Into<IpcError>,
) -> ReviewedFailure {
    reviewed_failure(
        error.into(),
        definition.category().as_str(),
        definition.default_phase(),
    )
}

fn central_failure(definition: OperationDefinition, error: CentralUpdatesError) -> ReviewedFailure {
    let category = error.diagnostic_category();
    let phase = error
        .reviewed_operation_failure()
        .map(|(_, phase)| reviewed_phase(phase))
        .unwrap_or(definition.default_phase());
    reviewed_failure(IpcError::from(error.to_ipc_error()), category, phase)
}

fn reviewed_phase(phase: &str) -> OperationPhase {
    match phase {
        "repository_snapshot" | "refresh" => OperationPhase::Network,
        "inventory_persistence" => OperationPhase::Database,
        "decision_apply" => OperationPhase::Filesystem,
        _ => OperationPhase::Command,
    }
}

fn inventory_result(result: &SkillUpdateInventory, summary: &'static str) -> SafeOperationResult {
    let safe = if result.failed_repositories.is_empty() {
        SafeOperationResult::succeeded(summary)
    } else {
        SafeOperationResult::partial("Update inventory completed with repository failures.")
    };
    safe.count(
        SafeDetailKey::AffectedCount,
        (result.updatable.len() + result.remote_added.len() + result.remote_missing.len()) as u64,
    )
    .count(
        SafeDetailKey::FailedCount,
        result.failed_repositories.len() as u64,
    )
}

fn apply_success_count(result: &SkillUpdateApplyResult) -> usize {
    result.updated_skill_ids.len()
        + result.kept_missing_skill_ids.len()
        + result.deleted_skill_ids.len()
        + result.imported_skill_ids.len()
        + result.skipped_additions.len()
        + result.unskipped_additions.len()
        + result.removed_platform_duplicate_paths.len()
        + result.removed_deleted_platform_copy_paths.len()
}

fn apply_result(result: &SkillUpdateApplyResult) -> SafeOperationResult {
    let succeeded = apply_success_count(result) as u64;
    let failed = result.failures.len() as u64;
    let safe = if failed == 0 {
        SafeOperationResult::succeeded("Skill update decisions applied.")
    } else if succeeded == 0 {
        SafeOperationResult::partial("Skill update decisions could not be applied.")
    } else {
        SafeOperationResult::partial("Skill update decisions completed with failures.")
    };
    safe.count(SafeDetailKey::SucceededCount, succeeded)
        .count(SafeDetailKey::FailedCount, failed)
}

fn force_update_result(result: &ForceSkillUpdateResult) -> SafeOperationResult {
    let safe = if result.failed.is_empty() {
        SafeOperationResult::succeeded("Central skills force-updated.")
    } else {
        SafeOperationResult::partial("Central skills force-update completed with failures.")
    };
    safe.count(
        SafeDetailKey::SucceededCount,
        result.overwritten.len() as u64,
    )
    .count(SafeDetailKey::SkippedCount, result.skipped.len() as u64)
    .count(SafeDetailKey::FailedCount, result.failed.len() as u64)
}

fn force_mirror_result(result: &ForceRepositoryMirrorResult) -> SafeOperationResult {
    let succeeded =
        result.overwritten.len() + result.imported.len() + result.deleted.succeeded.len();
    let failed =
        result.deleted.failed.len() + result.failed_repositories.len() + result.failed_items.len();
    let safe = if failed == 0 {
        SafeOperationResult::succeeded("Central repositories force-mirrored.")
    } else {
        SafeOperationResult::partial("Central repository mirror completed with failures.")
    };
    safe.count(SafeDetailKey::SucceededCount, succeeded as u64)
        .count(SafeDetailKey::SkippedCount, result.skipped.len() as u64)
        .count(SafeDetailKey::FailedCount, failed as u64)
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
        "refresh_skill_update_inventory",
        async move {
            let request_context = state.resolve_target_context().await?;
            let active_target = request_context.target().clone();
            let pool = request_context.db().clone();
            let (_, target) = audit_target(&active_target);
            // Only the Local target consults this machine's Skills CLI lock
            // when excluding CLI-owned skills from leftover buckets.
            let cli_lock_protect = crate::services::skills_cli::is_local_target(&active_target);
            let definition = operation_definition("refresh_skill_update_inventory");
            let batch_id = OperationBatchId::parse(&operation_id).unwrap_or_default();
            let progress = inventory_progress_reporter(app, operation_id);
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(target).batch(batch_id),
                |result| inventory_result(result, "Skill update inventory refreshed."),
                || async {
                    let fs = CentralFs::from_active_target(active_target)
                        .await
                        .map_err(|error| unexpected_failure(definition, error.to_string()))?;
                    let auth = github_import::github_direct_auth_from_secret_store(
                        &state.db,
                        state.secrets.as_ref(),
                    )
                    .await
                    .map_err(|error| {
                        unexpected_failure(definition, IpcError::from(error.to_ipc_error()))
                    })?;
                    let client = github_import::github_client()
                        .map_err(|error| unexpected_failure(definition, error.to_string()))?;
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
                    .map_err(|error| central_failure(definition, error))
                },
            )
            .await
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
        "retry_failed_update_repositories",
        async move {
            let request_context = state.resolve_target_context().await?;
            let active_target = request_context.target().clone();
            let pool = request_context.db().clone();
            let (_, target) = audit_target(&active_target);
            let cli_lock_protect = crate::services::skills_cli::is_local_target(&active_target);
            let definition = operation_definition("retry_failed_update_repositories");
            let batch_id = OperationBatchId::parse(&operation_id).unwrap_or_default();
            let progress = inventory_progress_reporter(app, operation_id);
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(target).batch(batch_id),
                |result| inventory_result(result, "Failed update repositories retried."),
                || async {
                    let fs = CentralFs::from_active_target(active_target)
                        .await
                        .map_err(|error| unexpected_failure(definition, error.to_string()))?;
                    let auth = github_import::github_direct_auth_from_secret_store(
                        &state.db,
                        state.secrets.as_ref(),
                    )
                    .await
                    .map_err(|error| {
                        unexpected_failure(definition, IpcError::from(error.to_ipc_error()))
                    })?;
                    let client = github_import::github_client()
                        .map_err(|error| unexpected_failure(definition, error.to_string()))?;
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
                    .map_err(|error| central_failure(definition, error))
                },
            )
            .await
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
        "get_skill_update_inventory",
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
        "clear_skill_update_inventory",
        async move {
            let request_context = state.resolve_target_context().await?;
            let (_, target) = audit_target(request_context.target());
            let pool = request_context.db().clone();
            let definition = operation_definition("clear_skill_update_inventory");
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(target),
                |_| SafeOperationResult::succeeded("Skill update inventory cleared."),
                || async {
                    clear_skill_update_inventory_impl(&pool, scope)
                        .await
                        .map_err(|error| central_failure(definition, error))
                },
            )
            .await
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
        "apply_skill_update_decisions",
        async move {
            let request_context = state.resolve_target_context().await?;
            let active_target = request_context.target().clone();
            let pool = request_context.db().clone();
            let (_, target) = audit_target(&active_target);
            let definition = operation_definition("apply_skill_update_decisions");
            let batch_id = OperationBatchId::parse(&job_id).unwrap_or_default();
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(target).batch(batch_id),
                apply_result,
                || async {
                    let lease = state
                        .central_update_jobs
                        .acquire(&job_id)
                        .map_err(|error| unexpected_failure(definition, error.to_string()))?;
                    let fs = CentralFs::from_active_target(active_target.clone())
                        .await
                        .map_err(|error| unexpected_failure(definition, error.to_string()))?;
                    let auth = github_import::github_direct_auth_from_secret_store(
                        &state.db,
                        state.secrets.as_ref(),
                    )
                    .await
                    .map_err(|error| {
                        unexpected_failure(definition, IpcError::from(error.to_ipc_error()))
                    })?;
                    let client = github_import::github_client()
                        .map_err(|error| unexpected_failure(definition, error.to_string()))?;
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
                    .map_err(|error| central_failure(definition, error))
                },
            )
            .await
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
        "force_update_central_skills",
        async move {
            let request_context = state.resolve_target_context().await?;
            let active_target = request_context.target().clone();
            let pool = request_context.db().clone();
            let (_, target) = audit_target(&active_target);
            let definition = operation_definition("force_update_central_skills");
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(target),
                force_update_result,
                || async {
                    let fs = CentralFs::from_active_target(active_target)
                        .await
                        .map_err(|error| unexpected_failure(definition, error.to_string()))?;
                    let auth = github_import::github_direct_auth_from_secret_store(
                        &state.db,
                        state.secrets.as_ref(),
                    )
                    .await
                    .map_err(|error| {
                        unexpected_failure(definition, IpcError::from(error.to_ipc_error()))
                    })?;
                    let client = github_import::github_client()
                        .map_err(|error| unexpected_failure(definition, error.to_string()))?;
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
                    .map_err(|error| central_failure(definition, error))
                },
            )
            .await
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
        "force_mirror_central_repositories",
        async move {
            let request_context = state.resolve_target_context().await?;
            let active_target = request_context.target().clone();
            let pool = request_context.db().clone();
            let (_, target) = audit_target(&active_target);
            let definition = operation_definition("force_mirror_central_repositories");
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(target),
                force_mirror_result,
                || async {
                    let fs = CentralFs::from_active_target(active_target.clone())
                        .await
                        .map_err(|error| unexpected_failure(definition, error.to_string()))?;
                    let auth = github_import::github_direct_auth_from_secret_store(
                        &state.db,
                        state.secrets.as_ref(),
                    )
                    .await
                    .map_err(|error| {
                        unexpected_failure(definition, IpcError::from(error.to_ipc_error()))
                    })?;
                    let client = github_import::github_client()
                        .map_err(|error| unexpected_failure(definition, error.to_string()))?;
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
                    .map_err(|error| central_failure(definition, error))
                },
            )
            .await
        }
        .await
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{self, DbPool, OperationLogFilter};
    use serde_json::{json, Value};

    fn test_app_state(pool: DbPool) -> AppState {
        AppState {
            db: pool,
            ai_tag_jobs: crate::AiTagJobRegistry::default(),
            central_update_jobs: crate::services::exclusive_job::ExclusiveJobRegistry::new(
                "job.central_update_busy",
                "A Central update job is already running.",
            ),
            central_update_snapshots: crate::CentralUpdateSnapshotCache::default(),
            portable_state_jobs: crate::services::exclusive_job::ExclusiveJobRegistry::new(
                "job.portability_busy",
                "A portability job is already running.",
            ),
            skills_cli_jobs: crate::services::exclusive_job::ExclusiveJobRegistry::new(
                "job.skills_cli_busy",
                "A Skills CLI job is already running.",
            ),
            secrets: std::sync::Arc::new(crate::secrets::MockSecretStore::default()),
            targets: crate::targets::TargetRegistry::default(),
        }
    }

    #[test]
    fn every_owned_mutation_uses_the_registry_operation_definition() {
        for command in [
            "refresh_skill_update_inventory",
            "retry_failed_update_repositories",
            "clear_skill_update_inventory",
            "apply_skill_update_decisions",
            "force_update_central_skills",
            "force_mirror_central_repositories",
            "scan_platform_duplicate_skills",
            "scan_deleted_platform_copies",
        ] {
            assert_eq!(operation_definition(command).action().as_str(), command);
        }
    }

    #[tokio::test]
    async fn failed_boundary_preserves_operation_id_and_drops_raw_seed() {
        let pool = crate::test_support::mem_pool().await;
        let state = test_app_state(pool.clone());
        let definition = operation_definition("refresh_skill_update_inventory");
        let raw_seed =
            "token=secret https://example.invalid C:\\Users\\alice\\private HTTP 301 response";
        let result = crate::observability::run_operation(
            &state,
            definition,
            OperationContext::new(OperationTarget::local()),
            |_| SafeOperationResult::succeeded("Skill update inventory refreshed."),
            || async {
                Err::<(), _>(central_failure(
                    definition,
                    CentralUpdatesError::GithubImport(
                        crate::services::github_import::GithubImportError::Http(
                            raw_seed.to_string(),
                        ),
                    ),
                ))
            },
        )
        .await;
        let operation_error = result.unwrap_err();
        let operation_id = operation_error.correlation_id.clone().unwrap();
        let boundary_error = crate::ipc_error::complete_named_boundary(
            "refresh_skill_update_inventory",
            std::time::Instant::now(),
            Err::<(), _>(operation_error),
        )
        .unwrap_err();
        assert_eq!(
            boundary_error.correlation_id.as_deref(),
            Some(operation_id.as_str())
        );

        let page = db::list_operation_logs(&pool, OperationLogFilter::default())
            .await
            .unwrap();
        assert_eq!(page.total, 1);
        let entry = &page.entries[0];
        assert_eq!(entry.id, operation_id);
        assert_eq!(entry.status, "failed");
        assert_eq!(entry.action, "refresh_skill_update_inventory");
        let details: Value = serde_json::from_str(entry.details_json.as_deref().unwrap()).unwrap();
        assert_eq!(details["operationId"], operation_id);
        assert_eq!(details["errorCode"], "github_import.transport_failed");
        assert_eq!(details["errorCategory"], "github_import.transport_failed");
        assert_eq!(details["phase"], "network");
        let serialized = serde_json::to_string(entry).unwrap();
        for seed in ["secret", "example.invalid", "Users", "301", "response"] {
            assert!(!serialized.contains(seed), "leaked {seed}");
        }
    }

    #[tokio::test]
    async fn partial_inventory_uses_one_row_and_keeps_progress_id_as_batch_only() {
        let pool = crate::test_support::mem_pool().await;
        let state = test_app_state(pool.clone());
        let definition = operation_definition("retry_failed_update_repositories");
        let progress_id = OperationBatchId::new();
        let raw_seed = "token=secret https://example.invalid/C:/Users/private";
        let inventory: SkillUpdateInventory = serde_json::from_value(json!({
            "updatable": [],
            "remoteAdded": [],
            "remoteMissing": [],
            "unsupported": [],
            "platformDuplicates": [],
            "deletedPlatformCopies": [],
            "orphans": [],
            "failedRepositories": [{
                "repositoryId": raw_seed,
                "error": raw_seed,
                "errorCode": "github_import.transport_failed",
                "diagnosticCategory": "github_import.archive_timeout",
                "retry": "retryable"
            }],
            "generatedAt": "2026-08-09T00:00:00Z"
        }))
        .unwrap();

        crate::observability::run_operation(
            &state,
            definition,
            OperationContext::new(OperationTarget::local()).batch(progress_id.clone()),
            |result| inventory_result(result, "Failed update repositories retried."),
            || async { Ok::<_, ReviewedFailure>(inventory) },
        )
        .await
        .unwrap();

        let page = db::list_operation_logs(&pool, OperationLogFilter::default())
            .await
            .unwrap();
        assert_eq!(page.total, 1);
        let entry = &page.entries[0];
        assert_eq!(entry.status, "partial");
        assert_eq!(entry.batch_id.as_deref(), Some(progress_id.as_str()));
        assert_ne!(entry.id, progress_id.as_str());
        let details: Value = serde_json::from_str(entry.details_json.as_deref().unwrap()).unwrap();
        assert_eq!(details["affectedCount"], 0);
        assert_eq!(details["failedCount"], 1);
        assert!(!serde_json::to_string(entry).unwrap().contains(raw_seed));
    }

    #[tokio::test]
    async fn apply_batch_partial_details_are_counts_only() {
        let pool = crate::test_support::mem_pool().await;
        let state = test_app_state(pool.clone());
        let definition = operation_definition("apply_skill_update_decisions");
        let batch_id = OperationBatchId::new();
        let raw_seed = "token=secret https://example.invalid/C:/Users/private";
        let result = SkillUpdateApplyResult {
            updated_skill_ids: vec![raw_seed.to_string()],
            failures: vec![
                crate::services::central_updates::inventory::SkillUpdateApplyFailure::new(
                    "update", raw_seed,
                ),
            ],
            ..SkillUpdateApplyResult::default()
        };

        crate::observability::run_operation(
            &state,
            definition,
            OperationContext::new(OperationTarget::local()).batch(batch_id.clone()),
            apply_result,
            || async { Ok::<_, ReviewedFailure>(result) },
        )
        .await
        .unwrap();

        let page = db::list_operation_logs(&pool, OperationLogFilter::default())
            .await
            .unwrap();
        assert_eq!(page.total, 1);
        let entry = &page.entries[0];
        assert_eq!(entry.status, "partial");
        assert_eq!(entry.batch_id.as_deref(), Some(batch_id.as_str()));
        let details: Value = serde_json::from_str(entry.details_json.as_deref().unwrap()).unwrap();
        assert_eq!(details["succeededCount"], 1);
        assert_eq!(details["failedCount"], 1);
        assert_eq!(details.as_object().unwrap().len(), 4);
        assert!(!serde_json::to_string(entry).unwrap().contains(raw_seed));
    }
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn scan_platform_duplicate_skills(
    state: State<'_, AppState>,
    agent_ids: Option<Vec<String>>,
) -> crate::ipc_error::IpcResult<Vec<PlatformDuplicateGroup>> {
    crate::ipc_boundary!(
        "scan_platform_duplicate_skills",
        async move {
            let request_context = state.resolve_target_context().await?;
            let (_, target) = audit_target(request_context.target());
            let pool = request_context.db().clone();
            let definition = operation_definition("scan_platform_duplicate_skills");
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(target),
                |groups: &Vec<PlatformDuplicateGroup>| {
                    SafeOperationResult::succeeded("Platform duplicate skills scanned.")
                        .count(SafeDetailKey::AffectedCount, groups.len() as u64)
                },
                || async {
                    scan_platform_duplicate_skills_with_pool(&pool, agent_ids)
                        .await
                        .map_err(|error| central_failure(definition, error))
                },
            )
            .await
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
        "scan_deleted_platform_copies",
        async move {
            let request_context = state.resolve_target_context().await?;
            let active_target = request_context.target().clone();
            let pool = request_context.db().clone();
            let (_, target) = audit_target(&active_target);
            // CLI lock protection applies only to the Local target; remote
            // scans never consult this machine's lock file.
            let cli_lock_protect = crate::services::skills_cli::is_local_target(&active_target);
            let definition = operation_definition("scan_deleted_platform_copies");
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(target),
                |groups: &Vec<DeletedPlatformCopyGroup>| {
                    SafeOperationResult::succeeded("Deleted platform copies scanned.")
                        .count(SafeDetailKey::AffectedCount, groups.len() as u64)
                },
                || async {
                    scan_deleted_platform_copies_with_pool(&pool, agent_ids, cli_lock_protect)
                        .await
                        .map_err(|error| central_failure(definition, error))
                },
            )
            .await
        }
        .await
    )
}
