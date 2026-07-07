# Add central skill platform uninstall button

## Goal

Add a per-card action in Central Skills, at the right-side action area shown in the screenshot, to uninstall that Central skill from every independently removable platform install without deleting the Central skill itself.

The user value is a fast single-skill operation: the user should not need to select the skill and use the bulk action bar when they only want to remove one skill from platforms.

## Confirmed Facts

- Central skill cards are rendered through `src/components/skill/UnifiedSkillCard.tsx`; Central-specific props are built in `src/components/central/centralSkillCardProps.ts`.
- `src/components/central/CentralSkillListContent.tsx` wires Central card actions for install, update, and Central delete.
- Existing bulk Central platform uninstall already exists and uses `src/lib/centralBatchUninstall.ts`, `src/pages/centralBatchUninstallView.ts`, `src/pages/centralSkillsActions.ts`, and `BatchUninstallCentralSkillsDialog`.
- Existing uninstall semantics exclude `central` and `shared_root_agents`, call `batch_uninstall_skills_from_agent` only for removable platform installs, and never delete Central skill files or rows.
- User-visible copy for Central batch uninstall already exists in English and Chinese locale files.

## Requirements

- Add a visible per-card uninstall-from-platforms button in the right-side Central skill card action area, near the install/update/delete actions.
- The button must mean uninstall from platforms only. It must not delete the Central skill directory, Central DB skill row, repository, tags, or skill files.
- Reuse the existing Central platform uninstall preview and confirmation dialog so the user sees the safety copy and no-op/skipped state.
- The per-card action must target exactly the clicked skill.
- The action must exclude `central` and `shared_root_agents`, matching the existing bulk uninstall behavior.
- If the clicked skill has no removable platform installs, show the existing no-op dialog state and do not call backend uninstall.
- On success or partial failure, reuse the existing Central refresh and retry semantics.
- All new user-visible text must be added through i18n if existing keys are insufficient.

## Acceptance Criteria

- [ ] Each Central skill card shows a per-card platform uninstall action in the action cluster highlighted by the screenshot.
- [ ] Clicking the action opens the existing Central platform uninstall confirmation dialog for only that skill.
- [ ] Confirming a skill with removable platform installs calls `batch_uninstall_skills_from_agent` only for installed removable agents.
- [ ] Confirming is disabled for a skill with no removable platform installs and no backend uninstall call is made.
- [ ] Shared-root platform links are listed as non-removable and are not sent to the backend.
- [ ] Central delete remains a separate destructive action and is not invoked by the new uninstall button.
- [ ] Focused tests cover the new per-card button and single-skill dialog/backend flow.
- [ ] Relevant Vitest tests, `pnpm typecheck`, `pnpm lint`, and `just ci` pass before completion.

## Out of Scope

- Reworking bulk action behavior.
- Adding a new Rust command.
- Changing platform uninstall semantics for project skills or platform pages.
- Deleting Central skills.

## Notes

- The screenshot points to the card-level action cluster; this task should add a single-skill entry there, not another bulk-bar control.
