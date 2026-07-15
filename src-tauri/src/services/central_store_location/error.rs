//! Typed errors for the central store location domain.
//!
//! The five `central_store_location_*` sentinel codes are matched verbatim by
//! the frontend for i18n messages, and the remaining Display texts flow into
//! toasts unchanged — keep every format string byte-identical.

use crate::services::installation::InstallationError;
use crate::services::scanner::ScannerError;

/// Failure categories for central store location preview / apply.
#[derive(Debug, thiserror::Error)]
pub enum CentralStoreLocationError {
    /// IO failure with an operation-context prefix.
    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },

    /// Database failures (db/repos passthrough + direct sqlx calls).
    #[error(transparent)]
    Db(#[from] sqlx::Error),

    #[error(transparent)]
    CentralMutation(#[from] crate::services::central_mutation::CentralMutationError),

    /// Skill directory copy / symlink creation via the installation domain.
    #[error(transparent)]
    Installation(#[from] InstallationError),

    /// Post-migration full rescan.
    #[error(transparent)]
    Scanner(#[from] ScannerError),

    // ── Frontend sentinel codes (matched verbatim for i18n) ─────────────────
    #[error("central_store_location_unsupported_target")]
    UnsupportedTarget,

    #[error("central_store_location_requires_overwrite")]
    RequiresOverwrite,

    #[error("central_store_location_empty_path")]
    EmptyPath,

    #[error("central_store_location_same_path")]
    SamePath,

    #[error("central_store_location_nested_path")]
    NestedPath,

    #[error("Central agent not found")]
    CentralAgentNotFound,

    #[error("'{0}' is not a symlink")]
    NotASymlink(String),

    #[error("Invalid project symlink owner id")]
    InvalidSymlinkOwner,

    /// Relative-path computation against the old central root failed
    /// (`std::path::StripPrefixError` display, preformatted at the call site).
    #[error("{0}")]
    PathPrefix(String),

    /// A `spawn_blocking` worker failed to join.
    #[error("Failed to join {label} task: {message}")]
    TaskJoin {
        label: &'static str,
        message: String,
    },
}

impl CentralStoreLocationError {
    /// Build an [`CentralStoreLocationError::Io`] with an operation-context prefix.
    pub(crate) fn io(context: impl Into<String>, source: std::io::Error) -> Self {
        Self::Io {
            context: context.into(),
            source,
        }
    }

    pub(crate) fn task_join(label: &'static str, message: String) -> Self {
        Self::TaskJoin { label, message }
    }
}
