# Exclusive Job Lifecycle Contract

## 1. Scope / Trigger

Apply this contract when a renderer-owned long-running command has start, progress, and cancel paths. Central update, SkillPort portability, and Skills CLI global add/remove are separate families: each family is exclusive within the process, while different families may run concurrently. Skills CLI filesystem mutual exclusion uses the Local target mutation guard, not this job family.

## 2. Signatures

```rust
pub fn acquire(&self, job_id: &str) -> Result<ExclusiveJobLease, ExclusiveJobError>;
pub fn cancel(&self, job_id: &str) -> Result<bool, ExclusiveJobError>;

pub fn job_id(&self) -> &str;
pub fn cancel_flag(&self) -> &AtomicBool;
```

Affected IPC commands require camelCase `jobId: string`:

| Family | Start commands | Cancel command |
| --- | --- | --- |
| Central update | `check_central_skill_updates`, `check_central_repository_sync`, `update_central_skills`, `apply_skill_update_decisions` | `cancel_central_skill_updates` |
| Portability | `export_skillport_state`, `preview_skillport_state_import`, `preview_skillport_state_import_file`, `import_skillport_state` | `cancel_skillport_state_portability` |
| Skills CLI global | `skills_cli_add_global`, `skills_cli_remove_global` | `cancel_skills_cli_job` |

## 3. Contracts

- The renderer creates the job ID before `invoke`; IDs contain 1-128 bytes after validation, are not whitespace-only, and contain no control characters.
- `AppState` owns one `ExclusiveJobRegistry` per family. A command acquires its family lease before its first `.await` and passes the lease's explicit ID and cancel flag into services.
- A lease releases only its own ID through `Drop`. Success, error, cancellation, and unwind therefore share the same release path; an old lease cannot clear a successor.
- With no active job, cancel remembers one bounded pending ID. Acquiring the same ID starts cancelled; acquiring a different ID discards the stale pending request.
- Every `central://skill-update-progress` and `central://state-portability-progress` payload carries the lease's `jobId`, including started, running, cancelled, failed, and completed events.
- A top-level command acquires exactly once. File-based portability preview reads the file and calls an established-job helper; it must not call the JSON preview command and reacquire.
- Services never inspect registry state. AI tagging, inventory refresh `operationId`, force update/mirror, and snapshot registries retain their own lifecycles.

## 4. Validation & Error Matrix

| Condition | Result |
| --- | --- |
| Same-family active job exists | `job.central_update_busy:...`, `job.portability_busy:...`, or `job.skills_cli_busy:...`; Skills CLI commands remap the last code to `skills_cli.busy`. Active flag unchanged |
| Cancel ID matches active ID | Set only that lease's flag; `Ok(true)` |
| Cancel arrives with no active job | Store one pending ID; idempotent `Ok(false)` |
| Cancel ID differs from active ID | `job.id_mismatch:...`; active flag unchanged |
| Empty, whitespace-only, oversized, or control-character ID | `job.invalid_id:...` |
| Registry mutex is poisoned | `job.registry_unavailable:...`; fail closed |

Commands keep the existing `Result<T, String>` IPC boundary and stringify these stable `code:summary` envelopes only at the command layer.

## 5. Good / Base / Bad Cases

- Good: job A owns a lease, stale cancel A cannot cancel active successor B, and a stale A event is ignored by the renderer.
- Base: Central update, portability, and Skills CLI each hold a lease and run concurrently because they use separate registries. Skills CLI add/remove still serialize filesystem writes through `acquire_target_mutation_guard`.
- Bad: a command resets a shared `AtomicBool` at start, allowing a second invocation to erase the first job's cancellation.

## 6. Tests Required

- Assert exact busy envelopes, same-family exclusion, cross-family independence, pending-cancel idempotence, stale pending discard, exact-ID cancellation, RAII release, stale-lease isolation, invalid IDs, and poison fail-closed behavior.
- Structurally assert production code has no `central_update_cancel` or `portable_state_cancel` fallback.
- Assert all ten start commands and three cancel commands serialize `jobId`, and all progress payloads contain the same ID.
- Run affected Rust/Vitest tests plus locked Clippy/tests and default-concurrency `just ci`.

## 7. Wrong vs Correct

```rust
// Wrong: the next start can clear another job's cancellation.
shared_cancel.store(false, Ordering::SeqCst);
run(&shared_cancel).await

// Correct: acquire before the first await and pass lease-owned state explicitly.
let lease = state.central_update_jobs.acquire(&job_id)?;
run(lease.job_id(), lease.cancel_flag()).await
```
