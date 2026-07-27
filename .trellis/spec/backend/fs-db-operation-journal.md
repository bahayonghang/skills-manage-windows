# Recoverable FS + DB Operation Journal Contract

## 1. Scope / Trigger

- Central skill delete or update mutates both canonical filesystem state and target-scoped SQLite state, so it must use this contract.
- Local, SSH, and WSL share the same phase model, durable manifest, per-target lease, and recovery rules.
- `fs_db_operations` is recovery state. It is separate from user-facing `operation_logs` and must never be included in operation-log export or telemetry.
- This contract does not replace the general job/cancel registry. It only owns Central delete/update consistency and recovery.

## 2. Signatures (Command / Service / DB)

```rust
pub async fn list_pending_fs_db_operations(
    state: State<'_, AppState>,
) -> Result<Vec<PendingOperationSummary>, String>;

pub async fn retry_fs_db_operation(
    state: State<'_, AppState>,
    operation_id: String,
) -> Result<Vec<PendingOperationSummary>, String>;

pub async fn recover_pending_operations(
    pool: &DbPool,
    target: &ActiveTarget,
) -> Result<Vec<PendingOperationSummary>, CentralOperationError>;

pub async fn insert_fs_db_operation(
    pool: &DbPool,
    operation: NewFsDbOperation<'_>,
) -> Result<FsDbOperationRow, sqlx::Error>;

pub(crate) async fn transition_fs_db_operation_in_transaction(
    transaction: &mut Transaction<'_, Sqlite>,
    operation_id: &str,
    expected_phase: &str,
    next_phase: &str,
) -> Result<(), sqlx::Error>;
```

Migration 3 adds:

```sql
fs_db_operations(
  id, batch_id, target_id, target_kind, operation_kind, skill_id, phase,
  manifest_version, manifest_json, old_fingerprint, new_fingerprint,
  last_error_code, last_error_message, created_at, updated_at, completed_at
)
```

The stable phases are:

```text
prepared -> fs_staged -> fs_swapped -> db_committed
                                    -> copies_pending -> completed
prepared/fs_staged/fs_swapped -> rolled_back
```

Delete may use `fs_staged -> db_committed` because its backup rename is the destructive filesystem step. Same-phase transitions are idempotent; all other shortcuts are rejected by the repository.

## 3. Contracts (State / Manifest / Recovery)

- Insert `prepared` before any destructive filesystem or business-database write. One nonterminal operation per `(target_id, skill_id)` is enforced by a partial unique index.
- A version-1 manifest contains only operation-owned paths, marker paths, fingerprints, target identity, and copy projection completion flags. It contains no file contents, credentials, command output, host diagnostics, or secret material.
- Staging, backup, marker creation, swap, restore, finalize, phase transition, and copy refresh are idempotent. Before restore/finalize, verify the marker identity and expected fingerprint; collision preserves evidence and fails closed.
- The business DB mutation and `db_committed` phase transition share one SQLite transaction. A visible `db_committed` row is the commit point; commit-unknown handling reads the row before deciding rollback or roll-forward.
- Delete renames Central/native installation paths to operation-scoped sibling backups, deletes only the `skills` parent in the DB transaction, and relies on FK cascade for the seven owned relations. Retained copy installations are not moved or deleted.
- Update keeps `update_skills_batch` as the only production orchestrator. It stages new contents, swaps canonical data, commits skill/repository state with the marker, then refreshes copied installations as a derived projection.
- Copy refresh failure leaves `copies_pending`; it does not roll back committed canonical state. Recovery retries only incomplete projections, then finalizes backup and transitions to `completed`.
- Cancellation may prevent an operation before its destructive phase. After durable staging begins, the operation must synchronously settle or retain recoverable journal state; it must not be returned as an unjournaled cancellation.
- Every Central delete/update/recovery acquires the same target-derived cross-process lease. New mutations recover pending rows for that target under the lease before proceeding. There is no unlocked fallback.
- Desktop startup recovers Local rows only. SSH/WSL rows are listed without transport; explicit retry or the next mutation for the same target may establish remote transport.
- IPC resolves one `TargetContext`. The operation row target ID/kind must match that context; active-target changes never substitute a different DB or transport.
- Only terminal rows are eligible for retention deletion. Pending rows and their recovery artifacts have no TTL cleanup.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Invalid manifest version, kind, empty path, or operation-ID mismatch | Reject before recovery mutation; retain row |
| Target ID/kind differs from resolved context | Reject; do not connect or mutate another target |
| Invalid phase shortcut or stale expected phase | Repository returns `sqlx::Error`; no row update |
| Marker/fingerprint mismatch or occupied restore target | Typed recovery collision; preserve row and artifacts |
| DB apply fails before commit | Roll back SQLite, restore old FS state, transition to `rolled_back` |
| Commit returns error but `db_committed` is visible | Roll forward; never restore old canonical data |
| Journal/error/rollback/finalize write fails | Propagate the failure; never silently continue |
| Copy projection fails after commit | Keep `copies_pending`, record a bounded redacted error, allow retry |
| Remote target is offline | Keep pending row; fail only that target's mutation/retry |
| Repeated restore/finalize/retry | Return the same converged old/new state without overwriting new user data |
| Operation Logs list/detail/export | Contain summary/code/ID only; never contain `manifest_json` or full paths |

## 5. Good / Base / Bad Cases

- **Good**: an update swaps canonical files, commits skill metadata plus `db_committed`, fails one copy, remains `copies_pending`, and later retries only that copy before cleanup.
- **Good**: a process dies after delete backups are staged; startup or next mutation restores all old paths and marks the operation `rolled_back`.
- **Base**: a Local operation with no copies reaches `completed`; repeated recovery finds no pending row and performs no filesystem work.
- **Bad**: delete the directory first and then call `db::delete_skill`; a DB failure loses user data.
- **Bad**: log `error = %error` when the source may contain a path or remote diagnostic; log the stable error code and persist `redacted_message()` instead.
- **Bad**: ignore marker cleanup, rollback, journal, or finalize errors with `let _ =`; the caller would report a false terminal state.

## 6. Tests Required (Assertion Points)

- Migration 3 digest lock, upgrade from all frozen fixtures and version 2, current reopen idempotency, future/checksum rejection, and no mutation when backup/preflight fails.
- Repository phase graph rejects `prepared -> db_committed` and `fs_staged -> completed`; active uniqueness and terminal-only retention are enforced.
- Independent-process same-target lease contention/crash release plus different-target non-contention for Local/SSH/WSL identities.
- Subprocess kill matrix at prepared, staged, swapped, DB apply/commit, copies pending, and pre-completion; reopen must converge to complete old or new state and preserve collision evidence.
- Delete DB/marker/rename/remote failures prove rollback propagation, retained copies, FK cascade, symlink/native/copy semantics, and idempotent retry.
- Update normal/force/mirror use one Saga; 33 remote writes remain three chunks and copy refresh remains chunks of 32.
- SSH and WSL FakeRunner tests assert complete supervised scripts, marker/fingerprint protocol, rollback/finalize idempotence, and redacted errors. Real WSL smoke stays ignored unless `SKILLPORT_TEST_WSL_DISTRO` is set.
- Frontend tests cover loading, empty, pending, retry success/failure, offline target, target-switch latest-wins, cached-log availability, and narrow-screen no-overlap.
- Minimum closeout gate: focused frontend/Rust tests, `cargo fmt --all -- --check`, locked all-target Clippy/tests, default-concurrency `just ci`, task validation, and `git diff --check`.

## 7. Wrong vs Correct

### Wrong

```rust
fs::remove_dir_all(path)?;
db::delete_skill(pool, skill_id).await?;
let _ = record_phase(pool, operation_id, "completed").await;
```

### Correct

```rust
let row = insert_prepared_operation(pool, plan).await?;
stage_to_sibling_backup(&row.manifest).await?;
transition_fs_db_operation(pool, &row.id, "prepared", "fs_staged").await?;
commit_delete_fs_db_operation(pool, &row.id, skill_id).await?;
finalize_backup(&row.manifest).await?;
transition_fs_db_operation(pool, &row.id, "db_committed", "completed").await?;
```

The correct sequence makes both commit order and recovery state durable; every failure remains observable and retryable.

> Source task: `07-24-fs-db-operation-journal` (2026-07-27)
