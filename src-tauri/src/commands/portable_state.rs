//! Tauri IPC shells for portable SkillPort state import/export.
//!
//! Business logic lives in `crate::services::portable_state`. This module keeps
//! the existing command names stable while translating `State<AppState>` into
//! service inputs and operation-log entries.

use std::sync::atomic::AtomicBool;
use tauri::{AppHandle, State};

use crate::ipc_error::{public_message_for_code, IpcError, REVIEWED_IPC_ERROR_CODES};
use crate::observability::{
    CommandLogPolicy, OperationContext, OperationDefinition, OperationTarget, OperationTargetKind,
    ReviewedDiagnostic, ReviewedFailure, SafeDetailKey, SafeOperationResult,
};
use crate::services::github_import;
use crate::services::portable_state::{
    build_remote_catalog, emit_portability_progress, export_skillport_state_impl,
    import_skillport_state_for_target, parse_manifest, preview_skillport_state_import_impl,
    read_skillport_state_file, write_skillport_state_file, PortabilityProgressUpdate,
    PortableStateError, PortableStateTargetContext,
};
use crate::targets::{ActiveTarget, TargetKind};
use crate::AppState;

pub use crate::services::portable_state::{
    ExportedFrom, PortableCentralSkill, PortableCentralSkillSource, PortableGithubSource,
    PortableSkillTag, PortableUnrestorableSkill, SkillPreviewStatus, SkillportStateExportOptions,
    SkillportStateImportFailure, SkillportStateImportPreview, SkillportStateImportPreviewSummary,
    SkillportStateImportResolution, SkillportStateImportResult, SkillportStateImportedSkill,
    SkillportStateManifest, SkillportStatePortabilityPhase,
    SkillportStatePortabilityProgressPayload, SkillportStatePortabilityStatus,
    SkillportStateSkillPreview, SkillportStateSourcePreview, SourcePreviewStatus,
};

fn operation_definition(command: &'static str) -> OperationDefinition {
    match crate::ipc_registry::command_policy(command)
        .expect("portable-state command must be registered")
        .policy
    {
        CommandLogPolicy::Operation(definition) => definition,
        _ => panic!("portable-state mutation must use Operation policy"),
    }
}

fn operation_target(target: &ActiveTarget) -> OperationTarget {
    match target {
        ActiveTarget::Local => OperationTarget::local(),
        ActiveTarget::Ssh(_) => OperationTarget::new(OperationTargetKind::Ssh, target.id()),
        ActiveTarget::Wsl(_) => OperationTarget::new(OperationTargetKind::Wsl, target.id()),
    }
}

fn portable_ipc_error(error: &PortableStateError) -> IpcError {
    match error {
        PortableStateError::Cancelled => {
            IpcError::new("operation.cancelled", "The operation was cancelled.", false)
        }
        PortableStateError::InvalidManifestJson(_) => IpcError::new(
            "portable_state.invalid_manifest_json",
            "The SkillPort state file is not valid JSON.",
            false,
        ),
        PortableStateError::UnsupportedExportKind => IpcError::new(
            "portable_state.unsupported_export_kind",
            "The SkillPort state export kind is not supported.",
            false,
        ),
        PortableStateError::UnsupportedExportVersion(_) => IpcError::new(
            "portable_state.unsupported_export_version",
            "The SkillPort state export version is not supported.",
            false,
        ),
        PortableStateError::GithubImport(error) => IpcError::from(error.to_ipc_error()),
        _ => IpcError::new(
            "internal.unexpected",
            "The operation failed. See runtime logs for details.",
            false,
        ),
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

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillportStateImportFilePreview {
    pub json: String,
    pub preview: SkillportStateImportPreview,
}

#[tauri::command]
pub async fn export_skillport_state(
    app: AppHandle,
    state: State<'_, AppState>,
    job_id: String,
    _options: Option<SkillportStateExportOptions>,
) -> crate::ipc_error::IpcResult<String> {
    crate::ipc_boundary!(
        "export_skillport_state",
        async move {
            let request_context = state
                .resolve_target_context()
                .await
                .map_err(IpcError::from)?;
            let active_target = request_context.target().clone();
            let export_target = portable_state_target_context(&active_target);
            let pool = request_context.db().clone();
            let definition = operation_definition("export_skillport_state");
            let lease = state
                .portable_state_jobs
                .acquire(&job_id)
                .map_err(IpcError::from_display)?;
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(operation_target(&active_target)),
                |payload: &String| {
                    let manifest = serde_json::from_str::<SkillportStateManifest>(payload).ok();
                    SafeOperationResult::succeeded("Exported portable SkillPort state.").count(
                        SafeDetailKey::AffectedCount,
                        manifest
                            .as_ref()
                            .map(|item| item.central_skills.len() as u64)
                            .unwrap_or(0),
                    )
                },
                || async move {
                    emit_portability_progress(
                        &app,
                        lease.job_id(),
                        PortabilityProgressUpdate {
                            phase: SkillportStatePortabilityPhase::Exporting,
                            status: SkillportStatePortabilityStatus::Running,
                            total: 1,
                            completed: 0,
                            message: Some("Preparing portable SkillPort state export"),
                            current_item: None,
                            error: None,
                        },
                    );
                    let result = export_skillport_state_impl(
                        &pool,
                        Some(&export_target),
                        lease.job_id(),
                        Some(&app),
                        Some(lease.cancel_flag()),
                    )
                    .await;
                    let (status, completed) = match &result {
                        Ok(_) => (SkillportStatePortabilityStatus::Completed, 1),
                        Err(PortableStateError::Cancelled) => {
                            (SkillportStatePortabilityStatus::Cancelled, 0)
                        }
                        Err(_) => (SkillportStatePortabilityStatus::Failed, 0),
                    };
                    emit_portability_progress(
                        &app,
                        lease.job_id(),
                        PortabilityProgressUpdate {
                            phase: SkillportStatePortabilityPhase::Exporting,
                            status,
                            total: 1,
                            completed,
                            message: result
                                .as_ref()
                                .ok()
                                .map(|_| "Portable SkillPort state export completed"),
                            current_item: None,
                            error: None,
                        },
                    );
                    result.map_err(|error| reviewed_failure(definition, portable_ipc_error(&error)))
                },
            )
            .await
        }
        .await
    )
}

#[tauri::command]
pub async fn preview_skillport_state_import(
    app: AppHandle,
    state: State<'_, AppState>,
    job_id: String,
    json: String,
) -> crate::ipc_error::IpcResult<SkillportStateImportPreview> {
    crate::ipc_boundary!(
        "preview_skillport_state_import",
        async move {
            let lease = state
                .portable_state_jobs
                .acquire(&job_id)
                .map_err(IpcError::from_display)?;
            preview_skillport_state_import_established(
                &app,
                state.inner(),
                json,
                lease.job_id(),
                lease.cancel_flag(),
            )
            .await
        }
        .await
    )
}

async fn preview_skillport_state_import_established(
    app: &AppHandle,
    state: &AppState,
    json: String,
    job_id: &str,
    cancel: &AtomicBool,
) -> crate::ipc_error::IpcResult<SkillportStateImportPreview> {
    let request_context = state
        .resolve_target_context()
        .await
        .map_err(IpcError::from)?;
    let pool = request_context.db().clone();
    emit_portability_progress(
        app,
        job_id,
        PortabilityProgressUpdate {
            phase: SkillportStatePortabilityPhase::Previewing,
            status: SkillportStatePortabilityStatus::Running,
            total: 3,
            completed: 0,
            message: Some("Parsing SkillPort state JSON"),
            current_item: None,
            error: None,
        },
    );
    let result = match parse_manifest(&json) {
        Ok(manifest) => {
            match build_remote_catalog(
                &state.db,
                state.secrets.as_ref(),
                &manifest,
                job_id,
                Some(app),
                Some(cancel),
            )
            .await
            {
                Ok(remote_catalog) => match preview_skillport_state_import_impl(
                    &pool,
                    &manifest,
                    Some(&remote_catalog),
                    job_id,
                    Some(app),
                    Some(cancel),
                )
                .await
                {
                    Ok(preview) => Ok(preview),
                    Err(error) => Err(error),
                },
                Err(error) => Err(error),
            }
        }
        Err(error) => Err(error),
    };
    match &result {
        Ok(_preview) => {
            emit_portability_progress(
                app,
                job_id,
                PortabilityProgressUpdate {
                    phase: SkillportStatePortabilityPhase::Previewing,
                    status: SkillportStatePortabilityStatus::Completed,
                    total: 3,
                    completed: 3,
                    message: Some("SkillPort state import preview completed"),
                    current_item: None,
                    error: None,
                },
            );
        }
        Err(error) => {
            let status = if matches!(error, PortableStateError::Cancelled) {
                SkillportStatePortabilityStatus::Cancelled
            } else {
                SkillportStatePortabilityStatus::Failed
            };
            emit_portability_progress(
                app,
                job_id,
                PortabilityProgressUpdate {
                    phase: SkillportStatePortabilityPhase::Previewing,
                    status,
                    total: 3,
                    completed: 0,
                    message: Some(if matches!(error, PortableStateError::Cancelled) {
                        "SkillPort state import preview cancelled"
                    } else {
                        "SkillPort state import preview failed"
                    }),
                    current_item: None,
                    error: None,
                },
            );
        }
    }
    result.map_err(|error| portable_ipc_error(&error))
}

#[tauri::command]
pub async fn preview_skillport_state_import_file(
    app: AppHandle,
    state: State<'_, AppState>,
    job_id: String,
    path: String,
) -> crate::ipc_error::IpcResult<SkillportStateImportFilePreview> {
    crate::ipc_boundary!(
        "preview_skillport_state_import_file",
        async move {
            let lease = state
                .portable_state_jobs
                .acquire(&job_id)
                .map_err(IpcError::from_display)?;
            let json = read_skillport_state_file(path.into())
                .await
                .map_err(|error| error.to_string())?;
            let preview = preview_skillport_state_import_established(
                &app,
                state.inner(),
                json.clone(),
                lease.job_id(),
                lease.cancel_flag(),
            )
            .await?;
            Ok::<SkillportStateImportFilePreview, crate::ipc_error::IpcError>(
                SkillportStateImportFilePreview { json, preview },
            )
        }
        .await
    )
}

#[tauri::command]
pub async fn save_skillport_state_export(
    state: State<'_, AppState>,
    path: String,
    json: String,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary!(
        "save_skillport_state_export",
        async move {
            let definition = operation_definition("save_skillport_state_export");
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(OperationTarget::local()),
                |_| SafeOperationResult::succeeded("Saved portable SkillPort state."),
                || async move {
                    write_skillport_state_file(path.into(), json)
                        .await
                        .map_err(|error| reviewed_failure(definition, portable_ipc_error(&error)))
                },
            )
            .await
        }
        .await
    )
}

#[tauri::command]
pub async fn import_skillport_state(
    app: AppHandle,
    state: State<'_, AppState>,
    job_id: String,
    json: String,
    resolutions: Vec<SkillportStateImportResolution>,
) -> crate::ipc_error::IpcResult<SkillportStateImportResult> {
    crate::ipc_boundary!(
        "import_skillport_state",
        async move {
            let lease = state
                .portable_state_jobs
                .acquire(&job_id)
                .map_err(IpcError::from_display)?;
            let request_context = state
                .resolve_target_context()
                .await
                .map_err(IpcError::from)?;
            let active_target = request_context.target().clone();
            let pool = request_context.db().clone();
            let definition = operation_definition("import_skillport_state");
            let local_db = state.db.clone();
            let secrets = std::sync::Arc::clone(&state.secrets);
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(operation_target(&active_target)),
                |result: &SkillportStateImportResult| {
                    let requested = result.imported_skills.len()
                        + result.failed_skills.len()
                        + result.skipped_skills.len();
                    let base = if result.cancelled {
                        SafeOperationResult::cancelled("Cancelled portable SkillPort state import.")
                    } else if result.failed_skills.is_empty() {
                        SafeOperationResult::succeeded("Imported portable SkillPort state.")
                    } else {
                        SafeOperationResult::partial(
                            "Portable SkillPort state import completed partially.",
                        )
                    };
                    base.count(SafeDetailKey::RequestedCount, requested as u64)
                        .count(
                            SafeDetailKey::SucceededCount,
                            result.imported_skills.len() as u64,
                        )
                        .count(
                            SafeDetailKey::FailedCount,
                            result.failed_skills.len() as u64,
                        )
                        .count(
                            SafeDetailKey::SkippedCount,
                            result.skipped_skills.len() as u64,
                        )
                },
                || async move {
                    emit_portability_progress(
                        &app,
                        lease.job_id(),
                        PortabilityProgressUpdate {
                            phase: SkillportStatePortabilityPhase::Importing,
                            status: SkillportStatePortabilityStatus::Running,
                            total: 1,
                            completed: 0,
                            message: Some("Preparing SkillPort state import"),
                            current_item: None,
                            error: None,
                        },
                    );
                    let result = match parse_manifest(&json) {
                        Ok(manifest) => {
                            match github_import::github_direct_auth_from_secret_store(
                                &local_db,
                                secrets.as_ref(),
                            )
                            .await
                            .map_err(PortableStateError::GithubImport)
                            {
                                Ok(auth) => {
                                    import_skillport_state_for_target(
                                        &pool,
                                        &active_target,
                                        auth.as_deref(),
                                        &manifest,
                                        resolutions,
                                        lease.job_id(),
                                        Some(&app),
                                        Some(lease.cancel_flag()),
                                    )
                                    .await
                                }
                                Err(error) => Err(error),
                            }
                        }
                        Err(error) => Err(error),
                    };
                    let (status, total, completed) = match &result {
                        Ok(import_result) => {
                            let total = import_result.imported_skills.len()
                                + import_result.failed_skills.len()
                                + import_result.skipped_skills.len();
                            let status = if import_result.cancelled {
                                SkillportStatePortabilityStatus::Cancelled
                            } else if import_result.failed_skills.is_empty() {
                                SkillportStatePortabilityStatus::Completed
                            } else {
                                SkillportStatePortabilityStatus::Failed
                            };
                            (status, total, total)
                        }
                        Err(PortableStateError::Cancelled) => {
                            (SkillportStatePortabilityStatus::Cancelled, 1, 0)
                        }
                        Err(_) => (SkillportStatePortabilityStatus::Failed, 1, 0),
                    };
                    emit_portability_progress(
                        &app,
                        lease.job_id(),
                        PortabilityProgressUpdate {
                            phase: SkillportStatePortabilityPhase::Importing,
                            status,
                            total,
                            completed,
                            message: Some("SkillPort state import finished"),
                            current_item: None,
                            error: None,
                        },
                    );
                    result.map_err(|error| reviewed_failure(definition, portable_ipc_error(&error)))
                },
            )
            .await
        }
        .await
    )
}

#[tauri::command]
pub async fn cancel_skillport_state_portability(
    state: State<'_, AppState>,
    job_id: String,
) -> crate::ipc_error::IpcResult<()> {
    crate::ipc_boundary!(
        "cancel_skillport_state_portability",
        async move {
            let definition = operation_definition("cancel_skillport_state_portability");
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(OperationTarget::local()),
                |changed: &bool| {
                    SafeOperationResult::succeeded("Requested portable-state cancellation.")
                        .flag(SafeDetailKey::Changed, *changed)
                },
                || async {
                    state.portable_state_jobs.cancel(&job_id).map_err(|error| {
                        reviewed_failure(definition, IpcError::from_display(error))
                    })
                },
            )
            .await
            .map(|_| ())
        }
        .await
    )
}

fn portable_state_target_context(active_target: &ActiveTarget) -> PortableStateTargetContext {
    let kind = match active_target.kind() {
        TargetKind::Local => "local",
        TargetKind::Ssh => "ssh",
        TargetKind::Wsl => "wsl",
    };
    PortableStateTargetContext {
        id: active_target.id().to_string(),
        kind: kind.to_string(),
        label: active_target.label().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn portable_state_errors_drop_paths_and_manifest_content() {
        let secret = r#"C:\Users\alice\private\state.json token=ghp_secret"#;
        let error = PortableStateError::InvalidFileExtension(secret.to_string());
        let serialized = serde_json::to_string(&portable_ipc_error(&error)).unwrap();
        assert!(serialized.contains("internal.unexpected"));
        assert!(!serialized.contains(secret));
        assert!(!serialized.contains("ghp_secret"));
    }

    #[test]
    fn portable_state_cancel_uses_the_stable_cancelled_envelope() {
        let error = portable_ipc_error(&PortableStateError::Cancelled);
        assert_eq!(error.code, "operation.cancelled");
        assert!(!error.retryable);
    }
}
