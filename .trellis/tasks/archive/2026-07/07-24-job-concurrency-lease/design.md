# Design: Exclusive UI Job Leases And Legacy Migration Coordination

## 1. Boundaries

This task adds two related but distinct coordination mechanisms:

- A process-local exclusive job registry owns renderer-visible command lifecycle and cancellation for Central update and portability families.
- The existing cross-process Central mutation file lock owns Local filesystem mutation serialization, including legacy migration.

The registry never replaces the file lock. The file lock never identifies renderer jobs or routes cancellation.

## 2. Exclusive Registry

Add a small shared service, conceptually:

```rust
pub struct ExclusiveJobRegistry { /* Arc<Mutex<RegistryState>> */ }
pub struct ExclusiveJobLease { /* registry inner + job id + cancel flag */ }

impl ExclusiveJobRegistry {
    pub fn acquire(&self, job_id: &str) -> Result<ExclusiveJobLease, JobRegistryError>;
    pub fn cancel(&self, job_id: &str) -> Result<bool, JobRegistryError>;
}

impl ExclusiveJobLease {
    pub fn job_id(&self) -> &str;
    pub fn cancel_flag(&self) -> &AtomicBool;
}
```

`RegistryState` contains at most one active job and one pending-cancel ID. `AppState` owns `central_update_jobs` and `portable_state_jobs`. `acquire` rejects an empty/oversized ID, rejects a second active job, and fails closed on poisoned state. If the pending ID matches, the new lease's flag starts cancelled; a different new ID discards the stale pending request. The lease owns an `Arc` to registry state; `Drop` removes the entry only when the stored ID still matches, so an old lease can never release a newer job.

`cancel` has three cases:

| State | Result |
| --- | --- |
| Active ID matches | set only that flag, return `Ok(true)` |
| No active job | remember this bounded pending ID, idempotent `Ok(false)` |
| Different active ID | coded mismatch error, leave active flag false |

Busy and mismatch errors expose fixed summaries only. IPC uses `job.central_update_busy`, `job.portability_busy`, and `job.id_mismatch` envelopes through the existing string error boundary.

## 3. Command Lifecycle

Renderer actions generate a UUID before invoke. Every affected command accepts `job_id: String` and calls its family registry before the first `.await`.

```text
renderer create ID -> set running state -> invoke(ID)
  -> backend acquire family lease
  -> resolve TargetContext / credentials / transport
  -> call existing service with lease cancel flag + explicit job ID
  -> emit ID-correlated progress
  -> return result/error
  -> lease Drop releases exact ID
```

Central update family members are the three cancel-aware legacy commands plus `apply_skill_update_decisions`. The Update Center apply action generates an ID even though its own UI does not expose cancellation; this still prevents it from racing a legacy update command.

Portability file preview must not invoke another Tauri command that reacquires the same lease. Extract a private command-layer helper that assumes an established lease/job context, or route both preview commands directly to the same service composition. Exactly one top-level lease covers file read through preview completion.

No service reads registry state. Services retain explicit cancel inputs and gain explicit job ID inputs only where they emit progress.

## 4. Event And Frontend State Contract

Add `jobId` to both Rust progress payloads and TypeScript payload interfaces. Every event emitted by one command carries its lease ID, including terminal/cancelled events.

Update and portability job state add `jobId: string | null`. Merge helpers return the current state unchanged when payload ID differs. Completion/error state writes after invoke are also conditional on the captured ID, preventing an older promise from overwriting a later job.

The store prevents a second same-store action from replacing an already running/cancelling state. Cross-store concurrency remains backend-authoritative. Cancel actions read the current ID; null means no-op.

Visible errors continue to live at the presentation boundary. Task Center already uses `formatBackendError`; Central update workflow, Update Center dialog, and portability dialog use the same formatter for new coded errors. Raw error strings remain in stores for diagnostics and stable branching.

## 5. Legacy Migration Flow

Production flow:

```text
read completed marker (fast path)
  -> acquire existing Local central-mutation.lock
  -> read completed marker again
  -> run one blocking copy unit
       create target
       read source entries
       copy missing skills
       clean only a failed partial target
       preserve source and existing targets
  -> serialize/write completed marker
  -> release lock
```

The blocking unit accepts owned source/target paths and returns the existing summary. `run_blocking_fs_with` maps join failure to a typed `CentralMigrationError::TaskJoin`. Lock errors are a typed transparent variant. DB and progress work remain async; `AppHandle` never enters the blocking closure.

Tests use injectable temp source/target/lock paths and short timeouts. A real held file guard must block migration; after release, the same call succeeds. Production uses `DEFAULT_CENTRAL_MUTATION_TIMEOUT` and the unchanged Local lock path.

The DB marker is written while the lock is still held. A lock timeout or copy failure writes no marker, so the existing next-start retry behavior remains intact.

## 6. Compatibility And Trade-offs

- Command names and business result payloads remain stable; affected argument payloads add `jobId`.
- Progress payloads add a field, which is backward-compatible for deserializers but the current frontend is updated atomically.
- Errors remain `Result<T, String>` at commands. Coded envelopes add localization without adopting the deferred structured `IpcError` migration.
- Two family registries allow update and portability read work concurrently. Destructive phases still contend on the existing target mutation lock.
- Backend-spawned jobs would require new result/event persistence and cleanup semantics. Explicit renderer IDs plus one pending-cancel slot cover start/cancel dispatch reordering with far less API churn and bounded memory.

## 7. Spec Updates

- Add backend exclusive job lifecycle contract and index entry.
- Add frontend job correlation/cancellation contract and index entry.
- Update Central mutation lock and spawn-blocking specs with the legacy migration case.
- Keep database migration, FS+DB Saga, AI tagging, GitHub preview and inventory progress contracts unchanged except cross-links where useful.

## 8. Rollback

- Registry, command args, event payloads and frontend filtering land in one work commit; partial rollout would break cancellation or event correlation.
- Migration blocking/lock change is behavior-preserving and can be reverted independently only before release; it does not change schema or persisted summary format.
- No migration row or new persistent job state is introduced, so rollback requires no data cleanup.
