# Central Skills batch uninstall from platforms

## Goal

Add a Central Skills bulk action that uninstalls selected Central skills from platform installations without deleting the Central skill records or files.

The user value is to select many Central skills, including a filtered or repo-selected set, and clear their platform installs in one operation. This is different from the existing bulk delete action, which removes skills from the Central library.

## Confirmed Facts

- The Central bottom bulk action bar is `src/components/central/BulkActionBar.tsx`. It currently exposes batch install, categorize, AI suggest, Central delete, and clear selection.
- `src/pages/CentralSkillsView.tsx` owns Central selection state and wires bulk actions through `useCentralSkillsActions`.
- Central skills expose `linked_agents` and `shared_root_agents` in `SkillWithLinks`. Backend Central query appends shared-root agent ids into `linked_agents` and also exposes them separately.
- Shared-root agents share the Central Skills directory. Backend uninstall refuses them with "cannot be uninstalled independently", so the new feature must not send shared-root agents as ordinary uninstall targets.
- Existing backend command `batch_uninstall_skills_from_agent` removes multiple skills from one agent and reports partial success via `succeeded` and `failed`.
- Existing platform batch delete already reuses `batch_uninstall_skills_from_agent` and keeps failed rows selected after partial failure.
- Existing Central batch install reports skipped items for already installed targets. The new Central batch uninstall needs analogous skipped/not-applicable handling for selected skills that have no removable platform installs.

## Requirements

- Add a new bottom bulk action button for selected Central skills using the compact label "Uninstall" / "批量卸载"; the confirmation dialog must clarify that this is platform uninstall, not Central skill deletion.
- The action must uninstall selected skills from all removable platform installs visible in Central skill metadata.
- The action must never delete Central skill directories, Central DB skill rows, repositories, tags, or skill files.
- The action must exclude `central` and every `shared_root_agents` entry from uninstall requests.
- If a selected skill is not installed on any removable platform, treat it as skipped/not applicable, not as a backend failure.
- If a selected skill is installed on some platforms and not others, uninstall only the installed removable platforms.
- If all selected skills have no removable platform installs, show a non-destructive no-op state and do not call backend uninstall.
- Support partial backend failures: successful platform uninstalls remain applied; failed selected skills stay selected or are otherwise clearly retryable.
- Refresh Central skills and platform counts after the operation so cards, badges, filters, and sidebar counts reflect the new install state.
- All new user-visible text must be added to both `src/i18n/locales/en.json` and `src/i18n/locales/zh.json`.

## Acceptance Criteria

- [ ] Selecting one or more Central skills shows a bottom bulk action button for uninstalling from platforms.
- [ ] Clicking the button opens a confirmation dialog that states the operation removes platform installs only and does not delete Central skills.
- [ ] The confirmation dialog summarizes removable installs by platform and reports selected skills with no removable platform install as skipped/not applicable.
- [ ] Confirming a mixed selection calls `batch_uninstall_skills_from_agent` only for agents that actually have selected skills installed, and excludes shared-root agents.
- [ ] A selection where nothing is installed on removable platforms does not call `batch_uninstall_skills_from_agent`.
- [ ] On full success, selection is cleared, dialog closes, Central list is refreshed, and platform counts are refreshed.
- [ ] On partial failure, successful uninstalls are applied, failures are reported, and failed skill ids remain selected for retry.
- [ ] Existing Central bulk delete continues to delete Central skills only through its existing flow.
- [ ] Targeted frontend tests cover button rendering, no-op skip handling, mixed installed/uninstalled selection, shared-root exclusion, full success, and partial failure.
- [ ] Targeted store/action tests cover grouped backend calls and refresh behavior.
- [ ] `pnpm typecheck`, `pnpm lint`, relevant Vitest tests, and `just ci` pass before completion.

## Out of Scope

- Project-scoped skill installs are not part of this bulk action.
- Read-only plugin/cache observations are not targeted by this Central action.
- Reworking Central delete or platform duplicate cleanup UX is out of scope.
- Adding a new Rust command is out of scope unless frontend-only orchestration through existing commands proves insufficient during implementation.

## Decisions

- Button wording is fixed as the compact label "Uninstall" / "批量卸载". The dialog title and description will carry the more explicit "from platforms" explanation.
