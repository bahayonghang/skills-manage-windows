//! Tauri IPC shells for Central Skills and skill detail operations.
//!
//! Business logic lives in `crate::services::central_skills`. This module keeps
//! the existing command names and public type paths stable while translating
//! `State<AppState>` into service inputs and routing destructive operations
//! through the registered observability boundary.

use tauri::State;

use crate::observability::{
    CommandLogPolicy, OperationContext, OperationDefinition, OperationTarget, OperationTargetKind,
    ReviewedDiagnostic, ReviewedFailure, SafeDetailKey, SafeOperationResult,
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

fn operation_definition(command: &'static str) -> OperationDefinition {
    match crate::ipc_registry::command_policy(command)
        .expect("Central skill command must be registered")
        .policy
    {
        CommandLogPolicy::Operation(definition) => definition,
        _ => unreachable!("Central skill command must have an operation policy"),
    }
}

fn audit_target(target: &ActiveTarget) -> OperationTarget {
    match target {
        ActiveTarget::Local => OperationTarget::local(),
        ActiveTarget::Ssh(target) => OperationTarget::new(OperationTargetKind::Ssh, &target.id),
        ActiveTarget::Wsl(target) => OperationTarget::new(OperationTargetKind::Wsl, &target.id),
    }
}

fn bounded_count(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

fn reviewed_central_failure(
    definition: OperationDefinition,
    error: &central_skills::CentralSkillsError,
) -> ReviewedFailure {
    use central_skills::CentralSkillsError;

    let diagnostic = match error {
        CentralSkillsError::CentralMutation(_) => ReviewedDiagnostic::new(
            "central_skills.mutation_lock_failed",
            "central_skills.central_mutation",
            definition.default_phase(),
            "This Central operation could not acquire the mutation lock.",
            true,
        ),
        CentralSkillsError::Remote(_) => ReviewedDiagnostic::new(
            "central_skills.remote_failed",
            "central_skills.remote",
            definition.default_phase(),
            "This Central operation could not complete on the selected target.",
            true,
        ),
        CentralSkillsError::Db(_) => ReviewedDiagnostic::new(
            "central_skills.database_failed",
            "central_skills.db",
            definition.default_phase(),
            "This Central operation could not update its records.",
            false,
        ),
        CentralSkillsError::Budget(_) => ReviewedDiagnostic::new(
            "central_skills.budget_exceeded",
            "central_skills.budget",
            definition.default_phase(),
            "This Central operation exceeded a safety limit.",
            false,
        ),
        CentralSkillsError::ForceDeleteBlocked => ReviewedDiagnostic::new(
            "central_skills.force_delete_blocked",
            "central_skills.validation",
            definition.default_phase(),
            "Force delete is not available for this Central skill.",
            false,
        ),
        CentralSkillsError::UpdateRecovery { .. } => ReviewedDiagnostic::new(
            "central_skills.update_recovery_failed",
            "central_skills.recovery",
            definition.default_phase(),
            "A pending Central update could not be recovered before deletion.",
            false,
        ),
        CentralSkillsError::CentralOperation(_) => ReviewedDiagnostic::new(
            "central_skills.central_operation_failed",
            "central_skills.central_operation",
            definition.default_phase(),
            "This Central operation could not be completed.",
            false,
        ),
        _ => ReviewedDiagnostic::new(
            "central_skills.delete_failed",
            error.diagnostic_category(),
            definition.default_phase(),
            "This Central skill could not be deleted.",
            false,
        ),
    };
    ReviewedFailure::new(diagnostic)
}

fn batch_delete_result(
    requested: usize,
    result: &BatchDeleteCentralSkillResult,
    success_summary: &'static str,
    partial_summary: &'static str,
) -> SafeOperationResult {
    let audit = if result.failed.is_empty() {
        SafeOperationResult::succeeded(success_summary)
    } else {
        SafeOperationResult::partial(partial_summary)
    };
    audit
        .count(SafeDetailKey::RequestedCount, bounded_count(requested))
        .count(
            SafeDetailKey::SucceededCount,
            bounded_count(result.succeeded.len()),
        )
        .count(
            SafeDetailKey::FailedCount,
            bounded_count(result.failed.len()),
        )
}

fn reviewed_file_open_failure(
    definition: OperationDefinition,
    error: &central_skills::CentralSkillsError,
) -> ReviewedFailure {
    let diagnostic = match error {
        central_skills::CentralSkillsError::RemoteOpenInFileManagerUnsupported => {
            ReviewedDiagnostic::new(
                "central_skills.remote_open_unsupported",
                "central_skills.validation",
                definition.default_phase(),
                "Remote paths cannot be opened in the local file manager.",
                false,
            )
        }
        central_skills::CentralSkillsError::Remote(_) => ReviewedDiagnostic::new(
            "central_skills.remote_failed",
            "central_skills.remote",
            definition.default_phase(),
            "The selected target could not be reached.",
            true,
        ),
        _ => ReviewedDiagnostic::new(
            "central_skills.file_open_failed",
            "central_skills.file_access",
            definition.default_phase(),
            "The skill location could not be opened.",
            false,
        ),
    };
    ReviewedFailure::new(diagnostic)
}

fn missing_file_context_failure(definition: OperationDefinition) -> ReviewedFailure {
    ReviewedFailure::new(ReviewedDiagnostic::new(
        "central_skills.file_context_required",
        "central_skills.validation",
        definition.default_phase(),
        "A skill context is required for file access.",
        false,
    ))
}

fn open_file_manager_result() -> SafeOperationResult {
    SafeOperationResult::succeeded("Skill location opened.")
        .count(SafeDetailKey::AffectedCount, 1)
        .stable(SafeDetailKey::Mode, "file_manager")
}

/// Tauri command: return all skills installed for a given agent, including
/// installation metadata needed by the platform-view skill cards.
#[tauri::command]
pub async fn get_skills_by_agent(
    state: State<'_, AppState>,
    agent_id: String,
) -> crate::ipc_error::IpcResult<Vec<SkillForAgent>> {
    crate::ipc_boundary!(
        "get_skills_by_agent",
        async move {
            let context = state.resolve_target_context().await?;
            let pool = context.db().clone();
            let mut skills = central_skills::get_skills_by_agent_impl(&pool, &agent_id)
                .await
                .map_err(|e| e.to_string())?;
            if crate::services::skills_cli::SkillsCliTransport::uses_local_cli_lock(
                context.target(),
            ) {
                crate::services::skills_cli::annotate_platform_install_origins(&mut skills);
            }
            Ok::<_, String>(skills)
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
        "get_central_skills",
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
        "get_central_skills_page",
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
        "preview_delete_central_skills",
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
        "delete_central_skill",
        async move {
            let force = force.unwrap_or(false);
            let request_context = state.resolve_target_context().await?;
            let active_target = request_context.target().clone();
            let pool = request_context.db().clone();
            let definition = operation_definition("delete_central_skill");
            let audit_mode = if force { "force" } else { "safe" };
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(audit_target(&active_target)),
                |result: &DeleteCentralSkillResult| {
                    SafeOperationResult::succeeded("Central skill deleted.")
                        .count(SafeDetailKey::AffectedCount, 1)
                        .count(
                            SafeDetailKey::SucceededCount,
                            bounded_count(result.removed_agent_ids.len()),
                        )
                        .count(
                            SafeDetailKey::SkippedCount,
                            bounded_count(result.retained_agent_ids.len()),
                        )
                        .stable(SafeDetailKey::Mode, audit_mode)
                },
                || async {
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
                    };
                    result.map_err(|error| reviewed_central_failure(definition, &error))
                },
            )
            .await
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
        "delete_central_skills",
        async move {
            let request_context = state.resolve_target_context().await?;
            let active_target = request_context.target().clone();
            let pool = request_context.db().clone();
            let definition = operation_definition("delete_central_skills");
            let requested = requests.len();
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(audit_target(&active_target)),
                |result: &BatchDeleteCentralSkillResult| {
                    batch_delete_result(
                        requested,
                        result,
                        "Central skills deleted.",
                        "Central skill deletion partially completed.",
                    )
                },
                || async {
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
                    };
                    result.map_err(|error| reviewed_central_failure(definition, &error))
                },
            )
            .await
        }
        .await
    )
}

#[tauri::command]
pub async fn preview_reset_unknown_source_skills(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<ResetUnknownSourceSkillsPreview> {
    crate::ipc_boundary!(
        "preview_reset_unknown_source_skills",
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
        "reset_unknown_source_skills",
        async move {
            let request_context = state.resolve_target_context().await?;
            let active_target = request_context.target().clone();
            let pool = request_context.db().clone();
            let definition = operation_definition("reset_unknown_source_skills");
            let requested = skill_ids.len();
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(audit_target(&active_target)),
                |result: &BatchDeleteCentralSkillResult| {
                    batch_delete_result(
                        requested,
                        result,
                        "Unknown-source Central skills reset.",
                        "Unknown-source Central skill reset partially completed.",
                    )
                },
                || async {
                    central_skills::reset_unknown_source_skills_impl(
                        &pool,
                        &active_target,
                        &skill_ids,
                        &remove_copy_agent_ids,
                    )
                    .await
                    .map_err(|error| reviewed_central_failure(definition, &error))
                },
            )
            .await
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
        "preview_delete_skill_repository",
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
        "delete_skill_repository",
        async move {
            let request_context = state.resolve_target_context().await?;
            let active_target = request_context.target().clone();
            let pool = request_context.db().clone();
            let definition = operation_definition("delete_skill_repository");
            let requested = requests.len();
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(audit_target(&active_target)),
                |result: &DeleteSkillRepositoryResult| {
                    batch_delete_result(
                        requested,
                        &result.delete_result,
                        "Central repository deleted.",
                        "Central repository deletion partially completed.",
                    )
                    .flag(SafeDetailKey::Changed, result.deleted_repository)
                },
                || async {
                    let result = match &active_target {
                        ActiveTarget::Local => {
                            central_skills::delete_skill_repository_impl(
                                &pool,
                                &repository_id,
                                &requests,
                            )
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
                    };
                    result.map_err(|error| reviewed_central_failure(definition, &error))
                },
            )
            .await
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
        "get_skill_detail",
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
        "read_skill_content",
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
        "read_file_by_path",
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
        "open_in_file_manager",
        async move {
            let request_context = state.resolve_target_context().await?;
            let pool = request_context.db().clone();
            let active_target = request_context.target().clone();
            let definition = operation_definition("open_in_file_manager");
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(audit_target(&active_target)),
                |_| open_file_manager_result(),
                || async {
                    let access = path_access_context(skill_id, agent_id, row_id)
                        .map_err(|_| missing_file_context_failure(definition))?;
                    central_skills::open_in_file_manager_for_target_impl(
                        &pool,
                        active_target,
                        &path,
                        &access,
                    )
                    .await
                    .map_err(|error| reviewed_file_open_failure(definition, &error))
                },
            )
            .await
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
        "list_directory_tree",
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

#[cfg(test)]
#[path = "skills_observability_tests.rs"]
mod observability_tests;
