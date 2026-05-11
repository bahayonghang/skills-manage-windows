//! Tauri IPC shells for portable SkillPort state import/export.
//!
//! Business logic lives in `crate::services::portable_state`. This module keeps
//! the existing command names stable while translating `State<AppState>` into
//! service inputs and operation-log entries.

use serde_json::json;
use std::sync::{atomic::Ordering, Arc};
use std::time::Instant;
use tauri::{AppHandle, State};

use crate::operation_log::{
    local_target_context, record_operation_log_best_effort, OperationLogEvent,
};
use crate::services::portable_state::{
    build_remote_catalog, emit_portability_progress, export_skillport_state_impl,
    import_skillport_state_impl, is_cancelled_error, parse_manifest,
    preview_skillport_state_import_impl, PortabilityProgressUpdate,
};
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

#[tauri::command]
pub async fn export_skillport_state(
    app: AppHandle,
    state: State<'_, AppState>,
    _options: Option<SkillportStateExportOptions>,
) -> Result<String, String> {
    state.portable_state_cancel.store(false, Ordering::SeqCst);
    let cancel = Arc::clone(&state.portable_state_cancel);
    let started_at = Instant::now();
    emit_portability_progress(
        &app,
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
    let result = export_skillport_state_impl(&state.db, Some(&app), Some(&cancel)).await;
    match &result {
        Ok(payload) => {
            emit_portability_progress(
                &app,
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
                local_target_context(),
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
            let status = if is_cancelled_error(error) {
                SkillportStatePortabilityStatus::Cancelled
            } else {
                SkillportStatePortabilityStatus::Failed
            };
            emit_portability_progress(
                &app,
                PortabilityProgressUpdate {
                    phase: SkillportStatePortabilityPhase::Exporting,
                    status,
                    total: 1,
                    completed: 0,
                    message: None,
                    current_item: None,
                    error: Some(error),
                },
            );
            record_operation_log_best_effort(
                &state.db,
                local_target_context(),
                OperationLogEvent::new(
                    "import_export",
                    "state.export",
                    "failed",
                    "Failed to export portable SkillPort state",
                )
                .subject("state", "skillport", "SkillPort state")
                .error(error)
                .duration_ms(started_at.elapsed().as_millis() as i64),
            )
            .await;
        }
    }
    result
}

#[tauri::command]
pub async fn preview_skillport_state_import(
    app: AppHandle,
    state: State<'_, AppState>,
    json: String,
) -> Result<SkillportStateImportPreview, String> {
    state.portable_state_cancel.store(false, Ordering::SeqCst);
    let cancel = Arc::clone(&state.portable_state_cancel);
    let started_at = Instant::now();
    emit_portability_progress(
        &app,
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
                Some(&app),
                Some(&cancel),
            )
            .await
            {
                Ok(remote_catalog) => match preview_skillport_state_import_impl(
                    &state.db,
                    &manifest,
                    Some(&remote_catalog),
                    Some(&app),
                    Some(&cancel),
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
                &app,
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
                local_target_context(),
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
            let status = if is_cancelled_error(error) {
                SkillportStatePortabilityStatus::Cancelled
            } else {
                SkillportStatePortabilityStatus::Failed
            };
            emit_portability_progress(
                &app,
                PortabilityProgressUpdate {
                    phase: SkillportStatePortabilityPhase::Previewing,
                    status,
                    total: 3,
                    completed: 0,
                    message: None,
                    current_item: None,
                    error: Some(error),
                },
            );
            record_operation_log_best_effort(
                &state.db,
                local_target_context(),
                OperationLogEvent::new(
                    "import_export",
                    "state.preview_import",
                    "failed",
                    "Failed to preview portable SkillPort state import",
                )
                .subject("state", "skillport", "SkillPort state")
                .error(error)
                .duration_ms(started_at.elapsed().as_millis() as i64),
            )
            .await;
        }
    }
    result
}

#[tauri::command]
pub async fn import_skillport_state(
    app: AppHandle,
    state: State<'_, AppState>,
    json: String,
    resolutions: Vec<SkillportStateImportResolution>,
) -> Result<SkillportStateImportResult, String> {
    state.portable_state_cancel.store(false, Ordering::SeqCst);
    let cancel = Arc::clone(&state.portable_state_cancel);
    let started_at = Instant::now();
    emit_portability_progress(
        &app,
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
            import_skillport_state_impl(
                &state.db,
                state.secrets.as_ref(),
                &manifest,
                resolutions,
                Some(&app),
                Some(&cancel),
            )
            .await
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
                local_target_context(),
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
            let status = if is_cancelled_error(error) {
                SkillportStatePortabilityStatus::Cancelled
            } else {
                SkillportStatePortabilityStatus::Failed
            };
            emit_portability_progress(
                &app,
                PortabilityProgressUpdate {
                    phase: SkillportStatePortabilityPhase::Importing,
                    status,
                    total: 1,
                    completed: 0,
                    message: None,
                    current_item: None,
                    error: Some(error),
                },
            );
            record_operation_log_best_effort(
                &state.db,
                local_target_context(),
                OperationLogEvent::new(
                    "import_export",
                    "state.import",
                    "failed",
                    "Failed to import portable SkillPort state",
                )
                .subject("state", "skillport", "SkillPort state")
                .error(error)
                .duration_ms(started_at.elapsed().as_millis() as i64),
            )
            .await;
        }
    }
    result
}

#[tauri::command]
pub async fn cancel_skillport_state_portability(state: State<'_, AppState>) -> Result<(), String> {
    state.portable_state_cancel.store(true, Ordering::SeqCst);
    Ok(())
}
