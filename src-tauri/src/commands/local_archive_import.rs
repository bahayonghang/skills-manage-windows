//! Tauri IPC shells for local skill archive (ZIP) import.
//!
//! Business logic lives in `crate::services::local_archive_import`. These
//! commands translate `State<AppState>` into service inputs and surface
//! typed errors as IPC error strings. The preview command never touches the
//! filesystem beyond reading the archive; the import command re-verifies
//! the fingerprint before any Central/DB mutation.

use std::time::Instant;

use serde_json::json;
use tauri::State;

use crate::db::DbPool;
use crate::operation_log::{
    record_operation_log_best_effort, target_context_from_active_target, OperationLogEvent,
    OperationLogTargetContext,
};
use crate::services::local_archive_import;
use crate::targets::ActiveTarget;
use crate::AppState;

pub use crate::services::local_archive_import::{
    ArchiveFingerprint, LocalArchiveImportError, LocalArchiveImportResolution,
    LocalArchiveImportResult, LocalArchivePreview, LocalSkillConflict,
};

/// Preview a local `.zip` skill archive without writing anything.
///
/// Returns the archive fingerprint, resolved skill candidate, file tree,
/// and any Central conflict. The frontend never receives absolute user
/// paths; only the archive basename and relative paths are exposed.
#[tauri::command]
pub async fn preview_local_skill_archive(
    state: State<'_, AppState>,
    archive_path: String,
) -> Result<LocalArchivePreview, String> {
    let request_context = state.resolve_target_context().await?;
    let active_target = request_context.target().clone();
    // ZIP import is local-only for MVP. SSH/WSL targets must disable the
    // ZIP intent in the frontend; a stray call is rejected here.
    if !matches!(active_target, ActiveTarget::Local) {
        return Err(
            local_archive_import::LocalArchiveImportError::RemoteTargetUnsupported.to_ipc_error(),
        );
    }
    let pool = request_context.db().clone();
    local_archive_import::preview_local_skill_archive_impl(&pool, &archive_path)
        .await
        .map_err(|e| e.to_ipc_error())
}

/// Import a local `.zip` skill archive into Central.
///
/// The caller must pass the `expected_fingerprint` returned by preview so
/// the backend can verify the archive on disk is byte-identical to the one
/// the user confirmed. Mismatch returns `archive_changed_since_preview`
/// before any staging/Central/DB write.
#[tauri::command]
pub async fn import_local_skill_archive(
    state: State<'_, AppState>,
    archive_path: String,
    expected_fingerprint: ArchiveFingerprint,
    resolution: LocalArchiveImportResolution,
    renamed_skill_id: Option<String>,
) -> Result<LocalArchiveImportResult, String> {
    let request_context = state.resolve_target_context().await?;
    let active_target = request_context.target().clone();
    if !matches!(active_target, ActiveTarget::Local) {
        return Err(
            local_archive_import::LocalArchiveImportError::RemoteTargetUnsupported.to_ipc_error(),
        );
    }
    let target_context = target_context_from_active_target(&active_target);
    let pool = request_context.db().clone();
    let started_at = Instant::now();
    let requested_resolution = resolution.clone();
    let result = local_archive_import::import_local_skill_archive_impl(
        &pool,
        &archive_path,
        expected_fingerprint,
        resolution,
        renamed_skill_id,
    )
    .await;
    record_local_archive_import_operation(
        &pool,
        target_context,
        &result,
        &requested_resolution,
        started_at.elapsed().as_millis() as i64,
    )
    .await;
    result.map_err(|e| e.to_ipc_error())
}

fn resolution_label(resolution: &LocalArchiveImportResolution) -> &'static str {
    match resolution {
        LocalArchiveImportResolution::Overwrite => "overwrite",
        LocalArchiveImportResolution::Skip => "skip",
        LocalArchiveImportResolution::Rename => "rename",
    }
}

async fn record_local_archive_import_operation(
    pool: &DbPool,
    target_context: OperationLogTargetContext,
    result: &Result<LocalArchiveImportResult, LocalArchiveImportError>,
    requested_resolution: &LocalArchiveImportResolution,
    duration_ms: i64,
) {
    let event = match result {
        Ok(imported) => OperationLogEvent::new(
            "central",
            "local_archive.import",
            "succeeded",
            if matches!(imported.resolution, LocalArchiveImportResolution::Skip) {
                "Skipped a local skill archive import"
            } else {
                "Imported a local skill archive"
            },
        )
        .subject("skill", &imported.imported_skill_id, &imported.skill_name)
        .details(json!({
            "sourceType": "local_archive",
            "resolution": resolution_label(&imported.resolution),
            "fileCount": imported.file_count,
            "totalExpandedBytes": imported.total_expanded_bytes,
            "replacedExisting": imported.replaced_existing,
        }))
        .duration_ms(duration_ms),
        Err(error) => OperationLogEvent::new(
            "central",
            "local_archive.import",
            "failed",
            "Failed to import a local skill archive",
        )
        .error(error.code())
        .details(json!({
            "sourceType": "local_archive",
            "resolution": resolution_label(requested_resolution),
        }))
        .duration_ms(duration_ms),
    };

    record_operation_log_best_effort(pool, target_context, event).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{self, OperationLogFilter};
    use crate::operation_log::local_target_context;
    use crate::test_support;

    fn import_result() -> LocalArchiveImportResult {
        LocalArchiveImportResult {
            imported_skill_id: "demo-skill".to_string(),
            skill_name: "Demo Skill".to_string(),
            root_directory: String::new(),
            resolution: LocalArchiveImportResolution::Overwrite,
            file_count: 2,
            total_expanded_bytes: 42,
            replaced_existing: true,
        }
    }

    #[tokio::test]
    async fn operation_log_records_success_and_redacted_failure_without_payloads() {
        let pool = test_support::mem_pool().await;
        let success = Ok(import_result());
        record_local_archive_import_operation(
            &pool,
            local_target_context(),
            &success,
            &LocalArchiveImportResolution::Overwrite,
            12,
        )
        .await;

        let failure = Err(LocalArchiveImportError::InvalidArchiveEntry {
            path: "../../Users/alice/token-secret.txt".to_string(),
            reason: "password=hunter2".to_string(),
        });
        record_local_archive_import_operation(
            &pool,
            local_target_context(),
            &failure,
            &LocalArchiveImportResolution::Overwrite,
            8,
        )
        .await;

        let page = db::list_operation_logs(
            &pool,
            OperationLogFilter {
                action: Some("local_archive.import".to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("list local archive operation logs");
        assert_eq!(page.total, 2);

        let success = page
            .entries
            .iter()
            .find(|entry| entry.status == "succeeded")
            .expect("success log");
        assert_eq!(success.category, "central");
        assert_eq!(success.subject_id.as_deref(), Some("demo-skill"));
        assert_eq!(success.error_summary, None);
        let details = success.details_json.as_deref().expect("success details");
        assert!(details.contains("local_archive"));
        assert!(details.contains("overwrite"));

        let failure = page
            .entries
            .iter()
            .find(|entry| entry.status == "failed")
            .expect("failure log");
        assert_eq!(
            failure.error_summary.as_deref(),
            Some("invalid_archive_entry")
        );
        let failure_details = failure.details_json.as_deref().expect("failure details");
        assert!(failure_details.contains("local_archive"));
        assert!(failure_details.contains("overwrite"));
        let serialized = serde_json::to_string(failure).expect("serialize failure log");
        assert!(!serialized.contains("alice"));
        assert!(!serialized.contains("token-secret"));
        assert!(!serialized.contains("hunter2"));
    }

    #[test]
    fn ipc_errors_use_stable_codes_without_sensitive_payloads() {
        let path_error = LocalArchiveImportError::ArchiveNotFound(
            r"C:\Users\alice\private\skill.zip".to_string(),
        );
        let entry_error = LocalArchiveImportError::InvalidArchiveEntry {
            path: "../../token-secret.txt".to_string(),
            reason: "password=hunter2".to_string(),
        };

        for error in [path_error, entry_error] {
            let ipc = error.to_ipc_error();
            assert!(ipc.starts_with("local_archive."));
            assert!(!ipc.contains("alice"));
            assert!(!ipc.contains("token-secret"));
            assert!(!ipc.contains("hunter2"));
        }
    }
}
