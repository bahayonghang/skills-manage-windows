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

    #[error("{0}")]
    CentralMutation(String),

    #[error(transparent)]
    CentralOperation(#[from] crate::services::central_operation::CentralOperationError),

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

    /// Batch archive/protocol failures that are not tied to one IO source.
    #[error("{0}")]
    Batch(String),

    #[error("Central update was cancelled.")]
    BatchCancelled,

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

    // ── Update inventory: force rescue actions ──────────────────────────────
    #[error("Select at least one Central skill to force update.")]
    NoForceUpdateSelection,

    #[error("Select at least one force mirror operation.")]
    NoMirrorOperation,

    #[error("Select at least one repository to force mirror.")]
    NoMirrorRepositorySelection,

    /// Inventory payload (de)serialization failures, stringified at the call
    /// site (serde_json::Error display).
    #[error("{0}")]
    Json(String),

    /// The service produced duplicate logical keys for one inventory run.
    /// No dynamic key data is retained because this variant crosses the
    /// operation-log and IPC classification boundaries.
    #[error("Update inventory contains duplicate entry keys.")]
    InventoryInvariant,

    /// A pending addition predates immutable inventory provenance or mixes
    /// identities from different refreshes. The user must refresh before any
    /// repository content can be imported safely.
    #[error("The update inventory must be refreshed before importing this repository.")]
    InventoryRefreshRequired,

    /// Pinned reacquisition returned bytes that do not match the digest which
    /// produced the pending inventory. Fail closed before Central mutation.
    #[error("The repository snapshot no longer matches the update inventory.")]
    SnapshotChanged,

    // ── Update inventory: platform-copy removal safety rails ────────────────
    #[error("Agent '{0}' not found")]
    AgentNotFound(String),

    #[error("Central agent entries cannot be removed as platform copies.")]
    CentralAgentPlatformCopy,

    #[error("Read-only plugin copies cannot be removed.")]
    ReadOnlyPluginCopy,

    #[error("Central skill '{0}' exists again; refusing to delete platform copy.")]
    CentralSkillReappeared(String),

    #[error(
        "Refusing to delete '{path}' because it is not the managed install path for '{skill_id}' on '{agent_id}'."
    )]
    NotManagedInstallPath {
        path: String,
        skill_id: String,
        agent_id: String,
    },

    #[error(
        "Refusing to delete '{path}' because it is not the managed remote install path for '{skill_id}' on '{agent_id}'."
    )]
    NotManagedRemoteInstallPath {
        path: String,
        skill_id: String,
        agent_id: String,
    },

    #[error("Refusing to delete the platform skills root for {0}")]
    PlatformRootDeletion(String),

    #[error("Path '{0}' has no parent")]
    PathNoParent(String),

    #[error("Refusing to delete '{child}' because it is outside platform skills root '{root}'")]
    OutsidePlatformRoot { child: String, root: String },

    #[error("Refusing to delete under remote root for {0}")]
    RemoteRootDeletionScope(String),

    #[error("Refusing to delete the remote root '{root}' for {label}")]
    RemoteRootDeletion { root: String, label: String },

    #[error("Refusing to delete '{child}' because it is outside remote root '{root}'")]
    OutsideRemoteRoot { child: String, root: String },

    #[error("Invalid remote path '{0}'")]
    InvalidRemotePath(String),

    #[error("Remote path '{0}' contains traversal")]
    RemotePathTraversal(String),

    #[error("Failed to remove a deleted platform copy.")]
    RemoteLeftoverDeleteFailed,

    #[error("Remote leftover delete protocol is invalid.")]
    RemoteLeftoverProtocol,

    #[error("Failed to update leftover records.")]
    LeftoverRecordCleanupFailed,
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

    /// Stable `(error_code, phase)` pair recorded in the Operation Log.
    ///
    /// Both halves are `&'static str` literals, so no dynamic detail can reach
    /// the persisted log. GitHub failures delegate to the domain's own code
    /// table, which keeps the Operation Log, the IPC envelope, and the Runtime
    /// Log on the same identifier.
    pub(crate) fn reviewed_operation_failure(&self) -> Option<(&'static str, &'static str)> {
        match self {
            Self::GithubImport(error) => error
                .ipc_error_code()
                .map(|code| (code, "repository_snapshot")),
            Self::BatchCancelled => Some(("operation.cancelled", "refresh")),
            Self::InventoryInvariant => Some((
                "central_updates.inventory_invariant",
                "inventory_persistence",
            )),
            Self::InventoryRefreshRequired => Some((
                "central_updates.inventory_refresh_required",
                "decision_apply",
            )),
            Self::SnapshotChanged => Some(("central_updates.snapshot_changed", "decision_apply")),
            _ => None,
        }
    }

    /// Static category label for runtime-log diagnostics.
    ///
    /// Unlike [`Self::reviewed_operation_failure`] this is total: every variant
    /// resolves to a fixed family name, so a failure without a reviewed IPC
    /// code is still identifiable in the Runtime Log without writing its
    /// Display text (which may carry paths, URLs, or command output).
    pub(crate) fn diagnostic_category(&self) -> &'static str {
        match self {
            Self::Io { .. } => "central_updates.io",
            Self::Db(_) => "central_updates.db",
            Self::CentralMutation(_) => "central_updates.central_mutation",
            Self::CentralOperation(_) => "central_updates.central_operation",
            Self::GithubImport(error) => error.diagnostic_category(),
            Self::Installation(_) => "central_updates.installation",
            Self::CentralSkills(_) => "central_updates.central_skills",
            Self::Remote(_) => "central_updates.remote",
            Self::Batch(_) => "central_updates.batch",
            Self::BatchCancelled => "central_updates.cancelled",
            Self::Json(_) => "central_updates.json",
            Self::InventoryInvariant => "central_updates.inventory_invariant",
            Self::InventoryRefreshRequired => "central_updates.inventory_refresh_required",
            Self::SnapshotChanged => "central_updates.snapshot_changed",
            Self::TaskJoin { .. } => "central_updates.task_join",
            Self::SnapshotDownloaderClosed => "central_updates.snapshot_downloader_closed",
            _ => "central_updates.validation",
        }
    }

    pub(crate) fn snapshot_diagnostic_category(&self) -> &'static str {
        match self {
            Self::GithubImport(error) => error.snapshot_diagnostic_category(),
            _ => self.diagnostic_category(),
        }
    }

    pub(crate) fn stable_error_code(&self) -> String {
        if let Self::CentralOperation(error) = self {
            return format!("central_operation.{}", error.code());
        }
        self.reviewed_operation_failure()
            .map(|(code, _)| code)
            .unwrap_or("central_updates.update_failed")
            .to_string()
    }

    pub(crate) const fn public_update_message(&self) -> &'static str {
        "This update item could not be applied."
    }

    pub(crate) fn to_ipc_error(&self) -> String {
        match self {
            Self::GithubImport(error) => error.to_ipc_error(),
            Self::InventoryInvariant => concat!(
                "central_updates.inventory_invariant: ",
                "The update inventory could not be finalized."
            )
            .to_string(),
            Self::InventoryRefreshRequired => concat!(
                "central_updates.inventory_refresh_required: ",
                "Refresh the update inventory before importing this repository."
            )
            .to_string(),
            Self::SnapshotChanged => concat!(
                "central_updates.snapshot_changed: ",
                "The repository content did not match the update inventory. Refresh and try again."
            )
            .to_string(),
            _ => self.to_string(),
        }
    }
}
