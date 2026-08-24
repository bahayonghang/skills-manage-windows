//! Tauri IPC shells for Skills CLI global (`-g`) management.
//!
//! Every command resolves a request-scoped [`TargetContext`] first and
//! rejects non-Local targets before spawning anything or reading local
//! state. Business logic lives in `crate::services::skills_cli`; this module
//! owns the Local gate, exclusive job lease, operation log, and the typed
//! error envelope.

use std::sync::Arc;
use std::time::Instant;

use serde_json::json;
use tauri::State;

use crate::ipc_error::{public_message_for_code, IpcError};
use crate::operation_log::{
    record_operation_log_best_effort, target_context_from_active_target, OperationLogEvent,
};
use crate::services::exclusive_job::ExclusiveJobError;
use crate::services::skills_cli as domain;
use crate::services::skills_cli::{
    NodeProcessRunner, SkillsCliAddResult, SkillsCliDoctorReport, SkillsCliError,
    SkillsCliGlobalSkill, SkillsCliInstallTarget, SkillsCliRunner, SkillsCliSourcePreview,
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

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn skills_cli_doctor(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<SkillsCliDoctorReport> {
    let context = state.resolve_target_context().await?;
    domain::ensure_local_target(context.target()).map_err(|error| to_ipc_error(&error))?;
    domain::doctor(domain_runner().as_ref())
        .await
        .map_err(|error| to_ipc_error(&error))
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn skills_cli_list_global(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<Vec<SkillsCliGlobalSkill>> {
    let context = state.resolve_target_context().await?;
    domain::ensure_local_target(context.target()).map_err(|error| to_ipc_error(&error))?;
    domain::list_global(domain_runner().as_ref())
        .await
        .map_err(|error| to_ipc_error(&error))
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn skills_cli_install_targets(
    state: State<'_, AppState>,
) -> crate::ipc_error::IpcResult<Vec<SkillsCliInstallTarget>> {
    let context = state.resolve_target_context().await?;
    let pool = context.db().clone();
    domain::ensure_local_target(context.target()).map_err(|error| to_ipc_error(&error))?;
    domain::install_targets(&pool)
        .await
        .map_err(|error| to_ipc_error(&error))
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn skills_cli_preview_source(
    state: State<'_, AppState>,
    source: String,
) -> crate::ipc_error::IpcResult<SkillsCliSourcePreview> {
    let context = state.resolve_target_context().await?;
    domain::ensure_local_target(context.target()).map_err(|error| to_ipc_error(&error))?;
    domain::preview_source(domain_runner().as_ref(), &source)
        .await
        .map_err(|error| to_ipc_error(&error))
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
    let lease = state
        .skills_cli_jobs
        .acquire(&job_id)
        .map_err(job_lease_error)?;
    let context = state.resolve_target_context().await?;
    let active_target = context.target().clone();
    let pool = context.db().clone();
    domain::ensure_local_target(&active_target).map_err(|error| to_ipc_error(&error))?;

    let started_at = Instant::now();
    let result = domain::add_global(
        domain_runner().as_ref(),
        &source,
        skill_names.clone(),
        skillport_agent_ids.clone(),
        Some(lease.cancel_flag()),
    )
    .await;

    let status = if result.is_ok() {
        "succeeded"
    } else {
        "failed"
    };
    let summary = match &result {
        Ok(report) => format!(
            "Installed {} Skills CLI global skill(s) onto {} platform(s)",
            report.installed_skills, report.targeted_platforms
        ),
        Err(_) => "Failed to install Skills CLI global skills".to_string(),
    };
    let event = OperationLogEvent::new("skills_cli", "skills_cli.add", status, summary)
        .details(json!({
            "skill_count": skill_names.len(),
            "platform_count": skillport_agent_ids.len(),
        }))
        .duration_ms(started_at.elapsed().as_millis() as i64);
    record_operation_log_best_effort(
        &pool,
        target_context_from_active_target(&active_target),
        event,
    )
    .await;

    result.map_err(|error| to_ipc_error(&error))
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn skills_cli_remove_global(
    state: State<'_, AppState>,
    job_id: String,
    skill_name: String,
) -> crate::ipc_error::IpcResult<()> {
    let lease = state
        .skills_cli_jobs
        .acquire(&job_id)
        .map_err(job_lease_error)?;
    let context = state.resolve_target_context().await?;
    let active_target = context.target().clone();
    let pool = context.db().clone();
    domain::ensure_local_target(&active_target).map_err(|error| to_ipc_error(&error))?;

    let started_at = Instant::now();
    let result = domain::remove_global(
        domain_runner().as_ref(),
        &skill_name,
        Some(lease.cancel_flag()),
    )
    .await;

    let status = if result.is_ok() {
        "succeeded"
    } else {
        "failed"
    };
    let summary = match &result {
        Ok(()) => format!("Uninstalled Skills CLI global skill {skill_name}"),
        Err(_) => "Failed to uninstall Skills CLI global skill".to_string(),
    };
    let event = OperationLogEvent::new("skills_cli", "skills_cli.remove", status, summary)
        .subject("skill", &skill_name, &skill_name)
        .duration_ms(started_at.elapsed().as_millis() as i64);
    record_operation_log_best_effort(
        &pool,
        target_context_from_active_target(&active_target),
        event,
    )
    .await;

    result.map_err(|error| to_ipc_error(&error))
}

#[tauri::command]
#[cfg_attr(feature = "ipc-codegen", specta::specta)]
pub async fn cancel_skills_cli_job(
    state: State<'_, AppState>,
    job_id: String,
) -> crate::ipc_error::IpcResult<bool> {
    let context = state.resolve_target_context().await?;
    domain::ensure_local_target(context.target()).map_err(|error| to_ipc_error(&error))?;
    state
        .skills_cli_jobs
        .cancel(&job_id)
        .map_err(job_lease_error)
}
