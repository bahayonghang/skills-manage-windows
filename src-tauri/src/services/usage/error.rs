//! Typed errors for the skill-usage domain.
//!
//! Variants cover the real failure categories of usage scanning: filesystem
//! backends (local + remote SSH/WSL), provider log collection, the OpenCode
//! SQLite reader, and skill-call persistence. Display texts intentionally
//! preserve the historical string-error wording: the IPC boundary stringifies
//! these errors and the frontend shows them in toasts verbatim.

/// Failure categories for skill-usage scanning and aggregation.
#[derive(Debug, thiserror::Error)]
pub enum UsageError {
    /// IO failure with an operation-context prefix (e.g. "local read x:
    /// <io error>").
    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },

    /// skill_calls / provider-health persistence and direct sqlx calls flow
    /// through transparently.
    #[error(transparent)]
    Db(#[from] sqlx::Error),

    /// Remote-target transport failures (read / command / listing over the
    /// SSH or WSL channel; targets module returns String).
    #[error("{0}")]
    Remote(String),

    /// Decoding failure for fetched log content (UTF-8 checks). Message
    /// preformatted at the call site.
    #[error("{0}")]
    Parse(String),

    /// Read-only open of the OpenCode SQLite database failed.
    #[error("opencode db open: {0}")]
    OpenCodeDbOpen(#[source] sqlx::Error),

    /// Skill-call query against the OpenCode SQLite database failed.
    #[error("opencode query: {0}")]
    OpenCodeQuery(#[source] sqlx::Error),

    /// A `spawn_blocking` worker failed to join.
    #[error("Failed to join {label} task: {message}")]
    TaskJoin {
        label: &'static str,
        message: String,
    },
}

impl UsageError {
    /// Build a [`UsageError::Io`] with an operation-context prefix.
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
