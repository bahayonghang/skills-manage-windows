mod error;

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use chrono::Utc;
use serde::Serialize;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::db::{self, DatabaseOpenFailure, DatabaseOpenStage, DbPool};
use crate::fs_util::run_blocking_fs_with;

pub use error::StartupError;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StartupIssue {
    DataDirectoryUnavailable,
    DatabaseOpenFailed,
    SchemaInitializationFailed,
    DatabaseRecoveryFailed,
}

impl StartupIssue {
    pub fn code(self) -> &'static str {
        match self {
            Self::DataDirectoryUnavailable => "startup.data_directory_unavailable",
            Self::DatabaseOpenFailed => "startup.database_open_failed",
            Self::SchemaInitializationFailed => "startup.schema_initialization_failed",
            Self::DatabaseRecoveryFailed => "startup.database_recovery_failed",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StartupDiagnostic {
    NotRun,
    Healthy,
    Corrupt,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum StartupStatus {
    Checking,
    Ready,
    RecoveryRequired {
        issue: StartupIssue,
        diagnostic: StartupDiagnostic,
        #[serde(rename = "canRebuild")]
        can_rebuild: bool,
        #[serde(rename = "backupCreated")]
        backup_created: bool,
    },
    Fatal {
        issue: StartupIssue,
    },
}

pub struct StartupCoordinator {
    db_path: PathBuf,
    status: RwLock<StartupStatus>,
    operation: Mutex<()>,
}

impl StartupCoordinator {
    pub fn new(db_path: PathBuf) -> Self {
        Self {
            db_path,
            status: RwLock::new(StartupStatus::Checking),
            operation: Mutex::new(()),
        }
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub fn status(&self) -> StartupStatus {
        self.status
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    pub fn set_status(&self, status: StartupStatus) {
        *self
            .status
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = status;
    }

    pub async fn lock_operation(&self) -> tokio::sync::MutexGuard<'_, ()> {
        self.operation.lock().await
    }
}

#[derive(Debug)]
pub struct StartupAttemptFailure {
    pub issue: StartupIssue,
    pub diagnostic: StartupDiagnostic,
    pub can_rebuild: bool,
    pub error: StartupError,
}

impl StartupAttemptFailure {
    pub fn status(&self, backup_created: bool) -> StartupStatus {
        match self.issue {
            StartupIssue::DataDirectoryUnavailable => StartupStatus::Fatal { issue: self.issue },
            _ => StartupStatus::RecoveryRequired {
                issue: self.issue,
                diagnostic: self.diagnostic,
                can_rebuild: self.can_rebuild,
                backup_created,
            },
        }
    }
}

pub async fn attempt_startup(db_path: &Path) -> Result<DbPool, StartupAttemptFailure> {
    let data_dir = db_path.parent().unwrap_or_else(|| Path::new("."));
    let create_dir = data_dir.to_path_buf();
    if let Err(error) = run_blocking_fs_with(
        "startup data directory creation",
        move || std::fs::create_dir_all(create_dir).map_err(StartupError::data_directory),
        StartupError::task_join,
    )
    .await
    {
        return Err(StartupAttemptFailure {
            issue: StartupIssue::DataDirectoryUnavailable,
            diagnostic: StartupDiagnostic::NotRun,
            can_rebuild: false,
            error,
        });
    }

    match db::open_database_for_startup(db_path).await {
        Ok(pool) => Ok(pool),
        Err(error) => {
            let stage = error.stage();
            let diagnostic = diagnose_database(db_path).await;
            let issue = match stage {
                DatabaseOpenStage::Open => StartupIssue::DatabaseOpenFailed,
                DatabaseOpenStage::Initialize => StartupIssue::SchemaInitializationFailed,
            };
            let error = match error {
                DatabaseOpenFailure::Open(source) => StartupError::DatabaseOpen { source },
                DatabaseOpenFailure::Initialize(source) => {
                    StartupError::SchemaInitialization { source }
                }
            };
            Err(StartupAttemptFailure {
                issue,
                diagnostic,
                can_rebuild: db_path.is_file(),
                error,
            })
        }
    }
}

async fn diagnose_database(db_path: &Path) -> StartupDiagnostic {
    if !db_path.is_file() {
        return StartupDiagnostic::NotRun;
    }
    match db::inspect_database_integrity(db_path).await {
        Ok(true) => StartupDiagnostic::Healthy,
        Ok(false) => StartupDiagnostic::Corrupt,
        Err(_error) => {
            tracing::warn!(
                code = "startup.integrity_check_unavailable",
                diagnostic = ?StartupDiagnostic::Unavailable,
                "Startup database integrity diagnosis was unavailable"
            );
            StartupDiagnostic::Unavailable
        }
    }
}

fn companion_path(path: &Path, suffix: &str) -> PathBuf {
    let mut value: OsString = path.as_os_str().to_owned();
    value.push(suffix);
    PathBuf::from(value)
}

fn rollback_database_set(moved: &[(PathBuf, PathBuf)], recovery_dir: &Path) -> io::Result<()> {
    for (source, temporary_target) in moved.iter().rev() {
        let file_name = temporary_target
            .file_name()
            .ok_or_else(|| io::Error::other("missing recovery file name"))?;
        std::fs::rename(recovery_dir.join(file_name), source)?;
    }
    std::fs::remove_dir(recovery_dir)
}

fn backup_database_set_sync_with_hooks<F, G>(
    db_path: &Path,
    mut before_move: F,
    after_publish: G,
) -> Result<PathBuf, StartupError>
where
    F: FnMut(usize, &Path) -> io::Result<()>,
    G: FnOnce() -> io::Result<()>,
{
    let parent = db_path
        .parent()
        .ok_or_else(|| StartupError::recovery_backup(io::Error::other("missing DB parent")))?;
    let database_name = db_path
        .file_name()
        .ok_or_else(|| StartupError::recovery_backup(io::Error::other("missing DB file name")))?;
    let token = format!(
        "{}-{}",
        Utc::now().format("%Y%m%dT%H%M%S%.3fZ"),
        Uuid::new_v4()
    );
    let temp_dir = parent.join(format!(".startup-recovery-{token}.tmp"));
    let final_dir = parent.join(format!("startup-recovery-{token}"));
    std::fs::create_dir(&temp_dir).map_err(StartupError::recovery_backup)?;

    let sources = [
        db_path.to_path_buf(),
        companion_path(db_path, "-wal"),
        companion_path(db_path, "-shm"),
    ];
    let mut moved: Vec<(PathBuf, PathBuf)> = Vec::new();
    let mut published = false;
    let move_result = (|| {
        for (index, source) in sources.iter().filter(|path| path.exists()).enumerate() {
            before_move(index, source).map_err(StartupError::recovery_backup)?;
            let file_name = source.file_name().unwrap_or(database_name);
            let target = temp_dir.join(file_name);
            std::fs::rename(source, &target).map_err(StartupError::recovery_backup)?;
            moved.push((source.clone(), target));
        }
        if moved.is_empty() {
            return Err(StartupError::NoDatabaseFiles);
        }

        #[cfg(unix)]
        std::fs::File::open(&temp_dir)
            .and_then(|directory| directory.sync_all())
            .map_err(StartupError::recovery_backup)?;

        std::fs::rename(&temp_dir, &final_dir).map_err(StartupError::recovery_backup)?;
        published = true;
        after_publish().map_err(StartupError::recovery_backup)?;

        #[cfg(unix)]
        std::fs::File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(StartupError::recovery_backup)?;
        Ok(())
    })();

    if let Err(move_error) = move_result {
        let recovery_dir = if published { &final_dir } else { &temp_dir };
        let rollback_result = rollback_database_set(&moved, recovery_dir);
        return match rollback_result {
            Ok(()) => Err(move_error),
            Err(rollback_error) => Err(StartupError::RecoveryRollback {
                move_error: move_error.to_string(),
                rollback_error: rollback_error.to_string(),
            }),
        };
    }

    Ok(final_dir)
}

fn backup_database_set_sync<F>(db_path: &Path, before_move: F) -> Result<PathBuf, StartupError>
where
    F: FnMut(usize, &Path) -> io::Result<()>,
{
    backup_database_set_sync_with_hooks(db_path, before_move, || Ok(()))
}

pub async fn backup_database_set(db_path: &Path) -> Result<PathBuf, StartupError> {
    let db_path = db_path.to_path_buf();
    run_blocking_fs_with(
        "startup database recovery backup",
        move || backup_database_set_sync(&db_path, |_index, _source| Ok(())),
        StartupError::task_join,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqliteConnectOptions;
    use sqlx::{Connection, SqliteConnection};
    use std::sync::Arc;
    use std::time::Duration;

    #[tokio::test]
    async fn coordinator_serializes_startup_operations() {
        let coordinator = Arc::new(StartupCoordinator::new(PathBuf::from("db.sqlite")));
        let first = coordinator.lock_operation().await;
        let waiting_coordinator = Arc::clone(&coordinator);
        let second = tokio::spawn(async move {
            let _operation = waiting_coordinator.lock_operation().await;
        });

        tokio::task::yield_now().await;
        assert!(!second.is_finished());

        drop(first);
        tokio::time::timeout(Duration::from_secs(1), second)
            .await
            .expect("second startup operation should acquire the released lease")
            .expect("startup operation task should complete");
    }

    #[tokio::test]
    async fn data_directory_failure_is_fatal_without_panicking() {
        let temp = tempfile::tempdir().unwrap();
        let occupied = temp.path().join("occupied");
        std::fs::write(&occupied, b"file").unwrap();

        let failure = attempt_startup(&occupied.join("db.sqlite"))
            .await
            .unwrap_err();

        assert_eq!(failure.issue, StartupIssue::DataDirectoryUnavailable);
        assert_eq!(failure.diagnostic, StartupDiagnostic::NotRun);
        assert!(!failure.can_rebuild);
        assert_eq!(
            failure.status(false),
            StartupStatus::Fatal {
                issue: StartupIssue::DataDirectoryUnavailable
            }
        );
    }

    #[tokio::test]
    async fn corrupt_database_is_recoverable_and_retry_does_not_modify_it() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("db.sqlite");
        let original = b"not a sqlite database";
        std::fs::write(&db_path, original).unwrap();

        let failure = attempt_startup(&db_path).await.unwrap_err();

        assert_eq!(failure.issue, StartupIssue::DatabaseOpenFailed);
        assert!(matches!(
            failure.diagnostic,
            StartupDiagnostic::Corrupt | StartupDiagnostic::Unavailable
        ));
        assert!(failure.can_rebuild);
        assert_eq!(std::fs::read(&db_path).unwrap(), original);
    }

    #[tokio::test]
    async fn schema_preflight_failure_is_classified_without_modifying_the_database() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("db.sqlite");
        let options = SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true);
        let mut connection = SqliteConnection::connect_with(&options).await.unwrap();
        sqlx::query(
            "CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY, checksum TEXT NOT NULL, applied_at TEXT NOT NULL)",
        )
        .execute(&mut connection)
        .await
        .unwrap();
        sqlx::query("INSERT INTO schema_migrations VALUES (99, 'future', 'now')")
            .execute(&mut connection)
            .await
            .unwrap();
        connection.close().await.unwrap();

        let failure = attempt_startup(&db_path).await.unwrap_err();

        assert_eq!(failure.issue, StartupIssue::SchemaInitializationFailed);
        assert_eq!(failure.diagnostic, StartupDiagnostic::Healthy);
    }

    #[test]
    fn recovery_backup_preserves_database_and_companions() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("db.sqlite");
        let wal_path = companion_path(&db_path, "-wal");
        let shm_path = companion_path(&db_path, "-shm");
        std::fs::write(&db_path, b"db").unwrap();
        std::fs::write(&wal_path, b"wal").unwrap();
        std::fs::write(&shm_path, b"shm").unwrap();

        let backup = backup_database_set_sync(&db_path, |_index, _source| Ok(())).unwrap();

        assert!(!db_path.exists());
        assert!(!wal_path.exists());
        assert!(!shm_path.exists());
        assert_eq!(std::fs::read(backup.join("db.sqlite")).unwrap(), b"db");
        assert_eq!(std::fs::read(backup.join("db.sqlite-wal")).unwrap(), b"wal");
        assert_eq!(std::fs::read(backup.join("db.sqlite-shm")).unwrap(), b"shm");
    }

    #[tokio::test]
    async fn recovery_backup_is_retained_after_clean_database_initialization() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("db.sqlite");
        let original = b"damaged database bytes";
        std::fs::write(&db_path, original).unwrap();

        let backup = backup_database_set(&db_path).await.unwrap();
        let pool = attempt_startup(&db_path).await.unwrap();

        assert_eq!(std::fs::read(backup.join("db.sqlite")).unwrap(), original);
        assert!(db_path.is_file());
        assert!(sqlx::query("SELECT 1").fetch_one(&pool).await.is_ok());
        pool.close().await;
    }

    #[test]
    fn partial_backup_failure_rolls_every_file_back_and_publishes_nothing() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("db.sqlite");
        let wal_path = companion_path(&db_path, "-wal");
        let shm_path = companion_path(&db_path, "-shm");
        std::fs::write(&db_path, b"db").unwrap();
        std::fs::write(&wal_path, b"wal").unwrap();
        std::fs::write(&shm_path, b"shm").unwrap();

        let error = backup_database_set_sync(&db_path, |index, _source| {
            if index == 1 {
                Err(io::Error::other("injected move failure"))
            } else {
                Ok(())
            }
        })
        .unwrap_err();

        assert!(matches!(error, StartupError::RecoveryBackup { .. }));
        assert_eq!(std::fs::read(&db_path).unwrap(), b"db");
        assert_eq!(std::fs::read(&wal_path).unwrap(), b"wal");
        assert_eq!(std::fs::read(&shm_path).unwrap(), b"shm");
        assert_eq!(
            std::fs::read_dir(temp.path())
                .unwrap()
                .filter_map(Result::ok)
                .filter(|entry| entry
                    .file_name()
                    .to_string_lossy()
                    .contains("startup-recovery"))
                .count(),
            0
        );
    }

    #[test]
    fn failure_after_backup_publish_restores_every_file() {
        let temp = tempfile::tempdir().unwrap();
        let db_path = temp.path().join("db.sqlite");
        let wal_path = companion_path(&db_path, "-wal");
        let shm_path = companion_path(&db_path, "-shm");
        std::fs::write(&db_path, b"db").unwrap();
        std::fs::write(&wal_path, b"wal").unwrap();
        std::fs::write(&shm_path, b"shm").unwrap();

        let error = backup_database_set_sync_with_hooks(
            &db_path,
            |_index, _source| Ok(()),
            || Err(io::Error::other("injected post-publish failure")),
        )
        .unwrap_err();

        assert!(matches!(error, StartupError::RecoveryBackup { .. }));
        assert_eq!(std::fs::read(&db_path).unwrap(), b"db");
        assert_eq!(std::fs::read(&wal_path).unwrap(), b"wal");
        assert_eq!(std::fs::read(&shm_path).unwrap(), b"shm");
        assert_eq!(
            std::fs::read_dir(temp.path())
                .unwrap()
                .filter_map(Result::ok)
                .filter(|entry| entry
                    .file_name()
                    .to_string_lossy()
                    .contains("startup-recovery"))
                .count(),
            0
        );
    }
}
