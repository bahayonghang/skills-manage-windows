# Implementation Plan

1. Extend the Central batch uninstall view helper.
   - Add a single-skill selected id override or equivalent `openForSkill(skillId)` API.
   - Verify the existing bulk behavior still uses the current selected ids.

2. Wire the new card action.
   - Add an `onUninstallFromPlatforms(skill)` callback through `CentralSkillListContent`, `CentralGroupedSkillList`, and `centralSkillCardProps`.
   - Add a non-trash card action in `UnifiedSkillCard`.
   - Use i18n labels for title and aria-label.

3. Keep the existing dialog/apply path.
   - Pass the same dialog state into `CentralSkillDialogs`.
   - Confirm that no new Rust command is needed.

4. Add focused tests.
   - Cover per-card button rendering and opening the single-skill uninstall dialog.
   - Cover backend call grouping for one clicked skill.
   - Cover no-op/shared-root behavior through the existing dialog state where practical.

5. Validate.
   - Run focused Vitest tests for Central uninstall/card behavior.
   - Run `pnpm typecheck`.
   - Run `pnpm lint`.
   - Run `just ci`.
