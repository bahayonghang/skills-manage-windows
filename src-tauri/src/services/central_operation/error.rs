#[derive(Debug, thiserror::Error)]
pub enum CentralOperationError {
    #[error(transparent)]
    Db(#[from] sqlx::Error),

    #[error("Invalid Central operation manifest: {0}")]
    InvalidManifest(String),

    #[error("Central operation recovery collision ({code})")]
    RecoveryCollision { code: &'static str },

    #[error("Central operation filesystem step failed ({code}): {source}")]
    Io {
        code: &'static str,
        #[source]
        source: std::io::Error,
    },

    #[error("Remote Central operation failed ({code})")]
    Remote { code: &'static str },

    #[error("Central operation reconciliation is blocked ({code})")]
    ReconciliationBlocked { code: &'static str },

    #[error("Failed to join {label} task: {message}")]
    TaskJoin {
        label: &'static str,
        message: String,
    },
}

impl CentralOperationError {
    pub(crate) fn io(code: &'static str, source: std::io::Error) -> Self {
        Self::Io { code, source }
    }

    pub(crate) fn task_join(label: &'static str, message: String) -> Self {
        Self::TaskJoin { label, message }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::Db(_) => "journal_db",
            Self::InvalidManifest(_) => "invalid_manifest",
            Self::RecoveryCollision { code }
            | Self::Io { code, .. }
            | Self::Remote { code }
            | Self::ReconciliationBlocked { code } => code,
            Self::TaskJoin { .. } => "task_join",
        }
    }

    pub fn redacted_message(&self) -> String {
        match self {
            Self::Db(_) => "Database operation failed".to_string(),
            Self::InvalidManifest(_) => "Operation manifest validation failed".to_string(),
            Self::RecoveryCollision { code } => format!("Recovery collision ({code})"),
            Self::Io { code, .. } => format!("Filesystem operation failed ({code})"),
            Self::Remote { code } => format!("Remote operation failed ({code})"),
            Self::ReconciliationBlocked { code } => {
                format!(
                    "recovery.{code}:Operation reconciliation cannot continue with the current evidence."
                )
            }
            Self::TaskJoin { .. } => "Filesystem worker failed".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CentralOperationError;

    #[test]
    fn recovery_diagnostic_message_never_renders_error_sources() {
        let error = CentralOperationError::io(
            "recovery_test",
            std::io::Error::other("C:/Users/private/manifest.json token=secret"),
        );
        let message = error.redacted_message();
        assert_eq!(message, "Filesystem operation failed (recovery_test)");
        assert!(!message.contains("private"));
        assert!(!message.contains("secret"));
        assert!(!message.contains("manifest"));
    }
}
