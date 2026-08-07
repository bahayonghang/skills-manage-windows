# Diagnosis: Update Center apply failure is logged as success

## Symptom

The Update Center reports item failures such as
`update: Central operation recovery collision (delete_restore_collision)`, but
Operation Logs displays the corresponding `update_center.apply` row as
`Succeeded` with the success summary.

## Read-Only Live Evidence

The local database was opened with SQLite `mode=ro`. No manifest paths,
fingerprints, credentials, or raw transport text were printed.

The latest two matching apply rows both had:

```text
status=succeeded
updates=3
deleteMissing=1
failures=4
updated=0
deleted=0
imported=0
```

The newest row was created at `2026-08-07T01:18:51Z`. The database also held
one nonterminal row:

```text
operation_kind=central_delete
skill_id=yao-meta
phase=prepared
last_error_code=delete_restore_collision
```

The manifest had 14 expected-present paths. Ten entries had the same normalized
original and therefore the same operation-owned backup and marker. Current
state was 13 entries with neither original nor backup and one entry with only
the original present.

Operation history around creation established this order:

1. `central.delete` failed with `delete_stage_collision`.
2. Fourteen `skill.batch_uninstall` operations for the same skill succeeded.
3. A retry failed with `delete_restore_collision`.
4. Explicit Central recovery recorded the same collision.

## Code-Level Causal Chain

1. `delete_central_skill_impl` collects one path per selected copy or symlink
   installation and then appends the Central path without deduplication:
   `src-tauri/src/services/central_skills/delete.rs:634-660`.
2. Both manifest builders preserve every input path:
   `src-tauri/src/services/central_operation/fs.rs:115-187`.
3. Local staging renames the first occurrence. A repeated occurrence sees a
   missing original or an existing backup and returns
   `delete_stage_collision`: `fs.rs:346-365`.
4. The row remains `prepared`. Generic uninstall commands do not acquire the
   target recovery boundary, so later uninstalls remove paths referenced by the
   pending manifest: `src-tauri/src/commands/linker.rs:100-215`.
5. Recovery treats `(original missing, backup missing)` as
   `delete_restore_collision`, correctly refusing to guess:
   `src-tauri/src/services/central_operation/fs.rs:393-424`.
6. Update batching converts recovery failure into per-skill outcomes instead
   of an outer error: `src-tauri/src/services/central_updates/core/batch.rs:59-67`.
7. Apply accumulates those outcomes in `SkillUpdateApplyResult.failures` and
   returns `Ok(result)`:
   `src-tauri/src/services/central_updates/inventory/mod.rs:596-648`.
8. `with_operation_log` maps every outer `Ok` to its success builder:
   `src-tauri/src/operation_log.rs:147-180`. The success details include the
   failure count but status and summary remain successful:
   `src-tauri/src/commands/skill_update_inventory.rs:472-516,598-605`.
9. The frontend independently checks `result.failures` and emits error toasts:
   `src/components/central/UpdateCenterDialog.tsx:192-233`.

## Root Causes

1. **Manifest invariant missing:** physical delete paths are not unique.
2. **Recovery boundary incomplete:** same-skill installation mutations can
   invalidate a nonterminal Central operation's recovery evidence.
3. **Outcome model mismatch:** operation logging classifies the outer transport/
   command `Result`, while the batch API carries business failures inside an
   otherwise successful result.

The Operation Logs query is not dropping a failure row. It receives and displays
a row that was incorrectly classified as `succeeded` at write time.

## Ranked Hypotheses And Results

1. **Outer `Ok` hides item failures. Confirmed.** Prediction: persisted details
   contain `failures > 0` while status is `succeeded`; live DB and source both
   match.
2. **Operation Logs filtering/query omits the failed row. Rejected.** The row is
   present and visible; its write-time status is wrong.
3. **The collision is a transient update download error. Rejected.** A durable
   `central_delete/prepared` row and exact recovery code exist.
4. **The original delete collided because of duplicate physical paths.
   Confirmed.** Ten manifest entries normalize to one path, and staging's second
   occurrence deterministically violates its precondition.
5. **Later mutation destroyed recovery evidence. Confirmed.** The uninstall
   operations occurred between the stage collision and restore collision, and
   the current manifest state has 13 neither/original/backup pairs.

## Feedback Command

```powershell
rtk python .\.trellis\tasks\08-07-update-center-failure-observability\research\verify_live_partial_apply_log.py
```

The current database must produce `RED` because the latest apply has item
failures but is stored as `succeeded`. The implementation phase must first add
a deterministic Rust regression at the command/operation-log seam; the live
probe is forensic evidence and is not a substitute for an isolated test.

## Safety Boundary

No live repair was attempted. The pending row and all remaining filesystem
evidence must be preserved until an operator explicitly authorizes a repair
after a read-only preflight.
