# Design: Confirmed one-click platform leftover cleanup

## Boundary

This task extends the existing Update Center UI. It does not add a new backend command because platform leftovers already flow through `SkillUpdateDecisions.removeDeletedPlatformCopies` and `apply_skill_update_decisions`.

## Data flow

1. `UpdateCenterDialog` reads the current `inventory.deletedPlatformCopies`.
2. The one-click action builds a cleanup-only decision state containing every `writablePaths` entry in the current inventory.
3. The user must confirm the number of paths before the action calls the existing `apply` store action.
4. `apply` reloads the inventory through the current refresh scope after the backend applies the decision.
5. Success and partial failure reuse the existing Update Center toast pattern.

## Scope semantics

- "Clean all leftovers" means all platform leftover paths in the currently loaded Update Center inventory.
- The action does not select or apply updates, additions, remote-missing decisions, platform duplicates, or orphans.
- Existing manual checkbox selection remains unchanged until the action succeeds and the inventory reload resets state.

## Trade-offs

- A dedicated button is clearer than teaching users to manually select all leftover paths and press Apply selected.
- Using `window.confirm` matches the existing force-update and force-mirror confirmation style, avoiding a new modal pattern for one destructive shortcut.
