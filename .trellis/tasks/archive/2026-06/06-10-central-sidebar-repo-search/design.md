# Design: Central Sidebar Repository Search

## Summary

Add a local, client-side repository search control to the expanded Central sidebar. The search narrows only the Repositories tree; it does not participate in Central view state, URL state, saved views, or backend data loading.

This is intentionally different from the existing top search bar. The top search answers "which skills match this query?" The new sidebar search answers "where is the repo row I want to click?"

MVP scope is repository search only. Tags, saved views, smart views, and other sidebar sections are not searched by this control.

## Boundaries

- UI boundary: `src/components/central/CentralSidebar.tsx` owns the search state and placement.
- Rendering boundary: `src/components/central/CentralSidebarBlocks.tsx` keeps rendering repository sections/groups/rows.
- Data boundary: `src/lib/centralRepositoryGroups.ts` should remain the source of grouping/sorting behavior. Add a small pure helper there if filtering grouped repos is cleaner than component-local filtering.
- State boundary: do not add repository search to `CentralViewState`; it is ephemeral UI state.
- i18n boundary: all visible copy and aria labels go through `central.v2.*` keys in `en.json` and `zh.json`.
- Scope boundary: do not route this query into tag groups, saved views, smart views, or global search.

## Data Flow

1. `CentralSidebar` receives `repositories`, `facetCounts`, `repoUpdateCounts`, and current selections as today.
2. Expanded sidebar owns `repositorySearchQuery` with `useState("")`.
3. The query is normalized with the existing `normalizeSearchQuery` helper if suitable, or a local normalization equivalent if import boundaries make that simpler.
4. Repository list is grouped through a pure function:
   - empty query: same output as current `groupRepositoriesForSidebar(repositories)`;
   - owner match: include the whole owner group;
   - repo match: include only matching repositories in that group;
   - local/flat group match: include matching local rows;
   - empty after filtering: render the repository empty-search state.
5. The displayed tree still passes original `facetCounts` and `repoUpdateCounts` into rows. Search only changes visibility.
6. Clicking a row calls the existing `onToggleRepo` path.

## Matching Contract

Repository search should be case-insensitive and substring-based for the MVP.

Candidate text per repository:

- `repository.owner`
- `repository.repo`
- `repository.name`
- full name `${owner}/${repo}` when both parts exist
- `repository.url`
- `repository.id`

Owner group behavior:

- If the owner string matches, all repositories under that owner remain visible.
- If the owner does not match but one or more child repositories match, render the owner group with only matching children.
- Group counts should summarize visible children after repository search, while row counts still use `facetCounts`.

## UX Placement

Place the input near the Repositories section header/content, not in the global page toolbar.

Recommended structure:

- a 32px high input row directly inside `sidebar-section-repos-content`, before repository groups;
- Search icon on the left and an icon-only clear button on the right when non-empty;
- placeholder examples should be short, e.g. "Search repositories..." / "搜索仓库...";
- no helper paragraph unless the empty state needs one.

This follows the product-register guidance: familiar control, dense layout, no modal, no decorative motion.

## Accessibility

- Input must have a localized `aria-label`.
- Clear button must be keyboard reachable and have a localized label.
- Pressing Escape while focused in the input may clear the local query if cheap to implement.
- Filtering must not steal focus from the input.
- Empty state copy should be visible text, not only a placeholder.

## Compatibility

- No backend changes.
- No database migration.
- No URL migration.
- Existing saved views remain valid because repository search is not serialized.
- Existing sidebar collapsed rail remains unchanged.

## Risks

- Confusion with the global search bar: mitigate by placement and placeholder copy focused on repositories.
- Hidden active selection while repo search is active: keep selected repo chips and the clear-selection footer visible, and do not clear current selection automatically.
- Test brittleness from localStorage pinning: reuse the current `CentralSidebar.test.tsx` pattern that pins the sidebar before assertions.

## Rollback

The change should be isolated to sidebar UI, repository grouping helper/tests, and i18n. Rollback is removing the local state/control and restoring `groupRepositoriesForSidebar(repositories)` at the call site.
