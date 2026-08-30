//! Manual performance harness for the usage scan pipeline.
//!
//! These tests are `#[ignore]`d: they scan the *real* home directory of the
//! machine running them (several GB of session logs) and exist so that
//! performance work has reproducible before/after numbers.
//!
//! Run with:
//!   cargo test --release --locked usage_bench -- --ignored --nocapture --test-threads=1
//!
//! `usage_bench_full_scan_wall_time` runs two consecutive forced refreshes
//! against an in-memory database: the first is a cold full scan, the second
//! is the steady-state rescan (near-zero file IO once the incremental file
//! cache is warm).

use std::time::Instant;

use crate::services::usage::{refresh, Scope};
use crate::test_support::mem_pool;

#[allow(clippy::await_holding_lock)]
#[tokio::test]
#[ignore = "manual benchmark against the real home directory"]
async fn usage_bench_full_scan_wall_time() {
    let _guard = super::ENV_LOCK.lock().unwrap();
    let pool = mem_pool().await;

    let cold_started = Instant::now();
    let cold = refresh(&pool, &Scope::Local, true).await.unwrap();
    let cold_elapsed = cold_started.elapsed();
    eprintln!(
        "BENCH cold full scan: {:?} (calls_written={}, providers_available={})",
        cold_elapsed, cold.calls_written, cold.providers_available
    );

    let warm_started = Instant::now();
    let warm = refresh(&pool, &Scope::Local, true).await.unwrap();
    let warm_elapsed = warm_started.elapsed();
    eprintln!(
        "BENCH steady-state rescan: {:?} (calls_written={})",
        warm_elapsed, warm.calls_written
    );
}
