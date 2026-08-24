# Central Mutation Lock Contract

## 1. Scope / Trigger

Apply this contract to operations that finalize writes under a Central skill root, relocate that root, recover a pending Central operation, copy the legacy Universal Central store into the private Local store, or write platform skill directories that share those roots. Holders of the Local target guard include Central install/uninstall, Skills CLI global add/remove, and leftover **local** apply (the delete loop). Remote leftover apply takes that remote target's guard. GUI and CLI must share the same Local lock; Local/SSH/WSL delete and update also use a target-derived lease so mutations and recovery for one target serialize across processes.

Skills CLI exclusive job family `skills_cli` is **not** this lock. Acquire order when both exist: exclusive job lease → `acquire_target_mutation_guard` → spawn/delete. See `skills-cli-global.md` and `exclusive-job-lifecycle.md`.

## 2. Signatures

```rust
pub async fn acquire_central_mutation_guard(
    operation: &'static str,
    timeout: Duration,
) -> Result<CentralMutationGuard, CentralMutationError>;

pub async fn acquire_target_mutation_guard(
    target: &ActiveTarget,
    operation: &'static str,
    timeout: Duration,
) -> Result<CentralMutationGuard, CentralMutationError>;
```

Local keeps `paths::central_mutation_lock_path()` as `app_data_dir()/locks/central-mutation.lock`. SSH/WSL lock filenames are SHA-256 digests of the validated target ID under the same locks directory; raw target IDs never become path segments.

## 3. Contracts

- Prepare network data, archives, previews, and unique staging directories before locking.
- After locking, recover the pending journal rows owned by the mutation's selected skills, reload DB/filesystem state, then perform final swap/delete, DB persistence, and existing best-effort operation logging. Startup and explicit recovery still scan the full target.
- Acquire only at the top-level mutation use case. Internal helpers accept the established boundary and must not acquire the same advisory lock again.
- Top-level single and batch installation use cases own the target guard from the pending-recovery check through filesystem and installation-row mutation. Centralization invoked by those use cases is an under-guard helper.
- Lock acquisition runs through `fs_util::run_blocking_fs_with`; async workers must not poll `fs2` directly.
- Lock failure stops the mutation. There is no unlocked fallback.
- Row recovery, journal insertion, filesystem mutation, business DB commit, copy refresh, and finalization remain in the same top-level guard lifetime; scoped recovery must not add a nested lock.
- Legacy Central migration may read its completion marker as an unlocked fast path, but must acquire the Local lock, re-read the marker under that guard, run the copy, and write the marker before releasing the guard. The source is preserved and existing targets are skipped.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Lock acquired before timeout | Return guard with operation and wait duration |
| Zero timeout while held | `CentralMutationError::Busy` |
| Positive timeout expires | `CentralMutationError::Timeout` |
| Windows raw OS code 32 or 33 | Treat as lock contention, not generic IO |
| Other open/lock failure | `CentralMutationError::Io` |
| Blocking task join failure | `CentralMutationError::TaskJoin` |
| Holder process exits or crashes | OS releases lock; next process can acquire |
| Legacy migration waits behind another process | Recheck marker after acquisition; skip copy if already completed |
| Legacy migration lock or copy fails | Write no completion marker; retry on the next startup |

## 5. Good / Base / Bad Cases

- Good: GitHub import stages and parses first, then locks around target recheck, swap, and DB upsert.
- Base: pure queries, previews, marketplace search, and remote read-only commands acquire no mutation lease and do not connect solely for recovery.
- Bad: CLI locks outside a service that locks again, causing self-deadlock.
- Good: legacy migration holds the Local guard from its second marker read through copy and marker write.

## 6. Tests Required

- Spawn an independent helper process that holds the same lock file.
- Assert a contender receives typed timeout, then terminate the helper and assert acquisition succeeds.
- Run affected mutation-domain tests and `just ci` after moving a lock boundary.
- Search all production `acquire_central_mutation_guard` call sites and verify they are Local top-level final-apply paths.
- Assert same-target Local/SSH/WSL delete, update, and recovery contend; different target digests do not contend.
- Unit tests using the real default lock path serialize through the `cfg(test)` in-process guard so unrelated parallel tests cannot consume the production timeout. Contention/crash tests bypass that guard through `acquire_central_mutation_guard_at` and use isolated paths/processes.
- Hold an isolated Local guard, assert legacy migration returns typed timeout and writes no marker, release it, then assert the same migration succeeds and preserves the source.

## 7. Wrong vs Correct

```rust
// Wrong: Windows lock violations arrive as Uncategorized on this toolchain.
if error.kind() == std::io::ErrorKind::WouldBlock { retry(); }

// Correct: classify portable WouldBlock plus Windows sharing/lock violations.
if error.kind() == std::io::ErrorKind::WouldBlock
    || cfg!(windows) && matches!(error.raw_os_error(), Some(32 | 33))
{
    retry();
}
```

```rust
// Wrong: marker decision and copy happen outside the shared mutation boundary.
if marker_missing(pool).await? { copy_legacy().await?; write_marker(pool).await?; }

// Correct: recheck and marker persistence are inside the same Local guard lifetime.
let _guard = acquire_central_mutation_guard("legacy Central store migration", timeout).await?;
if let Some(summary) = completed_summary(pool).await? { return Ok(summary); }
let summary = copy_legacy_blocking().await?;
write_marker(pool, &summary).await?;
```
