//! Typed errors for the portable SkillPort state domain.
//!
//! Variants cover the real failure categories of manifest parsing, state
//! export, import preview, and GitHub-backed import orchestration. Display
//! texts intentionally preserve the historical string-error wording: the IPC
//! boundary stringifies these errors and the frontend shows them in toasts
//! verbatim.
//!
//! Per-skill import failures stay as `error: String` fields inside the IPC
//! payload types (`SkillportStateImportFailure`, `RemoteCatalogEntry`); only
//! whole-operation failures use this enum.

use crate::services::github_import::GithubImportError;
use crate::services::resource_budget::BudgetExceeded;

use super::types::PORTABILITY_CANCELLED_MESSAGE;

/// Failure categories for portable SkillPort state operations.
#[derive(Debug, thiserror::Error)]
pub enum PortableStateError {
    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },

    /// Direct sqlx calls (registry queries) and db/repos calls flow through
    /// transparently.
    #[error(transparent)]
    Db(#[from] sqlx::Error),

    /// Errors propagated from the GitHub import domain (PAT auth, repo
    /// inspection, partial imports).
    #[error(transparent)]
    GithubImport(#[from] GithubImportError),

    /// Manifest serialization failure during export.
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error("Invalid SkillPort state JSON: {0}")]
    InvalidManifestJson(#[source] serde_json::Error),

    #[error("SkillPort state file must use the .json extension: {0}")]
    InvalidFileExtension(String),

    #[error("SkillPort state import path is not a regular file: {0}")]
    NotRegularFile(String),

    #[error("SkillPort state file changed while it was being read: {0}")]
    FileChangedDuringRead(String),

    #[error("SkillPort state file '{path}' is not valid UTF-8: {source}")]
    InvalidUtf8 {
        path: String,
        #[source]
        source: std::string::FromUtf8Error,
    },

    #[error("{0}")]
    Budget(BudgetExceeded),

    #[error("Unsupported SkillPort state export kind")]
    UnsupportedExportKind,

    #[error("Unsupported SkillPort state export version: {0}")]
    UnsupportedExportVersion(u32),

    /// The user cancelled the running portability operation.
    #[error("{}", PORTABILITY_CANCELLED_MESSAGE)]
    Cancelled,

    #[error("Failed to join {label} task: {message}")]
    TaskJoin {
        label: &'static str,
        message: String,
    },
}

impl PortableStateError {
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
