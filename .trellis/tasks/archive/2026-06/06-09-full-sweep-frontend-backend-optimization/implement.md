# Implementation Plan

## Pre-flight

- [x] Create Trellis task.
- [x] Confirm requested scope: `src/` and `src-tauri/`.
- [x] Count tracked scope files: 671.
- [x] Count tracked test files in scope: 123.
- [x] Check Brooks config: no `.brooks-lint.yaml`.
- [x] Run baseline `just ci`: passed.
- [x] Receive Brooks Full Sweep pre-flight approval from the user.

## Execution

1. Enumerate the final scope.
   - Verify: file list comes from `git ls-files src src-tauri`.

2. Run review pass.
   - Look for local code decay, duplicated constants, avoidable complexity,
     unsafe null/empty handling, and local public-contract hazards.
   - Apply only Safe or Extended-Safe fixes.
   - Verify: focused tests for touched files, then relevant lint/typecheck or
     cargo checks.

3. Run test-quality pass.
   - Look for brittle assertions, excessive fixture duplication, missing focused
     coverage for edited pure helpers, and tests that only exercise setup.
   - Apply only test changes that clarify existing behavior without adding new
     infrastructure.
   - Verify: focused Vitest or cargo test filters.

4. Run debt pass.
   - Re-scan repeated local findings for Pain x Spread priority.
   - Apply low-risk consolidation only when it reduces real duplication and
     stays within existing module boundaries.
   - Verify: focused tests for consolidated behavior.

5. Run architecture pass.
   - Map frontend store/component boundaries and backend command/service/db
     boundaries.
   - Record most architecture findings as residual unless the remedy is a small
     local extraction with coverage.
   - Verify: no new direct component-to-Tauri calls, no public IPC/schema drift.

6. Iterate on modified files, same-module files, and static consumers.
   - Stop on a clean round, residual-only state, or the non-critical iteration
     cap.
   - Retire any finding that fails verification three times.

7. Run final verification.
   - Required: `just ci`.
   - Additional focused commands are recorded in the report.

8. Produce final Full Sweep report.
   - Include applied fixes, residuals, unresolvable items, and verification.

## Risk Gates

- Stop and ask before any change that would alter public API, IPC command names,
  database schema/seed semantics, persisted data shape, or user-visible product
  behavior.
- Do not commit, push, amend, or clean unrelated work.
- Do not delete unrelated dead code found during the scan.

## Completion Criteria

- [x] Task artifacts are up to date.
- [x] Applied fixes are limited to Safe or Extended-Safe.
- [x] Final `just ci` passes.
- [x] Final report is captured in `report.md` and delivered to the user.

## Completion Log

- Focused frontend tests passed:
  `pnpm exec vitest run src/test/runtimeLogger.test.ts`
- Focused frontend tests passed:
  `pnpm exec vitest run src/test/discoverDeprecationPreference.test.ts`
- Focused Central card/grid tests passed:
  `pnpm exec vitest run src/test/CentralSkillsView.shell.test.tsx src/test/UnifiedSkillCard.test.tsx src/test/centralSkillGrid.test.ts src/test/CentralSkillsView.updates-and-search.test.tsx`
- Focused GitHub import tests passed:
  `pnpm exec vitest run src/test/CentralSkillsView.github-import-preview.test.tsx src/test/CentralSkillsView.github-import-error.test.tsx`
- Frontend gates passed: `pnpm typecheck`, targeted ESLint, `pnpm lint`.
- Rust focused tests passed:
  `cargo test --manifest-path src-tauri/Cargo.toml commands::collections`,
  `commands::saved_views`, `commands::tag_groups`, and `central_migration`.
- Rust clippy passed:
  `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`.
- Final aggregate gate passed on 2026-06-09: `just ci`.
