# Live Job Concurrency Evidence

Date: 2026-07-27
Branch: `dev` after `8394e8c7`, `a0721e27`, `e467ba15`

## Shared cancellation state

- `AppState` owns one `Arc<AtomicBool>` for all Central update work and one for all portability work.
- Central update service entrypoints call `cancel.store(false, SeqCst)` on entry. A later job can therefore clear an earlier cancellation request before that earlier loop observes it.
- Portability commands also reset the shared flag before resolving target context or doing remote work.
- Existing comments claim at-most-one behavior, but no backend lease enforces it.

## Affected command surface

Central update flag consumers:

1. `check_central_skill_updates`
2. `check_central_repository_sync`
3. `update_central_skills`
4. `apply_skill_update_decisions`

Portability flag consumers:

1. `export_skillport_state`
2. `preview_skillport_state_import`
3. `preview_skillport_state_import_file` through nested preview command invocation
4. `import_skillport_state`

Force update/mirror call `update_skills_batch(..., None)` and do not share the cancel flag. Inventory refresh already has an independent renderer-generated `operationId` correlation contract.

## Existing patterns and why they are not direct reuse

- `AiTagJobRegistry` maps multiple job IDs to cancel flags. It does not enforce one active job and falls back open after mutex poison.
- GitHub preview snapshot import lease has expiry, consume, deferred discard, and storage cleanup semantics that do not belong in a general command job registry.
- Update Center inventory progress provides the correct cross-layer correlation pattern: renderer creates ID before invoke, listener filters by ID, backend echoes it in every event.
- `backendError.ts` already parses `domain.code:summary` and localizes `backendErrors.<domain.code>` without changing the Rust command string boundary.

## Legacy migration drift from the old PRD

- `central_migration.rs` still performs recursive synchronous filesystem work directly inside async code.
- The completed FS+DB operation journal task added a target-scoped extension to the existing Central file lock. Local compatibility deliberately remains `central-mutation.lock`.
- Migration is Local and source-preserving, so it should acquire that existing Local lock, recheck its DB marker under the lock, run the copy in `run_blocking_fs_with`, and write the marker before releasing. A new lock or Saga row would duplicate current infrastructure without improving its recovery model.

## Planning conclusion

Use two instances of one fail-closed exclusive registry, explicit renderer-generated job IDs, one bounded pending-cancel slot, RAII release, ID-scoped cancel, event correlation, and coded bilingual errors. Keep file-lock serialization separate and reuse it only for the legacy migration's Local copy boundary.
