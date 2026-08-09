//! Typed errors for the Central Skills domain.
//!
//! Variants cover the real failure categories of skill detail hydration,
//! Central skill / repository deletion (local and remote), and the file /
//! directory-tree access guards. Display texts intentionally preserve the
//! historical string-error wording: the IPC boundary stringifies these
//! errors and the frontend shows them in toasts verbatim.

/// Failure categories for Central Skills operations.
#[derive(Debug, thiserror::Error)]
pub enum CentralSkillsError {
    /// IO failure with an operation-context prefix (e.g. "Failed to remove
    /// skill directory 'x': <io error>").
    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },

    /// Database failures (db/repos passthrough + direct sqlx calls) flow
    /// through transparently.
    #[error(transparent)]
    Db(#[from] sqlx::Error),

    #[error(transparent)]
    CentralMutation(#[from] crate::services::central_mutation::CentralMutationError),

    #[error(transparent)]
    CentralOperation(#[from] crate::services::central_operation::CentralOperationError),

    #[error("A pending Central update could not be recovered before deletion.")]
    UpdateRecovery {
        error_code: String,
        error_category: &'static str,
    },

    /// Remote-target transport failures (connect / inspect / read / list /
    /// remove over the SSH or WSL channel; targets module returns String).
    #[error("{0}")]
    Remote(String),

    /// Resource-budget violation (typed `BudgetExceeded` from the
    /// resource_budget module; Display is the budget message verbatim).
    #[error("{0}")]
    Budget(crate::services::resource_budget::BudgetExceeded),

    // ── Skill detail hydration ───────────────────────────────────────────────
    #[error("Source row '{row_id}' not found for skill '{skill_id}'")]
    SourceRowNotFound { row_id: String, skill_id: String },

    #[error("Multiple source rows found for skill '{0}'; row_id is required")]
    MultipleSourceRows(String),

    #[error("Skill '{0}' not found")]
    SkillNotFound(String),

    #[error("Multiple skills are named '{0}'; use uid or slug")]
    AmbiguousSkillReference(String),

    #[error("Central skills {field} filter accepts at most {limit} unique values")]
    PageFilterValuesExceeded { field: &'static str, limit: usize },

    // ── Central skill deletion ───────────────────────────────────────────────
    #[error("Skill '{0}' is not a Central skill")]
    NotCentralSkill(String),

    #[error("Central agent not found in database")]
    CentralAgentMissing,

    #[error("Agent '{0}' not found")]
    AgentNotFound(String),

    #[error("Skill '{0}' has no canonical directory")]
    SkillNoCanonicalDir(String),

    #[error("Skill '{skill_id}' is not installed for '{agent_id}'")]
    SkillNotInstalledForAgent { skill_id: String, agent_id: String },

    #[error("Only copy installations can be selected for platform deletion: {0}")]
    OnlyCopyInstallationsDeletable(String),

    #[error("Refusing to delete the Central Skills root for {0}")]
    CentralRootDeleteRefused(String),

    #[error("Refusing to delete '{path}' because it is outside Central Skills root '{root}'")]
    OutsideCentralRoot { path: String, root: String },

    #[error("Path '{0}' is not a directory. Refusing to delete.")]
    NotADirectoryDeleteRefused(String),

    #[error("Path '{0}' is not a managed copy. Refusing to delete.")]
    NotAManagedCopy(String),

    #[error("Path '{0}' is not a directory or symlink. Refusing to delete.")]
    NotDirectoryOrSymlink(String),

    // ── Remote deletion path guards ──────────────────────────────────────────
    #[error("Invalid remote path '{0}'")]
    InvalidRemotePath(String),

    #[error("Remote path '{0}' contains traversal")]
    RemotePathTraversal(String),

    #[error("Refusing to delete under remote root for {0}")]
    RemoteRootDeleteRefused(String),

    #[error("Refusing to delete the remote root '{root}' for {label}")]
    RemoteRootDeletion { root: String, label: String },

    #[error("Refusing to delete '{path}' because it is outside remote root '{root}'")]
    OutsideRemoteRoot { path: String, root: String },

    // ── Repository deletion ──────────────────────────────────────────────────
    #[error("Repository '{0}' not found")]
    RepositoryNotFound(String),

    #[error("The system unknown-source repository cannot be deleted")]
    UnknownRepositoryUndeletable,

    #[error("Skill '{skill_id}' does not belong to repository '{repository_id}'")]
    SkillNotInRepository {
        skill_id: String,
        repository_id: String,
    },

    // ── File / directory-tree access ─────────────────────────────────────────
    #[error("Remote file '{path}' is not valid UTF-8: {source}")]
    RemoteFileNotUtf8 {
        path: String,
        #[source]
        source: std::string::FromUtf8Error,
    },

    #[error("{target} skill file is not valid UTF-8.")]
    SkillFileNotUtf8 { target: &'static str },

    #[error("Path is not a file: {0}")]
    NotAFile(String),

    #[error("Path is not a directory: {0}")]
    NotADirectory(String),

    #[error("Path does not exist: {0}")]
    PathMissing(String),

    #[error("Refusing to traverse '{path}': directory tree exceeds {limit} entries.")]
    TreeEntriesExceeded { path: String, limit: usize },

    #[error("Refusing to traverse '{path}': directory depth exceeds {limit}.")]
    TreeDepthExceeded { path: String, limit: usize },

    #[error("Remote path '{0}' does not exist.")]
    RemotePathMissing(String),

    #[error("Remote path '{0}' is not a directory.")]
    RemotePathNotDirectory(String),

    #[error("Remote path '{0}' is not a file.")]
    RemotePathNotFile(String),

    #[error(
        "Remote paths cannot be opened in the local file manager. Copy the remote path instead."
    )]
    RemoteOpenInFileManagerUnsupported,

    // ── Skill-root path guards ───────────────────────────────────────────────
    #[error("Allowed skill root '{0}' is not a directory.")]
    SkillRootNotDirectory(String),

    #[error("Refusing to access '{path}': path escapes skill root '{root}'.")]
    PathEscapesSkillRoot { path: String, root: String },

    #[error("Skill path context is empty.")]
    SkillPathContextEmpty,

    #[error("Refusing to access '{0}': backslashes are not allowed in remote paths.")]
    RemotePathBackslash(String),

    #[error("Refusing to access '{0}': parent traversal is not allowed.")]
    RemoteParentTraversal(String),

    #[error("Failed to resolve the remote skill root.")]
    RemoteCanonicalRootResolution,

    #[error("The resolved remote skill root is not a directory.")]
    RemoteCanonicalRootNotDirectory,

    #[error("Failed to resolve the requested remote skill path.")]
    RemoteCanonicalCandidateResolution,

    #[error("Refusing to access a remote path outside the resolved skill root.")]
    RemoteCanonicalEscape,

    #[error("Remote canonical path resolution is unavailable.")]
    RemoteCanonicalResolverUnavailable,

    #[error("Remote canonical path resolution returned an invalid response.")]
    RemoteCanonicalProtocol,

    #[error("Remote canonical path resolution failed.")]
    RemoteCanonicalResolution,

    /// A `spawn_blocking` worker failed to join.
    #[error("Failed to join {label} task: {message}")]
    TaskJoin {
        label: &'static str,
        message: String,
    },
}

impl CentralSkillsError {
    /// Build a [`CentralSkillsError::Io`] with an operation-context prefix.
    pub(crate) fn io(context: impl Into<String>, source: std::io::Error) -> Self {
        Self::Io {
            context: context.into(),
            source,
        }
    }

    pub(crate) fn task_join(label: &'static str, message: String) -> Self {
        Self::TaskJoin { label, message }
    }

    pub(crate) fn stable_delete_error_code(&self) -> String {
        match self {
            Self::CentralOperation(error) => format!("central_operation.{}", error.code()),
            Self::UpdateRecovery { error_code, .. } => error_code.clone(),
            Self::CentralMutation(_) => "central_skills.mutation_lock_failed".to_string(),
            Self::Db(_) => "central_skills.database_failed".to_string(),
            Self::Remote(_) => "central_skills.remote_failed".to_string(),
            Self::Budget(_) => "central_skills.budget_exceeded".to_string(),
            _ => "central_skills.delete_failed".to_string(),
        }
    }

    pub(crate) const fn diagnostic_category(&self) -> &'static str {
        match self {
            Self::Io { .. } => "central_skills.io",
            Self::Db(_) => "central_skills.db",
            Self::CentralMutation(_) => "central_skills.central_mutation",
            Self::CentralOperation(_) => "central_skills.central_operation",
            Self::UpdateRecovery { error_category, .. } => error_category,
            Self::Remote(_) => "central_skills.remote",
            Self::Budget(_) => "central_skills.budget",
            _ => "central_skills.validation",
        }
    }

    pub(crate) const fn public_delete_message(&self) -> &'static str {
        "This Central skill could not be deleted."
    }

    pub(crate) const fn delete_failure_phase(&self) -> &'static str {
        match self {
            Self::CentralMutation(_) => "mutation_lock",
            Self::Db(_) | Self::CentralOperation(_) | Self::UpdateRecovery { .. } => "recovery",
            _ => "prepare",
        }
    }
}
