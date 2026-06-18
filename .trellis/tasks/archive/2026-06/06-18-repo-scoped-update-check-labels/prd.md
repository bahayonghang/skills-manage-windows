# Align repository-scoped update check labels

## Goal

Make the Central Skills update-check CTA describe the actual refresh boundary. In incremental/removal mode, repository-backed refreshes should be presented as repository-scoped work, not as a count of visible skills.

The user-visible problem came from a single GitHub repository filter such as `mattpocock/skills`: the button can display `Check mattpocock/skills (26)`, where `26` is the number of skills in that repository. The underlying sync action is repository-scoped, so the primary count should be the selected repository count, not the skill count.

## Confirmed Facts

- Upstream `npx skills update` exposes skill-name arguments, but the implementation groups tracked skills by `source`, fetches/checks repository state once per source where possible, detects upstream deletions from discovered paths, then applies updates at skill granularity. This supports a "repo/source refresh, skill-level inventory/apply" model.
- Current SkillPort already has separate refresh scopes: `regular` mode builds `kind: "skills"` with `skillIds`, while `sync` mode builds `kind: "repositories"` for a single repository filter and `kind: "all"` otherwise.
- The backend `refresh_skill_update_inventory` treats `Repositories` as a repository boundary, derives member skill IDs from each repository, and only scans remote additions/deletions when sync buckets are enabled and repository IDs are present.
- The current CTA label is mode-unaware. `centralSkillsCheckButton.ts` builds one `label` from `targetSkillIds.length`, so a single repository sync can look like a skill-count action.
- Existing Vitest coverage already verifies that a single repository filter in sync mode calls `refreshUpdateInventory({ kind: "repositories", mode: "sync", repositoryIds: [...] })`; the failing surface is primarily visible copy/count semantics, not the backend route.
- Prior repo memory warns that V2 list rendering can diverge from action scope if handlers read old list state; label logic should remain close to the existing `centralSkillsCheckButton.ts` helper rather than duplicating ad hoc counts in components.

## Requirements

- Regular mode keeps the existing skill-count semantics:
  - selected skills: selected skill count
  - filtered/current results: visible skill count
  - all: total central skill count
- Incremental/removal mode uses repository-scope semantics in the CTA and confirmation dialog:
  - a single selected syncable repository shows that repository as one repository target, not its skill count
  - no single repository filter shows the count of syncable GitHub repositories that will be checked
  - selected skills or non-repository filters must not imply that sync mode will only check those skills if the actual refresh scope is `all`
- The update-check mode confirmation dialog must use the same mode-aware scope wording as the CTA. If the user changes modes inside the dialog, the scope description should still match the selected mode.
- Result tabs and inventory rows remain skill/action-item based. Do not change Update Center inventory aggregation, apply decisions, backend refresh behavior, or repository sync semantics in this task.
- All user-visible strings must go through English and Chinese i18n resources.
- Keep changes surgical: prefer extending existing helper state and tests over moving update logic into the React shell.

## Acceptance Criteria

- [ ] In regular mode, existing tests for selected skills/current results/all still show skill counts and call `kind: "skills"` as before.
- [ ] In sync mode with one GitHub repository selected, the CTA and dialog describe one repository target, while the refresh call remains `kind: "repositories"` with the selected repository ID.
- [ ] In sync mode without a single repository scope, the CTA and dialog describe all syncable GitHub repositories and use the syncable repository count, while the refresh call remains `kind: "all"`.
- [ ] A selected-skill or non-repository-filtered view in sync mode does not display a misleading selected/current skill count for the CTA if confirming sync would check all repositories.
- [ ] English and Chinese locale files contain the new repository-scoped labels.
- [ ] Targeted Vitest coverage in `src/test/CentralSkillsView.updates-and-search.test.tsx` passes.
- [ ] Final verification includes `pnpm typecheck`, `pnpm lint`, and `just ci` unless the implementation phase records a concrete blocker.

## Notes

- Recommended product choice: use explicit unit labels such as `Check mattpocock/skills (1 repo)` / `检查 mattpocock/skills（1 个仓库）` and `Check all repositories (N)` / `检查全部仓库（N 个）`. This is slightly longer than the current copy but avoids another ambiguous bare number.
