# Retained bytes and clone evidence

## Baseline

- Before this task, `CentralUpdateSnapshotCache::get_fresh` returned
  `cached.snapshot.clone()`. `GitHubRepoSnapshot.files` is a
  `HashMap<String, Vec<u8>>`, so every cache hit deep-copied all retained file
  payloads.
- The download path also called `cache.insert(key, snapshot.clone())` before
  inserting the owned snapshot into the request result map, causing a second
  full payload copy per downloaded repository.
- The map had no entry or aggregate byte limit. A single repository already had
  an expanded archive budget of 256 MiB, so repeated repositories had no
  process-level retained-memory bound.
- The focused identity fixture retains 12 payload bytes (`shared bytes`). The
  production upper case remains the existing 256 MiB single-snapshot resource
  budget; this task does not claim production telemetry or a surveyed typical
  repository size.

## Final state

- Download creates one `Arc<GitHubRepoSnapshot>`. Cache insertion and the current
  request map use `Arc::clone`; hits return the same allocation. There is no
  `GitHubRepoSnapshot::clone()` in the Central snapshot cache/result path.
- `GitHubRepoSnapshot::retained_bytes()` checked-adds each `Vec<u8>::len()` into
  `u64`; `retained_byte_accounting_detects_checked_overflow` proves
  `u64::MAX + 1` returns `SnapshotSizeOverflow`.
- Production Central policy is 8 entries, 256 MiB aggregate and 10-minute TTL.
  An item over the aggregate cap remains available to its current request but is
  not cached. Replacement subtracts the old retained bytes before insertion.
- Monotonic access sequences select deterministic LRU victims without wall-clock
  ties. Cache state logs expose only entry count, retained bytes and reason.

## Focused evidence

- `central_updates::snapshots::tests`: 9 passed, including pointer identity,
  entry/LRU, byte cap, TTL, replacement, oversized current-use-only and
  invalidation of an older same-key snapshot during an oversized refresh.
- `github_import::snapshot_registry::tests`: 12 passed, including checked
  overflow, per-target/byte/global caps, active lease, deferred discard,
  A/B ownership, cleanup retry, stale ack, concurrent reservation and the
  reserved-slot cleanup-failure and cancellation-after-workspace-claim caps.
- `preview_snapshot`: 23 passed for 10 consecutive parallel rounds after making
  the test-only expiry sweep target-scoped, preserving digest, binding,
  immutable bytes, import failure retry, success consume and per-skill
  provenance without process-global test interference.
- FakeRunner exact tests: 3 passed. They prove remove failure remains
  cleanup-pending until owning-target retry ack, target A never removes target
  B's workspace, and a same-id connection with a different target kind cannot
  remove or acknowledge the workspace.

## Full gates

- `cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check`: passed.
- all-targets locked Clippy with `-D warnings`: passed.
- locked Rust tests: 1,078 passed and 6 ignored across all targets.
- Node 22.23.2 frontend contract test: 13 passed.
- `pnpm docs:gen` and `pnpm docs:gen:check`: passed; both generated
  architecture documents were already current and produced no tracked diff.
- Node 22.23.2 `just ci`: passed, including 1,609 Vitest tests with 1 skipped,
  Rust tests, typecheck, lint, sizecheck, IPC codegen, builds and docs.
