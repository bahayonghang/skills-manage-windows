# Implementation Plan: Skill Detail Visual Hierarchy

## Preconditions

- Do not run `task.py start` or edit product code until the user reviews and approves this plan.
- Before implementation, load `trellis-before-dev` and the frontend spec index.
- Preserve the current worktree and keep this task frontend-only.

## Ordered Checklist

1. Establish focused regression coverage.
   - Extend `SkillDetailFileTree.test.tsx` with representative entries for every planned category, fallback, collapsed default, unique directory labels and `aria-expanded`.
   - Extend `SkillDetailView.test.tsx` for sidebar order, all update status presentations, metadata actions, platform `aria-pressed`/locked cues and empty states.
   - Prefer semantic assertions and stable `data-file-kind` values over brittle full class snapshots.

2. Implement file-family presentation in `SkillDetailFileTree.tsx`.
   - Add the local classifier and Lucide mapping.
   - Keep file names readable and apply category color to the icon/non-text cue.
   - Default-collapse root directories, add unique accessible disclosure/open labels, `aria-expanded`, focus rings and `aria-busy`.
   - Preserve indentation, wrapping, symlink handling and `onOpenPath` behavior.

3. Fix shared inspector semantics in `SkillDetailViewShared.tsx`.
   - Align section/field labels with the documented label scale and remove contrast-reducing opacity.
   - Add optional metadata icon/tone support only if required by two or more rows.
   - Add announced installed/locked state and a persistent non-color cue to `PlatformToggleIcon` without changing callbacks.

4. Reorder and restyle `SkillDetailSidebar.tsx`.
   - Move complete guarded sections into the order defined by `design.md`.
   - Reduce repeated bordered slab treatment in Metadata and File Tree.
   - Add restrained local/GitHub/technical visual cues.
   - Add the update-status presentation map and preserve current enablement/confirmation behavior.
   - Keep Repository/Tag management vertically grouped with consistent hover, focus, disabled and updating states.
   - Normalize Projects/Collections loading, empty and populated hierarchy without changing data.

5. Update i18n and theme contracts.
   - Add English/Chinese accessible action/state labels only where existing keys cannot express the directory name or platform state.
   - Extend theme contrast tests for any new meaningful foreground/surface combination.
   - Run the Impeccable detector again and resolve target-component `design-system-font-size` findings.

6. Run focused validation.

```powershell
pnpm exec vitest run src/test/SkillDetailFileTree.test.tsx src/test/SkillDetailView.test.tsx src/test/themeContrast.test.ts
pnpm typecheck
pnpm lint
```

7. Run the repository gate.

```powershell
just ci
```

8. Perform Tauri visual verification.

```powershell
just dev
```

   - Inspect a populated Central skill in Mocha, Latte and Claude light.
   - Verify the wide page rail and narrower drawer layout.
   - Check long Windows and Unix paths, a deep mixed file tree, all update statuses, installed/uninstalled/locked platforms, and empty Projects/Collections.
   - Confirm keyboard focus order and that no text, icon or button overlaps.

## Review Gates

- Confirm file colors remain categorical reinforcement and do not look like warnings/errors.
- Confirm installation/update status is visible before the file tree without hiding any metadata.
- Confirm reduction of cards improves hierarchy without making form boundaries ambiguous.
- Confirm all six themes remain readable; accent changes must not erase state meaning.

## Rollback Points

- File classifier and tree row treatment can be reverted independently.
- Sidebar section order and contained-surface changes are presentation-only and can be reverted without store changes.
- Shared label/platform semantics are isolated in `SkillDetailViewShared.tsx`; focused tests protect the rollback.

## Completion Evidence

- Focused Vitest output.
- `pnpm typecheck`, `pnpm lint` and `just ci` output.
- Impeccable detector count for the two target components.
- Tauri screenshots/check notes for Mocha, Latte and Claude light at wide and narrow layouts.
