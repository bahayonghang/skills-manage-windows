# Recoverable FS + DB Operation Journal Contract

## 1. Scope / Trigger

- Central skill delete, update, or GitHub-backed content upsert mutates both canonical filesystem state and target-scoped SQLite state, so it must use this contract.
- Local, SSH, and WSL share the same phase model, durable manifest, per-target lease, and recovery rules.
- `fs_db_operations` is recovery state. It is separate from user-facing `operation_logs` and must never be included in operation-log export or telemetry.
- This contract does not replace the general job/cancel registry. It only owns Central delete/update/content-upsert consistency and recovery.

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

pub async fn preview_pending_delete_recovery(
    pool: &DbPool,
    target: &ActiveTarget,
    skill_id: &str,
    remote: Option<&ConnectedRemoteTarget>,
) -> Result<Option<PendingDeleteRecoveryPreview>, CentralOperationError>;

pub async fn force_abandon_prepared_delete_under_guard(
    pool: &DbPool,
    target: &ActiveTarget,
    skill_id: &str,
    remote: Option<&ConnectedRemoteTarget>,
) -> Result<ForceAbandonDecision, CentralOperationError>;

delete_central_skill(skill_id, remove_agent_ids, force: Option<bool>)
delete_central_skills(requests: Vec<BatchDeleteCentralSkillRequest /* force: bool */>)

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
- New delete manifests contain one entry per physical Local path or normalized absolute POSIX remote path, preserving first-occurrence order. Legacy duplicate entries remain decodable and may be collapsed only by explicit prepared-delete reconciliation when all evidence fields agree.
- Staging, backup, marker creation, swap, restore, finalize, phase transition, and copy refresh are idempotent. Before restore/finalize, verify the marker identity and expected fingerprint; collision preserves evidence and fails closed.
- The business DB mutation and `db_committed` phase transition share one SQLite transaction. A visible `db_committed` row is the commit point; commit-unknown handling reads the row before deciding rollback or roll-forward.
- Delete renames Central/native installation paths to operation-scoped sibling backups, deletes only the `skills` parent in the DB transaction, and relies on FK cascade for the seven owned relations. Retained copy installations are not moved or deleted.
- Update and GitHub-backed content upsert keep `update_skills_batch` as the only production orchestrator. They stage new contents, swap canonical data, and commit skill/repository state with the marker; update may then refresh copied installations as a derived projection.
- A first content upsert uses `OperationKind::CentralUpdate` plus `UpdateManifest(had_target=false)`. It does not introduce a parallel journal kind or schema. Candidate validation and snapshot acquisition finish before the target lock; final apply acquires that lock, recovers pending rows, and commits the skill row, repository membership, commit/digest provenance, and `db_committed` transition in one SQLite transaction.
- Copy refresh failure leaves `copies_pending`; it does not roll back committed canonical state. Recovery retries only incomplete projections, then finalizes backup and transitions to `completed`.
- Cancellation may prevent an operation before its destructive phase. After durable staging begins, the operation must synchronously settle or retain recoverable journal state; it must not be returned as an unjournaled cancellation.
- Every Central delete/update/recovery acquires the same target-derived cross-process lease. Under that lease, a new batch mutation recovers only pending rows for the selected skills before proceeding; startup recovery and explicit Retry remain full-target and fail-fast. There is no unlocked fallback.
- Desktop startup recovers Local rows only. SSH/WSL rows are listed without transport; explicit retry or the next mutation for the same target may establish remote transport.
- IPC resolves one `TargetContext`. The operation row target ID/kind must match that context; active-target changes never substitute a different DB or transport.
- Only terminal rows are eligible for retention deletion. Pending rows and their recovery artifacts have no TTL cleanup.
- Installation mutations check matching nonterminal rows while holding the same target mutation guard. A pending row blocks only the same target and skill.
- Explicit reconciliation previews `central_delete/prepared` evidence under the target guard and applies only `prepared -> rolled_back` after a fresh preview. Apply never mutates filesystem or business tables; remaining artifacts, owned missing paths, fingerprint drift, target mismatch, or inspection failure block it.
- Force-delete is a separate delete-dialog escape hatch. With `force=true`, the same target mutation lock first abandons an eligible `central_delete/prepared` row (`prepared -> rolled_back`, journal only) and then runs a new journaled delete of **current** owned paths. Restore / Retry stay fail-closed: `(original missing, backup missing)` remains `delete_restore_collision`.
- Force-abandon eligibility ignores fingerprint drift and owned-missing paths. It still blocks on backup/marker remaining, `phase != prepared`, unsupported kind, invalid/inconsistent manifest, or target mismatch. Missing unowned platform paths do not block.
- Delete preview may attach `pending_recovery` (operation id/kind/phase, stable error code, `force_delete_eligible`, blocker codes). IPC, Operation Logs, and toasts must not include paths, fingerprints, or `manifest_json`. Single-delete collisions map to `central_operation.delete_restore_collision`, not `internal.unexpected`.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| Invalid manifest version, kind, empty path, or operation-ID mismatch | Reject before recovery mutation; retain row |
| Target ID/kind differs from resolved context | Reject; do not connect or mutate another target |
| Invalid phase shortcut or stale expected phase | Repository returns `sqlx::Error`; no row update |
| Marker/fingerprint mismatch or occupied restore target | Typed recovery collision; preserve row and artifacts |
| DB apply fails before commit | Roll back SQLite, restore old FS state, transition to `rolled_back` |
| First content upsert DB apply fails after swap | Remove the new target, keep Marketplace installed cache false, transition to `rolled_back` |
| Commit returns error but `db_committed` is visible | Roll forward; never restore old canonical data |
| Journal/error/rollback/finalize write fails | Propagate the failure; never silently continue |
| Copy projection fails after commit | Keep `copies_pending`, record a bounded redacted error, allow retry |
| Remote target is offline | Keep pending row; fail only that target's mutation/retry |
| Unrelated skill has a pending row when a batch starts | Do not inspect, retry, timestamp, or rewrite that row's recovery evidence |
| Repeated restore/finalize/retry | Return the same converged old/new state without overwriting new user data |
| Operation Logs list/detail/export | Contain summary/code/ID only; never contain `manifest_json` or full paths |
| `force=false` and selected skill has a `prepared` delete whose expected paths are `(false, false)` | Restore fail-closes with `delete_restore_collision`; retain the pending row |
| `force=true`, no backup/marker, `central_delete/prepared`, fingerprint may have drifted | `prepared -> rolled_back`, then a new journaled delete of current owned paths |
| `force=true` but backup/marker remains or phase is not `prepared` | `central_skills.force_delete_blocked`; do not mutate filesystem or the journal |

## 5. Good / Base / Bad Cases

- **Good**: an update swaps canonical files, commits skill metadata plus `db_committed`, fails one copy, remains `copies_pending`, and later retries only that copy before cleanup.
- **Good**: a process dies after delete backups are staged; startup or next mutation restores all old paths and marks the operation `rolled_back`.
- **Base**: a Local operation with no copies reaches `completed`; repeated recovery finds no pending row and performs no filesystem work.
- **Bad**: delete the directory first and then call `db::delete_skill`; a DB failure loses user data.
- **Bad**: log `error = %error` when the source may contain a path or remote diagnostic; log the stable error code and persist `redacted_message()` instead.
- **Bad**: ignore marker cleanup, rollback, journal, or finalize errors with `let _ =`; the caller would report a false terminal state.
- **Good**: a stale `prepared` delete lists vanished platform copies and a drifted Central fingerprint, but no backup/marker; force-delete rolls the journal back and deletes the current Central copy.
- **Bad**: treat `(false, false)` as already-gone inside restore/Retry so a later delete proceeds without an explicit force confirmation.

## 6. Tests Required (Assertion Points)

- Migration 3 digest lock, upgrade from all frozen fixtures and version 2, current reopen idempotency, future/checksum rejection, and no mutation when backup/preflight fails.
- Repository phase graph rejects `prepared -> db_committed` and `fs_staged -> completed`; active uniqueness and terminal-only retention are enforced.
- Independent-process same-target lease contention/crash release plus different-target non-contention for Local/SSH/WSL identities.
- Subprocess kill matrix at prepared, staged, swapped, DB apply/commit, copies pending, and pre-completion; reopen must converge to complete old or new state and preserve collision evidence.
- Delete DB/marker/rename/remote failures prove rollback propagation, retained copies, FK cascade, symlink/native/copy semantics, and idempotent retry.
- Update normal/force/mirror use one Saga; 33 remote writes remain three chunks and copy refresh remains chunks of 32.
- A multi-file first content upsert preserves identical `SKILL.md`, references, scripts, and assets payloads across Local, Fake SSH, and Fake WSL, with `had_target=false` and per-skill commit/digest provenance.
- SSH and WSL FakeRunner tests assert complete supervised scripts, marker/fingerprint protocol, rollback/finalize idempotence, and redacted errors. Real WSL smoke stays ignored unless `SKILLPORT_TEST_WSL_DISTRO` is set.
- Frontend tests cover loading, empty, pending, retry success/failure, offline target, target-switch latest-wins, cached-log availability, and narrow-screen no-overlap.
- Force-delete tests cover the yao-meta shape (duplicate gone platform paths, Central present, drifted fingerprint), backup/marker rejection, non-`prepared` rejection, and reviewed `delete_restore_collision` / `force_delete_blocked` IPC codes with no path leak.
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
