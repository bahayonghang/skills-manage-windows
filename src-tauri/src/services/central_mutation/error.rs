#[derive(Debug, thiserror::Error)]
pub enum CentralMutationError {
    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Central library is busy while running '{operation}'")]
    Busy { operation: &'static str },

    #[error("Timed out waiting {timeout_ms} ms for Central library mutation '{operation}'")]
    Timeout {
        operation: &'static str,
        timeout_ms: u128,
    },

    #[error("Failed to join {label} task: {message}")]
    TaskJoin {
        label: &'static str,
        message: String,
    },
}

impl CentralMutationError {
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
