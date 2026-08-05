//! Typed errors for the GitHub repository import domain.
//!
//! Variants cover the real failure categories of source resolution, archive
//! download/extraction, candidate discovery, staging/import, PAT management,
//! and remote (SSH/WSL) preview workspaces. Display texts intentionally
//! preserve the historical string-error wording: the IPC boundary stringifies
//! these errors and the frontend shows them in toasts verbatim.
//!
//! HTTP failures follow the parent design's Http-variant convention:
//! `Http` / `RateLimited` / `AccessDenied` / `Parse` carry preformatted
//! messages so transport, throttling, and decoding problems stay
//! distinguishable for callers.

use super::types::{GitHubAccessDenial, GitHubAccessDenialKind, NO_IMPORTABLE_SKILLS_ERROR};

/// Failure categories for GitHub repository import operations.
#[derive(Debug, thiserror::Error)]
pub enum GithubImportError {
    /// IO failure with an operation-context prefix (e.g. "Failed to create
    /// staging directory 'x': <io error>").
    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },

    /// Direct sqlx calls (non-repos) flow through transparently.
    #[error(transparent)]
    Db(#[from] sqlx::Error),

    #[error(transparent)]
    CentralMutation(#[from] crate::services::central_mutation::CentralMutationError),

    // ── HTTP categories (parent design.md 1.2 Http-variant convention) ──────
    /// HTTP transport/protocol failure (connect, timeout, non-2xx status,
    /// mirror-fallback summaries). Message preformatted at the call site.
    #[error("{0}")]
    Http(String),

    /// GitHub rate-limit denial (429 / x-ratelimit classification). Message
    /// is the denial classifier's Display output.
    #[error("{0}")]
    RateLimited(String),

    /// Authentication/permission denial (401/403, non-rate-limited).
    #[error("{0}")]
    AccessDenied(String),

    /// Response-body / archive parse failure (JSON decode, UTF-8 checks).
    #[error("{0}")]
    Parse(String),

    /// Resource-budget violation (typed `BudgetExceeded` from the
    /// resource_budget module; Display is the budget message verbatim).
    #[error("{0}")]
    Budget(crate::services::resource_budget::BudgetExceeded),

    /// Remote-target transport failures (connect / script / read / mkdir
    /// over the SSH or WSL channel; targets module returns String).
    #[error("{0}")]
    Remote(String),

    // ── URL / source validation ──────────────────────────────────────────────
    /// Preformatted "Invalid GitHub URL '<url>': <reason>" / "GitHub URL
    /// '<url>' has no host." messages from the host rate limiter.
    #[error("{0}")]
    InvalidUrl(String),

    #[error("Invalid GitHub repository URL.")]
    InvalidRepoUrl,

    #[error("Repository subpath traversal is not supported.")]
    SubpathTraversal,

    #[error("Only https:// GitHub repository URLs are supported.")]
    RepoUrlNotHttps,

    #[error("Only github.com repository URLs are supported.")]
    RepoUrlNotGithub,

    #[error("GitHub repository URL must include an owner.")]
    RepoUrlMissingOwner,

    #[error("GitHub repository URL must include a repository name.")]
    RepoUrlMissingRepo,

    #[error("GitHub repository URL is missing owner or repository.")]
    RepoUrlMissingOwnerRepo,

    #[error("GitHub tree URL must include a branch.")]
    TreeUrlMissingBranch,

    #[error("GitHub blob URLs are not supported for repository import.")]
    BlobUrlUnsupported,

    #[error("Repository subpath '{0}' is not supported.")]
    UnsupportedSubpath(String),

    #[error("Repository path '{0}' is not supported.")]
    UnsupportedRepoPath(String),

    #[error("GitHub repository {field} '{value}' is not supported.")]
    InvalidRepoComponent { field: &'static str, value: String },

    #[error("GitHub branch must be a safe single-segment name.")]
    InvalidBranchSelection,

    #[error("GitHub branch in the repository URL does not match the selected branch.")]
    BranchSelectionConflict,

    #[error("Skill identifier '{0}' is not supported.")]
    InvalidSkillIdentifier(String),

    #[error("GitHub repository not found.")]
    RepoNotFound,

    // ── Archive download / extraction ────────────────────────────────────────
    #[error("GitHub repository archive is unavailable.")]
    ArchiveUnavailable,

    #[error("GitHub repository archive redirect was rejected.")]
    ArchiveRedirectRejected,

    #[error("GitHub repository archive exceeds the resource budget (more than {0} files).")]
    ArchiveFileBudgetExceeded(usize),

    #[error("GitHub repository expanded archive contents size overflowed.")]
    ArchiveSizeOverflow,

    #[error("GitHub repository snapshot retained byte size overflowed.")]
    SnapshotSizeOverflow,

    #[error("GitHub repository archive contains an unsupported path.")]
    ArchiveUnsupportedPath,

    #[error("GitHub repository archive contains an unsupported path '{0}'.")]
    ArchiveUnsupportedPathNamed(String),

    // ── Candidate discovery / preview ────────────────────────────────────────
    #[error("{}", NO_IMPORTABLE_SKILLS_ERROR)]
    NoImportableSkills,

    /// An invalid skill candidate aborted candidate building (message is the
    /// candidate's preformatted detail text).
    #[error("{0}")]
    InvalidCandidate(String),

    #[error("GitHub preview file manifest for skill '{0}' is incomplete.")]
    PreviewFileManifestIncomplete(String),

    #[error("Remote GitHub preview returned an invalid file manifest.")]
    RemotePreviewInvalidFileManifest,

    // ── Tree manifest acquisition (fast-path) ───────────────────────────────
    /// Recursive Git tree API response had `truncated: true`. The dispatcher
    /// must fall back to archive acquisition; archive reads the full tarball
    /// so truncation does not affect candidate/preview parity.
    #[error("GitHub repository tree response was truncated; falling back to archive.")]
    TreeManifestTruncated,

    /// A regular blob entry in the Git tree response is missing its `size`
    /// field. Raw download budgeting requires a known size, so the dispatcher
    /// falls back to archive.
    #[error("GitHub repository tree entry '{0}' is missing a byte size.")]
    TreeManifestEntryMissingSize(String),

    /// The Git tree response contains a mode/type combination the TreeRaw
    /// fast-path cannot classify (neither a regular blob, symlink blob, nor
    /// gitlink). Falling back to archive keeps parity with the tar regular
    /// file filter.
    #[error("GitHub repository tree entry '{path}' has unsupported mode '{mode}'.")]
    TreeManifestUnsupportedMode { path: String, mode: String },

    /// The recursive Git tree response exceeded the tree-entry budget. The
    /// dispatcher falls back to archive (which has its own larger file budget).
    #[error("GitHub repository tree exceeds the resource budget (more than {0} entries).")]
    TreeManifestEntryBudgetExceeded(usize),

    /// Summed regular-blob sizes in the tree response overflowed `u64`. Treated
    /// as a budget/integrity failure so the dispatcher falls back to archive.
    #[error("GitHub repository tree expanded contents size overflowed.")]
    TreeManifestSizeOverflow,

    // ── Import staging / execution ───────────────────────────────────────────
    #[error("Select at least one skill to import.")]
    NoSelections,

    #[error("Selected skill '{0}' is no longer available in the preview.")]
    SelectionUnavailable(String),

    #[error("Skill '{0}' was selected more than once.")]
    DuplicateSelection(String),

    #[error("Skill '{0}' requires a renamed skill id for rename resolution.")]
    RenameIdRequired(String),

    #[error("Renamed skill id '{0}' is already in use.")]
    RenameIdInUse(String),

    #[error("No valid import operations were requested.")]
    NoValidOperations,

    #[error("Central agent not found in database")]
    CentralAgentMissing,

    #[error("Imported skill '{0}' is missing valid frontmatter.")]
    ImportedSkillMissingFrontmatter(String),

    #[error("Target directory '{0}' already exists.")]
    TargetDirExists(String),

    #[error("Repository path '{0}' is no longer available in the archive.")]
    RepoPathGone(String),

    #[error("Repository contains an unsupported path '{0}'.")]
    RepoContainsUnsupportedPath(String),

    #[error("Repository file '{0}' is no longer available in the archive.")]
    RepoFileGone(String),

    #[error(
        "Repository file '{path}' changed size while it was being downloaded (expected {expected} bytes, received {actual} bytes)."
    )]
    RepoFileSizeMismatch {
        path: String,
        expected: u64,
        actual: u64,
    },

    #[error("Failed to determine imported file parent directory.")]
    ImportParentDirUnknown,

    // ── Immutable preview snapshots ──────────────────────────────────────────
    #[error("Remote GitHub preview did not return a workspace path.")]
    RemotePreviewNoWorkspacePath,

    #[error(
        "GitHub preview snapshot does not match the active target or repository. Preview the repository again."
    )]
    PreviewWorkspaceMismatch,

    #[error("GitHub preview snapshot has expired. Preview the repository again.")]
    PreviewWorkspaceExpired,

    #[error("The active target changed after preview. Preview the repository again.")]
    PreviewTargetChanged,

    /// The token is unknown, was already consumed by a successful import, or
    /// belongs to a previous application session.
    #[error("GitHub preview snapshot is no longer available. Preview the repository again.")]
    PreviewSnapshotMissing,

    /// Another import is already running for the same preview snapshot.
    #[error("This GitHub preview is already being imported. Wait for it to finish.")]
    PreviewSnapshotBusy,

    #[error("GitHub preview capacity is full. Close an older preview and try again.")]
    PreviewCapacity,

    #[error("GitHub preview cleanup is still pending. Preview the repository again.")]
    PreviewCleanupPending,

    /// The retained snapshot no longer matches the digest confirmed at preview
    /// time. Fails closed before any Central or database mutation.
    #[error(
        "GitHub preview snapshot content changed after preview. Preview the repository again."
    )]
    PreviewSnapshotIntegrity,

    /// The repository branch could not be resolved to an immutable commit.
    #[error("GitHub repository commit could not be resolved. Retry the preview.")]
    PreviewCommitUnresolved,

    // ── PAT management ───────────────────────────────────────────────────────
    /// Secret-store failures and PAT save/verify problems (messages
    /// preformatted by `map_secret_error` and friends).
    #[error("{0}")]
    Secret(String),

    #[error("GitHub token cannot be empty; clear the token instead.")]
    PatTokenEmpty,

    #[error("GitHub token contains unsupported newline characters.")]
    PatTokenHasNewline,

    /// A `spawn_blocking` worker failed to join.
    #[error("Failed to join {label} task: {message}")]
    TaskJoin {
        label: &'static str,
        message: String,
    },
}

impl GithubImportError {
    /// Build a [`GithubImportError::Io`] with an operation-context prefix.
    pub(crate) fn io(context: impl Into<String>, source: std::io::Error) -> Self {
        Self::Io {
            context: context.into(),
            source,
        }
    }

    /// Classify a GitHub access denial into `RateLimited` / `AccessDenied`,
    /// keeping the denial's preformatted Display text.
    pub(super) fn from_denial(denial: GitHubAccessDenial) -> Self {
        match denial.kind {
            GitHubAccessDenialKind::RateLimited { .. } => Self::RateLimited(denial.to_string()),
            GitHubAccessDenialKind::AuthenticationOrPermission => {
                Self::AccessDenied(denial.to_string())
            }
        }
    }

    pub(crate) fn task_join(label: &'static str, message: String) -> Self {
        Self::TaskJoin { label, message }
    }

    /// Stable, locale-neutral code for the preview snapshot lifecycle failures
    /// the wizard must translate into a "preview again" state.
    ///
    /// Only snapshot-lifecycle variants are coded; every other variant keeps
    /// its historical Display text so existing toasts stay unchanged.
    pub fn preview_snapshot_code(&self) -> Option<&'static str> {
        match self {
            Self::PreviewSnapshotMissing => Some("preview_missing"),
            Self::PreviewWorkspaceExpired => Some("preview_expired"),
            Self::PreviewWorkspaceMismatch | Self::PreviewTargetChanged => Some("preview_mismatch"),
            Self::PreviewSnapshotIntegrity => Some("preview_integrity"),
            Self::PreviewSnapshotBusy => Some("preview_busy"),
            Self::PreviewCapacity => Some("preview_capacity"),
            Self::PreviewCleanupPending => Some("preview_cleanup_pending"),
            Self::PreviewCommitUnresolved => Some("preview_commit_unresolved"),
            _ => None,
        }
    }

    /// Fully-qualified stable IPC code for reviewed GitHub-import failures.
    ///
    /// This is the single source of truth for both the IPC envelope and the
    /// Operation Log / Runtime Log diagnostics, so a coded failure can never be
    /// stable on one surface and opaque on another. Every code is a `&'static
    /// str` literal: no dynamic detail, path, URL, or token can reach it.
    pub fn ipc_error_code(&self) -> Option<&'static str> {
        let code = match self {
            // ── Preview snapshot lifecycle ──────────────────────────────────
            Self::PreviewSnapshotMissing => "github_import.preview_missing",
            Self::PreviewWorkspaceExpired => "github_import.preview_expired",
            Self::PreviewWorkspaceMismatch | Self::PreviewTargetChanged => {
                "github_import.preview_mismatch"
            }
            Self::PreviewSnapshotIntegrity => "github_import.preview_integrity",
            Self::PreviewSnapshotBusy => "github_import.preview_busy",
            Self::PreviewCapacity => "github_import.preview_capacity",
            Self::PreviewCleanupPending => "github_import.preview_cleanup_pending",
            Self::PreviewCommitUnresolved => "github_import.preview_commit_unresolved",

            // ── Branch selection ────────────────────────────────────────────
            Self::InvalidBranchSelection => "github_import.branch_invalid",
            Self::BranchSelectionConflict => "github_import.branch_conflict",

            // ── Network / archive acquisition ───────────────────────────────
            Self::ArchiveRedirectRejected => "github_import.archive_redirect_rejected",
            Self::Http(_) => "github_import.transport_failed",
            Self::RateLimited(_) => "github_import.rate_limited",
            Self::AccessDenied(_) => "github_import.access_denied",
            Self::RepoNotFound => "github_import.repo_not_found",
            Self::ArchiveUnavailable => "github_import.archive_unavailable",
            Self::Parse(_) => "github_import.response_invalid",
            Self::InvalidUrl(_) => "github_import.invalid_url",
            Self::Budget(_)
            | Self::ArchiveFileBudgetExceeded(_)
            | Self::ArchiveSizeOverflow
            | Self::SnapshotSizeOverflow
            | Self::TreeManifestEntryBudgetExceeded(_)
            | Self::TreeManifestSizeOverflow => "github_import.budget_exceeded",
            Self::Secret(_) => "github_import.credential_unavailable",

            _ => return None,
        };
        Some(code)
    }

    /// Stable IPC codes for reviewed GitHub-import failures, without the
    /// `github_import.` prefix. Derived from [`Self::ipc_error_code`] so the
    /// two can never disagree.
    pub fn ipc_code(&self) -> Option<&'static str> {
        self.ipc_error_code()
            .map(|code| code.trim_start_matches("github_import."))
    }

    /// Static category label for runtime-log diagnostics. Coded failures report
    /// their IPC code; uncoded ones report a fixed variant family so an
    /// unmapped failure is still identifiable without logging its Display text.
    pub fn diagnostic_category(&self) -> &'static str {
        if let Some(code) = self.ipc_error_code() {
            return code;
        }
        match self {
            Self::Io { .. } => "github_import.io",
            Self::Db(_) => "github_import.db",
            Self::CentralMutation(_) => "github_import.central_mutation",
            Self::Remote(_) => "github_import.remote",
            Self::TaskJoin { .. } => "github_import.task_join",
            _ => "github_import.other",
        }
    }

    /// Serialize for the IPC boundary.
    ///
    /// Reviewed snapshot lifecycle and branch-selection failures use a stable
    /// `github_import.<code>:<summary>` envelope the frontend maps through i18n.
    /// The summary is fixed and never contains a token, workspace path, digest,
    /// branch value, or file content.
    pub fn to_ipc_error(&self) -> String {
        match self.ipc_code() {
            Some(code) => format!("github_import.{}:{}", code, self),
            None => self.to_string(),
        }
    }
}
