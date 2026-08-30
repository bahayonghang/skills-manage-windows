//! Tauri IPC shells for Skills CLI global (`-g`) management.
//!
//! Every command resolves a request-scoped [`TargetContext`] first, queries
//! the capability matrix (before any remote handshake), then builds
//! [`SkillsCliTransport`] only when the capability is open. Business logic
//! lives in `crate::services::skills_cli`.

mod helpers;
#[cfg(test)]
mod tests;

use helpers::{
    job_lease_error, open_transport, operation_definition, require_capability, reviewed_failure,
    skills_cli_failure, to_ipc_error,
};
use tauri::{AppHandle, Emitter, State};

use crate::ipc_error::IpcError;
use crate::observability::{
    OperationContext, OperationSubjectKind, OperationTarget, SafeDetailKey, SafeIdentifier,
    SafeOperationResult,
};
use crate::services::github_import;
use crate::services::skills_cli as domain;
use crate::services::skills_cli::updates::{
    apply_updates, check_updates, load_update_inventory_for_pool, retry_update_recovery,
    verify_update_baseline, ProductionSkillsCliGithub, SkillsCliApplyRecoveryResult,
    SkillsCliApplyResult, SkillsCliApplyUpdateRequest, SkillsCliUpdateInventory,
    SkillsCliUpdateProgress, UpdateProgressEmitter, UPDATE_PROGRESS_EVENT,
};
use crate::services::skills_cli::{
    SkillsCliAddResult, SkillsCliCapability, SkillsCliDoctorReport, SkillsCliError,
    SkillsCliGlobalSnapshot, SkillsCliInstallTarget, SkillsCliPlacement,
    SkillsCliPlacementBatchItem, SkillsCliPlacementMutationOutcome, SkillsCliRemovePlan,
    SkillsCliRemoveResult, SkillsCliSkillDoc, SkillsCliSourcePreview,
};
use crate::AppState;

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn skills_cli_doctor(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<SkillsCliDoctorReport> {
    crate::ipc_boundary_async!("skills_cli_doctor", {
        let context = state.resolve_target_context().await?;
        require_capability(context.target(), SkillsCliCapability::Doctor)?;
        let tx = open_transport(context.target()).await?;
        domain::doctor(&tx)
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
        require_capability(context.target(), SkillsCliCapability::ListGlobal)?;
        let tx = open_transport(context.target()).await?;
        let pool = context.db().clone();
        domain::list_global(&tx, &pool)
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
        require_capability(context.target(), SkillsCliCapability::InstallTargets)?;
        let tx = open_transport(context.target()).await?;
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
                domain::install_targets(&tx, &pool)
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
        require_capability(context.target(), SkillsCliCapability::PreviewSource)?;
        let tx = open_transport(context.target()).await?;
        domain::preview_source(&tx, &source)
            .await
            .map_err(|error| to_ipc_error(&error))
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
        require_capability(context.target(), SkillsCliCapability::ReadSkillMd)?;
        let tx = open_transport(context.target()).await?;
        domain::read_skill_md(&tx, &skill_name)
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
        require_capability(context.target(), SkillsCliCapability::RevealFolder)?;
        let tx = open_transport(context.target()).await?;
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
                domain::reveal_skill_folder(&tx, &skill_name)
                    .map_err(|error| skills_cli_failure(definition, &error))
            },
        )
        .await
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
        require_capability(context.target(), SkillsCliCapability::ExportInventory)?;
        let tx = open_transport(context.target()).await?;
        let definition = operation_definition("skills_cli_export_inventory");
        crate::observability::run_operation(
            &state,
            definition,
            OperationContext::new(OperationTarget::local()),
            |_| SafeOperationResult::succeeded("Exported the Skills CLI inventory."),
            || async move {
                domain::export_inventory(&tx, std::path::PathBuf::from(path), json)
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
        require_capability(context.target(), SkillsCliCapability::CancelJob)?;
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
        require_capability(&active_target, SkillsCliCapability::AddGlobal)?;
        let tx = open_transport(&active_target).await?;
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
                    &tx,
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
    force: bool,
) -> crate::ipc_error::IpcResult<SkillsCliRemoveResult> {
    crate::ipc_boundary_async!("skills_cli_remove_global", {
        let lease = state
            .skills_cli_jobs
            .acquire(&job_id)
            .map_err(job_lease_error)?;
        let context = state.resolve_target_context().await?;
        let active_target = context.target().clone();
        let pool = context.db().clone();
        require_capability(&active_target, SkillsCliCapability::RemoveGlobal)?;
        let tx = open_transport(&active_target).await?;
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
                domain::remove_global(&tx, &pool, &skill_name, force, Some(lease.cancel_flag()))
                    .await
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
        require_capability(&active_target, SkillsCliCapability::LinkPlatform)?;
        let tx = open_transport(&active_target).await?;

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
                    &tx,
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
    force: bool,
) -> crate::ipc_error::IpcResult<SkillsCliPlacement> {
    crate::ipc_boundary_async!("skills_cli_unlink_platform", {
        let lease = state
            .skills_cli_jobs
            .acquire(&job_id)
            .map_err(job_lease_error)?;
        let context = state.resolve_target_context().await?;
        let active_target = context.target().clone();
        let pool = context.db().clone();
        require_capability(&active_target, SkillsCliCapability::UnlinkPlatform)?;
        let tx = open_transport(&active_target).await?;

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
                    &tx,
                    &pool,
                    &skill_name,
                    &skillport_agent_id,
                    force,
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
pub async fn skills_cli_link_platform_batch(
    state: State<'_, AppState>,
    job_id: String,
    items: Vec<SkillsCliPlacementBatchItem>,
) -> crate::ipc_error::IpcResult<SkillsCliPlacementMutationOutcome> {
    crate::ipc_boundary_async!("skills_cli_link_platform_batch", {
        if items.is_empty() {
            return Ok(SkillsCliPlacementMutationOutcome::default());
        }
        let lease = state
            .skills_cli_jobs
            .acquire(&job_id)
            .map_err(job_lease_error)?;
        let context = state.resolve_target_context().await?;
        let active_target = context.target().clone();
        let pool = context.db().clone();
        require_capability(&active_target, SkillsCliCapability::LinkPlatform)?;
        let tx = open_transport(&active_target).await?;
        let pairs: Vec<(String, String)> = items
            .iter()
            .map(|item| (item.skill_name.clone(), item.skillport_agent_id.clone()))
            .collect();
        let requested = pairs.len() as u64;
        let definition = operation_definition("skills_cli_link_platform_batch");
        let subject = items[0].skill_name.clone();
        let operation_context = OperationContext::new(OperationTarget::local())
            .subject(OperationSubjectKind::Skill, SafeIdentifier::new(&subject));
        crate::observability::run_operation(
            &state,
            definition,
            operation_context,
            move |outcome: &SkillsCliPlacementMutationOutcome| {
                SafeOperationResult::succeeded("Linked Skills CLI platform placements.")
                    .flag(SafeDetailKey::Changed, !outcome.succeeded.is_empty())
                    .count(SafeDetailKey::RequestedCount, requested)
                    .count(
                        SafeDetailKey::SucceededCount,
                        outcome.succeeded.len() as u64,
                    )
                    .count(SafeDetailKey::FailedCount, outcome.failed.len() as u64)
                    .count(SafeDetailKey::SkippedCount, outcome.skipped.len() as u64)
            },
            || async move {
                domain::link_platforms_batch(&tx, &pool, &pairs, Some(lease.cancel_flag()))
                    .await
                    .map_err(|error| skills_cli_failure(definition, &error))
            },
        )
        .await
    })
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn skills_cli_unlink_platform_batch(
    state: State<'_, AppState>,
    job_id: String,
    items: Vec<SkillsCliPlacementBatchItem>,
    force: bool,
) -> crate::ipc_error::IpcResult<SkillsCliPlacementMutationOutcome> {
    crate::ipc_boundary_async!("skills_cli_unlink_platform_batch", {
        if items.is_empty() {
            return Ok(SkillsCliPlacementMutationOutcome::default());
        }
        let lease = state
            .skills_cli_jobs
            .acquire(&job_id)
            .map_err(job_lease_error)?;
        let context = state.resolve_target_context().await?;
        let active_target = context.target().clone();
        let pool = context.db().clone();
        require_capability(&active_target, SkillsCliCapability::UnlinkPlatform)?;
        let tx = open_transport(&active_target).await?;
        let pairs: Vec<(String, String)> = items
            .iter()
            .map(|item| (item.skill_name.clone(), item.skillport_agent_id.clone()))
            .collect();
        let requested = pairs.len() as u64;
        let definition = operation_definition("skills_cli_unlink_platform_batch");
        let subject = items[0].skill_name.clone();
        let operation_context = OperationContext::new(OperationTarget::local())
            .subject(OperationSubjectKind::Skill, SafeIdentifier::new(&subject));
        crate::observability::run_operation(
            &state,
            definition,
            operation_context,
            move |outcome: &SkillsCliPlacementMutationOutcome| {
                SafeOperationResult::succeeded("Unlinked Skills CLI platform placements.")
                    .flag(SafeDetailKey::Changed, !outcome.succeeded.is_empty())
                    .count(SafeDetailKey::RequestedCount, requested)
                    .count(
                        SafeDetailKey::SucceededCount,
                        outcome.succeeded.len() as u64,
                    )
                    .count(SafeDetailKey::FailedCount, outcome.failed.len() as u64)
                    .count(SafeDetailKey::SkippedCount, outcome.skipped.len() as u64)
            },
            || async move {
                domain::unlink_platforms_batch(&tx, &pool, &pairs, force, Some(lease.cancel_flag()))
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
        require_capability(context.target(), SkillsCliCapability::PreviewRemove)?;
        let tx = open_transport(context.target()).await?;
        let pool = context.db().clone();
        domain::preview_remove_global(&tx, &pool, &skill_name)
            .await
            .map_err(|error| to_ipc_error(&error))
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
        require_capability(&active_target, SkillsCliCapability::CheckUpdates)?;
        let tx = open_transport(&active_target).await?;
        let github = github_from_state(&state).await?;
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
                check_updates(
                    &tx,
                    &pool,
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
        require_capability(context.target(), SkillsCliCapability::UpdateInventory)?;
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
        require_capability(context.target(), SkillsCliCapability::VerifyUpdateBaseline)?;
        let tx = open_transport(context.target()).await?;
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
                verify_update_baseline(&tx, &pool, &skill_names, Some(lease.cancel_flag()))
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
        require_capability(&active_target, SkillsCliCapability::ApplyUpdates)?;
        let tx = open_transport(&active_target).await?;
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
                    &tx,
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
        require_capability(context.target(), SkillsCliCapability::RetryUpdateRecovery)?;
        let tx = open_transport(context.target()).await?;
        let pool = context.db().clone();
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
                retry_update_recovery(&tx, &pool, &operation_id, Some(lease.cancel_flag()))
                    .await
                    .map_err(|error| skills_cli_failure(definition, &error))
            },
        )
        .await
    })
}
