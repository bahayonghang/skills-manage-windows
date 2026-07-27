use std::io;

#[derive(Debug, thiserror::Error)]
pub enum StartupError {
    #[error("Application data directory is unavailable")]
    DataDirectory {
        #[source]
        source: io::Error,
    },

    #[error("SQLite database could not be opened")]
    DatabaseOpen {
        #[source]
        source: sqlx::Error,
    },

    #[error("SQLite schema initialization failed")]
    SchemaInitialization {
        #[source]
        source: sqlx::Error,
    },

    #[error("Database recovery backup could not be created")]
    RecoveryBackup {
        #[source]
        source: io::Error,
    },

    #[error("Database recovery backup failed and rollback was incomplete")]
    RecoveryRollback {
        move_error: String,
        rollback_error: String,
    },

    #[error("No database files are available to back up")]
    NoDatabaseFiles,

    #[error("Application state is already installed")]
    StateAlreadyInstalled,

    #[error("Failed to join {label} task: {message}")]
    TaskJoin {
        label: &'static str,
        message: String,
    },
}

impl StartupError {
    pub(super) fn data_directory(source: io::Error) -> Self {
        Self::DataDirectory { source }
    }

    pub(super) fn recovery_backup(source: io::Error) -> Self {
        Self::RecoveryBackup { source }
    }

    pub(super) fn task_join(label: &'static str, message: String) -> Self {
        Self::TaskJoin { label, message }
    }
}
