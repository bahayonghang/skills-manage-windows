//! Typed errors for the local→remote sync domain.
//!
//! Variants cover the real failure categories of snapshot collection,
//! archive building, remote hash inspection and remote apply. Display texts
//! intentionally preserve the historical string-error wording: the IPC
//! boundary stringifies these errors and the frontend shows them in toasts
//! verbatim.

/// Failure categories for local→remote sync operations.
#[derive(Debug, thiserror::Error)]
pub enum LocalRemoteSyncError {
    /// IO failure with an operation-context prefix (e.g. "Failed to read
    /// 'path': <io error>").
    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },

    /// Remote-target transport failures (connect / hash probe / apply
    /// channel; targets module returns String, call sites wrap).
    #[error("{0}")]
    Remote(String),

    #[error("Repository path '{0}' is not a directory.")]
    RepoPathNotDirectory(String),

    /// Remote repository snapshot inspection failed (inner error
    /// preformatted by the hash probe).
    #[error("Failed to inspect remote repository snapshot: {0}")]
    RepoRemoteInspect(String),

    #[error("Local skill path '{0}' has no UTF-8 directory name.")]
    SkillDirNameNotUtf8(String),

    #[error("Local skill id '{0}' is not safe for remote sync.")]
    UnsafeSkillId(String),

    #[error("Snapshot '{id}' contains unsupported path '{path}'.")]
    UnsafeSnapshotPath { id: String, path: String },

    #[error("Local path '{0}' is not safe for archive sync.")]
    UnsafeLocalPath(String),

    #[error("Failed to compute relative path for '{path}': {source}")]
    RelativePath {
        path: String,
        #[source]
        source: std::path::StripPrefixError,
    },

    #[error("Local path '{0}' contains unsupported components.")]
    UnsupportedPathComponents(String),

    #[error("Remote hash output contains unsafe path '{0}'.")]
    UnsafeRemoteHashPath(String),

    #[error("Remote hash output line is not supported: '{0}'.")]
    UnsupportedRemoteHashLine(String),

    #[error("Remote hash digest is not supported for '{0}'.")]
    UnsupportedRemoteHashDigest(String),

    #[error("Remote target path '{0}' has no parent.")]
    RemotePathNoParent(String),

    #[error("Remote apply failed for '{path}': {message}")]
    RemoteApply { path: String, message: String },

    /// A `spawn_blocking` worker failed to join.
    #[error("Failed to join {label} task: {message}")]
    TaskJoin {
        label: &'static str,
        message: String,
    },
}

impl LocalRemoteSyncError {
    /// Build an [`LocalRemoteSyncError::Io`] with an operation-context prefix.
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
