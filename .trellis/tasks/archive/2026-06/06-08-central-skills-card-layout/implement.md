# Optimize Central Skills Card Layout Implementation Plan

## Checklist

1. Define the Central card grid sizing contract.
   - Add local constants in `CentralSkillListContent` or a tiny shared Central
     layout helper if both grouped and ungrouped list components need it.
   - Recommended names:
     `CENTRAL_SKILL_CARD_MIN_WIDTH = 220`,
     `CENTRAL_SKILL_CARD_MAX_COLUMNS = 4`,
     `CENTRAL_SKILL_CARD_GRID_GAP = 16`.

2. Align the ungrouped non-virtualized grid.
   - Replace the inline `240px` value with the shared Central minimum width.
   - Keep `auto-fill` and `minmax(min(100%, ...), 1fr)`.

3. Align the ungrouped virtualized grid.
   - Pass the same Central minimum width into `VirtualizedGrid`.
   - Keep `maxColumns={4}`.
   - Rely on the shared minimum width and gap formula so four columns appear
     only after the content region reaches the documented wide-screen threshold.

4. Align the grouped grid.
   - Replace `grid grid-cols-1 gap-4 lg:grid-cols-2` in
     `CentralGroupedSkillList` with the same responsive CSS grid template used
     by the ungrouped non-virtualized path.
   - Do not add virtualization to grouped views in this task.

5. Add focused tests.
   - Add or update a Central grouped-list test that asserts the grouped body no
     longer contains a `lg:grid-cols-2` hard cap and has the expected responsive
     grid template.
   - If a `VirtualizedGrid` helper is introduced, test width-to-column behavior:
     widths around 691px, 692px, 928px, and a wide max-column case.

6. Visual verification.
   - Start the app with the normal dev command.
   - Check Central Skills at desktop width with the Central sidebar pinned and
     collapsed.
   - Verify grouped view and ungrouped grid view.
   - Inspect long skill names, repo footer labels, status chips, checkbox,
     install/update/delete icons, platform icons, and editable tag rows.

## Validation Commands

Run after implementation:

```powershell
pnpm typecheck
pnpm lint
pnpm test -- src/test/CentralSkillsView.shell.test.tsx src/test/UnifiedSkillCard.test.tsx
just ci
```

If the implementation touches `VirtualizedGrid`, also run the focused test file
that covers it, or add one if none exists.

## Risk Points

- Reducing the minimum card width too far can make compact card actions and
  localized text feel cramped.
- `VirtualizedGrid` is shared. Avoid changing its defaults unless every caller is
  checked.
- CSS grid assertions in jsdom are structural only. The final acceptance still
  needs a rendered desktop screenshot check.

## Review Gate Before `task.py start`

- Product density decision confirmed: allow up to four columns only when the
  content region is wide enough.
- Confirm that grouped and ungrouped grids should share one Central card minimum
  width.
