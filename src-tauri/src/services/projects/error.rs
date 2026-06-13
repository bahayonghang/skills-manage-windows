//! Typed errors for the projects domain.
//!
//! Variants cover the real failure categories of project CRUD, project
//! scanning, and project-scoped skill install/uninstall. Display texts
//! intentionally preserve the historical string-error wording: the IPC
//! boundary stringifies these errors and the frontend shows them in toasts
//! verbatim.

use crate::services::installation::InstallationError;

/// Failure categories for project-level skill management.
#[derive(Debug, thiserror::Error)]
pub enum ProjectsError {
    /// IO failure with an operation-context prefix (e.g. "Failed to remove
    /// symlink 'x': <io error>").
    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },

    /// Direct sqlx calls (skill metadata fallback, reverse lookup) flow
    /// through transparently.
    #[error(transparent)]
    Db(#[from] sqlx::Error),

    /// Errors propagated from the installation domain (relative skills dir,
    /// target replacement guard, copy, symlink creation).
    #[error(transparent)]
    Installation(#[from] InstallationError),

    #[error("Project path cannot be empty")]
    ProjectPathEmpty,

    #[error("Project path '{0}' does not exist or is not a directory")]
    ProjectPathInvalid(String),

    #[error("Project path '{0}' is missing or not a directory")]
    ProjectPathMissingOrNotDir(String),

    #[error("Project name cannot be empty")]
    ProjectNameEmpty,

    #[error("Project '{0}' not found")]
    ProjectNotFound(String),

    #[error("Cannot install a project skill to the central agent itself")]
    CentralAgentProjectTarget,

    #[error("Agent '{0}' not found")]
    AgentNotFound(String),

    #[error("Agent '{0}' is disabled")]
    AgentDisabled(String),

    #[error("Skill '{0}' not found in central library")]
    SkillNotFoundInCentral(String),

    #[error("Skill '{0}' is not centralized; centralize it before installing to a project")]
    SkillNotCentralized(String),

    #[error("Skill '{0}' has no canonical_path; cannot install to a project")]
    SkillNoCanonicalPath(String),

    #[error("Central skill directory '{0}' does not exist")]
    CentralSkillDirMissing(String),

    #[error(
        "Skill '{skill_id}' is not installed in project '{project_id}' for agent '{agent_id}'"
    )]
    SkillNotInstalledInProject {
        skill_id: String,
        project_id: String,
        agent_id: String,
    },

    /// A `spawn_blocking` worker failed to join.
    #[error("Failed to join {label} task: {message}")]
    TaskJoin {
        label: &'static str,
        message: String,
    },
}

impl ProjectsError {
    /// Build a [`ProjectsError::Io`] with an operation-context prefix.
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
