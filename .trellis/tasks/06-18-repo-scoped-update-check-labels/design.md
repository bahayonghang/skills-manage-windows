# Design

## Boundary

This is a frontend semantics alignment task. The backend refresh commands, inventory persistence, repository membership derivation, and apply/force actions are already repository-aware enough for the requested behavior.

The implementation should stay in these surfaces:

- `src/pages/centralSkillsCheckButton.ts`
- `src/pages/centralUpdateCheckModeController.tsx`
- `src/components/central/UpdateCheckModeDialog.tsx`
- `src/i18n/locales/en.json`
- `src/i18n/locales/zh.json`
- `src/test/CentralSkillsView.updates-and-search.test.tsx`

## Current Data Flow

1. `CentralSkillsView` derives visible skills, selected skill IDs, selected repo IDs, and repository metadata.
2. `getCentralSkillsCheckButtonState` computes `targetSkillIds`, optional `repositoryIds`, and one `label`.
3. `useCentralUpdateCheckModeController` reads the persisted update-check mode preference and opens `UpdateCheckModeDialog`.
4. On confirmation, `buildUpdateCheckScope(mode, checkButtonState)` decides the actual refresh scope:
   - `regular` -> `kind: "skills"` with `targetSkillIds`
   - `sync` + single repository filter -> `kind: "repositories"`
   - other `sync` cases -> `kind: "all"`
5. `refresh_skill_update_inventory` executes that scope; Update Center then displays skill/action-item inventory.

The mismatch is step 2: the label is computed before the chosen mode is considered.

## Proposed Shape

Extend `CentralSkillsCheckButtonState` with mode-aware display labels while preserving the existing scope data used by refresh calls.

Suggested fields:

- `regularLabel`: existing skill-count label.
- `syncLabel`: repository-scope label based on the sync scope that `buildUpdateCheckScope("sync", state)` will use.
- `syncableRepositoryIds`: all syncable GitHub repository IDs, used for the `kind: "all"` sync label.
- Keep `label` temporarily only if needed for compatibility, but prefer using explicit labels in the controller to avoid future ambiguity.

The controller should derive an effective display mode:

- if the saved preference is `sync` and at least one syncable repository exists, use `syncLabel`
- otherwise use `regularLabel`

`UpdateCheckModeDialog` should accept both labels instead of one static `scopeLabel`, then choose the description label from its local selected mode. This keeps the dialog accurate when the user switches between regular and sync inside the confirmation dialog.

## Label Rules

Regular mode:

- Preserve current i18n keys where possible:
  - `central.checkUpdatesSelected`
  - `central.checkUpdatesCurrentResults`
  - `central.checkUpdatesRepository`
  - `central.checkUpdatesAll`

Sync mode:

- Single repository scope: new i18n key, e.g. `central.checkUpdatesRepositorySync`, with `repo` and `count: 1`.
- All/sync fallback scope: new i18n key, e.g. `central.checkUpdatesAllRepositories`, with count equal to `syncableRepositoryIds.length`.
- If no syncable repository exists, controller falls back to regular label because sync mode is disabled in the dialog.

## Compatibility Notes

- Do not change `buildUpdateCheckScope` behavior in this task unless tests prove the visible label cannot be made truthful without doing so.
- Do not change the Update Center toolbar scope selector. It already has a repository scope option when the open context carries repository IDs.
- Do not change inventory counts in tabs; those remain action-item counts.

## Trade-Offs

- Showing repository count in the main CTA can be longer than the old skill-count label, but it matches the action boundary and prevents the user from thinking a repo sync is a 26-skill-only check.
- Making selected-skill + sync mode display "all repositories" may expose an existing behavior surprise. That is acceptable for this task because the requirement is truthful scope wording. A future product task could choose to disable sync when selected skills are active or force regular mode for selected-skill flows.
