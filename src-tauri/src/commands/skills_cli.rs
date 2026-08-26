//! Tauri IPC shells for Skills CLI global (`-g`) management.
//!
//! Every command resolves a request-scoped [`TargetContext`] first and
//! rejects non-Local targets before spawning anything or reading local
//! state. Business logic lives in `crate::services::skills_cli`; this module
//! owns the Local gate, exclusive job lease, operation log, and the typed
//! error envelope.

use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};

use crate::ipc_error::{public_message_for_code, IpcError, REVIEWED_IPC_ERROR_CODES};
use crate::observability::{
    CommandLogPolicy, OperationContext, OperationDefinition, OperationSubjectKind, OperationTarget,
    ReviewedDiagnostic, ReviewedFailure, SafeDetailKey, SafeIdentifier, SafeOperationResult,
};
use crate::services::exclusive_job::ExclusiveJobError;
use crate::services::github_import;
use crate::services::skills_cli as domain;
use crate::services::skills_cli::updates::{
    apply_updates, load_update_inventory_for_pool, retry_update_recovery,
    verify_update_baseline_at, ProductionSkillsCliGithub, SkillsCliApplyRecoveryResult,
    SkillsCliApplyResult, SkillsCliApplyUpdateRequest, SkillsCliUpdateInventory,
    SkillsCliUpdateProgress, UpdateProgressEmitter, UPDATE_PROGRESS_EVENT,
};
use crate::services::skills_cli::{
    NodeProcessRunner, SkillsCliAddResult, SkillsCliDoctorReport, SkillsCliError,
    SkillsCliGlobalSnapshot, SkillsCliInstallTarget, SkillsCliPlacement, SkillsCliRemovePlan,
    SkillsCliRemoveResult, SkillsCliRunner, SkillsCliSkillDoc, SkillsCliSourcePreview,
};
use crate::AppState;

fn to_ipc_error(error: &SkillsCliError) -> IpcError {
    let code = error.ipc_code();
    let message = public_message_for_code(code).unwrap_or(
        // Only the internal family falls through; keep its fixed sentence.
        "The operation failed. See runtime logs for details.",
    );
    IpcError::new(code, message, error.retryable())
}

fn job_lease_error(error: ExclusiveJobError) -> IpcError {
    match error {
        ExclusiveJobError::InvalidId => {
            IpcError::new("job.invalid_id", "The job identifier is invalid.", false)
        }
        ExclusiveJobError::Busy { .. } => IpcError::new(
            "skills_cli.busy",
            "Another skill operation is using this target.",
            true,
        ),
        ExclusiveJobError::IdMismatch => IpcError::new(
            "job.id_mismatch",
            "The cancellation request does not match the active job.",
            false,
        ),
        ExclusiveJobError::RegistryUnavailable => IpcError::new(
            "job.registry_unavailable",
            "The job registry is unavailable.",
            false,
        ),
    }
}

fn domain_runner() -> Arc<dyn SkillsCliRunner> {
    Arc::new(NodeProcessRunner)
}

fn operation_definition(command: &'static str) -> OperationDefinition {
    match crate::ipc_registry::command_policy(command)
        .expect("Skills CLI command must be registered")
        .policy
    {
        CommandLogPolicy::Operation(definition) => definition,
        _ => panic!("Skills CLI mutation must use Operation policy"),
    }
}

fn reviewed_failure(definition: OperationDefinition, error: IpcError) -> ReviewedFailure {
    let code = REVIEWED_IPC_ERROR_CODES
        .iter()
        .copied()
        .find(|code| *code == error.safe_code())
        .unwrap_or("internal.unexpected");
    let message = public_message_for_code(code)
        .unwrap_or("The operation failed. See runtime logs for details.");
    ReviewedFailure::new(ReviewedDiagnostic::new(
        code,
        definition.category().as_str(),
        definition.default_phase(),
        message,
        error.retryable,
    ))
}

fn skills_cli_failure(definition: OperationDefinition, error: &SkillsCliError) -> ReviewedFailure {
    reviewed_failure(definition, to_ipc_error(error))
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn skills_cli_doctor(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<SkillsCliDoctorReport> {
    crate::ipc_boundary_async!("skills_cli_doctor", {
        let context = state.resolve_target_context().await?;
        domain::ensure_local_target(context.target()).map_err(|error| to_ipc_error(&error))?;
        domain::doctor(domain_runner().as_ref())
            .await
            .map_err(|error| to_ipc_error(&error))
    })
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn skills_cli_list_global(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<SkillsCliGlobalSnapshot> {
    crate::ipc_boundary_async!("skills_cli_list_global", {
        let context = state.resolve_target_context().await?;
        domain::ensure_local_target(context.target()).map_err(|error| to_ipc_error(&error))?;
        let pool = context.db().clone();
        domain::list_global(&pool)
            .await
            .map_err(|error| to_ipc_error(&error))
    })
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn skills_cli_install_targets(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<Vec<SkillsCliInstallTarget>> {
    crate::ipc_boundary_async!("skills_cli_install_targets", {
        let context = state.resolve_target_context().await?;
        let pool = context.db().clone();
        domain::ensure_local_target(context.target()).map_err(|error| to_ipc_error(&error))?;
        let definition = operation_definition("skills_cli_install_targets");
        crate::observability::run_operation(
            &state,
            definition,
            OperationContext::new(OperationTarget::local()),
            |targets: &Vec<SkillsCliInstallTarget>| {
                SafeOperationResult::succeeded("Refreshed Skills CLI install targets.")
                    .count(SafeDetailKey::AffectedCount, targets.len() as u64)
            },
            || async move {
                domain::install_targets(&pool)
                    .await
                    .map_err(|error| skills_cli_failure(definition, &error))
            },
        )
        .await
    })
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn skills_cli_preview_source(
    state: State<'_, AppState>,
    source: String,
) -> crate::ipc_error::IpcResult<SkillsCliSourcePreview> {
    crate::ipc_boundary_async!("skills_cli_preview_source", {
        let context = state.resolve_target_context().await?;
        domain::ensure_local_target(context.target()).map_err(|error| to_ipc_error(&error))?;
        domain::preview_source(domain_runner().as_ref(), &source)
            .await
            .map_err(|error| to_ipc_error(&error))
    })
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn skills_cli_add_global(
    state: State<'_, AppState>,
    job_id: String,
    source: String,
    skill_names: Vec<String>,
    skillport_agent_ids: Vec<String>,
) -> crate::ipc_error::IpcResult<SkillsCliAddResult> {
    crate::ipc_boundary_async!("skills_cli_add_global", {
        let lease = state
            .skills_cli_jobs
            .acquire(&job_id)
            .map_err(job_lease_error)?;
        let context = state.resolve_target_context().await?;
        let active_target = context.target().clone();
        domain::ensure_local_target(&active_target).map_err(|error| to_ipc_error(&error))?;
        let definition = operation_definition("skills_cli_add_global");
        let requested_skills = skill_names.len() as u64;
        let requested_platforms = skillport_agent_ids.len() as u64;
        crate::observability::run_operation(
            &state,
            definition,
            OperationContext::new(OperationTarget::local()),
            |result: &SkillsCliAddResult| {
                SafeOperationResult::succeeded("Installed Skills CLI global skills.")
                    .count(SafeDetailKey::RequestedCount, requested_skills)
                    .count(
                        SafeDetailKey::SucceededCount,
                        result.installed_skills as u64,
                    )
                    .count(SafeDetailKey::AffectedCount, requested_platforms)
            },
            || async move {
                domain::add_global(
                    domain_runner().as_ref(),
                    &source,
                    skill_names,
                    skillport_agent_ids,
                    Some(lease.cancel_flag()),
                )
                .await
                .map_err(|error| skills_cli_failure(definition, &error))
            },
        )
        .await
    })
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn skills_cli_remove_global(
    state: State<'_, AppState>,
    job_id: String,
    skill_name: String,
) -> crate::ipc_error::IpcResult<SkillsCliRemoveResult> {
    crate::ipc_boundary_async!("skills_cli_remove_global", {
        let lease = state
            .skills_cli_jobs
            .acquire(&job_id)
            .map_err(job_lease_error)?;
        let context = state.resolve_target_context().await?;
        let active_target = context.target().clone();
        let pool = context.db().clone();
        domain::ensure_local_target(&active_target).map_err(|error| to_ipc_error(&error))?;
        let definition = operation_definition("skills_cli_remove_global");
        let operation_context = OperationContext::new(OperationTarget::local()).subject(
            OperationSubjectKind::Skill,
            SafeIdentifier::new(&skill_name),
        );
        crate::observability::run_operation(
            &state,
            definition,
            operation_context,
            |result: &SkillsCliRemoveResult| {
                SafeOperationResult::succeeded("Removed a Skills CLI global skill.")
                    .flag(SafeDetailKey::Changed, result.removed_canonical)
                    .count(
                        SafeDetailKey::AffectedCount,
                        result.removed_managed_agent_ids.len() as u64,
                    )
                    .count(
                        SafeDetailKey::SkippedCount,
                        result.retained_direct_copy_agent_ids.len() as u64,
                    )
            },
            || async move {
                domain::remove_global(&pool, &skill_name, Some(lease.cancel_flag()))
                    .await
                    .map_err(|error| skills_cli_failure(definition, &error))
            },
        )
        .await
    })
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn skills_cli_read_skill_md(
    state: State<'_, AppState>,
    skill_name: String,
) -> crate::ipc_error::IpcResult<SkillsCliSkillDoc> {
    crate::ipc_boundary_async!("skills_cli_read_skill_md", {
        let context = state.resolve_target_context().await?;
        domain::ensure_local_target(context.target()).map_err(|error| to_ipc_error(&error))?;
        domain::read_skill_md(&skill_name)
            .await
            .map_err(|error| to_ipc_error(&error))
    })
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn skills_cli_reveal_skill_folder(
    state: State<'_, AppState>,
    skill_name: String,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary_async!("skills_cli_reveal_skill_folder", {
        let context = state.resolve_target_context().await?;
        domain::ensure_local_target(context.target()).map_err(|error| to_ipc_error(&error))?;
        let definition = operation_definition("skills_cli_reveal_skill_folder");
        let operation_context = OperationContext::new(OperationTarget::local()).subject(
            OperationSubjectKind::Skill,
            SafeIdentifier::new(&skill_name),
        );
        crate::observability::run_operation(
            &state,
            definition,
            operation_context,
            |_| SafeOperationResult::succeeded("Revealed a Skills CLI skill folder."),
            || async move {
                domain::reveal_skill_folder(&skill_name)
                    .map_err(|error| skills_cli_failure(definition, &error))
            },
        )
        .await
    })
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn skills_cli_link_platform(
    state: State<'_, AppState>,
    job_id: String,
    skill_name: String,
    skillport_agent_id: String,
) -> crate::ipc_error::IpcResult<SkillsCliPlacement> {
    crate::ipc_boundary_async!("skills_cli_link_platform", {
        let lease = state
            .skills_cli_jobs
            .acquire(&job_id)
            .map_err(job_lease_error)?;
        let context = state.resolve_target_context().await?;
        let active_target = context.target().clone();
        let pool = context.db().clone();
        domain::ensure_local_target(&active_target).map_err(|error| to_ipc_error(&error))?;

        let definition = operation_definition("skills_cli_link_platform");
        let operation_context = OperationContext::new(OperationTarget::local()).subject(
            OperationSubjectKind::Skill,
            SafeIdentifier::new(&skill_name),
        );
        let logged_agent = SafeIdentifier::new(&skillport_agent_id);
        crate::observability::run_operation(
            &state,
            definition,
            operation_context,
            move |_| {
                SafeOperationResult::succeeded("Linked a Skills CLI platform placement.")
                    .flag(SafeDetailKey::Changed, true)
                    .identifier(SafeDetailKey::Identifier, logged_agent)
            },
            || async move {
                domain::link_platform(
                    &pool,
                    &skill_name,
                    &skillport_agent_id,
                    Some(lease.cancel_flag()),
                )
                .await
                .map_err(|error| skills_cli_failure(definition, &error))
            },
        )
        .await
    })
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn skills_cli_unlink_platform(
    state: State<'_, AppState>,
    job_id: String,
    skill_name: String,
    skillport_agent_id: String,
) -> crate::ipc_error::IpcResult<SkillsCliPlacement> {
    crate::ipc_boundary_async!("skills_cli_unlink_platform", {
        let lease = state
            .skills_cli_jobs
            .acquire(&job_id)
            .map_err(job_lease_error)?;
        let context = state.resolve_target_context().await?;
        let active_target = context.target().clone();
        let pool = context.db().clone();
        domain::ensure_local_target(&active_target).map_err(|error| to_ipc_error(&error))?;

        let definition = operation_definition("skills_cli_unlink_platform");
        let operation_context = OperationContext::new(OperationTarget::local()).subject(
            OperationSubjectKind::Skill,
            SafeIdentifier::new(&skill_name),
        );
        let logged_agent = SafeIdentifier::new(&skillport_agent_id);
        crate::observability::run_operation(
            &state,
            definition,
            operation_context,
            move |_| {
                SafeOperationResult::succeeded("Unlinked a Skills CLI platform placement.")
                    .flag(SafeDetailKey::Changed, true)
                    .identifier(SafeDetailKey::Identifier, logged_agent)
            },
            || async move {
                domain::unlink_platform(
                    &pool,
                    &skill_name,
                    &skillport_agent_id,
                    Some(lease.cancel_flag()),
                )
                .await
                .map_err(|error| skills_cli_failure(definition, &error))
            },
        )
        .await
    })
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn skills_cli_preview_remove_global(
    state: State<'_, AppState>,
    skill_name: String,
) -> crate::ipc_error::IpcResult<SkillsCliRemovePlan> {
    crate::ipc_boundary_async!("skills_cli_preview_remove_global", {
        let context = state.resolve_target_context().await?;
        domain::ensure_local_target(context.target()).map_err(|error| to_ipc_error(&error))?;
        let pool = context.db().clone();
        domain::preview_remove_global(&pool, &skill_name)
            .await
            .map_err(|error| to_ipc_error(&error))
    })
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn skills_cli_export_inventory(
    state: State<'_, AppState>,
    path: String,
    json: String,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary_async!("skills_cli_export_inventory", {
        let context = state.resolve_target_context().await?;
        domain::ensure_local_target(context.target()).map_err(|error| to_ipc_error(&error))?;
        let definition = operation_definition("skills_cli_export_inventory");
        crate::observability::run_operation(
            &state,
            definition,
            OperationContext::new(OperationTarget::local()),
            |_| SafeOperationResult::succeeded("Exported the Skills CLI inventory."),
            || async move {
                domain::export_inventory(std::path::PathBuf::from(path), json)
                    .await
                    .map_err(|error| skills_cli_failure(definition, &error))
            },
        )
        .await
    })
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn cancel_skills_cli_job(
    state: State<'_, AppState>,
    job_id: String,
) -> crate::ipc_error::IpcResult<bool> {
    crate::ipc_boundary_async!("cancel_skills_cli_job", {
        let context = state.resolve_target_context().await?;
        domain::ensure_local_target(context.target()).map_err(|error| to_ipc_error(&error))?;
        let definition = operation_definition("cancel_skills_cli_job");
        crate::observability::run_operation(
            &state,
            definition,
            OperationContext::new(OperationTarget::local()),
            |changed: &bool| {
                SafeOperationResult::succeeded("Requested Skills CLI cancellation.")
                    .flag(SafeDetailKey::Changed, *changed)
            },
            || async {
                state
                    .skills_cli_jobs
                    .cancel(&job_id)
                    .map_err(|error| reviewed_failure(definition, job_lease_error(error)))
            },
        )
        .await
    })
}

struct AppUpdateProgress {
    app: AppHandle,
}

impl UpdateProgressEmitter for AppUpdateProgress {
    fn emit_update_progress(&self, payload: &SkillsCliUpdateProgress) {
        let _ = self.app.emit(UPDATE_PROGRESS_EVENT, payload);
    }
}

async fn github_from_state(state: &AppState) -> Result<ProductionSkillsCliGithub, IpcError> {
    let auth =
        github_import::github_direct_auth_from_secret_store(&state.db, state.secrets.as_ref())
            .await
            .map_err(|_| to_ipc_error(&SkillsCliError::UpdateCheckFailed))?;
    let client = github_import::github_client()
        .map_err(|_| to_ipc_error(&SkillsCliError::UpdateCheckFailed))?;
    Ok(ProductionSkillsCliGithub { client, auth })
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn skills_cli_check_updates(
    app: AppHandle,
    state: State<'_, AppState>,
    job_id: String,
) -> crate::ipc_error::IpcResult<SkillsCliUpdateInventory> {
    crate::ipc_boundary_async!("skills_cli_check_updates", {
        let lease = state
            .skills_cli_jobs
            .acquire(&job_id)
            .map_err(job_lease_error)?;
        let context = state.resolve_target_context().await?;
        let active_target = context.target().clone();
        let pool = context.db().clone();
        domain::ensure_local_target(&active_target).map_err(|error| to_ipc_error(&error))?;
        let github = github_from_state(&state).await?;
        let home = crate::paths::resolve_home_dir();
        let definition = operation_definition("skills_cli_check_updates");
        crate::observability::run_operation(
            &state,
            definition,
            OperationContext::new(OperationTarget::local()),
            |inventory: &SkillsCliUpdateInventory| {
                SafeOperationResult::succeeded("Checked Skills CLI updates.")
                    .count(SafeDetailKey::AffectedCount, inventory.skills.len() as u64)
            },
            || async move {
                domain::updates::check_updates_at(
                    &pool,
                    &crate::paths::universal_skills_dir(),
                    &domain::skills_cli_lock_path(&home),
                    &github,
                    &AppUpdateProgress { app },
                    &job_id,
                    Some(lease.cancel_flag()),
                )
                .await
                .map_err(|error| skills_cli_failure(definition, &error))
            },
        )
        .await
    })
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn skills_cli_update_inventory(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<SkillsCliUpdateInventory> {
    crate::ipc_boundary_async!("skills_cli_update_inventory", {
        let context = state.resolve_target_context().await?;
        domain::ensure_local_target(context.target()).map_err(|error| to_ipc_error(&error))?;
        load_update_inventory_for_pool(context.db())
            .await
            .map_err(|error| to_ipc_error(&error))
    })
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn skills_cli_verify_update_baseline(
    state: State<'_, AppState>,
    job_id: String,
    skill_names: Vec<String>,
) -> crate::ipc_error::IpcResult<SkillsCliUpdateInventory> {
    crate::ipc_boundary_async!("skills_cli_verify_update_baseline", {
        let lease = state
            .skills_cli_jobs
            .acquire(&job_id)
            .map_err(job_lease_error)?;
        let context = state.resolve_target_context().await?;
        domain::ensure_local_target(context.target()).map_err(|error| to_ipc_error(&error))?;
        let home = crate::paths::resolve_home_dir();
        let pool = context.db().clone();
        let definition = operation_definition("skills_cli_verify_update_baseline");
        let requested = skill_names.len() as u64;
        crate::observability::run_operation(
            &state,
            definition,
            OperationContext::new(OperationTarget::local()),
            |inventory: &SkillsCliUpdateInventory| {
                SafeOperationResult::succeeded("Verified Skills CLI update baselines.")
                    .count(SafeDetailKey::RequestedCount, requested)
                    .count(SafeDetailKey::AffectedCount, inventory.skills.len() as u64)
            },
            || async move {
                verify_update_baseline_at(
                    &pool,
                    &crate::paths::universal_skills_dir(),
                    &domain::skills_cli_lock_path(&home),
                    &skill_names,
                    Some(lease.cancel_flag()),
                )
                .await
                .map_err(|error| skills_cli_failure(definition, &error))
            },
        )
        .await
    })
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn skills_cli_apply_updates(
    app: AppHandle,
    state: State<'_, AppState>,
    request: SkillsCliApplyUpdateRequest,
) -> crate::ipc_error::IpcResult<SkillsCliApplyResult> {
    crate::ipc_boundary_async!("skills_cli_apply_updates", {
        let lease = state
            .skills_cli_jobs
            .acquire(&request.job_id)
            .map_err(job_lease_error)?;
        let context = state.resolve_target_context().await?;
        let active_target = context.target().clone();
        let pool = context.db().clone();
        domain::ensure_local_target(&active_target).map_err(|error| to_ipc_error(&error))?;
        let github = github_from_state(&state).await?;
        let definition = operation_definition("skills_cli_apply_updates");
        let requested = request.selections.len() as u64;
        crate::observability::run_operation(
            &state,
            definition,
            OperationContext::new(OperationTarget::local()),
            |result: &SkillsCliApplyResult| {
                SafeOperationResult::succeeded("Applied Skills CLI updates.")
                    .count(SafeDetailKey::RequestedCount, requested)
                    .count(
                        SafeDetailKey::SucceededCount,
                        result.applied_skill_names.len() as u64,
                    )
            },
            || async move {
                apply_updates(
                    &pool,
                    &github,
                    &AppUpdateProgress { app },
                    &request,
                    Some(lease.cancel_flag()),
                )
                .await
                .map_err(|error| skills_cli_failure(definition, &error))
            },
        )
        .await
    })
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn skills_cli_retry_update_recovery(
    state: State<'_, AppState>,
    job_id: String,
    operation_id: String,
) -> crate::ipc_error::IpcResult<SkillsCliApplyRecoveryResult> {
    crate::ipc_boundary_async!("skills_cli_retry_update_recovery", {
        let lease = state
            .skills_cli_jobs
            .acquire(&job_id)
            .map_err(job_lease_error)?;
        let context = state.resolve_target_context().await?;
        domain::ensure_local_target(context.target()).map_err(|error| to_ipc_error(&error))?;
        let pool = context.db().clone();
        let home = crate::paths::resolve_home_dir();
        let definition = operation_definition("skills_cli_retry_update_recovery");
        let operation_context = OperationContext::new(OperationTarget::local()).subject(
            OperationSubjectKind::Operation,
            SafeIdentifier::new(&operation_id),
        );
        crate::observability::run_operation(
            &state,
            definition,
            operation_context,
            |_| SafeOperationResult::succeeded("Retried Skills CLI update recovery."),
            || async move {
                retry_update_recovery(
                    &pool,
                    &operation_id,
                    &crate::paths::universal_skills_dir(),
                    &domain::skills_cli_lock_path(&home),
                    &crate::paths::skills_cli_update_recovery_dir(),
                    Some(lease.cancel_flag()),
                )
                .await
                .map_err(|error| skills_cli_failure(definition, &error))
            },
        )
        .await
    })
}

#[cfg(test)]
mod tests {
    use super::to_ipc_error;
    use crate::services::skills_cli::SkillsCliError;

    #[test]
    fn dynamic_process_details_do_not_enter_the_ipc_envelope() {
        let secret = r"C:\Users\alice\private --force token=ghp_secret";
        let error = SkillsCliError::TaskJoin {
            label: "skills-cli",
            message: secret.to_string(),
        };
        let serialized = serde_json::to_string(&to_ipc_error(&error)).unwrap();
        assert!(serialized.contains("internal.unexpected"));
        assert!(!serialized.contains(secret));
        assert!(!serialized.contains("ghp_secret"));
    }

    #[test]
    fn update_check_failed_is_retryable_at_the_ipc_boundary() {
        let error = SkillsCliError::UpdateCheckFailed;
        assert_eq!(error.ipc_code(), "skills_cli.update_check_failed");
        assert!(error.retryable());
        assert!(!SkillsCliError::UpdateBaselineRequired.retryable());
    }
}
