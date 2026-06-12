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

use super::types::PORTABILITY_CANCELLED_MESSAGE;

/// Failure categories for portable SkillPort state operations.
#[derive(Debug, thiserror::Error)]
pub enum PortableStateError {
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

    #[error("Unsupported SkillPort state export kind")]
    UnsupportedExportKind,

    #[error("Unsupported SkillPort state export version: {0}")]
    UnsupportedExportVersion(u32),

    /// The user cancelled the running portability operation.
    #[error("{}", PORTABILITY_CANCELLED_MESSAGE)]
    Cancelled,
}
