#[derive(Debug, thiserror::Error)]
pub enum RulesError {
    #[error("Invalid rule name.")]
    InvalidName,
    #[error("The rule changed outside SkillPort.")]
    RevisionConflict,
    #[error("The rule target contains conflicting data.")]
    TargetConflict,
    #[error("Permission to access a rule was denied.")]
    PermissionDenied,
    #[error("Another local mutation is running.")]
    LockBusy,
    #[error("Rules require the Local target.")]
    LocalOnly,
    #[error("The rule format or tool configuration is not supported.")]
    Unsupported,
    #[error("Rule recovery was blocked by external changes.")]
    RecoveryBlocked,
    #[error("The rules resource budget was exceeded.")]
    BudgetExceeded,
    #[error("The rule could not be accessed.")]
    Io,
    #[error("The rule was not found.")]
    NotFound,
}

impl RulesError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidName => "rules.invalid_name",
            Self::RevisionConflict => "rules.revision_conflict",
            Self::TargetConflict => "rules.target_conflict",
            Self::PermissionDenied => "rules.permission_denied",
            Self::LockBusy => "rules.lock_busy",
            Self::LocalOnly => "rules.local_only",
            Self::Unsupported => "rules.unsupported",
            Self::RecoveryBlocked => "rules.recovery_blocked",
            Self::BudgetExceeded => "rules.budget_exceeded",
            Self::Io => "rules.io",
            Self::NotFound => "rules.not_found",
        }
    }
    pub fn ipc(&self) -> crate::ipc_error::IpcError {
        crate::ipc_error::IpcError::new(
            self.code(),
            crate::ipc_error::public_message_for_code(self.code()).expect("reviewed rules error"),
            matches!(self, Self::LockBusy),
        )
    }
}

impl From<std::io::Error> for RulesError {
    fn from(error: std::io::Error) -> Self {
        match error.kind() {
            std::io::ErrorKind::NotFound => Self::NotFound,
            std::io::ErrorKind::PermissionDenied => Self::PermissionDenied,
            _ if cfg!(windows) && matches!(error.raw_os_error(), Some(5 | 32 | 33 | 1314)) => {
                Self::PermissionDenied
            }
            _ => Self::Io,
        }
    }
}
