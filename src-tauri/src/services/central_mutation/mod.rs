mod error;

use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use fs2::FileExt;

use crate::fs_util::run_blocking_fs_with;

pub use error::CentralMutationError;

const RETRY_INTERVAL: Duration = Duration::from_millis(25);
pub const DEFAULT_CENTRAL_MUTATION_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug)]
pub struct CentralMutationGuard {
    file: File,
    operation: &'static str,
    waited: Duration,
}

impl CentralMutationGuard {
    pub fn operation(&self) -> &'static str {
        self.operation
    }

    pub fn waited(&self) -> Duration {
        self.waited
    }
}

impl Drop for CentralMutationGuard {
    fn drop(&mut self) {
        if let Err(error) = FileExt::unlock(&self.file) {
            tracing::warn!(operation = self.operation, error = %error, "failed to unlock Central mutation file");
        }
    }
}

pub async fn acquire_central_mutation_guard(
    operation: &'static str,
    timeout: Duration,
) -> Result<CentralMutationGuard, CentralMutationError> {
    acquire_central_mutation_guard_at(crate::paths::central_mutation_lock_path(), operation, timeout)
        .await
}

pub(crate) async fn acquire_central_mutation_guard_at(
    path: PathBuf,
    operation: &'static str,
    timeout: Duration,
) -> Result<CentralMutationGuard, CentralMutationError> {
    run_blocking_fs_with(
        "acquire Central mutation lock",
        move || acquire_lock_blocking(&path, operation, timeout),
        CentralMutationError::task_join,
    )
    .await
}

fn acquire_lock_blocking(
    path: &Path,
    operation: &'static str,
    timeout: Duration,
) -> Result<CentralMutationGuard, CentralMutationError> {
    let parent = path.parent().ok_or_else(|| {
        CentralMutationError::io(
            "Central mutation lock path has no parent",
            std::io::Error::new(std::io::ErrorKind::InvalidInput, path.display().to_string()),
        )
    })?;
    std::fs::create_dir_all(parent).map_err(|error| {
        CentralMutationError::io("Failed to create Central mutation lock directory", error)
    })?;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|error| CentralMutationError::io("Failed to open Central mutation lock", error))?;

    let started = Instant::now();
    loop {
        match file.try_lock_exclusive() {
            Ok(()) => {
                let waited = started.elapsed();
                tracing::debug!(operation, waited_ms = waited.as_millis(), "acquired Central mutation lock");
                return Ok(CentralMutationGuard {
                    file,
                    operation,
                    waited,
                });
            }
            Err(error) if is_lock_contention(&error) => {
                if timeout.is_zero() {
                    return Err(CentralMutationError::Busy { operation });
                }
                if started.elapsed() >= timeout {
                    return Err(CentralMutationError::Timeout {
                        operation,
                        timeout_ms: timeout.as_millis(),
                    });
                }
                std::thread::sleep(RETRY_INTERVAL.min(timeout.saturating_sub(started.elapsed())));
            }
            Err(error) => {
                return Err(CentralMutationError::io(
                    "Failed to acquire Central mutation lock",
                    error,
                ));
            }
        }
    }
}

fn is_lock_contention(error: &std::io::Error) -> bool {
    if error.kind() == std::io::ErrorKind::WouldBlock {
        return true;
    }

    #[cfg(windows)]
    {
        // Windows reports sharing/lock violations as Uncategorized.
        matches!(error.raw_os_error(), Some(32 | 33))
    }

    #[cfg(not(windows))]
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    const HELPER_ENV: &str = "SKILLPORT_CENTRAL_LOCK_HELPER";
    const LOCK_PATH_ENV: &str = "SKILLPORT_CENTRAL_LOCK_PATH";
    const READY_PATH_ENV: &str = "SKILLPORT_CENTRAL_LOCK_READY";

    #[tokio::test]
    async fn central_mutation_lock_process_helper() {
        if std::env::var_os(HELPER_ENV).is_none() {
            return;
        }
        let lock_path = PathBuf::from(std::env::var_os(LOCK_PATH_ENV).unwrap());
        let ready_path = PathBuf::from(std::env::var_os(READY_PATH_ENV).unwrap());
        let _guard = acquire_central_mutation_guard_at(
            lock_path,
            "process helper",
            Duration::from_secs(5),
        )
        .await
        .unwrap();
        std::fs::write(&ready_path, b"ready").unwrap();
        std::thread::sleep(Duration::from_secs(30));
    }

    #[tokio::test]
    async fn independent_process_contention_times_out_and_crash_releases_lock() {
        let temp = tempfile::tempdir().unwrap();
        let lock_path = temp.path().join("central-mutation.lock");
        let ready_path = temp.path().join("ready");
        let mut child = Command::new(std::env::current_exe().unwrap())
            .arg("central_mutation_lock_process_helper")
            .arg("--nocapture")
            .env(HELPER_ENV, "1")
            .env(LOCK_PATH_ENV, &lock_path)
            .env(READY_PATH_ENV, &ready_path)
            .spawn()
            .unwrap();

        let started = Instant::now();
        while !ready_path.exists() && started.elapsed() < Duration::from_secs(10) {
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        assert!(ready_path.exists(), "helper process did not acquire the lock");

        let error = acquire_central_mutation_guard_at(
            lock_path.clone(),
            "contender",
            Duration::from_millis(100),
        )
        .await
        .unwrap_err();
        assert!(
            matches!(error, CentralMutationError::Timeout { .. }),
            "unexpected contention error: {error:?}"
        );

        child.kill().unwrap();
        child.wait().unwrap();
        let _guard = acquire_central_mutation_guard_at(
            lock_path,
            "after crash",
            Duration::from_secs(2),
        )
        .await
        .unwrap();
    }
}
