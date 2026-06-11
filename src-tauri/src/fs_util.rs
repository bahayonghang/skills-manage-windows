//! Cross-domain filesystem helper: the canonical `spawn_blocking` wrapper for
//! running synchronous `std::fs` work from async contexts.
//!
//! Heavy IO (recursive copy/delete/traversal, batch writes, directory moves)
//! must go through [`run_blocking_fs`] (string errors) or
//! [`run_blocking_fs_with`] (typed domain errors) so the Tauri async runtime
//! workers are never blocked by disk latency. Domain modules re-export or
//! import these wrappers directly — do not introduce a second wrapping pattern.

/// Run a synchronous filesystem task on the blocking-thread pool, mapping a
/// failed join into the caller's error type via `join_error`.
///
/// Typed-error domains (e.g. `InstallationError`, `ScannerError`) call this
/// with their own join-error constructor so the task error type flows through
/// unchanged.
pub(crate) async fn run_blocking_fs_with<T, E, F>(
    label: &'static str,
    task: F,
    join_error: fn(&'static str, String) -> E,
) -> Result<T, E>
where
    T: Send + 'static,
    E: Send + 'static,
    F: FnOnce() -> Result<T, E> + Send + 'static,
{
    match tauri::async_runtime::spawn_blocking(task).await {
        Ok(result) => result,
        Err(e) => Err(join_error(label, e.to_string())),
    }
}

/// Run a synchronous filesystem task on the blocking-thread pool with
/// string errors (legacy domains; typed domains use [`run_blocking_fs_with`]).
pub(crate) async fn run_blocking_fs<T, F>(label: &'static str, task: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    run_blocking_fs_with(label, task, |label, message| {
        format!("Failed to join {} task: {}", label, message)
    })
    .await
}
