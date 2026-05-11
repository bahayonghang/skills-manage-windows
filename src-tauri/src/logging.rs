use crate::paths;
use chrono::Local;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;

static LOG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

#[derive(Debug)]
struct DailyLogWriter {
    log_dir: PathBuf,
}

impl DailyLogWriter {
    fn new(log_dir: PathBuf) -> Self {
        Self { log_dir }
    }

    fn current_log_path(&self) -> PathBuf {
        daily_log_path(&self.log_dir)
    }
}

impl Write for DailyLogWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let path = self.current_log_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        file.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub fn logs_dir() -> PathBuf {
    paths::app_data_dir().join("logs")
}

pub fn init_file_logging() -> Result<(), String> {
    match init_file_logging_with_dir(&logs_dir()) {
        Ok(()) => {
            tracing::info!(
                target: "skillport::startup",
                log_dir = %paths::path_to_string(&logs_dir()),
                "SkillPort file logging initialized"
            );
            Ok(())
        }
        Err(error) => {
            init_stderr_logging()?;
            tracing::warn!(
                target: "skillport::startup",
                error = %error,
                "SkillPort file logging unavailable; using stderr logging"
            );
            Ok(())
        }
    }
}

pub fn init_file_logging_with_dir(log_dir: &Path) -> Result<(), String> {
    if LOG_GUARD.get().is_some() {
        return Ok(());
    }

    fs::create_dir_all(log_dir).map_err(|error| {
        format!(
            "Failed to create log directory '{}': {error}",
            log_dir.display()
        )
    })?;

    let appender = DailyLogWriter::new(log_dir.to_path_buf());
    init_file_logging_with_appender(appender)
}

fn daily_log_path(log_dir: &Path) -> PathBuf {
    log_dir.join(format!("skillport-{}.log", Local::now().format("%Y-%m-%d")))
}

fn init_file_logging_with_appender(appender: DailyLogWriter) -> Result<(), String> {
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(default_env_filter())
        .with_writer(writer)
        .with_ansi(false)
        .compact()
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .map_err(|error| format!("Failed to install tracing subscriber: {error}"))?;
    LOG_GUARD
        .set(guard)
        .map_err(|_| "Failed to retain tracing worker guard.".to_string())
}

fn init_stderr_logging() -> Result<(), String> {
    tracing_subscriber::fmt()
        .with_env_filter(default_env_filter())
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .compact()
        .try_init()
        .map_err(|error| format!("Failed to install stderr tracing subscriber: {error}"))
}

fn default_env_filter() -> EnvFilter {
    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logs_dir_is_under_app_data_dir() {
        assert!(logs_dir().ends_with(".skillsmanage/logs"));
    }

    #[test]
    fn daily_log_writer_writes_skillport_dated_log_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut appender = DailyLogWriter::new(temp_dir.path().to_path_buf());

        appender
            .write_all(b"SkillPort file logging initialized\n")
            .unwrap();
        appender.flush().unwrap();

        let entries = fs::read_dir(temp_dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        assert_eq!(entries.len(), 1);
        assert!(entries[0].starts_with("skillport-"));
        assert!(entries[0].ends_with(".log"));
        let content = fs::read_to_string(temp_dir.path().join(&entries[0])).unwrap();
        assert!(content.contains("SkillPort file logging initialized"));
    }

    #[test]
    fn operation_log_failure_warning_enters_daily_log_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let appender = DailyLogWriter::new(temp_dir.path().to_path_buf());
        let (writer, guard) = tracing_appender::non_blocking(appender);
        let subscriber = tracing_subscriber::fmt()
            .with_writer(writer)
            .with_ansi(false)
            .compact()
            .finish();
        let dispatch = tracing::Dispatch::new(subscriber);

        tracing::dispatcher::with_default(&dispatch, || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async {
                let pool = sqlx::SqlitePool::connect(":memory:").await.unwrap();
                crate::operation_log::record_operation_log_best_effort(
                    &pool,
                    crate::operation_log::local_target_context(),
                    crate::operation_log::OperationLogEvent::new(
                        "test",
                        "test.action",
                        "succeeded",
                        "No schema",
                    ),
                )
                .await;
            });
        });
        drop(guard);

        let entries = fs::read_dir(temp_dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert_eq!(entries.len(), 1);
        let content = fs::read_to_string(temp_dir.path().join(&entries[0])).unwrap();
        assert!(content.contains("Failed to record operation log"));
    }
}
