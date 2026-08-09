# Research: Delete-missing shared recovery scope

- Query: Why did the 2026-08-09 `update_center.apply` request with `deleteMissing=1` fail for `claude-md-improver` while the only nonterminal journal belonged to `yao-meta`, and where should the regression be fixed and tested?
- Scope: internal
- Date: 2026-08-09

## Findings

### Read-only live evidence

- The latest relevant Operation Log row is `2026-08-09T05:13:34.818301+00:00`, `status=failed`, `duration_ms=58`, `deleteMissing=1`, `updates=0`, and `failures=1`.
- Its sole bounded failure item is `step=delete_missing`, `identifier=claude-md-improver`, `phase=decision_apply`, `errorCode=central_updates.delete_missing_failed`, and `errorCategory=central_updates.item_failure`.
- The only nonterminal `fs_db_operations` row is still `skill_id=yao-meta`, `operation_kind=central_delete`, `phase=prepared`, and `last_error_code=delete_restore_collision`. No nonterminal row belongs to `claude-md-improver`.
- Evidence was read through SQLite URI `mode=ro`. No manifest, path, fingerprint, URL, credential, or raw error text was read or emitted.

### Confirmed causal path

1. Inventory apply always enters `apply_delete_missing_step` before import/update steps (`src-tauri/src/services/central_updates/inventory/mod.rs:490-503`).
2. The Local branch delegates the whole request slice to the shared `central_skills::delete_central_skills_impl`; SSH/WSL delegates to its remote counterpart (`src-tauri/src/services/central_updates/inventory/apply_steps.rs:37-55`).
3. The Local batch loops over deduplicated requests and invokes `delete_central_skill_with_batch` once per skill (`src-tauri/src/services/central_skills/delete.rs:690-738`). The remote batch has the same per-skill orchestration (`src-tauri/src/services/central_skills/delete.rs:444-494`).
4. Each Local per-skill call independently acquires the target mutation guard and then calls the full-target `recover_pending_operations_under_guard` before loading the requested skill (`src-tauri/src/services/central_skills/delete.rs:585-605`). The remote call does the same (`src-tauri/src/services/central_skills/delete.rs:303-320`). Therefore the delete batch still performs full-target recovery, once per selected skill.
5. Full-target recovery loads every nonterminal row for the target. If any update row exists it dispatches full update recovery; otherwise it iterates every row and fails fast after recording the first row error (`src-tauri/src/services/central_operation/recovery.rs:47-93`). The database query itself filters only by `target_id`, not by selected `skill_id` (`src-tauri/src/db/repos/fs_db_operations_repo.rs:85-97`).
6. Consequently, deleting `claude-md-improver` first retries `yao-meta`; `delete_restore_collision` returns before `get_skill_by_id(claude-md-improver)` and before a new delete journal can be inserted. This matches the 58 ms live failure and the absence of a `claude-md-improver` pending row.
7. The shared batch converts `CentralSkillsError` to `FailedCentralSkillDelete.error: String` (`src-tauri/src/services/central_skills/delete.rs:731-734`; DTO at `src-tauri/src/services/central_skills/types.rs:129-134`). The inventory adapter then discards even that string and constructs a generic failure from only the step and skill ID (`src-tauri/src/services/central_updates/inventory/apply_steps.rs:56-70`). `controlled_apply_step` assigns the observed generic code (`src-tauri/src/services/central_updates/inventory/types.rs:339-355`).

### Correct shared-service seam

The recovery-scope fix belongs in the shared batch-delete service, not in the Update Center adapter:

- Local ownership: `central_skills::delete_central_skills_impl`.
- SSH/WSL ownership: `central_skills::delete_central_skills_remote_impl`.
- Both should acquire one target guard for the batch, load pending rows once, recover only rows whose `skill_id` appears in the deduplicated request set, and then run an under-guard single-skill delete helper. The current single-skill helper must not reacquire the guard or call full-target recovery.
- Startup recovery and explicit Retry/Reconcile must continue calling `recover_pending_operations[_under_guard]` with full-target, fail-fast semantics.
- The update batch already demonstrates the needed selected-row behavior: it builds a selected ID set, reads pending rows once, skips unselected rows, and stores row-specific recovery failures (`src-tauri/src/services/central_updates/core/batch/recovery.rs:33-67`). That implementation is update-domain-specific because update-row recovery needs `CentralFs`; common row selection and delete-row dispatch can be extracted into `central_operation`, while delete batch outcome ownership stays in `central_skills`.
- Typed diagnostic propagation is a second necessary boundary change: `FailedCentralSkillDelete` (or an internal equivalent consumed by inventory apply) must retain a typed `CentralSkillsError::CentralOperation` rather than only `String`, so a selected-skill recovery failure can map to `phase=recovery` and `central_operation.<code>`.

This boundary covers direct desktop/CLI batch deletes and repository deletion, because repository deletion also delegates to the same batch functions (`src-tauri/src/services/central_skills/delete/repository.rs:111-150`). Fixing only `apply_delete_missing_step` would leave those shared callers with the same recovery-scope defect.

### Fast red-capable regression

Add the primary regression beside the existing batch-delete service tests in `src-tauri/src/services/central_skills/tests.rs`:

1. Use the normal memory DB and temporary Central root helpers.
2. Seed a valid deletable `claude-md-improver` skill.
3. Insert an unrelated `yao-meta` `central_delete/prepared` manifest whose restore deterministically returns `delete_restore_collision`. The existing reusable fixture shape is `insert_pending_delete_collision` in `src-tauri/src/services/central_updates/core/batch_tests.rs:79-126`.
4. Set the unrelated row's `updated_at` to a sentinel and clear its `last_error_code` before the call.
5. Call the production shared boundary `delete_central_skills_impl` with only `claude-md-improver`.
6. Assert `claude-md-improver` succeeds and is removed, its own journal reaches `completed`, and the unrelated `yao-meta` row retains the sentinel `updated_at`, null error evidence, and `prepared` phase.

This test is red on the current code at the service boundary: full-target recovery touches `yao-meta`, returns the collision, and places `claude-md-improver` in `failed`. It is faster and more diagnostic than a command/UI test because it uses only SQLite memory state and a temporary directory.

Add parity cases for Fake SSH and Fake WSL at the same shared boundary after the Local red test. The remote assertion must also verify one connected transport/guard lifetime for the batch, because the current per-skill helper reconnects only after recovery but reacquires the guard for every item.

An inventory-layer companion test in `src-tauri/src/services/central_updates/inventory/tests.rs` should assert that a selected skill's own recovery collision becomes `identifier=<skill>`, `phase=recovery`, and `errorCode=central_operation.delete_restore_collision`. It does not replace the shared-service scoping test.

## Files Found

- `src-tauri/src/services/central_updates/inventory/mod.rs` - top-level apply ordering.
- `src-tauri/src/services/central_updates/inventory/apply_steps.rs` - delete-missing adapter and generic error flattening.
- `src-tauri/src/services/central_updates/inventory/types.rs` - generic step-to-code mapping.
- `src-tauri/src/services/central_skills/delete.rs` - shared Local/SSH/WSL single and batch delete orchestration.
- `src-tauri/src/services/central_skills/types.rs` - string-only batch delete failure DTO.
- `src-tauri/src/services/central_skills/delete/repository.rs` - repository delete reuse of shared batch functions.
- `src-tauri/src/services/central_operation/recovery.rs` - full-target recovery implementation.
- `src-tauri/src/services/central_updates/core/batch/recovery.rs` - working selected-skill recovery precedent for update batches.
- `src-tauri/src/db/repos/fs_db_operations_repo.rs` - target-only pending-row query.
- `src-tauri/src/services/central_skills/tests.rs` - correct test ownership for the shared deletion contract.
- `src-tauri/src/services/central_updates/core/batch_tests.rs` - deterministic pending collision fixture shape.

## Related Specs

- `.trellis/spec/backend/central-mutation-lock.md` - new mutations recover only selected skills under one top-level target guard; startup/explicit recovery remains full-target.
- `.trellis/spec/backend/fs-db-operation-journal.md` - selected-skill recovery and full-target startup/Retry split.
- `.trellis/spec/backend/central-update-batching.md` - selected recovery must not touch unrelated journal evidence.
- `.trellis/spec/backend/transport-seam.md` - shared orchestration and FakeRunner expectations for Local/SSH/WSL boundaries.
- `.trellis/spec/backend/test-support.md` - memory DB, temporary filesystem, and FakeRunner fixture conventions.

## External References

- None. This conclusion depends only on repository code, persisted local state, and project contracts.

## Ranked Falsifiable Hypotheses

1. **Confirmed: delete batch reuses full-target recovery.** Prediction: an unrelated collision prevents the selected skill from being loaded or journaled. The code path and live state match exactly.
2. **Rejected: `claude-md-improver` has its own pending recovery collision.** Prediction: a nonterminal row for that skill exists. The only nonterminal row belongs to `yao-meta`.
3. **Rejected as the root: inventory decision construction selected an invalid object.** Prediction: failure occurs after loading/validating `claude-md-improver`. Current code returns from unrelated recovery first.
4. **Confirmed secondary defect: typed delete error is flattened.** Prediction: even a selected-skill `CentralOperationError` appears as `central_updates.delete_missing_failed / decision_apply`. The DTO and adapter discard the type, matching the Operation Log.

## Caveats / Not Found

- The unrelated row's `updated_at` now reflects a later application startup recovery at `2026-08-09T05:15:06Z`, so that timestamp alone cannot attribute the last write specifically to the `05:13:34Z` apply. The production call graph is sufficient to prove that the apply invoked the same full-target recovery path.
- No live Apply, Retry, Reconcile, Central filesystem mutation, or database write was performed.

