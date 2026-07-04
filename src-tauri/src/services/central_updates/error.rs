//! Typed errors for the central updates domain (Update Center).
//!
//! Display texts intentionally preserve the historical string-error wording:
//! the IPC boundary stringifies these errors, the frontend shows them in
//! toasts verbatim, and several of them are also persisted as
//! `SkillUpdateState.error` reasons — keep every format string byte-identical.

use crate::services::central_skills::CentralSkillsError;
use crate::services::github_import::GithubImportError;
use crate::services::installation::InstallationError;

/// Failure categories for Central skill update checks, updates, repository
/// sync, and the update-inventory panel.
#[derive(Debug, thiserror::Error)]
pub enum CentralUpdatesError {
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

    /// GitHub snapshot download / auth / repo resolution via the
    /// github_import domain.
    #[error(transparent)]
    GithubImport(#[from] GithubImportError),

    /// Local copy refresh via the installation domain.
    #[error(transparent)]
    Installation(#[from] InstallationError),

    /// Central skill batch deletes via the central_skills domain.
    #[error(transparent)]
    CentralSkills(#[from] CentralSkillsError),

    /// Remote-target transport failures (connect / script / list / read over
    /// the SSH or WSL channel), stringified at the call site.
    #[error("{0}")]
    Remote(String),

    // ── Update selection / state validation ─────────────────────────────────
    #[error("Select at least one Central skill to update.")]
    NoUpdateSelection,

    #[error("Skill '{0}' not found")]
    SkillNotFound(String),

    #[error("Skill '{0}' is not a Central skill")]
    NotCentralSkill(String),

    #[error("Skill '{0}' does not have a remote-missing update state.")]
    NoRemoteMissingState(String),

    #[error("Skill '{0}' is not marked as removed remotely.")]
    NotRemoteMissing(String),

    #[error("GitHub source for skill '{0}' is missing repository URL.")]
    MissingRepositoryUrl(String),

    #[error("Skill '{0}' has no canonical directory.")]
    NoCanonicalDirectory(String),

    #[error("Central update snapshot downloader closed.")]
    SnapshotDownloaderClosed,

    // ── Remote skill content / repo paths ───────────────────────────────────
    #[error("Repository path '{0}' is not supported.")]
    UnsupportedRepoPath(String),

    #[error("Repository path '{0}' is no longer available.")]
    RepoPathUnavailable(String),

    #[error("Remote skill no longer contains SKILL.md.")]
    RemoteManifestMissing,

    #[error("Repository contains an unsupported path '{0}'.")]
    UnsupportedRepoFilePath(String),

    // ── Atomic skill directory writes ────────────────────────────────────────
    #[error("Failed to determine parent directory for '{0}'.")]
    NoParentDirectory(String),

    #[error("Skill '{0}' target directory has no parent.")]
    TargetDirNoParent(String),

    #[error("Skill '{skill_id}' target directory '{target_dir}' has no parent.")]
    RemoteTargetDirNoParent {
        skill_id: String,
        target_dir: String,
    },

    #[error("Updated skill directory, but failed to remove backup '{path}': {message}")]
    BackupCleanup { path: String, message: String },

    #[error("Refusing to refresh copy install outside expected skill directory '{0}'.")]
    CopyInstallOutsideSkillDir(String),

    // ── Directory hashing ────────────────────────────────────────────────────
    /// Malformed remote hash-script output (message preformatted at the call
    /// site to keep the historical texts).
    #[error("{0}")]
    RemoteHashOutput(String),

    #[error("Remote skill path '{0}' contains unsupported components.")]
    UnsupportedRemotePath(String),

    #[error("Remote path '{child}' is not under expected root '{root}'.")]
    RemotePathOutsideRoot { child: String, root: String },

    #[error("Local skill path '{0}' contains unsupported components.")]
    UnsupportedLocalPath(String),

    #[error("Failed to compute relative path for '{path}': {message}")]
    RelativePath { path: String, message: String },

    /// A `spawn_blocking` worker failed to join.
    #[error("Failed to join {label} task: {message}")]
    TaskJoin {
        label: &'static str,
        message: String,
    },
}

impl CentralUpdatesError {
    /// Build an [`CentralUpdatesError::Io`] with an operation-context prefix.
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
