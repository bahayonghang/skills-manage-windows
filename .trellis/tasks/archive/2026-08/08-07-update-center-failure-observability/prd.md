# Improve Update Center failure observability and recovery integrity

## Goal

Prevent shared physical paths from creating unrecoverable Central delete
journals, prevent same-skill installation mutations from invalidating pending
recovery evidence, and make Update Center apply logs reflect partial or total
item failure instead of reporting a false success.

## Background And Confirmed Facts

- The 2026-08-07 screenshots show `Apply selected (4)` reporting
  `Central operation recovery collision (delete_restore_collision)` while the
  Operation Logs page shows `update_center.apply` as `Succeeded`.
- A read-only query of the live local database found the latest apply row at
  `2026-08-07T01:18:51Z`: `status=succeeded`, `updates=3`,
  `deleteMissing=1`, `failures=4`, and zero updated/deleted/imported items.
- The same database contains one nonterminal `central_delete` row for
  `yao-meta`, still at `prepared`, with
  `last_error_code=delete_restore_collision`.
- The persisted delete manifest contains 14 expected-present paths. Ten entries
  resolve to the same physical original, backup, and marker path. The manifest
  builders currently preserve duplicate inputs
  (`src-tauri/src/services/central_operation/fs.rs:115` and `:145`).
- Local delete collects one removable path per selected copy/symlink
  installation and then the Central path without enforcing path uniqueness
  (`src-tauri/src/services/central_skills/delete.rs:634`). Staging renames the
  first duplicate, then treats the next occurrence as a collision
  (`src-tauri/src/services/central_operation/fs.rs:346`).
- Operation history records the causal sequence: initial `central.delete`
  failed with `delete_stage_collision`; fourteen platform uninstall operations
  then succeeded; the next delete/recovery attempt failed with
  `delete_restore_collision`. Thirteen manifest paths now have neither an
  original nor an operation-owned backup, so fail-closed recovery cannot infer
  intent.
- The single and batch uninstall command paths do not acquire the Central
  target mutation guard or reject a same-skill nonterminal recovery row
  (`src-tauri/src/commands/linker.rs:100` and `:161`).
- Update apply intentionally accumulates item failures and returns
  `Ok(SkillUpdateApplyResult)` (`src-tauri/src/services/central_updates/inventory/mod.rs:468`
  and `:648`). `with_operation_log` classifies every outer `Ok` as success
  (`src-tauri/src/operation_log.rs:147`), while the dialog reads
  `result.failures` and emits error toasts
  (`src/components/central/UpdateCenterDialog.tsx:192`).

Detailed evidence and the read-only live probe are in
`research/diagnosis.md` and `research/verify_live_partial_apply_log.py`.

## Requirements

### R1. Duplicate-Free Delete Manifests

- Local, SSH, and WSL delete manifests must contain at most one entry for each
  physical original path and at most one corresponding backup/marker pair.
- Deduplication must preserve first-occurrence order and run at the shared
  manifest boundary so every Central delete caller receives the invariant.
- Local comparison must follow the repository's Windows/path policy; remote
  comparison must use validated normalized POSIX paths. String substring or
  display-text heuristics are not acceptable.
- Duplicate installation rows may still be removed from SQLite by the normal
  parent-delete cascade; filesystem staging/finalization happens once per
  unique path.

### R2. Pending-Recovery Mutation Isolation

- An install, uninstall, batch install, batch uninstall, CLI mutation, or other
  service entry point that can change a Central skill's canonical/install path
  must fail closed before filesystem or business-DB mutation when that target
  and skill have a nonterminal `fs_db_operations` row.
- The pending-row check and final mutation must share the target mutation
  boundary; a check-before-lock race is not acceptable.
- A pending row for another target or another skill must not block unrelated
  work.
- Rejections must use a typed, stable, localized error contract. Operation
  logs, runtime logs, IPC, and UI must not expose manifest paths, fingerprints,
  credentials, or raw transport output.

### R3. Truthful Update Apply Operation Logs

- `update_center.apply` status must derive from item outcomes:
  `succeeded` when there are no failures, `failed` when failures exist and no
  item succeeded/skipped, and `partial` when both success/skip and failure are
  present.
- Summary and safe details must match the status and include request counts,
  success counts, failure count, and reviewed stable failure codes/categories.
  Raw `SkillUpdateApplyFailure.error` strings must not be persisted.
- The existing batch outcome classifier should be reused or generalized rather
  than introducing a conflicting status rule.
- The frontend continues to show bounded per-item feedback, but reviewed coded
  failures must render through i18n and `formatBackendError`; dynamic path/URL/
  token details must not reach a toast.

### R4. Explicit Legacy Prepared-Delete Reconciliation

- Operation Logs must provide a general, explicit workflow for legacy
  `central_delete/prepared` rows that cannot pass ordinary restore because
  installation state changed after the row was created.
- The workflow has two backend steps: a read-only preview and a separately
  confirmed apply. Apply must reacquire the target mutation guard, reload the
  row, and rerun the full preview; a stale preview never authorizes mutation.
- Legacy duplicate manifest entries may be collapsed for evaluation only when
  their expected-presence, fingerprint, backup, and marker evidence agrees.
- Reconciliation is eligible only when the Central skill still exists in the
  business DB, every existing original matches its recorded fingerprint, no
  operation-owned backup or marker remains, and every missing originally
  present path is no longer owned by the current canonical skill or any current
  installation row.
- Eligible apply performs exactly one durable change:
  `prepared -> rolled_back`. It must not create, delete, restore, overwrite, or
  rename filesystem entries, and it must not edit skill or installation rows.
- Unsupported kind/phase, inconsistent duplicate evidence, a missing DB-owned
  path, fingerprint drift, remaining backup/marker, target mismatch, or remote
  inspection failure keeps the row pending and returns stable blocker codes.
- Preview/result IPC exposes only operation/skill identity, eligibility,
  counts, and reviewed blocker codes. Operation Logs and Runtime Logs must not
  contain manifest paths, fingerprints, raw errors, credentials, or transport
  output.

## Acceptance Criteria

- [ ] A Local fixture with two or more installation rows sharing one physical
      path builds a unique manifest and completes delete without
      `delete_stage_collision`; filesystem mutation occurs once and DB cleanup
      remains complete.
- [ ] Equivalent Fake SSH and Fake WSL fixtures prove unique remote manifests,
      stable order, and idempotent stage/restore/finalize behavior.
- [ ] Manifest-level tests cover exact duplicates and platform-appropriate
      normalized duplicates without weakening outside-root or traversal guards.
- [ ] With a nonterminal operation for skill A, same-target install/uninstall
      entry points for A reject before FS/DB mutation with a stable safe code;
      skill B and another target remain usable.
- [ ] Desktop and `skillport-cli` reach the same service-level isolation rule;
      no caller can bypass it by invoking a lower-level Central mutation path.
- [ ] An apply result with 0 successes and 4 failures writes one
      `update_center.apply` row with `status=failed`; a mixed result writes
      `partial`; a zero-failure result remains `succeeded`.
- [ ] Failure log details contain only reviewed codes/categories and counts.
      Adversarial token, URL, absolute path, manifest, and raw error text are
      absent from list, detail, export, runtime log, and frontend toast tests.
- [ ] Local, Fake SSH, and Fake WSL preview fixtures cover eligible legacy
      duplicate rows plus every fail-closed blocker: wrong kind/phase,
      inconsistent duplicate evidence, missing skill, DB-owned path missing,
      fingerprint drift, backup/marker remaining, target mismatch, and remote
      inspection failure.
- [ ] Reconcile apply reruns preflight under the target guard, rejects a
      preview/apply race, and for an eligible row changes only
      `prepared -> rolled_back`; filesystem bytes and business tables are
      unchanged.
- [ ] Operation Logs presents preview blockers, an explicit confirmation, and
      separate retry/reconcile loading states. Coded errors are localized; a
      failed/stale preview leaves the row visible and actionable.
- [ ] Successful and failed reconciliation attempts write redacted operation
      rows with action `central.operation_reconcile`; preview remains a
      read-only query and writes no operation row.
- [ ] Focused Rust and frontend regressions pass, followed by
      `cargo fmt --all -- --check`,
      `cargo clippy --all-targets --locked -- -D warnings`,
      `cargo test --locked`, `pnpm typecheck`, `pnpm lint`, relevant Vitest
      suites, generated-contract checks if IPC changes, and final `just ci`.

## Out Of Scope

- Weakening marker/fingerprint collision checks or automatically overwriting an
  unexpected path.
- Logging raw per-item failure text, manifests, full paths, repository URLs, or
  credentials.
- Changing Update Center selection, repository refresh, retry, or batch
  ordering semantics unrelated to recovery isolation.
- Mutating the user's live database or filesystem during planning.
- Automatically reconciling a row at startup, during normal retry, or without
  an operator confirmation.
- Automatically invoking the new reconcile action on the user's live
  `yao-meta` row. After implementation and validation, that live action still
  requires a fresh preview and explicit authorization.
