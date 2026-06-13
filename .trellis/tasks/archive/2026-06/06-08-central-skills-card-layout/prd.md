# Optimize Central Skills Card Layout

## Goal

Improve the Central Skills card grid so the main content area uses available
desktop width consistently when the Central sidebar is pinned/expanded or
collapsed. Users should not see the expanded sidebar state artificially cap the
skill cards at two columns when the remaining content area can still support a
denser, readable layout.

## User Value

- More skills remain visible above the fold in Central Skills, especially when
  users keep the repository/filter sidebar expanded.
- The grid behaves predictably across the default grouped view and the plain
  card grid.
- The fix preserves the compact card design and avoids a broad redesign of the
  Central Skills shell.

## Confirmed Facts

- The Central Skills shell switches between two list implementations:
  `CentralSkillListContent` when `viewState.group === "none"` and
  `CentralGroupedSkillList` for grouped views.
- `CentralSkillListContent` already uses width-driven grid behavior:
  non-virtualized cards use `repeat(auto-fill, minmax(min(100%, 240px), 1fr))`;
  virtualized cards use `minColumnWidth={240}` and `maxColumns={4}`.
- `CentralGroupedSkillList` uses `grid grid-cols-1 gap-4 lg:grid-cols-2`,
  which hard-caps grouped sections at two columns on large screens.
- The Central filter/sidebar default width is `286px`, with a min/max range of
  `220px` to `460px`, so pinned sidebar state materially changes the available
  card grid width.
- `UnifiedSkillCard` compact mode has a `min-h-[168px]` card body and text is
  already truncated/clamped, so a narrower minimum column width is possible but
  must be visually verified with long names and localized labels.
- Existing Central view tests cover shell state, pinned sidebar behavior, scroll
  safety, and selection behavior, but not the exact card grid column policy.

## Requirements

- The grouped Central Skills card grid must use the same responsive column
  policy as the ungrouped grid unless there is a specific grouped-view reason to
  diverge.
- The expanded/pinned Central sidebar state must still allow at least three
  columns on desktop widths where the remaining content region is wide enough
  for readable compact cards.
- The card grid may render up to four columns, but only in a genuinely wide
  content region. The implementation must not force four columns on ordinary
  desktop widths by making cards cramped.
- The grid must avoid text overlap, button overlap, or clipped action icons for
  common skill names, long repository names, localized labels, status chips, and
  platform icons.
- The solution should be surgical: prefer changing shared sizing constants or
  local grid templates over rewriting the shell, sidebar, or `UnifiedSkillCard`.
- Virtualized and non-virtualized grid paths must stay consistent enough that
  large and small result sets do not appear to follow unrelated layout rules.
- Existing list mode, search-active list mode, selection/bulk bar behavior, and
  sidebar resizing must keep working.

## Acceptance Criteria

- [ ] With Central sidebar pinned/expanded at the default width, grouped card
      sections can render three columns on a typical wide desktop content area
      when enough horizontal space is available.
- [ ] With Central sidebar collapsed to rail, the same view continues to render
      at least as many columns as it does today and may reach four columns only
      when the content region crosses the documented wide-screen threshold.
- [ ] Grouped and ungrouped card grids use one documented minimum column width
      policy or an explicitly documented pair of policies.
- [ ] Large result sets using `VirtualizedGrid` and small result sets using CSS
      grid calculate compatible column counts for the same content width.
- [ ] Cards remain readable: no overlapping names, tags, footer repo labels,
      status chips, platform icons, or action buttons in expanded and collapsed
      sidebar states.
- [ ] Focused tests cover the grouped grid policy and the virtualized grid column
      calculation or its owning helper, if a helper is introduced.
- [ ] `pnpm typecheck`, `pnpm lint`, the focused Vitest tests, and `just ci`
      pass before implementation is reported complete.

## Notes

- Screenshot diagnosis: the defect is a density inconsistency in the Central
  Skills workspace. Keeping the filter/repository sidebar expanded reduces the
  main content width, but the current grouped card grid also contains a hard
  two-column cap, so the view can waste usable horizontal space.
- Likely out of scope: changing global app sidebar behavior, redesigning
  `UnifiedSkillCard`, adding user-configurable card width settings, or changing
  Central Skills data/query behavior.
- Product decision: allow up to four columns, but only under a wide-screen /
  wide-content condition. Three columns should remain the normal dense desktop
  target when the sidebar is expanded and the content area is not wide enough
  for comfortable four-column cards.
