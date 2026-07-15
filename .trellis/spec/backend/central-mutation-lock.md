# Central Mutation Lock Contract

## 1. Scope / Trigger

Apply this contract to Local operations that finalize writes under the Central skill root or relocate that root. GUI and CLI must share the same lock; SSH/WSL mutations keep their remote atomic mechanisms and do not take this Local lock.

## 2. Signatures

```rust
pub async fn acquire_central_mutation_guard(
    operation: &'static str,
    timeout: Duration,
) -> Result<CentralMutationGuard, CentralMutationError>;
```

The lock path is defined only by `paths::central_mutation_lock_path()` as `app_data_dir()/locks/central-mutation.lock`.

## 3. Contracts

- Prepare network data, archives, previews, and unique staging directories before locking.
- After locking, reload DB/filesystem state, then perform final swap/delete, DB persistence, and existing best-effort operation logging.
- Acquire only at the top-level mutation use case. Internal helpers accept the established boundary and must not acquire the same advisory lock again.
- Lock acquisition runs through `fs_util::run_blocking_fs_with`; async workers must not poll `fs2` directly.
- Lock failure stops the mutation. There is no unlocked fallback.

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

## 5. Good / Base / Bad Cases

- Good: GitHub import stages and parses first, then locks around target recheck, swap, and DB upsert.
- Base: pure queries, previews, marketplace search, and remote mutations do not acquire the Local lock.
- Bad: CLI locks outside a service that locks again, causing self-deadlock.

## 6. Tests Required

- Spawn an independent helper process that holds the same lock file.
- Assert a contender receives typed timeout, then terminate the helper and assert acquisition succeeds.
- Run affected mutation-domain tests and `just ci` after moving a lock boundary.
- Search all production `acquire_central_mutation_guard` call sites and verify they are Local top-level final-apply paths.

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
