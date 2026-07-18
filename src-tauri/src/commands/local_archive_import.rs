//! Tauri IPC shells for local skill archive (ZIP) import.
//!
//! Business logic lives in `crate::services::local_archive_import`. These
//! commands translate `State<AppState>` into service inputs and surface
//! typed errors as IPC error strings. The preview command never touches the
//! filesystem beyond reading the archive; the import command re-verifies
//! the fingerprint before any Central/DB mutation.

use tauri::State;

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
    let active_target = state.active_target().await?;
    // ZIP import is local-only for MVP. SSH/WSL targets must disable the
    // ZIP intent in the frontend; a stray call is rejected here.
    if !matches!(active_target, ActiveTarget::Local) {
        return Err(
            local_archive_import::LocalArchiveImportError::RemoteTargetUnsupported.to_string(),
        );
    }
    let pool = state.active_db().await?;
    local_archive_import::preview_local_skill_archive_impl(&pool, &archive_path)
        .await
        .map_err(|e| e.to_string())
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
    let active_target = state.active_target().await?;
    if !matches!(active_target, ActiveTarget::Local) {
        return Err(
            local_archive_import::LocalArchiveImportError::RemoteTargetUnsupported.to_string(),
        );
    }
    let pool = state.active_db().await?;
    local_archive_import::import_local_skill_archive_impl(
        &pool,
        &archive_path,
        expected_fingerprint,
        resolution,
        renamed_skill_id,
    )
    .await
    .map_err(|e| e.to_string())
}
