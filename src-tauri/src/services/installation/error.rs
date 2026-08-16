//! Typed errors for the installation domain.
//!
//! Variants cover the real failure categories of local / project-scoped /
//! remote skill installs and uninstalls. Display texts intentionally preserve
//! the historical string-error wording: the IPC boundary stringifies these
//! errors and the frontend shows them in toasts verbatim.

/// Failure categories for skill install / uninstall operations.
#[derive(Debug, thiserror::Error)]
pub enum InstallationError {
    /// IO failure with an operation-context prefix (e.g. "Failed to copy
    /// 'a' -> 'b': <io error>").
    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },

    /// Symlink creation failed (drives the Windows copy fallback).
    #[error("Failed to create symlink: {0}")]
    SymlinkCreate(#[source] std::io::Error),

    /// Database failures (db/repos passthrough + direct sqlx calls) flow
    /// through transparently.
    #[error(transparent)]
    Db(#[from] sqlx::Error),

    #[error(transparent)]
    CentralMutation(#[from] crate::services::central_mutation::CentralMutationError),

    #[error("installation.pending_central_recovery:Central recovery is pending for this skill.")]
    PendingCentralRecovery,

    /// Platform without symlink support.
    #[error("Symlink creation is only supported on Unix systems")]
    SymlinkUnsupported,

    #[error("Cannot install a skill to the central agent itself")]
    CentralAgentTarget,

    #[error("Cannot install a project skill to the central agent itself")]
    CentralAgentProjectTarget,

    #[error("Agent '{0}' not found")]
    AgentNotFound(String),

    #[error("Central agent not found in database")]
    CentralAgentMissing,

    #[error("Skill '{0}' not found in database")]
    SkillNotFound(String),

    #[error("Skill source not found at '{0}'")]
    SkillSourceMissing(String),

    #[error("Canonical skill not found at '{0}'")]
    CanonicalSkillMissing(String),

    #[error("Invalid file_path for skill '{0}'")]
    InvalidSkillFilePath(String),

    #[error(
        "{display_name} shares the Central Skills directory and cannot be uninstalled independently."
    )]
    SharedCentralUninstall { display_name: String },

    /// A non-replaceable entry occupies the local install slot.
    /// `entry_kind` is "real directory" or "file".
    #[error("A {entry_kind} already exists at '{path}'. Refusing to overwrite.")]
    TargetOccupied {
        entry_kind: &'static str,
        path: String,
    },

    #[error("Path '{0}' exists but is not a symlink. Refusing to delete.")]
    NotASymlink(String),

    #[error("Refusing to copy '{root}': directory tree exceeds {limit} entries.")]
    CopyEntryBudgetExceeded { root: String, limit: usize },

    #[error(
        "Refusing to copy '{root}': total copied bytes would exceed {limit} while copying '{path}'."
    )]
    CopyByteBudgetExceeded {
        root: String,
        limit: u64,
        path: String,
    },

    // ── Project-scoped installs ──────────────────────────────────────────────
    #[error("Project path '{0}' does not exist.")]
    ProjectPathMissing(String),

    #[error("Project path '{0}' is not a directory.")]
    ProjectPathNotDirectory(String),

    #[error("Agent '{0}' uses an absolute project skills directory pattern.")]
    ProjectSkillsDirAbsolute(String),

    #[error("Agent '{0}' does not define a home-relative skills directory pattern.")]
    ProjectSkillsDirNotHomeRelative(String),

    #[error("Agent '{0}' does not define a project skills directory pattern.")]
    ProjectSkillsDirUndefined(String),

    // ── Remote (SSH/WSL) installs ────────────────────────────────────────────
    #[error("Remote symlink install is disabled for this target.")]
    RemoteSymlinkDisabled,

    #[error("Remote project path '{0}' must be an absolute POSIX path.")]
    RemoteProjectPathNotAbsolute(String),

    #[error("Remote project path '{0}' does not exist.")]
    RemoteProjectPathMissing(String),

    #[error("Remote project path '{0}' is not a directory.")]
    RemoteProjectPathNotDirectory(String),

    /// Remote install slot already occupied by a non-replaceable entry
    /// (message preformatted by the classify helpers).
    #[error("{0}")]
    RemoteTargetOccupied(String),

    /// Remote-target transport failures (connect / script / inspect / copy /
    /// remove over the SSH or WSL channel).
    #[error("{0}")]
    Remote(String),

    // ── Observation row-aware uninstall ──────────────────────────────────────
    #[error("Observation row '{0}' not found")]
    ObservationRowNotFound(String),

    #[error("Observation row '{row_id}' belongs to agent '{actual}', not '{expected}'")]
    ObservationRowAgentMismatch {
        row_id: String,
        actual: String,
        expected: String,
    },

    #[error("Observation row '{row_id}' belongs to skill '{actual}', not '{expected}'")]
    ObservationRowSkillMismatch {
        row_id: String,
        actual: String,
        expected: String,
    },

    #[error("Observation row '{0}' is read-only and cannot be uninstalled")]
    ObservationRowReadOnly(String),

    #[error("Observation row '{0}' has inconsistent path identity")]
    ObservationRowPathMismatch(String),

    #[error("Refusing to delete the agent skills root '{0}'")]
    ObservationRootDeletion(String),

    #[error("Path '{0}' has no parent")]
    PathHasNoParent(String),

    #[error("Refusing to delete '{child}' because it is outside the agent skills root '{root}'")]
    OutsideObservationRoot { child: String, root: String },

    /// A `spawn_blocking` worker failed to join.
    #[error("Failed to join {label} task: {message}")]
    TaskJoin {
        label: &'static str,
        message: String,
    },
}

impl InstallationError {
    /// Build an [`InstallationError::Io`] with an operation-context prefix.
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
