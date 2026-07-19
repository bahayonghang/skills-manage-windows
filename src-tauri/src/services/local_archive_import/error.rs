//! Typed errors for local skill archive import.
//!
//! The archive import pipeline must fail closed at well-defined boundaries
//! (inventory, candidate discovery, fingerprint verification, staging, and
//! Central mutation). Each variant carries enough signal for the frontend to
//! render a localized error and for `Operation Log` to persist a redacted
//! summary without leaking absolute user paths.

use crate::services::resource_budget::BudgetExceeded;
use std::io;
use std::path::Path;

/// Local skill archive import error envelope.
///
/// The `code` string is stable for each variant and is what the frontend IPC
/// layer surfaces. Internal Display messages may retain diagnostic context;
/// commands must use [`LocalArchiveImportError::to_ipc_error`] so those
/// details never cross IPC or enter user-visible text.
#[derive(Debug, thiserror::Error)]
pub enum LocalArchiveImportError {
    #[error("archive_not_found: {0}")]
    ArchiveNotFound(String),

    #[error("archive_read_failed: {0}")]
    ArchiveReadFailed(String),

    #[error("archive_changed_since_preview: fingerprint mismatch ({detail})")]
    ArchiveChangedSincePreview { detail: String },

    #[error("ambiguous_archive_layout: {0}")]
    AmbiguousArchiveLayout(String),

    #[error("no_skill_manifest: {0}")]
    NoSkillManifest(String),

    #[error("invalid_archive_entry: {path}: {reason}")]
    InvalidArchiveEntry { path: String, reason: String },

    #[error("unsupported_archive_entry: {path}: {reason}")]
    UnsupportedArchiveEntry { path: String, reason: String },

    #[error("budget_exceeded: {0}")]
    BudgetExceeded(#[from] BudgetExceeded),

    #[error("path_conflict: {0}")]
    PathConflict(String),

    #[error("skill_frontmatter_missing: {0}")]
    SkillFrontmatterMissing(String),

    #[error("invalid_skill_identifier: {0}")]
    InvalidSkillIdentifier(String),

    #[error("io: {0}")]
    Io(#[from] io::Error),

    #[error("db: {0}")]
    Db(#[from] sqlx::Error),

    #[error("central_mutation: {0}")]
    CentralMutation(#[from] crate::services::central_mutation::CentralMutationError),

    #[error("rollback_failed: {stage}: {source}")]
    RollbackFailed {
        stage: &'static str,
        #[source]
        source: io::Error,
    },

    #[error("remote_target_unsupported: local ZIP import is disabled for active remote target")]
    RemoteTargetUnsupported,

    #[error("internal: {0}")]
    Internal(String),
}

impl LocalArchiveImportError {
    /// Stable error code used by the IPC layer and frontend. Never reveals
    /// absolute paths or user directory names.
    pub fn code(&self) -> &'static str {
        match self {
            Self::ArchiveNotFound(_) => "archive_not_found",
            Self::ArchiveReadFailed(_) => "archive_read_failed",
            Self::ArchiveChangedSincePreview { .. } => "archive_changed_since_preview",
            Self::AmbiguousArchiveLayout(_) => "ambiguous_archive_layout",
            Self::NoSkillManifest(_) => "no_skill_manifest",
            Self::InvalidArchiveEntry { .. } => "invalid_archive_entry",
            Self::UnsupportedArchiveEntry { .. } => "unsupported_archive_entry",
            Self::BudgetExceeded(_) => "budget_exceeded",
            Self::PathConflict(_) => "path_conflict",
            Self::SkillFrontmatterMissing(_) => "skill_frontmatter_missing",
            Self::InvalidSkillIdentifier(_) => "invalid_skill_identifier",
            Self::Io(_) => "io",
            Self::Db(_) => "db",
            Self::CentralMutation(_) => "central_mutation",
            Self::RollbackFailed { .. } => "rollback_failed",
            Self::RemoteTargetUnsupported => "remote_target_unsupported",
            Self::Internal(_) => "internal",
        }
    }

    /// Serialize a locale-neutral IPC error without attacker-controlled or
    /// machine-local details. The frontend maps the code through i18n.
    pub fn to_ipc_error(&self) -> String {
        format!("local_archive.{}:{}", self.code(), self.safe_summary())
    }

    fn safe_summary(&self) -> &'static str {
        match self {
            Self::ArchiveNotFound(_) => "The selected archive was not found.",
            Self::ArchiveReadFailed(_) => "The selected archive could not be read.",
            Self::ArchiveChangedSincePreview { .. } => {
                "The archive changed after it was previewed."
            }
            Self::AmbiguousArchiveLayout(_) => "The archive layout is ambiguous.",
            Self::NoSkillManifest(_) => "The archive does not contain an importable skill.",
            Self::InvalidArchiveEntry { .. } => "The archive contains an unsafe path.",
            Self::UnsupportedArchiveEntry { .. } => "The archive contains an unsupported entry.",
            Self::BudgetExceeded(_) => "The archive exceeds the import resource limits.",
            Self::PathConflict(_) => "The selected destination conflicts with existing data.",
            Self::SkillFrontmatterMissing(_) => "The skill manifest metadata is invalid.",
            Self::InvalidSkillIdentifier(_) => "The requested skill identifier is invalid.",
            Self::Io(_) => "A filesystem operation failed.",
            Self::Db(_) => "The skill database could not be updated.",
            Self::CentralMutation(_) => "Central is busy with another change.",
            Self::RollbackFailed { .. } => "The previous skill could not be restored.",
            Self::RemoteTargetUnsupported => "Local ZIP import is unavailable for remote targets.",
            Self::Internal(_) => "The local archive import failed.",
        }
    }
}

/// Helper for converting an IO error with context.
#[allow(dead_code)]
pub(crate) fn io_context(context: impl Into<String>, error: io::Error) -> LocalArchiveImportError {
    LocalArchiveImportError::Internal(format!("{}: {}", context.into(), error))
}

/// Join-error constructor for [`crate::fs_util::run_blocking_fs_with`].
///
/// Maps a failed `spawn_blocking` join into a typed import error without
/// leaking absolute paths: only the stable label and the join error message
/// are preserved.
pub(crate) fn task_join(label: &'static str, error: String) -> LocalArchiveImportError {
    LocalArchiveImportError::Internal(format!("blocking fs task '{label}' join failed: {error}"))
}

/// Helper: the caller wants to remember a staging path for cleanup without
/// surfacing it in error messages. This is purely an internal convenience.
#[allow(dead_code)]
pub(crate) fn staging_path_display(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}
