# Design: Recoverable FS + DB Operations

## 1. Problem Boundary

Central delete and update mutate two durability domains: SQLite and one or more Local/SSH/WSL filesystem paths. They cannot share a physical transaction, so this task adds a durable Saga whose commit point and recovery direction are mechanically derivable.

The design covers only Central skill delete and Central skill update. It exposes reusable journal, lease, filesystem-workspace, and recovery contracts, but does not migrate import or store-relocation flows in this child.

## 2. Decisions

| Decision | Choice | Reason |
|---|---|---|
| Availability with offline remote recovery | Target-scoped fail closed | Keep the application, other targets, and cached reads available; block only new mutations for the affected target. |
| Recovery unit | One row per skill, optional batch ID | Matches existing delete/update partial-success semantics and prevents one failed skill from owning unrelated backups. |
| Commit direction | `db_committed` is written with business data in the same transaction | Recovery never guesses whether an unobserved SQLite commit succeeded. |
| Copy refresh failure | Roll forward canonical source + DB, retry derived copies | Preserves current authority order and avoids rolling back a committed update because a projection is temporarily unavailable. |
| Local lease path | Keep the existing `central-mutation.lock` | Older GUI/CLI processes must still contend with the new implementation. Remote targets use digest-derived sibling locks. |
| Recovery UI | Current-target pending panel in Operation Logs | Reuses an existing operational surface and avoids a new global dashboard or background remote polling. |
| Abstraction level | Shared Central-operation state machine, domain-specific delete/update planners | Shares the difficult invariants without a generic transaction framework or async callback machinery. |

## 3. Component Map

### Database

- Append migration 3 under `db/migrations/versions/` and extend the immutable descriptor registry.
- Add `db/repos/fs_db_operations_repo.rs` for journal CRUD and transition-in-transaction APIs.
- Keep the table independent of `skills`: a delete operation must survive deletion of its skill row.

### Shared Service

Add `services/central_operation/` with narrowly scoped modules:

- `types.rs`: phase, operation kind, manifest version, delete/update manifests, pending summaries, recovery result.
- `lease.rs`: Local legacy lock plus SHA-256 target-derived remote lock paths using the existing `fs2` acquisition implementation.
- `fs.rs`: Local/remote operation-owned staging, backup, swap, restore, fingerprint, and finalize primitives.
- `recovery.rs`: phase-driven recovery and collision checks.
- `error.rs`: typed journal, lease, manifest, fingerprint, collision, DB, task-join, and remote errors.

`central_skills` still owns delete validation/result types. `central_updates` still owns snapshot preparation, batching, cancellation, and update results. Both delegate destructive application and recovery mechanics to the shared service.

### Command And Frontend

- Existing mutation commands continue to resolve one `TargetContext` before transport/DB work.
- Add thin commands to list pending operations for the active target and retry recovery for that same explicit context.
- Add typed command-map entries, a focused Zustand recovery slice, bilingual i18n, and a compact pending-recovery band in `OperationLogsView` with refresh/retry actions.
- Durable manifests remain in target DBs. The always-local `operation_logs` table receives only redacted summaries and remains independently clearable/exportable.

## 4. Journal Model

`fs_db_operations` contains:

```text
id TEXT PRIMARY KEY
batch_id TEXT NULL
target_id TEXT NOT NULL
target_kind TEXT NOT NULL
operation_kind TEXT NOT NULL        -- central_delete | central_update
skill_id TEXT NOT NULL
phase TEXT NOT NULL                 -- prepared | fs_staged | fs_swapped |
                                    -- db_committed | copies_pending |
                                    -- completed | rolled_back
manifest_version INTEGER NOT NULL
manifest_json TEXT NOT NULL
old_fingerprint TEXT NULL
new_fingerprint TEXT NULL
last_error_code TEXT NULL
last_error_message TEXT NULL         -- redacted bounded summary
created_at TEXT NOT NULL
updated_at TEXT NOT NULL
completed_at TEXT NULL
```

Indexes support `(target_id, phase, updated_at)` and `(batch_id, skill_id)`. A partial unique index rejects two nonterminal rows for the same target/skill. Terminal rows remain audit/recovery evidence until retention cleanup.

Manifest JSON is a private versioned recovery contract, not an IPC or export type. It stores operation-owned sibling path names, expected path kinds, installation link types, and fingerprints. It never stores file bytes, credentials, process commands, captured output, or secret-bearing URLs.

## 5. Filesystem Workspace Contract

- Every destructive path receives a same-filesystem sibling name containing only a constant prefix plus operation UUID. Atomic rename never depends on a cross-volume temp directory.
- Local recursive preparation/fingerprinting/cleanup runs through `run_blocking_fs_with`.
- Remote work stays behind `ConnectedRemoteTarget` and supervised compound scripts. Scripts expose idempotent `stage`, `swap`, `restore`, and `finalize` actions and do not trap-delete durable backups.
- Each action checks an operation-owned marker before touching a target. A marker mismatch, unexpected replacement, or fingerprint drift returns `RecoveryCollision` and preserves all evidence.
- Terminal cleanup may delete only paths named by a validated manifest and owned marker. Pending backups never expire by TTL.

## 6. State Machine

### Normal Delete

```text
validate + fingerprint
  -> commit prepared
  -> rename removable Central/install paths to sibling backups
  -> commit fs_staged
  -> begin DB transaction
       delete skills parent (FK cascade)
       mark db_committed
     commit
  -> finalize backups
  -> mark completed
```

### Normal Update

```text
snapshot validation + plan
  -> commit prepared
  -> write operation staging + fingerprint
  -> commit fs_staged
  -> begin DB transaction
       upsert skill + repository assignment
       rename old target to backup; swap staging to target
       mark fs_swapped, then db_committed in this transaction
     commit
  -> mark copies_pending when copied installations exist
  -> refresh only planned copies
  -> finalize backup + mark completed
```

The FS rename occurs while the SQLite transaction is open. WAL readers remain available; the per-target lease prevents competing mutations. Remote scripts perform only bounded rename/swap work inside the transaction, not archive download or staging construction.

## 7. Recovery Decision Table

| Durable phase | DB authority | Expected FS | Recovery action |
|---|---|---|---|
| `prepared` | old | canonical unchanged; staging may be partial | Delete owned staging, mark `rolled_back`. |
| delete `fs_staged` | old | removable paths in backups | Validate markers/fingerprints, restore every backup, mark `rolled_back`. |
| update `fs_staged` | old | old canonical intact; new staging complete | Delete owned staging, preserve old canonical, mark `rolled_back`. |
| update `fs_swapped` | old because transaction did not commit | new canonical + old backup | Remove marker-owned new target, restore old backup, mark `rolled_back`. |
| `db_committed` delete | new | targets absent, backups retained | Finalize backups, mark `completed`. |
| `db_committed` update | new | new canonical + old backup | Continue copy projection or mark `copies_pending`, then finalize. |
| `copies_pending` | new | new canonical authoritative | Retry only missing copy targets; finalize and complete when all succeed. |
| `completed` / `rolled_back` | terminal | no required work | No-op except retention cleanup. |

If actual DB/FS/marker state contradicts the row, recovery writes a bounded typed error without advancing phase. The target remains mutation-blocked, but cached reads and other targets remain available.

## 8. Entry And Recovery Flow

1. Desktop startup scans the Local DB and known target cache DBs without connecting remote transports.
2. Local pending rows are recovered under the Local lease. Remote pending rows are surfaced but not contacted.
3. A delete/update command resolves one `TargetContext`, acquires that target lease, and runs pending recovery before planning new mutations.
4. If SSH/WSL is offline or recovery collides, the new mutation returns typed `PendingRecovery`; the row and backups stay intact.
5. The Operation Logs pending panel lists rows from the active target DB. Retry resolves the same target ID, acquires its lease, connects only then, and returns a redacted summary.

## 9. Batch And Cancellation Semantics

- The existing update batch remains the sole orchestrator. Preparation and remote archive upload stay chunked; journal execution produces ordered per-skill outcomes.
- Duplicate skill IDs collapse before any row is created.
- Cancellation before `prepared` creates no row. Cancellation after `prepared` invokes synchronous rollback or leaves a recoverable nonterminal row; it never abandons destructive work as plain `cancelled`.
- Batch delete continues independent per-skill execution. A failed recovery for one skill blocks that target's next mutation until reconciled but does not rewrite already terminal sibling operations.

## 10. Compatibility And Security

- Migration 3 appends to versions 1/2 and participates in the existing database backup/restore path.
- Local lock compatibility is preserved. Remote lease names use a SHA-256 digest, never raw target IDs or hostnames.
- Operation Logs and errors expose operation ID, target ID/kind, kind, phase, counts, and bounded error code/message only. Full recovery paths and manifests are excluded from IPC exports and tracing.
- No recovery path may overwrite an unexpected user-created directory. Collision requires explicit operator retry after the conflicting state is resolved.

## 11. Rollback Strategy

- Before migration 3 lands, the service change is not enabled.
- Migration 3 is additive and older binaries ignore the new table; released migration source is never edited.
- Delete and update integrations land in the same work commit as their journal/recovery tests, so no production path writes non-recoverable intermediate rows.
- A failed implementation rollout can stop creating new journal rows while retaining the table and recovery command until all nonterminal rows are terminal.
