# Implementation Plan

## Pre-Implementation

- [ ] Re-run `python ./.trellis/scripts/get_context.py` and confirm current task is `06-10-central-sidebar-repo-search` in `planning`.
- [ ] After user approval to implement, run `python ./.trellis/scripts/task.py start .trellis/tasks/06-10-central-sidebar-repo-search`.
- [ ] Load `trellis-before-dev` before editing source files.
- [ ] Re-check dirty tree and avoid touching existing unrelated changes in update-center files/tests.

## Implementation Steps

1. Add repository-search matching helper
   - Candidate location: `src/lib/centralRepositoryGroups.ts`.
   - Preserve current behavior for empty query.
   - Add tests for owner match, repo match, local repo match, empty match, pinned ordering, and unknown-source hiding.

2. Add local UI state and input
   - Candidate files: `src/components/central/CentralSidebar.tsx` and possibly `CentralSidebarBlocks.tsx`.
   - Keep query local to expanded sidebar.
   - Render search input in the Repositories section before repository groups.
   - Add icon-only clear button and optional Escape-to-clear.

3. Add localized text
   - Update `src/i18n/locales/en.json`.
   - Update `src/i18n/locales/zh.json`.
   - Required keys likely include placeholder, aria label, clear label, and empty-state text.

4. Preserve existing sidebar behavior
   - Repository row click still calls `onToggleRepo`.
   - Pin/delete actions still stop propagation.
   - Bulk expand/collapse still applies to owner groups in the filtered result.
   - Collapsed rail remains unchanged.
   - Tags, saved views, and smart views are not filtered by the repository search query.

5. Add/extend component tests
   - Candidate file: `src/test/CentralSidebar.test.tsx`.
   - Cover localized render, owner filtering, repo filtering, clear action, empty state, and selecting a filtered repo.

## Validation

- [ ] `pnpm exec vitest run src/test/centralRepositoryGroups.test.ts src/test/CentralSidebar.test.tsx`
- [ ] `pnpm typecheck`
- [ ] `pnpm lint`
- [ ] `just ci`

## Review Checklist

- [ ] No changes to backend commands, database, or Tauri packaging.
- [ ] No serialization of the local repository search query into URL/Saved Views.
- [ ] Repository search is scoped to the Repositories section only.
- [ ] All user-facing strings are localized.
- [ ] UI remains compact at the current 280px overlay width and pinned sidebar widths.
- [ ] Active repo selection remains recoverable through existing chips/clear controls when filtered out by the local search.

## Rollback Points

- If helper logic gets too complex, keep `groupRepositoriesForSidebar` unchanged and filter grouped sections in a separate pure function with unit tests.
- If the input crowds the sidebar, move it under the Repositories header content rather than adding header-level controls.
