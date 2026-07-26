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

    #[error("Skill identifier '{0}' is not supported.")]
    InvalidSkillIdentifier(String),

    #[error("GitHub repository not found.")]
    RepoNotFound,

    // ── Archive download / extraction ────────────────────────────────────────
    #[error("GitHub repository archive is unavailable.")]
    ArchiveUnavailable,

    #[error("GitHub repository archive exceeds the resource budget (more than {0} files).")]
    ArchiveFileBudgetExceeded(usize),

    #[error("GitHub repository expanded archive contents size overflowed.")]
    ArchiveSizeOverflow,

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

    // ── Remote preview workspaces ────────────────────────────────────────────
    #[error("Remote GitHub preview did not return a workspace path.")]
    RemotePreviewNoWorkspacePath,

    #[error(
        "GitHub preview workspace does not match the active target or repository. Preview the repository again."
    )]
    PreviewWorkspaceMismatch,

    #[error("GitHub preview workspace has expired. Preview the repository again.")]
    PreviewWorkspaceExpired,

    #[error("The active remote target changed after preview. Preview the repository again.")]
    PreviewTargetChanged,

    #[error("Remote GitHub preview workspace is only available on its remote target.")]
    PreviewTargetNotRemote,

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
}
