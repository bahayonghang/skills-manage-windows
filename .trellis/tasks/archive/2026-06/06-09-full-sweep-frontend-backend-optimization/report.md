# Brooks-Lint Full Sweep Report

Mode: Full Sweep
Scope: `src/`, `src-tauri/`
Config: no `.brooks-lint.yaml`

## Dimension Summary

| Dimension | Scanned | Safe Applied | Extended Applied | Reverted | Residual |
|-----------|---------|--------------|------------------|----------|----------|
| Review (R1-R6) | 671 tracked scope files | 2 | 1 | 0 | 2 |
| Test (T1-T6) | 123 tracked test files | 2 | 0 | 0 | 1 |
| Debt | modified files, same-module files, direct consumers | 2 | 1 | 0 | 3 |
| Audit | frontend store/component and backend command/service boundaries | 1 | 1 | 0 | 3 |

## Iteration History

- Round 1: mixed; applied safe frontend duplication, direct invoke, and backend helper reductions.
- Round 2: residual-only; larger dependency-direction and module-size findings were left for human review.
- Stopped at: no outstanding criticals; final `just ci` passed.

## Fix Log

| # | File | Risk | Outcome | Change |
|---|------|------|---------|--------|
| 1 | `src/test/runtimeLogger.test.ts` | T4 | applied | Suppressed expected console error noise inside the local test. |
| 2 | `src/components/central/CentralSkillListContent.tsx`, `src/components/central/CentralGroupedSkillList.tsx`, `src/components/central/centralSkillCardProps.ts` | R2 | applied | Extracted duplicated Central skill-card prop assembly. |
| 3 | `src/components/projects/DiscoverDeprecationBanner.tsx`, `src/lib/discoverDeprecationPreference.ts`, `src/test/discoverDeprecationPreference.test.ts` | R5/T2 | applied | Moved setting persistence behind a helper and added focused coverage. |
| 4 | `src-tauri/src/commands/collections.rs` | R2 | applied | Extracted private collection lookup helper. |
| 5 | `src-tauri/src/commands/saved_views.rs`, `src-tauri/src/commands/tag_groups.rs`, `src-tauri/src/commands/serde_helpers.rs`, `src-tauri/src/commands/mod.rs` | R2 | applied | Shared duplicated optional-string serde helper. |
| 6 | `src-tauri/src/central_migration.rs`, `src-tauri/src/commands/linker.rs` | R5 | applied | Removed migration dependency on command-layer linker copy helper. |

## Health Score Delta

Before: estimated 78/100
After: estimated 84/100

Re-run Brooks health for an exact score.

## Residual Items

### Frontend Upward Dependencies

Symptom: store/lib modules import page-level helpers and view-model types.
Source: `src/stores/settingsStore.ts`, `src/stores/updateCenterStore.ts`, `src/lib/updateCenterRefreshScope.ts`, `src/lib/centralViewState.ts`.
Consequence: lower-level state code becomes coupled to page layout modules.
Remedy: move shared update-mode and view-state contracts into `src/lib` or `src/types`, then import downward.
Not applied because: cross-module architecture move.

### Backend Service to Command Coupling

Symptom: services import `crate::commands::APP_USER_AGENT`.
Source: AI provider, GitHub import, AI tagging, and marketplace service modules.
Consequence: service layer depends on command-layer constants.
Remedy: move the app user-agent constant to a neutral module and import it from both commands and services.
Not applied because: shared boundary change across multiple service modules.

### Frozen Size-Budget Exceptions

Symptom: sizecheck still reports frozen oversize files.
Source: `src/pages/CentralSkillsView.tsx`, `src-tauri/src/commands/central_updates.rs`, `src-tauri/src/commands/collections.rs`, `src-tauri/src/db/seed.rs`.
Consequence: future edits in these files remain harder to review and easier to bloat.
Remedy: plan dedicated module extractions with focused behavior tests.
Not applied because: broad refactor outside safe sweep scope.

## Verification

- `pnpm exec vitest run src/test/runtimeLogger.test.ts`: passed
- `pnpm exec vitest run src/test/discoverDeprecationPreference.test.ts`: passed
- `pnpm exec vitest run src/test/CentralSkillsView.shell.test.tsx src/test/UnifiedSkillCard.test.tsx src/test/centralSkillGrid.test.ts src/test/CentralSkillsView.updates-and-search.test.tsx`: passed
- `pnpm exec vitest run src/test/CentralSkillsView.github-import-preview.test.tsx src/test/CentralSkillsView.github-import-error.test.tsx`: passed
- `pnpm typecheck`: passed
- `pnpm lint`: passed
- `cargo test --manifest-path src-tauri/Cargo.toml commands::collections`: passed
- `cargo test --manifest-path src-tauri/Cargo.toml commands::saved_views`: passed
- `cargo test --manifest-path src-tauri/Cargo.toml commands::tag_groups`: passed
- `cargo test --manifest-path src-tauri/Cargo.toml central_migration`: passed
- `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings`: passed
- `just ci`: passed

## Summary

- Total findings detected: 9
- Fixed this sweep: 6
- Residual needing human review: 3
- Unresolvable: 0
