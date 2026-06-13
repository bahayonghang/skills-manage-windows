# Implementation Plan

## Phase 1: Preview Helper

1. Add a small pure helper for Central bulk uninstall planning.
   - Candidate path: `src/lib/centralBatchUninstall.ts`
   - Inputs: selected skill ids, `SkillWithLinks[]`
   - Outputs: per-agent request groups, skipped/no-install skill ids, shared-root/non-removable entries, totals
   - Verify: unit tests for mixed installed/uninstalled skills, shared-root exclusion, dedupe, and all-no-op selection

2. Add frontend result types if needed.
   - Reuse `BatchUninstallSkillRequest` and `BatchUninstallSkillResult`
   - Keep Central aggregate result frontend-only unless a backend command becomes necessary
   - Verify: `pnpm typecheck`

## Phase 2: Central Action Orchestration

3. Wire a handler in `src/pages/centralSkillsActions.ts`.
   - Import `useSkillStore.batchUninstallSkillsFromAgent` or call the existing Tauri command through an existing store boundary
   - Run per-agent uninstall calls for preview groups
   - Refresh counts and Central skills after completion
   - On success, clear selection
   - On partial failure, keep failed skill ids selected
   - Verify: focused tests in `centralSkillsStore.test.ts` or a new helper/action test

4. Add local loading/dialog state in `CentralSkillsView.tsx`.
   - `isBatchUninstallDialogOpen`
   - `isBatchUninstalling`
   - derived preview from selected ids and current skills
   - Keep `CentralSkillsView.tsx` small; move nontrivial derivation out to helper
   - Verify: size impact before final `just ci`

## Phase 3: UI

5. Extend `BulkActionBar`.
   - Add `onBatchUninstall`, `isUninstalling`, and button test id such as `bulk-bar-batch-uninstall`
   - Use a non-trash icon
   - Keep Central delete button unchanged
   - Verify: update `CentralSkillsView.shell.test.tsx`

6. Add `BatchUninstallCentralSkillsDialog`.
   - Candidate path: `src/components/central/BatchUninstallCentralSkillsDialog.tsx`
   - Show operation warning: platform installs only, Central skills remain
   - Show removable platforms and skipped/non-removable summaries
   - Disable confirm when removable install count is zero
   - Show partial failure list if confirm returns failures
   - Verify: component tests or Central view interaction tests

7. Wire dialog through `CentralSkillDialogs` or keep it directly near Central page dialogs.
   - Prefer `CentralSkillDialogs` for consistency if prop growth remains manageable
   - Verify: Central view tests still render all dialogs

8. Add i18n strings in `en.json` and `zh.json`.
   - Button label
   - Dialog title/description/warning
   - no-op/skipped/shared-root summaries
   - success/partial/error toasts
   - Verify: `pnpm lint` and tests

## Phase 4: Tests

9. Update existing tests:
   - `CentralSkillsView.shell.test.tsx`: button appears in bulk bar
   - `CentralSkillsView.updates-and-search.test.tsx`: mixed selection calls existing uninstall command/store by installed agents only
   - `centralSkillsStore.test.ts` or new helper tests: request grouping and refresh behavior

10. Add edge-case tests:
    - all selected skills not installed anywhere: no backend calls; confirm disabled or no-op message shown
    - selected skill with shared-root agent only: no backend call and shared-root explanation shown
    - partial failure: failed skill ids remain selected and success count is not rolled back

## Validation Commands

Run in this order:

```powershell
pnpm exec vitest run src/test/CentralSkillsView.shell.test.tsx src/test/CentralSkillsView.updates-and-search.test.tsx src/test/centralSkillsStore.test.ts
pnpm typecheck
pnpm lint
just ci
```

If Rust code is changed unexpectedly, also run:

```powershell
cd src-tauri; cargo test installation
cd src-tauri; cargo clippy -- -D warnings
```

## Rollback Points

- If frontend orchestration becomes too complex or inconsistent with remote targets, stop and design a dedicated backend aggregate command before implementation.
- If `CentralSkillDialogs` prop growth becomes excessive, keep the new dialog in `CentralSkillsView.tsx` near other local state and extract only after tests pass.
- If `CentralSkillsView.tsx` approaches the repo line-size budget, move preview/action helpers out before continuing.
