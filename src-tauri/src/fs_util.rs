//! Cross-domain filesystem helper: the canonical `spawn_blocking` wrapper for
//! running synchronous `std::fs` work from async contexts.
//!
//! Heavy IO (recursive copy/delete/traversal, batch writes, directory moves)
//! must go through [`run_blocking_fs`] so the Tauri async runtime workers are
//! never blocked by disk latency. Domain modules re-export or import this
//! wrapper directly — do not introduce a second wrapping pattern.

/// Run a synchronous filesystem task on the blocking-thread pool.
pub(crate) async fn run_blocking_fs<T, F>(label: &'static str, task: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(task)
        .await
        .map_err(|e| format!("Failed to join {} task: {}", label, e))?
}
