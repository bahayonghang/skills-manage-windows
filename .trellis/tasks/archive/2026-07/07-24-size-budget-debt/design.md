# 07-24-size-budget-debt Design

## Objective

Remove the five historical size-budget exceptions without changing runtime behavior, public API shape, or the established service/component ownership boundaries. This task is intentionally a mechanical decomposition, not an architectural redesign.

## Boundaries

| Current file | New boundary | Compatibility rule |
|---|---|---|
| `src-tauri/src/services/central_updates/core.rs` | `core.rs` retains check/update orchestration and crate-internal re-export surface; `core/state.rs` owns remote content loading, repository metadata, and `SkillUpdateState` construction helpers. | Existing `central_updates::mod` exports and callers in batch, inventory, and repository-sync continue to resolve the same names. No update batching or FS+DB operation flow moves. |
| `src-tauri/src/commands/collections.rs` | Production structs, implementation helpers, and Tauri commands stay in `collections.rs`; the existing `#[cfg(test)] mod tests` body moves to `collections/tests.rs`. | The nested test module keeps `use super::*`; IPC names, `specta` attributes, and install semantics stay byte-for-byte equivalent apart from module paths. |
| `src-tauri/src/db/seed.rs` | `seed.rs` owns initialization and seed ordering; `seed/agents.rs` owns builtin agent construction, home normalization, and universal-agent predicates. | `seed.rs` re-exports the existing public builtin-agent helpers so `crate::db::*` consumers retain their paths and return values. |
| `src/pages/CentralSkillsView.tsx` | The page retains store/view state, shell composition, dialogs, and route-level entrypoint. A sibling action-binding hook owns only the adapter object passed to `useCentralSkillsActions` plus scroll-preserving selection behavior. | Existing action implementation, DOM composition, test fixtures, and store APIs remain unchanged. The hook receives dependencies explicitly and returns the same handlers. |
| `src/components/skill/UnifiedSkillCard.tsx` | Main module retains the sole public component, private `SkillCardModel`, and `toModel`. A types module owns exported discriminated-union declarations; a leaf-parts module owns `CardActionButton` and `SkillCardSummary`. | `UnifiedSkillCard.tsx` re-exports all current public type names. `SkillCardModel` is not exported, and no alternate card renderer is introduced. |

## Size-Guard Design

The general checker remains a single `MAX_LINES = 800` policy. Once all five original files are under 600 lines, remove `BASELINE_ALLOWLIST`, its special baseline comparison, and its exception reporting. The delivery gate additionally runs a direct count over those five paths; it is evidence for the P3-01 repayment target, not a new per-file or global policy exception.

## Test And Compatibility Design

- Rust moves retain existing unit tests and module visibility. Collections tests move with their module body; central updates and database tests exercise their pre-existing exports and behavior.
- Frontend moves retain public import paths. The discriminated-union negative cases remain the source of truth for card-prop exclusivity. CentralSkillsView tests continue to drive the page through its public DOM, using the existing async UI stability convention.
- No migration, wire-contract, or persisted-data compatibility work is needed because no runtime data shape changes.

## Rollback

Each extraction is independently reversible: move the module back, restore the single import/re-export line, and retain the same test bodies. The size-checker allowlist is removed only after all five direct-count checks pass; if any file remains at or above 600, retain the prior checker until the refactor is complete.

