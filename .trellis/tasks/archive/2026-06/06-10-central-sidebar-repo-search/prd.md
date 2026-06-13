# Add Repository Search to Central Sidebar

## Goal

Make the Central Skills sidebar repository tree searchable so users can quickly find a source repository when the list contains many owners and repositories.

The user-facing pain is visible in the current Central Skills screen: the global skill search can filter skills by `repo:` syntax, but the left repository tree itself remains long and hard to navigate.

## Confirmed Facts

- Central Skills already has a global search bar driven by `viewState.q` and `centralSearchQuery.ts`.
- The existing global search supports `repo:<owner/repo>` and `owner:<name>` filters, but it filters the skill result list, not the sidebar repository tree.
- The sidebar repository tree is rendered by `src/components/central/CentralSidebar.tsx` and `src/components/central/CentralSidebarBlocks.tsx`.
- Repository grouping and sorting are centralized in `src/lib/centralRepositoryGroups.ts`.
- Sidebar repository rows already toggle `viewState.repos` through `CentralSkillsShell`; selecting a repo is the action that filters the main skill list.
- Sidebar text must be localized in both `src/i18n/locales/en.json` and `src/i18n/locales/zh.json`.
- Existing tests cover repository grouping in `src/test/centralRepositoryGroups.test.ts` and sidebar interactions in `src/test/CentralSidebar.test.tsx`.

## Requirements

- Add a compact repository search input inside the expanded Central sidebar's Repositories section.
- The search input filters only the repository tree shown in the sidebar. It must not change `viewState.q`, URL state, Saved Views, or the main skill list by itself.
- Clicking a repository after filtering must keep using the existing repository-selection behavior.
- Search should match repository owner, repository name, full `owner/repo` label, local repository display name, and repository id as a fallback.
- When an owner matches the query, show all repositories under that owner. When only repository rows match, show only those matching rows under their owner.
- Preserve existing repository grouping rules: GitHub before Local, owner groups, pinned repositories first, unknown-source repository hidden when it has no skills.
- Counts and update badges must continue to represent the current facet/update state and must not be recalculated from the search query as if it were a skill filter.
- Provide a clear affordance for non-empty search and a localized empty state when no repositories match.
- Keep the collapsed rail unchanged; repository search appears only when the sidebar panel is expanded.
- Keep the UI dense, keyboard reachable, and consistent with the existing sidebar/search control vocabulary.

## Non-Goals

- Do not add backend search or database queries.
- Do not change the global Central search syntax or behavior.
- Do not make repository search part of URL sharing or Saved Views.
- Do not search or filter tags, saved views, smart views, or other sidebar sections in this MVP.
- Do not redesign the entire sidebar or change tag/saved-view grouping.
- Do not introduce fuzzy-search dependencies unless a simple normalized substring match proves insufficient.

## Acceptance Criteria

- [ ] Expanded Central sidebar renders a localized repository search field in the Repositories section.
- [ ] Typing part of an owner name filters owner groups as expected and keeps all repos for a matched owner visible.
- [ ] Typing part of a repo name/full name filters the tree to matching repo rows while preserving owner/group context.
- [ ] Clearing the query restores the full repository tree with existing sorting and pinned ordering.
- [ ] Empty matches show a localized "no matching repositories" state without affecting current skill results.
- [ ] Selecting a visible repo after filtering updates `viewState.repos` exactly as before.
- [ ] Existing selected repo chips and the sidebar clear-selection footer still work when the local repository search query is active.
- [ ] `CentralSidebar` component tests cover rendering, filtering, empty state, clear action, and selection after filtering.
- [ ] `centralRepositoryGroups` tests cover repository-tree filtering or the new helper's equivalent behavior.
- [ ] `pnpm typecheck`, `pnpm lint`, targeted Vitest tests, and final `just ci` pass before completion.

## Scope Decision

The MVP is repository search only. Tags, saved views, smart views, and broader sidebar-wide search are out of scope.

Reason: the immediate user pain is finding a repository in a long repo tree, while the existing top search already handles skills/tags and structured `repo:` / `owner:` queries for the main result list. A broader sidebar search can be considered later only if the same navigation problem appears in other sidebar sections.
