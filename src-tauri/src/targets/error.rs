//! Typed errors for the targets remote-transport layer (SSH / WSL / local).
//!
//! Variants cover the real failure categories of target CRUD, credential
//! storage, and the SSH/WSL command transport. Display texts intentionally
//! preserve the historical string-error wording: the IPC boundary stringifies
//! these errors and the frontend shows them in toasts verbatim.

/// Failure categories for remote-target management and transport operations.
#[derive(Debug, thiserror::Error)]
pub enum TargetsError {
    /// IO failure with an operation-context prefix (e.g. "Failed to start
    /// ssh: <io error>").
    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },

    /// Database failures (settings reads/writes, remote cache pool init)
    /// flow through transparently.
    #[error(transparent)]
    Db(#[from] sqlx::Error),

    /// JSON serialization failures (stringified verbatim like the historical
    /// `e.to_string()` mapping).
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error("Failed to parse remote targets: {0}")]
    ParseRemoteTargets(#[source] serde_json::Error),

    #[error("Failed to parse WSL targets: {0}")]
    ParseWslTargets(#[source] serde_json::Error),

    // ── SSH command transport ───────────────────────────────────────────────
    /// Remote command exited non-zero; `detail` is the classified stderr.
    #[error("Remote command failed with status {status}: {detail}")]
    RemoteCommandFailed {
        status: std::process::ExitStatus,
        detail: String,
    },

    #[error("Remote stdout is not valid UTF-8: {0}")]
    RemoteStdoutNotUtf8(#[source] std::string::FromUtf8Error),

    #[error("Remote path '{0}' does not exist.")]
    RemotePathMissing(String),

    #[error("Failed to inspect remote path '{path}': {detail}")]
    RemoteInspectFailed { path: String, detail: String },

    // ── WSL command transport ───────────────────────────────────────────────
    #[error("WSL command failed with status {status}: {detail}")]
    WslCommandFailed {
        status: std::process::ExitStatus,
        detail: String,
    },

    #[error("WSL stdout is not valid UTF-8: {0}")]
    WslStdoutNotUtf8(#[source] std::string::FromUtf8Error),

    #[error("WSL path '{0}' does not exist.")]
    WslPathMissing(String),

    #[error("Failed to inspect WSL path '{path}': {detail}")]
    WslInspectFailed { path: String, detail: String },

    #[error("WSL targets are only supported on Windows.")]
    WslWindowsOnly,

    #[error("WSL distributions can only be discovered on Windows.")]
    WslDiscoveryWindowsOnly,

    #[error("Failed to list WSL distributions. Verify WSL is installed with `wsl.exe -l -v`.")]
    WslListFailed,

    #[error("Failed to list WSL distributions: {0}")]
    WslListFailedDetail(String),

    // ── Connection probe ────────────────────────────────────────────────────
    #[error("Remote HOME probe did not return an absolute POSIX path.")]
    ProbeHomeMissing,

    #[error("Remote probe did not confirm ~/.skillsmanage/skills creation.")]
    ProbeMkdirUnconfirmed,

    #[error("Remote OS '{0}' is not supported in this version. Linux and macOS are supported.")]
    UnsupportedRemoteOs(String),

    #[error("WSL OS '{0}' is not supported in this version. Linux is expected for WSL targets.")]
    UnsupportedWslOs(String),

    // ── Target registry / CRUD ──────────────────────────────────────────────
    #[error("Target '{0}' not found")]
    TargetNotFound(String),

    #[error("Active target '{0}' no longer exists. Switch back to Local.")]
    ActiveTargetMissing(String),

    #[error("Local target cannot be deleted.")]
    LocalTargetUndeletable,

    #[error("Local target is not a remote target.")]
    LocalTargetNotRemote,

    #[error("Target id cannot be changed.")]
    TargetIdImmutable,

    #[error("{0} is required.")]
    RequiredField(String),

    #[error("Invalid target id.")]
    InvalidTargetId,

    #[error(
        "Passphrase-protected keys are not supported yet. Use ssh-agent or an unencrypted key."
    )]
    PassphraseUnsupported,

    // ── Credentials ─────────────────────────────────────────────────────────
    #[error("This SSH target does not use password authentication.")]
    NotPasswordAuth,

    #[error("Password target is missing its credential key.")]
    MissingCredentialKey,

    #[error("Password is required for password authentication.")]
    PasswordRequired,

    #[error(
        "SSH password for target '{0}' is not available. Open Settings, enter the password for this target, save it, and retry."
    )]
    PasswordUnavailable(String),

    /// System credential store failures (message preformatted by the
    /// credential-store helpers).
    #[error("{0}")]
    CredentialStore(String),

    #[error("Failed to update SSH password session cache.")]
    SessionCacheUnavailable,

    // ── Windows DPAPI protected-password fallback ───────────────────────────
    #[error("Failed to {action} SSH password with Windows DPAPI: {source}")]
    Dpapi {
        action: &'static str,
        #[source]
        source: std::io::Error,
    },

    #[error("Protected password payload is not valid hex.")]
    ProtectedPayloadNotHex,

    #[error("SSH password is too large to protect.")]
    PasswordTooLarge,

    #[error("Protected SSH password payload is too large.")]
    ProtectedPayloadTooLarge,

    #[error("Protected SSH password is not valid UTF-8: {0}")]
    ProtectedPasswordNotUtf8(#[source] std::string::FromUtf8Error),

    #[error("App-local protected SSH password fallback is only available on Windows.")]
    ProtectedFallbackWindowsOnly,
}

impl TargetsError {
    /// Build a [`TargetsError::Io`] with an operation-context prefix.
    pub(crate) fn io(context: impl Into<String>, source: std::io::Error) -> Self {
        Self::Io {
            context: context.into(),
            source,
        }
    }
}
