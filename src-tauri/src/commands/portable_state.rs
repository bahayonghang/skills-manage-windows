//! Tauri IPC shells for portable SkillPort state import/export.
//!
//! Business logic lives in `crate::services::portable_state`. This module keeps
//! the existing command names stable while translating `State<AppState>` into
//! service inputs and operation-log entries.

use serde_json::json;
use std::sync::atomic::AtomicBool;
use std::time::Instant;
use tauri::{AppHandle, State};

use crate::operation_log::{
    record_operation_log_best_effort, target_context_from_active_target, OperationLogEvent,
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
) -> Result<String, String> {
    let lease = state
        .portable_state_jobs
        .acquire(&job_id)
        .map_err(|e| e.to_string())?;
    let started_at = Instant::now();
    let request_context = state.resolve_target_context().await?;
    let active_target = request_context.target().clone();
    let target_context = target_context_from_active_target(&active_target);
    let export_target = portable_state_target_context(&active_target);
    let pool = request_context.db().clone();
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
    match &result {
        Ok(payload) => {
            emit_portability_progress(
                &app,
                lease.job_id(),
                PortabilityProgressUpdate {
                    phase: SkillportStatePortabilityPhase::Exporting,
                    status: SkillportStatePortabilityStatus::Completed,
                    total: 1,
                    completed: 1,
                    message: Some("Portable SkillPort state export completed"),
                    current_item: None,
                    error: None,
                },
            );
            let manifest = serde_json::from_str::<SkillportStateManifest>(payload).ok();
            record_operation_log_best_effort(
                &state.db,
                target_context.clone(),
                OperationLogEvent::new(
                    "import_export",
                    "state.export",
                    "succeeded",
                    "Exported portable SkillPort state",
                )
                .subject("state", "skillport", "SkillPort state")
                .details(json!({
                    "githubSources": manifest.as_ref().map(|item| item.github_sources.len()),
                    "centralSkills": manifest.as_ref().map(|item| item.central_skills.len()),
                    "unrestorableSkills": manifest.as_ref().map(|item| item.unrestorable_skills.len()),
                }))
                .duration_ms(started_at.elapsed().as_millis() as i64),
            )
            .await;
        }
        Err(error) => {
            let error_text = error.to_string();
            let status = if matches!(error, PortableStateError::Cancelled) {
                SkillportStatePortabilityStatus::Cancelled
            } else {
                SkillportStatePortabilityStatus::Failed
            };
            emit_portability_progress(
                &app,
                lease.job_id(),
                PortabilityProgressUpdate {
                    phase: SkillportStatePortabilityPhase::Exporting,
                    status,
                    total: 1,
                    completed: 0,
                    message: None,
                    current_item: None,
                    error: Some(&error_text),
                },
            );
            record_operation_log_best_effort(
                &state.db,
                target_context,
                OperationLogEvent::new(
                    "import_export",
                    "state.export",
                    "failed",
                    "Failed to export portable SkillPort state",
                )
                .subject("state", "skillport", "SkillPort state")
                .error(&error_text)
                .duration_ms(started_at.elapsed().as_millis() as i64),
            )
            .await;
        }
    }
    result.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn preview_skillport_state_import(
    app: AppHandle,
    state: State<'_, AppState>,
    job_id: String,
    json: String,
) -> Result<SkillportStateImportPreview, String> {
    let lease = state
        .portable_state_jobs
        .acquire(&job_id)
        .map_err(|e| e.to_string())?;
    preview_skillport_state_import_established(
        &app,
        state.inner(),
        json,
        lease.job_id(),
        lease.cancel_flag(),
    )
    .await
}

async fn preview_skillport_state_import_established(
    app: &AppHandle,
    state: &AppState,
    json: String,
    job_id: &str,
    cancel: &AtomicBool,
) -> Result<SkillportStateImportPreview, String> {
    let started_at = Instant::now();
    let request_context = state.resolve_target_context().await?;
    let active_target = request_context.target().clone();
    let target_context = target_context_from_active_target(&active_target);
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
        Ok(preview) => {
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
            record_operation_log_best_effort(
                &state.db,
                target_context.clone(),
                OperationLogEvent::new(
                    "import_export",
                    "state.preview_import",
                    "succeeded",
                    "Previewed portable SkillPort state import",
                )
                .subject("state", "skillport", "SkillPort state")
                .details(json!({
                    "summary": &preview.summary,
                }))
                .duration_ms(started_at.elapsed().as_millis() as i64),
            )
            .await;
        }
        Err(error) => {
            let error_text = error.to_string();
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
                    message: None,
                    current_item: None,
                    error: Some(&error_text),
                },
            );
            record_operation_log_best_effort(
                &state.db,
                target_context,
                OperationLogEvent::new(
                    "import_export",
                    "state.preview_import",
                    "failed",
                    "Failed to preview portable SkillPort state import",
                )
                .subject("state", "skillport", "SkillPort state")
                .error(&error_text)
                .duration_ms(started_at.elapsed().as_millis() as i64),
            )
            .await;
        }
    }
    result.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn preview_skillport_state_import_file(
    app: AppHandle,
    state: State<'_, AppState>,
    job_id: String,
    path: String,
) -> Result<SkillportStateImportFilePreview, String> {
    let lease = state
        .portable_state_jobs
        .acquire(&job_id)
        .map_err(|e| e.to_string())?;
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
    Ok(SkillportStateImportFilePreview { json, preview })
}

#[tauri::command]
pub async fn save_skillport_state_export(path: String, json: String) -> Result<(), String> {
    write_skillport_state_file(path.into(), json)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn import_skillport_state(
    app: AppHandle,
    state: State<'_, AppState>,
    job_id: String,
    json: String,
    resolutions: Vec<SkillportStateImportResolution>,
) -> Result<SkillportStateImportResult, String> {
    let lease = state
        .portable_state_jobs
        .acquire(&job_id)
        .map_err(|e| e.to_string())?;
    let started_at = Instant::now();
    let request_context = state.resolve_target_context().await?;
    let active_target = request_context.target().clone();
    let target_context = target_context_from_active_target(&active_target);
    let pool = request_context.db().clone();
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
            let auth = github_import::github_direct_auth_from_secret_store(
                &state.db,
                state.secrets.as_ref(),
            )
            .await
            .map_err(PortableStateError::GithubImport);
            match auth {
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
    match &result {
        Ok(import_result) => {
            let status = if import_result.cancelled {
                "cancelled"
            } else {
                match (
                    import_result.imported_skills.len() + import_result.sources_added,
                    import_result.failed_skills.len(),
                ) {
                    (_, 0) => "succeeded",
                    (0, _) => "failed",
                    _ => "partial",
                }
            };
            emit_portability_progress(
                &app,
                lease.job_id(),
                PortabilityProgressUpdate {
                    phase: SkillportStatePortabilityPhase::Importing,
                    status: if import_result.cancelled {
                        SkillportStatePortabilityStatus::Cancelled
                    } else if import_result.failed_skills.is_empty() {
                        SkillportStatePortabilityStatus::Completed
                    } else {
                        SkillportStatePortabilityStatus::Failed
                    },
                    total: import_result.imported_skills.len()
                        + import_result.failed_skills.len()
                        + import_result.skipped_skills.len(),
                    completed: import_result.imported_skills.len()
                        + import_result.failed_skills.len()
                        + import_result.skipped_skills.len(),
                    message: Some("SkillPort state import finished"),
                    current_item: None,
                    error: None,
                },
            );
            record_operation_log_best_effort(
                &state.db,
                target_context.clone(),
                OperationLogEvent::new(
                    "import_export",
                    "state.import",
                    status,
                    format!(
                        "Imported {} skill(s), {} failed",
                        import_result.imported_skills.len(),
                        import_result.failed_skills.len()
                    ),
                )
                .subject("state", "skillport", "SkillPort state")
                .details(json!({
                    "sourcesAdded": import_result.sources_added,
                    "sourcesSkipped": import_result.sources_skipped,
                    "importedSkills": &import_result.imported_skills,
                    "skippedSkills": &import_result.skipped_skills,
                    "failedSkills": &import_result.failed_skills,
                    "tagsRestored": import_result.tags_restored,
                    "cancelled": import_result.cancelled,
                }))
                .duration_ms(started_at.elapsed().as_millis() as i64),
            )
            .await;
        }
        Err(error) => {
            let error_text = error.to_string();
            let status = if matches!(error, PortableStateError::Cancelled) {
                SkillportStatePortabilityStatus::Cancelled
            } else {
                SkillportStatePortabilityStatus::Failed
            };
            emit_portability_progress(
                &app,
                lease.job_id(),
                PortabilityProgressUpdate {
                    phase: SkillportStatePortabilityPhase::Importing,
                    status,
                    total: 1,
                    completed: 0,
                    message: None,
                    current_item: None,
                    error: Some(&error_text),
                },
            );
            record_operation_log_best_effort(
                &state.db,
                target_context,
                OperationLogEvent::new(
                    "import_export",
                    "state.import",
                    "failed",
                    "Failed to import portable SkillPort state",
                )
                .subject("state", "skillport", "SkillPort state")
                .error(&error_text)
                .duration_ms(started_at.elapsed().as_millis() as i64),
            )
            .await;
        }
    }
    result.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn cancel_skillport_state_portability(
    state: State<'_, AppState>,
    job_id: String,
) -> Result<(), String> {
    state
        .portable_state_jobs
        .cancel(&job_id)
        .map_err(|e| e.to_string())?;
    Ok(())
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
