//! Typed errors for the skill-usage domain.
//!
//! Variants cover the real failure categories of usage scanning: filesystem
//! backends (local + remote SSH/WSL), provider log collection, the OpenCode
//! SQLite reader, and skill-call persistence. Local-tolerable failures keep
//! historical Display wording. Target-fatal remote failures keep the
//! underlying [`TargetsError`] as an internal source and expose only a stable
//! code, retryable flag, and bounded public message.

use crate::targets::TargetsError;

const REMOTE_TRANSPORT_CODE: &str = "usage.remote_transport";
const REMOTE_PROTOCOL_CODE: &str = "usage.remote_protocol";
const REMOTE_PERMISSION_CODE: &str = "usage.remote_permission";

const REMOTE_TRANSPORT_MESSAGE: &str =
    "Remote usage refresh failed because the target is unavailable.";
const REMOTE_PROTOCOL_MESSAGE: &str =
    "Remote usage refresh failed because the target protocol is invalid.";
const REMOTE_PERMISSION_MESSAGE: &str = "Remote usage refresh failed because access was denied.";

/// Stable classification for a target-fatal remote usage failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageRemoteKind {
    Transport,
    Protocol,
    Permission,
}

impl UsageRemoteKind {
    pub(crate) const fn stable_code(self) -> &'static str {
        match self {
            Self::Transport => REMOTE_TRANSPORT_CODE,
            Self::Protocol => REMOTE_PROTOCOL_CODE,
            Self::Permission => REMOTE_PERMISSION_CODE,
        }
    }

    pub(crate) const fn public_message(self) -> &'static str {
        match self {
            Self::Transport => REMOTE_TRANSPORT_MESSAGE,
            Self::Protocol => REMOTE_PROTOCOL_MESSAGE,
            Self::Permission => REMOTE_PERMISSION_MESSAGE,
        }
    }
}

/// Failure categories for skill-usage scanning and aggregation.
#[derive(Debug, thiserror::Error)]
pub enum UsageError {
    /// IO failure with an operation-context prefix (e.g. "local read x:
    /// <io error>").
    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },

    /// skill_calls / provider-health persistence and direct sqlx calls flow
    /// through transparently.
    #[error(transparent)]
    Db(#[from] sqlx::Error),

    /// Target-fatal remote transport, protocol, or permission failure.
    /// Public Display is a fixed redacted sentence; the source is retained
    /// only for typed classification and never copied into IPC or logs.
    #[error("{}", .kind.public_message())]
    TargetFatalRemote {
        kind: UsageRemoteKind,
        retryable: bool,
        #[source]
        source: TargetsError,
    },

    /// Decoding failure for fetched log content (UTF-8 checks). Message
    /// preformatted at the call site.
    #[error("{0}")]
    Parse(String),

    /// Read-only open of the OpenCode SQLite database failed.
    #[error("opencode db open: {0}")]
    OpenCodeDbOpen(#[source] sqlx::Error),

    /// Skill-call query against the OpenCode SQLite database failed.
    #[error("opencode query: {0}")]
    OpenCodeQuery(#[source] sqlx::Error),

    /// A `spawn_blocking` worker failed to join.
    #[error("Failed to join {label} task: {message}")]
    TaskJoin {
        label: &'static str,
        message: String,
    },
}

impl UsageError {
    /// Build a [`UsageError::Io`] with an operation-context prefix.
    pub(crate) fn io(context: impl Into<String>, source: std::io::Error) -> Self {
        Self::Io {
            context: context.into(),
            source,
        }
    }

    pub(crate) fn task_join(label: &'static str, message: String) -> Self {
        Self::TaskJoin { label, message }
    }

    /// Map a remote-target transport error into a target-fatal usage error.
    ///
    /// Classification uses the [`TargetsError`] variant, never the Display
    /// text, path, command, or captured streams.
    pub(crate) fn from_remote(source: TargetsError) -> Self {
        let (kind, retryable) = classify_remote(&source);
        Self::TargetFatalRemote {
            kind,
            retryable,
            source,
        }
    }

    pub fn stable_code(&self) -> &'static str {
        match self {
            Self::TargetFatalRemote { kind, .. } => kind.stable_code(),
            Self::Io { .. } => "usage.io",
            Self::Db(_) => "usage.db",
            Self::Parse(_) => "usage.parse",
            Self::OpenCodeDbOpen(_) | Self::OpenCodeQuery(_) => "usage.opencode",
            Self::TaskJoin { .. } => "usage.task_join",
        }
    }

    pub fn public_message(&self) -> &'static str {
        match self {
            Self::TargetFatalRemote { kind, .. } => kind.public_message(),
            _ => "The operation failed. See runtime logs for details.",
        }
    }

    pub fn retryable(&self) -> bool {
        match self {
            Self::TargetFatalRemote { retryable, .. } => *retryable,
            Self::Io { .. } => true,
            Self::Db(_)
            | Self::Parse(_)
            | Self::OpenCodeDbOpen(_)
            | Self::OpenCodeQuery(_)
            | Self::TaskJoin { .. } => false,
        }
    }

    pub fn is_target_fatal(&self) -> bool {
        matches!(self, Self::TargetFatalRemote { .. })
    }
}

fn classify_remote(error: &TargetsError) -> (UsageRemoteKind, bool) {
    match error {
        TargetsError::Io { .. }
        | TargetsError::ProcessTimedOut { .. }
        | TargetsError::ProcessTerminationFailed { .. }
        | TargetsError::RemoteFileReadFailed { .. }
        | TargetsError::WslListFailed
        | TargetsError::WslListFailedDetail(_)
        | TargetsError::WslWindowsOnly
        | TargetsError::WslDiscoveryWindowsOnly => (UsageRemoteKind::Transport, true),
        TargetsError::ProcessCancelled(_) => (UsageRemoteKind::Transport, false),
        TargetsError::RemoteStdoutNotUtf8(_)
        | TargetsError::WslStdoutNotUtf8(_)
        | TargetsError::RemoteFileSizeProtocol
        | TargetsError::RemoteFileReadLimitUnsupported
        | TargetsError::RemoteFileTooLarge { .. }
        | TargetsError::ProcessOutputLimitExceeded { .. }
        | TargetsError::ProbeHomeMissing
        | TargetsError::ProbeMkdirUnconfirmed
        | TargetsError::UnsupportedRemoteOs(_)
        | TargetsError::UnsupportedWslOs(_)
        | TargetsError::Json(_)
        | TargetsError::ParseRemoteTargets(_)
        | TargetsError::ParseWslTargets(_) => (UsageRemoteKind::Protocol, false),
        TargetsError::RemoteCommandFailed { .. }
        | TargetsError::WslCommandFailed { .. }
        | TargetsError::RemoteInspectFailed { .. }
        | TargetsError::WslInspectFailed { .. }
        | TargetsError::RemotePathMissing(_)
        | TargetsError::WslPathMissing(_)
        | TargetsError::PasswordUnavailable(_)
        | TargetsError::PasswordRequired
        | TargetsError::CredentialStore(_)
        | TargetsError::NotPasswordAuth
        | TargetsError::MissingCredentialKey
        | TargetsError::PassphraseUnsupported
        | TargetsError::Dpapi { .. }
        | TargetsError::ProtectedPayloadNotHex
        | TargetsError::PasswordTooLarge
        | TargetsError::ProtectedPayloadTooLarge
        | TargetsError::ProtectedPasswordNotUtf8(_)
        | TargetsError::ProtectedFallbackWindowsOnly
        | TargetsError::SessionCacheUnavailable
        | TargetsError::TargetNotFound(_)
        | TargetsError::ActiveTargetMissing(_)
        | TargetsError::LocalTargetUndeletable
        | TargetsError::LocalTargetNotRemote
        | TargetsError::TargetIdImmutable
        | TargetsError::RequiredField(_)
        | TargetsError::InvalidTargetId
        | TargetsError::Db(_) => (UsageRemoteKind::Permission, false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::exit_status;

    const PATH_SEED: &str = "/home/alice/.ssh/id_ed25519";
    const COMMAND_SEED: &str = "ssh -i private.pem host -- find /home/alice";
    const STDERR_SEED: &str = "Permission denied: /var/log/secret.stderr";
    const HOST_SEED: &str = "alice@prod.example.invalid";

    fn adversarial_seeds() -> [&'static str; 4] {
        [PATH_SEED, COMMAND_SEED, STDERR_SEED, HOST_SEED]
    }

    fn assert_public_surface_redacted(error: &UsageError) {
        let display = error.to_string();
        let debug = format!("{error:?}");
        for seed in adversarial_seeds() {
            assert!(!display.contains(seed), "Display leaked {seed}: {display}");
        }
        assert_eq!(display, error.public_message());
        // Debug may retain the source for diagnostics; IPC/logs must not use it.
        let _ = debug;
    }

    #[test]
    fn transport_timeout_is_retryable_target_fatal() {
        let error = UsageError::from_remote(TargetsError::ProcessTimedOut {
            transport: "SSH",
            class: "probe",
            timeout_ms: 10_000,
        });
        assert!(error.is_target_fatal());
        assert!(error.retryable());
        assert_eq!(error.stable_code(), REMOTE_TRANSPORT_CODE);
        assert_eq!(error.public_message(), REMOTE_TRANSPORT_MESSAGE);
        assert_redacted_across_source_text(&error);
    }

    #[test]
    fn inspect_permission_is_non_retryable_and_ignores_source_text() {
        let with_path = UsageError::from_remote(TargetsError::RemoteInspectFailed {
            path: PATH_SEED.to_string(),
            detail: format!("{STDERR_SEED}\n{COMMAND_SEED}\n{HOST_SEED}"),
        });
        let with_other_text = UsageError::from_remote(TargetsError::WslInspectFailed {
            path: "/opt/other".to_string(),
            detail: "totally different fixture text".to_string(),
        });
        for error in [&with_path, &with_other_text] {
            assert!(error.is_target_fatal());
            assert!(!error.retryable());
            assert_eq!(error.stable_code(), REMOTE_PERMISSION_CODE);
            assert_eq!(error.public_message(), REMOTE_PERMISSION_MESSAGE);
        }
        assert_eq!(with_path.to_string(), with_other_text.to_string());
        assert_public_surface_redacted(&with_path);
    }

    #[test]
    fn stdout_protocol_failure_is_stable_across_fixtures() {
        let first = UsageError::from_remote(TargetsError::RemoteStdoutNotUtf8(
            String::from_utf8(vec![0xff]).unwrap_err(),
        ));
        let second = UsageError::from_remote(TargetsError::WslStdoutNotUtf8(
            String::from_utf8(vec![0xfe, 0xfd]).unwrap_err(),
        ));
        assert_eq!(first.stable_code(), REMOTE_PROTOCOL_CODE);
        assert_eq!(second.stable_code(), first.stable_code());
        assert_eq!(first.public_message(), second.public_message());
        assert!(!first.retryable());
        assert!(first.is_target_fatal());
        assert_eq!(first.to_string(), REMOTE_PROTOCOL_MESSAGE);
    }

    #[test]
    fn remote_command_failure_is_not_classified_by_message_text() {
        let permission_text = UsageError::from_remote(TargetsError::RemoteCommandFailed {
            status: exit_status(1),
            detail: STDERR_SEED.to_string(),
        });
        let other_text = UsageError::from_remote(TargetsError::WslCommandFailed {
            status: exit_status(13),
            detail: "no such file or directory".to_string(),
        });
        assert_eq!(permission_text.stable_code(), REMOTE_PERMISSION_CODE);
        assert_eq!(other_text.stable_code(), permission_text.stable_code());
        assert_eq!(
            permission_text.public_message(),
            other_text.public_message()
        );
        assert_public_surface_redacted(&permission_text);
    }

    #[test]
    fn local_parse_failure_is_not_target_fatal() {
        let error = UsageError::Parse("fixture failure".to_string());
        assert!(!error.is_target_fatal());
        assert!(!error.retryable());
        assert_eq!(error.stable_code(), "usage.parse");
    }

    fn assert_redacted_across_source_text(error: &UsageError) {
        assert_public_surface_redacted(error);
        let serialized = error.to_string();
        assert!(!serialized.contains("SSH"));
        assert!(!serialized.contains("probe"));
        assert!(!serialized.contains("10_000"));
        assert!(!serialized.contains("10000"));
    }
}
