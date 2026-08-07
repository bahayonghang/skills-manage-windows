# Implementation plan: Update Center failure observability and recovery integrity

## Stage 0: Red Regressions

- [ ] Add a Local Central-delete fixture with multiple installations resolving
      to one physical path; prove current code returns
      `delete_stage_collision`.
- [ ] Add Fake SSH/WSL duplicate-path fixtures at the manifest/stage seam.
- [ ] Add an operation-log integration test where
      `SkillUpdateApplyResult` has four failures and no successes; prove current
      row is incorrectly `succeeded`.
- [ ] Add a pending-row + uninstall fixture proving current installation code
      mutates filesystem/DB despite same-skill recovery.
- [ ] Add preview fixtures for the live-state shape: consistent duplicate
      entries, unowned missing paths, one DB-owned fingerprint-matching
      original, and no backup/marker.

Focused feedback commands:

```powershell
cd src-tauri
cargo test central_operation --locked
cargo test central_skills --locked
cargo test installation --locked
cargo test skill_update_inventory --locked
```

## Stage 1: Unique Delete Manifests

- [ ] Stable-deduplicate Local inputs using `paths_equivalent` before
      fingerprinting and `ManagedPath` construction.
- [ ] Ensure remote delete planning validates/normalizes POSIX paths, then
      stable-deduplicate in the shared remote builder.
- [ ] Retain version-1 decode compatibility; do not reject legacy duplicate
      manifests in ordinary list/retry paths.
- [ ] Extend Local/Fake SSH/Fake WSL tests for exact/normalized duplicates,
      unique backup/marker generation, one filesystem mutation, and idempotent
      stage/restore/finalize.

Review gate: duplicate DB installation rows remain until the existing parent
delete transaction; only filesystem plan entries are deduplicated.

## Stage 2: Installation Recovery Isolation

- [ ] Add a DB/service helper that resolves nonterminal rows for requested
      target/skill IDs without exposing manifest data.
- [ ] Add typed `PendingCentralRecovery` installation error with stable IPC and
      diagnostic mapping.
- [ ] Introduce top-level per-target single/batch install/uninstall use cases:
      prepare transport, acquire target guard, classify blocked skill IDs, then
      call private under-guard logic.
- [ ] Split Central centralization into guarded entry and under-guard helper so
      the new top-level boundary cannot self-deadlock.
- [ ] Route Tauri commands, project-install paths that can centralize, and
      `skillport-cli` through the guarded service APIs; add a caller-inventory
      contract test/grep.
- [ ] Test same skill blocked before FS/DB mutation, mixed batch partial result,
      different skill allowed, different target non-contention, and no nested
      lock timeout.

Review gate: connection/archive preparation remains outside the lock; every
target filesystem or business-DB mutation and pending check is inside it.

## Stage 3: Structured Partial Failure Diagnostics

- [ ] Preserve reviewed public message, optional error code, and category from
      `CentralUpdatesError`/`CentralOperationError` through update batch and
      `SkillUpdateApplyFailure`; remove early `.to_string()` diagnostic loss.
- [ ] Promote/generalize the existing batch status classifier and reuse it from
      installation plus Update Center.
- [ ] Add `apply_operation_spec` so outer `Ok` results write `succeeded`,
      `partial`, or `failed` from item counts with matching summaries.
- [ ] Persist/log only counts and sorted unique reviewed codes/categories;
      add one safe runtime event for partial/failed item outcomes.
- [ ] Update TypeScript types, fixtures, store/component handling, and EN/ZH
      backend-error resources so reviewed codes use `formatBackendError` and
      legacy unknown items use a fixed generic message.
- [ ] Add Rust/Frontend adversarial tests for token, URL, absolute path,
      manifest content, and raw transport output absence.

Review gate: the original 0-success/4-failure fixture writes exactly one
`update_center.apply` row with `status=failed`; mixed is `partial`; zero failure
is unchanged.

## Stage 4: Explicit Prepared-Delete Reconciliation Backend

- [ ] Add safe preview/result/blocker types and stable error codes.
- [ ] Implement shared Local/remote preflight: identity/kind/phase validation,
      consistent legacy duplicate collapse, current DB ownership set, artifact
      absence, fingerprint verification, and remote inspection.
- [ ] Add preview service/command that inspects under the target guard and
      returns only IDs, eligibility, counts, and blocker codes.
- [ ] Add reconcile service/command that independently reacquires the guard,
      reloads/re-previews, then performs only `prepared -> rolled_back` when
      eligible.
- [ ] Record reconcile apply success/failure as
      `central.operation_reconcile`; preview writes no operation row.
- [ ] Test every blocker, preview/apply race, transition failure rollback,
      unchanged filesystem/business tables, Local/Fake SSH/Fake WSL parity,
      and list/detail/export/runtime redaction.

Review gate: no code path can reconcile `fs_staged`, `db_committed`, update
operations, or a row with remaining artifacts/owned missing data.

## Stage 5: Operation Logs Reconcile UI

- [ ] Register typed IPC commands and browser fixtures; extend pending operation
      types without exposing manifest/path fields.
- [ ] Extend `operationLogStore` with preview/apply actions, separate loading
      IDs, latest-wins target handling, and failure state that preserves the
      pending row.
- [ ] Add a Reconcile action only for `central_delete/prepared`, a compact
      preview/confirmation dialog, localized blocker list, and success/failure
      feedback through `formatBackendError`.
- [ ] Preserve Retry behavior and keyboard/focus accessibility; verify narrow
      layout does not overlap.
- [ ] Add store/view tests for eligible confirm/apply, blocked preview, cancel,
      stale response, target switch, apply failure, and bounded safe messages.

Focused frontend validation:

```powershell
pnpm vitest run src/test/stores/operationLogStore.test.ts src/test/pages/OperationLogsView.test.tsx
pnpm vitest run src/test/components/central
pnpm typecheck
pnpm lint
```

## Stage 6: Generated Contracts And Specs

- [ ] Register both commands in `ipc_registry.rs` and the typed frontend map;
      update command-count/parity tests and fixtures.
- [ ] Run `pnpm ipc:codegen`, then run `pnpm ipc:codegen:check` twice.
- [ ] Because Tauri commands change, run `pnpm docs:gen` and commit both files
      under `docs/architecture/_generated/`; verify `pnpm docs:gen:check` and
      `pnpm docs:build` are read-only.
- [ ] Update executable specs for unique journal paths, installation/recovery
      isolation, explicit reconciliation, and result-aware operation statuses.

## Stage 7: Full Validation

```powershell
pnpm typecheck
pnpm lint
pnpm test
pnpm ipc:codegen:check
pnpm docs:gen:check
pnpm docs:build
cd src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cd ..
just ci
```

- [ ] Run the read-only live probe and record its pre-fix evidence separately;
      do not call live reconcile during automated tests.
- [ ] Inspect final diff for raw-error/path leakage, caller bypasses, unrelated
      reformatting, generated drift, and user-owned file overlap.
- [ ] Run Trellis task validation and an independent quality review.

## Rollback Points

- Manifest uniqueness is self-contained and can remain if later stages roll
  back.
- Installation isolation and centralization lock ownership must roll back
  together to avoid nested locking or an unlocked centralization path.
- Structured failure fields, TS types, i18n, and apply logging roll back
  together.
- Reconcile backend commands, typed maps/generated docs, store, and UI roll
  back together. Already terminal `rolled_back` rows need no reverse migration.

## Live Row Gate

After implementation, tests, and review, run only the new read-only preview for
the live `yao-meta` row and present its eligibility/blockers. Applying reconcile
to that row requires a separate explicit user authorization; task start or
implementation approval does not authorize the live state change.
