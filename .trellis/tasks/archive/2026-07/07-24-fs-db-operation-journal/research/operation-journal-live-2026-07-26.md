# Operation Journal Live Evidence - 2026-07-26

## Dependency State

- `07-24-target-context-snapshot` is archived. `TargetContext` owns the exact `ActiveTarget` and matching `DbPool` used by a request.
- `07-24-db-schema-versioning-fk` is archived at work commit `6594b85e`. Database opens now run checksum-locked migrations, backups, recovery validation, per-connection FK enforcement, and seed through one path-aware boundary.
- The parent `07-24-audit-remediation` reports `7/16` children complete before this child starts.

## Delete Failure Windows

### Local

- `services/central_skills/delete.rs:509-579` acquires the Local Central mutation lock, validates rows, removes installation paths and the Central directory inside `run_blocking_fs`, then calls `db::delete_skill`.
- A DB failure after the blocking FS phase leaves metadata for paths that no longer exist. There is no backup or compensation marker.
- Batch delete at `:588+` loops the same single-skill path, so partial success is already per skill.

### SSH / WSL

- `services/central_skills/delete.rs:287-359` connects to the frozen `ActiveTarget`, removes remote installation paths and the Central path, then calls `db::delete_skill` against the matching target cache DB.
- The remote path uses no mutation guard. A DB/cache failure after remote deletion is the same split-brain window as Local.
- `ConnectedRemoteTarget` exposes supervised command, exists, read, write, and remove operations. Idempotent rename/stage/restore/finalize operations can remain behind compound scripts and this transport boundary.

## Update Failure Windows

- `services/central_updates/core/batch.rs:28-133` is the unique production batch orchestrator required by `central-update-batching.md`.
- It first calls `write_skill_dirs_atomic_cancellable` for every plan, then executes `upsert_skill` and repository assignment per skill, then builds and executes copy refresh requests.
- `persist_updated_skill` uses multiple pool-level writes rather than one transaction.
- Local `replace_target_dir` in `services/central_updates/fs.rs:197-269` renames old to backup, swaps staging to target, and deletes backup before DB persistence starts.
- `REMOTE_CENTRAL_BATCH_UPDATE_SCRIPT` similarly deletes each backup before emitting `OK`. Its trap removes the batch workspace on exit, so the remote host retains no durable recovery material after a crash.
- Copy refresh is already a derived, per-skill outcome: a refresh failure reports an error but does not restore canonical source or DB metadata. The Saga should preserve this roll-forward semantic and make retries durable.

## Reusable Boundaries

- `local_archive_import/import.rs:85-201` demonstrates preview revalidation, unique staging, sibling backup, target swap, DB persistence, restore on error, and backup cleanup. The missing capability is durable state across process exit.
- `central_mutation` uses an `fs2` cross-process lock and proves holder crash releases the OS lock. It can be generalized to a digest-derived per-target lock without inventing another lock mechanism.
- Existing Operation Logs support `succeeded`, `failed`, `partial`, and `cancelled` UI states. They are user-facing audit summaries, not durable compensation storage.
- The current migration registry contains immutable versions 1 and 2. The journal must append version 3 and lock its source checksum.

## Recovery Ordering Proof

1. Commit `prepared` before destructive work.
2. Stage new data or rename old data to operation-owned backups; commit the resulting phase marker.
3. Begin one SQLite transaction and apply business mutations.
4. Perform the idempotent FS swap/rename while the transaction is uncommitted.
5. Write `db_committed` in the same transaction and commit.
6. Refresh derived copies, finalize backups, and mark `completed`.

If the process exits before step 5 commits, SQLite rolls back business data and the journal still exposes the pre-commit phase, so recovery restores old FS. If commit succeeds, `db_committed` is visible with business data, so recovery rolls forward. No recovery decision depends on whether the caller observed the commit return value.

## Planning Risk

Remote recovery cannot be globally synchronous at desktop startup because SSH/WSL targets may be offline. The safe boundary is target-scoped: never run a new mutation for a target with unresolved journal rows. Whether read-only cached use remains available while remote recovery is pending is a product availability decision.
