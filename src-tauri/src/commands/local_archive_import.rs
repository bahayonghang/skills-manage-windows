//! Tauri IPC shells for local skill archive (ZIP) import.
//!
//! Business logic lives in `crate::services::local_archive_import`. These
//! commands translate `State<AppState>` into service inputs and surface
//! typed errors as IPC error strings. The preview command never touches the
//! filesystem beyond reading the archive; the import command re-verifies
//! the fingerprint before any Central/DB mutation.

use tauri::State;

use crate::ipc_error::{public_message_for_code, IpcError, REVIEWED_IPC_ERROR_CODES};
use crate::observability::{
    CommandLogPolicy, OperationContext, OperationDefinition, OperationTarget, ReviewedDiagnostic,
    ReviewedFailure, SafeDetailKey, SafeIdentifier, SafeOperationResult,
};
use crate::services::local_archive_import;
use crate::targets::ActiveTarget;
use crate::AppState;

pub use crate::services::local_archive_import::{
    ArchiveFingerprint, LocalArchiveImportError, LocalArchiveImportResolution,
    LocalArchiveImportResult, LocalArchivePreview, LocalSkillConflict,
};

fn operation_definition() -> OperationDefinition {
    match crate::ipc_registry::command_policy("import_local_skill_archive")
        .expect("local archive import command must be registered")
        .policy
    {
        CommandLogPolicy::Operation(definition) => definition,
        _ => panic!("local archive import must use Operation policy"),
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

/// Preview a local `.zip` skill archive without writing anything.
///
/// Returns the archive fingerprint, resolved skill candidate, file tree,
/// and any Central conflict. The frontend never receives absolute user
/// paths; only the archive basename and relative paths are exposed.
#[tauri::command]
pub async fn preview_local_skill_archive(
    state: State<'_, AppState>,
    archive_path: String,
) -> crate::ipc_error::IpcResult<LocalArchivePreview> {
    crate::ipc_boundary!(
        "preview_local_skill_archive",
        async move {
            let request_context = state
                .resolve_target_context()
                .await
                .map_err(IpcError::from)?;
            let active_target = request_context.target().clone();
            // ZIP import is local-only for MVP. SSH/WSL targets must disable the
            // ZIP intent in the frontend; a stray call is rejected here.
            if !matches!(active_target, ActiveTarget::Local) {
                return Err(IpcError::from(
                    local_archive_import::LocalArchiveImportError::RemoteTargetUnsupported
                        .to_ipc_error(),
                ));
            }
            let pool = request_context.db().clone();
            local_archive_import::preview_local_skill_archive_impl(&pool, &archive_path)
                .await
                .map_err(|error| IpcError::from(error.to_ipc_error()))
        }
        .await
    )
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
) -> crate::ipc_error::IpcResult<LocalArchiveImportResult> {
    crate::ipc_boundary!(
        "import_local_skill_archive",
        async move {
            let request_context = state
                .resolve_target_context()
                .await
                .map_err(IpcError::from)?;
            let active_target = request_context.target().clone();
            if !matches!(active_target, ActiveTarget::Local) {
                return Err(IpcError::from(
                    local_archive_import::LocalArchiveImportError::RemoteTargetUnsupported
                        .to_ipc_error(),
                ));
            }
            let pool = request_context.db().clone();
            let definition = operation_definition();
            crate::observability::run_operation(
                &state,
                definition,
                OperationContext::new(OperationTarget::local()),
                |result: &LocalArchiveImportResult| {
                    let base = if matches!(result.resolution, LocalArchiveImportResolution::Skip) {
                        SafeOperationResult::partial("Skipped the local skill archive import.")
                    } else {
                        SafeOperationResult::succeeded("Imported a local skill archive.")
                    };
                    base.identifier(
                        SafeDetailKey::Identifier,
                        SafeIdentifier::new(&result.imported_skill_id),
                    )
                    .count(SafeDetailKey::AffectedCount, result.file_count as u64)
                    .flag(SafeDetailKey::Changed, result.replaced_existing)
                    .stable(SafeDetailKey::Mode, resolution_label(&result.resolution))
                },
                || async move {
                    local_archive_import::import_local_skill_archive_impl(
                        &pool,
                        &archive_path,
                        expected_fingerprint,
                        resolution,
                        renamed_skill_id,
                    )
                    .await
                    .map_err(|error| {
                        reviewed_failure(definition, IpcError::from(error.to_ipc_error()))
                    })
                },
            )
            .await
        }
        .await
    )
}

fn resolution_label(resolution: &LocalArchiveImportResolution) -> &'static str {
    match resolution {
        LocalArchiveImportResolution::Overwrite => "overwrite",
        LocalArchiveImportResolution::Skip => "skip",
        LocalArchiveImportResolution::Rename => "rename",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
