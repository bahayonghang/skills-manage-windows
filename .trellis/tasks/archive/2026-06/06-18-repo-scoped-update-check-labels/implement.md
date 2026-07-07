# Implementation Plan

## Checklist

1. Update helper state in `src/pages/centralSkillsCheckButton.ts`.
   - Add explicit regular/sync labels and all-syncable repository IDs.
   - Keep existing `targetSkillIds` and `repositoryIds` semantics unchanged.
   - Verify: focused unit coverage through existing Central view tests.

2. Make `useCentralUpdateCheckModeController` choose the displayed CTA label by effective mode.
   - Sync label only when sync is available.
   - Regular fallback when no syncable repository exists.
   - Verify: tests see the expected button text before clicking.

3. Make `UpdateCheckModeDialog` mode-aware.
   - Replace static `scopeLabel` with labels for regular and sync modes.
   - Use the current local mode when rendering `central.updateCheckMode.description`.
   - Verify: tests can assert dialog copy for both modes if needed.

4. Add i18n keys in English and Chinese.
   - Suggested English:
     - `checkUpdatesRepositorySync`: `Check {{repo}} ({{count}} repo)`
     - `checkUpdatesAllRepositories`: `Check all repositories ({{count}})`
   - Suggested Chinese:
     - `checkUpdatesRepositorySync`: `检查 {{repo}}（{{count}} 个仓库）`
     - `checkUpdatesAllRepositories`: `检查全部仓库（{{count}} 个）`
   - Verify: no raw visible strings are introduced.

5. Update Vitest coverage in `src/test/CentralSkillsView.updates-and-search.test.tsx`.
   - Regular single repository filter still shows skill count and calls `kind: "skills"`.
   - Sync single repository filter shows repository count and calls `kind: "repositories"`.
   - Sync all/non-single scope shows all repository count and calls `kind: "all"`.
   - Optional edge case: sync preference + selected skills shows all repository count rather than selected skill count.

## Validation

Run after implementation:

```powershell
pnpm vitest run src/test/CentralSkillsView.updates-and-search.test.tsx
pnpm typecheck
pnpm lint
just ci
```

If `just ci` fails on a pre-existing unrelated dirty-tree or environment issue, record the exact failing command/output in this task before stopping.

## Risk Points

- `CentralSkillsView.tsx` has a known line-budget risk from prior work. Keep logic in helper/controller files rather than growing the page component.
- Avoid duplicating scope derivation in the shell. The label should be derived from the same state used by `buildUpdateCheckScope`.
- i18n JSON edits are easy to make asymmetric; update `en.json` and `zh.json` together.

## Rollback

Reverting this task should be limited to the helper/controller/dialog/i18n/test changes. No database or backend migration is planned.
