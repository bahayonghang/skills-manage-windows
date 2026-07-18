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
/// layer surfaces; messages may wrap lower-level causes but never include
/// absolute user-directory paths.
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
            Self::RemoteTargetUnsupported => "remote_target_unsupported",
            Self::Internal(_) => "internal",
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
