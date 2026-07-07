# Design: Central Card Platform Uninstall Action

## Boundaries

Reuse the existing Central platform uninstall flow instead of adding new backend behavior.

- Card props: `src/components/central/centralSkillCardProps.ts`
- Card rendering: `src/components/skill/UnifiedSkillCard.tsx`
- List wiring: `src/components/central/CentralSkillListContent.tsx`
- Grouped list wiring: `src/components/central/CentralGroupedSkillList.tsx`
- Dialog state: `src/pages/centralBatchUninstallView.ts`
- Existing confirmation dialog: `src/components/central/BatchUninstallCentralSkillsDialog.tsx`

## Data Flow

1. User clicks the card-level platform uninstall action for one Central skill.
2. The action opens the existing Central platform uninstall dialog with a single selected skill id.
3. The preview is produced by `createCentralBatchUninstallPreview([skill.id], skills)`.
4. The dialog shows removable installs, skipped/no-op state, and shared-root links using existing copy.
5. On confirm, `handleBatchUninstallCentralSkills` calls the existing per-agent batch uninstall store action.
6. Existing refresh logic updates Central skills and platform counts.

## Safety

- The new card action must call the platform uninstall flow only.
- It must not call `onDeleteFromCentral`.
- It must not bypass `shared_root_agents` filtering; preview creation remains the source of truth.
- Use a non-trash icon so the action does not look like Central deletion.

## Trade-offs

- Extending `useCentralBatchUninstallView` with an `openForSkill(skillId)` helper keeps single and bulk paths sharing one dialog and apply path.
- Adding a separate single-skill dialog would duplicate safety copy and edge-case handling.
