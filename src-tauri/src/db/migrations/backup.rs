use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};

use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{Connection, Row, SqliteConnection};
use uuid::Uuid;

use crate::db::DbPool;

#[cfg(test)]
static FAIL_NEXT_RESTORE_AFTER_QUARANTINE: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);
#[cfg(test)]
static FAIL_NEXT_BACKUP_VALIDATION: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

#[cfg(test)]
pub(super) fn fail_next_restore_after_quarantine() {
    FAIL_NEXT_RESTORE_AFTER_QUARANTINE.store(true, std::sync::atomic::Ordering::SeqCst);
}

#[cfg(test)]
pub(super) fn fail_next_backup_validation() {
    FAIL_NEXT_BACKUP_VALIDATION.store(true, std::sync::atomic::Ordering::SeqCst);
}

fn join_error(_label: &'static str, message: String) -> sqlx::Error {
    sqlx::Error::Io(io::Error::other(message))
}

fn io_error(error: io::Error) -> sqlx::Error {
    sqlx::Error::Io(error)
}

fn io_context(context: &str, error: io::Error) -> sqlx::Error {
    sqlx::Error::Io(io::Error::new(error.kind(), format!("{context}: {error}")))
}

fn companion_path(path: &Path, suffix: &str) -> PathBuf {
    let mut value: OsString = path.as_os_str().to_owned();
    value.push(suffix);
    PathBuf::from(value)
}

fn sibling_path(path: &Path, suffix: &str) -> Result<PathBuf, sqlx::Error> {
    let file_name = path.file_name().ok_or_else(|| {
        sqlx::Error::InvalidArgument(format!(
            "Database path '{}' has no file name",
            path.display()
        ))
    })?;
    let mut name = file_name.to_os_string();
    name.push(suffix);
    Ok(path.with_file_name(name))
}

fn rename_with_retry(source: &Path, target: &Path) -> io::Result<()> {
    const ATTEMPTS: usize = 40;
    for attempt in 0..ATTEMPTS {
        match std::fs::rename(source, target) {
            Ok(()) => return Ok(()),
            Err(error)
                if cfg!(windows)
                    && attempt + 1 < ATTEMPTS
                    && matches!(error.raw_os_error(), Some(5 | 32)) =>
            {
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Err(error) => return Err(error),
        }
    }
    unreachable!("rename retry loop always returns")
}

pub(super) async fn validate_sqlite_file(path: &Path) -> Result<(), sqlx::Error> {
    let options = SqliteConnectOptions::new().filename(path).read_only(true);
    let mut connection = SqliteConnection::connect_with(&options).await?;
    let result = sqlx::query("PRAGMA integrity_check")
        .fetch_one(&mut connection)
        .await?
        .try_get::<String, _>(0)?;
    connection.close().await?;
    if result != "ok" {
        return Err(sqlx::Error::InvalidArgument(format!(
            "SQLite integrity check failed for '{}': {result}",
            path.display()
        )));
    }
    Ok(())
}

pub(super) async fn create_backup(
    pool: &DbPool,
    database_path: &Path,
    source_version: i64,
) -> Result<PathBuf, sqlx::Error> {
    let attempt = Uuid::new_v4();
    let suffix = format!(".pre-migration-v{source_version}-{attempt}.sqlite3");
    let final_path = sibling_path(database_path, &suffix)?;
    let temp_path = sibling_path(database_path, &format!("{suffix}.tmp"))?;

    sqlx::query("VACUUM INTO ?")
        .bind(temp_path.to_string_lossy().into_owned())
        .execute(pool)
        .await?;
    #[cfg(test)]
    if FAIL_NEXT_BACKUP_VALIDATION.swap(false, std::sync::atomic::Ordering::SeqCst) {
        let cleanup_path = temp_path.clone();
        crate::fs_util::run_blocking_fs_with(
            "rejected database backup cleanup",
            move || std::fs::remove_file(cleanup_path).map_err(io_error),
            join_error,
        )
        .await?;
        return Err(sqlx::Error::InvalidArgument(
            "injected backup validation failure".to_string(),
        ));
    }
    if let Err(error) = validate_sqlite_file(&temp_path).await {
        let cleanup_path = temp_path.clone();
        let _ = crate::fs_util::run_blocking_fs_with(
            "database backup cleanup",
            move || std::fs::remove_file(cleanup_path).map_err(io_error),
            join_error,
        )
        .await;
        return Err(error);
    }

    let database_name = database_path
        .file_name()
        .ok_or_else(|| {
            sqlx::Error::InvalidArgument(format!(
                "Database path '{}' has no file name",
                database_path.display()
            ))
        })?
        .to_string_lossy()
        .into_owned();
    let backup_prefix = format!("{database_name}.pre-migration-v{source_version}-");
    let publish_temp = temp_path.clone();
    let publish_final = final_path.clone();
    let parent = database_path.parent().map(Path::to_path_buf);
    crate::fs_util::run_blocking_fs_with(
        "database backup publish",
        move || {
            std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&publish_temp)
                .and_then(|file| file.sync_all())
                .map_err(io_error)?;
            std::fs::rename(&publish_temp, &publish_final).map_err(io_error)?;

            if let Some(parent) = parent {
                #[cfg(unix)]
                std::fs::File::open(&parent)
                    .and_then(|directory| directory.sync_all())
                    .map_err(io_error)?;

                for entry in std::fs::read_dir(parent).map_err(io_error)? {
                    let entry = entry.map_err(io_error)?;
                    let path = entry.path();
                    let name = entry.file_name().to_string_lossy().into_owned();
                    if path != publish_final
                        && name.starts_with(&backup_prefix)
                        && name.ends_with(".sqlite3")
                    {
                        std::fs::remove_file(path).map_err(io_error)?;
                    }
                }
            }
            Ok(())
        },
        join_error,
    )
    .await?;

    Ok(final_path)
}

pub(super) async fn restore_backup(
    database_path: &Path,
    backup_path: &Path,
) -> Result<PathBuf, sqlx::Error> {
    let attempt = Uuid::new_v4();
    let failed_path = sibling_path(
        database_path,
        &format!(".failed-migration-{attempt}.sqlite3"),
    )?;
    let restore_temp = sibling_path(database_path, &format!(".restore-{attempt}.tmp"))?;
    let database_path = database_path.to_path_buf();
    let restore_database_path = database_path.clone();
    let backup_path = backup_path.to_path_buf();
    let failed_result = failed_path.clone();
    #[cfg(test)]
    let fail_after_quarantine =
        FAIL_NEXT_RESTORE_AFTER_QUARANTINE.swap(false, std::sync::atomic::Ordering::SeqCst);
    #[cfg(not(test))]
    let fail_after_quarantine = false;

    crate::fs_util::run_blocking_fs_with(
        "database migration restore",
        move || {
            if restore_database_path.exists() {
                rename_with_retry(&restore_database_path, &failed_path)
                    .map_err(|error| io_context("quarantine database", error))?;
            }
            for companion in [
                companion_path(&restore_database_path, "-wal"),
                companion_path(&restore_database_path, "-shm"),
            ] {
                if companion.exists() {
                    std::fs::remove_file(companion)
                        .map_err(|error| io_context("remove SQLite companion", error))?;
                }
            }

            if fail_after_quarantine {
                return Err(sqlx::Error::Io(io::Error::other(
                    "injected restore failure after quarantine",
                )));
            }

            std::fs::copy(&backup_path, &restore_temp)
                .map_err(|error| io_context("copy migration backup", error))?;
            std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&restore_temp)
                .and_then(|file| file.sync_all())
                .map_err(|error| io_context("sync restored database", error))?;
            std::fs::rename(&restore_temp, &restore_database_path)
                .map_err(|error| io_context("publish restored database", error))?;
            Ok(())
        },
        join_error,
    )
    .await?;

    validate_sqlite_file(&database_path).await?;
    Ok(failed_result)
}
