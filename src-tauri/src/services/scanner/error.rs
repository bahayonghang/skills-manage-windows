//! Typed errors for the scanner domain.
//!
//! Variants cover the real failure categories of local/remote scan
//! orchestration and scan persistence. Per-file parse/IO problems are *not*
//! errors here: `parse_skill_md` / `scan_directory` skip unreadable or
//! malformed entries by design (they return `Option` / empty lists).

/// Failure categories for skill scanning.
#[derive(Debug, thiserror::Error)]
pub enum ScannerError {
    /// Direct SQLx failures from scan persistence (transactions + upserts).
    #[error(transparent)]
    Db(#[from] sqlx::Error),

    /// A scan row carried a `link_type` outside the supported vocabulary.
    #[error("{0}")]
    InvalidLinkType(String),

    /// Remote-target transport failures (connect / probe / batch read).
    #[error("{0}")]
    Remote(String),

    /// The remote scan exceeded its time budget (seconds).
    #[error("Remote skill scan timed out after {0}s.")]
    Timeout(u64),

    /// The bounded-parallelism semaphore was closed mid-scan.
    #[error("Directory scan semaphore was closed.")]
    SemaphoreClosed,

    /// A `spawn_blocking` worker failed to join.
    #[error("Failed to join {label} task: {message}")]
    TaskJoin {
        label: &'static str,
        message: String,
    },

    // TODO(C3): remove once db/repos return typed errors instead of String.
    /// Stringly-typed db/repos errors awaiting the C3 repos migration.
    #[error("{0}")]
    Other(String),
}

impl ScannerError {
    pub(crate) fn task_join(label: &'static str, message: String) -> Self {
        Self::TaskJoin { label, message }
    }
}
