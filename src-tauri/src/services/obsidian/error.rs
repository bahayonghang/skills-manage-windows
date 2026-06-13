//! Typed errors for the Obsidian vault domain.
//!
//! Variants cover the real failure categories of vault discovery and
//! source-import (to central or to a platform). Display texts intentionally
//! preserve the historical string-error wording: the IPC boundary stringifies
//! these errors and the frontend shows them in toasts verbatim.

use crate::services::installation::InstallationError;

/// Failure categories for Obsidian vault scanning and skill import.
#[derive(Debug, thiserror::Error)]
pub enum ObsidianError {
    /// IO failure with an operation-context prefix (e.g. "Failed to create
    /// agent skills directory: <io error>").
    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },

    /// Skill / installation upserts flow through transparently.
    #[error(transparent)]
    Db(#[from] sqlx::Error),

    /// Errors propagated from the installation domain (directory copy,
    /// symlink creation).
    #[error(transparent)]
    Installation(#[from] InstallationError),

    #[error("Unsupported install method '{0}'")]
    UnsupportedInstallMethod(String),

    #[error("Cannot extract skill directory name")]
    SkillDirNameUnavailable,

    #[error("Skill source directory '{0}' does not exist.")]
    SourceDirMissing(String),

    #[error("A skill named '{0}' already exists in central skills")]
    CentralSkillExists(String),

    #[error("Agent '{0}' not found")]
    AgentNotFound(String),

    #[error("Skill '{skill}' already exists in {agent}")]
    SkillExistsInAgent { skill: String, agent: String },

    #[error("Obsidian vault '{0}' not found")]
    VaultNotFound(String),

    /// A `spawn_blocking` worker failed to join.
    #[error("Failed to join {label} task: {message}")]
    TaskJoin {
        label: &'static str,
        message: String,
    },
}

impl ObsidianError {
    /// Build an [`ObsidianError::Io`] with an operation-context prefix.
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
