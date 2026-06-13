# Optimize Central Skills Card Layout Design

## Architecture and Boundaries

The change should stay in the Central Skills frontend list/layout layer.

- Primary files:
  - `src/components/central/CentralGroupedSkillList.tsx`
  - `src/components/central/CentralSkillListContent.tsx`
  - `src/components/ui/virtualized-grid.tsx`, only if column calculation needs a
    small exported helper for testing or consistency
- Keep `UnifiedSkillCard` behavior unchanged unless verification proves the
  current compact card cannot safely support the selected minimum column width.
- Do not change central skills data loading, filtering, grouping, update checks,
  repository sync, or sidebar persistence.

## Current Data Flow

`CentralSkillsShell` receives sorted and filtered skill data through
`listContent`.

- If `viewState.group === "none"`, it renders `CentralSkillListContent`.
- If grouped, it derives groups with `groupSkillsByMode` and renders
  `CentralGroupedSkillList`.
- Both list components render `UnifiedSkillCard` in compact card mode for the
  Central card grid.

## Layout Contract

Introduce one card grid sizing contract for Central Skills:

- A named minimum card width constant should own the desktop grid threshold.
- The CSS grid path should use that minimum with
  `repeat(auto-fill, minmax(min(100%, <minWidth>), 1fr))`.
- The virtualized grid path should use the same minimum as `minColumnWidth`.
- The grouped grid path should use the same CSS grid policy rather than
  `lg:grid-cols-2`.

Recommended starting policy:

- `CENTRAL_SKILL_CARD_MIN_WIDTH = 220`
- `CENTRAL_SKILL_CARD_MAX_COLUMNS = 4`
- Four columns are allowed only when the content region itself is wide enough.
  With `220px` cards and `16px` gaps, the four-column threshold is roughly
  `220 * 4 + 16 * 3 = 928px` of list content width.

Rationale:

- The current `240px` minimum means three columns require roughly
  `240 * 3 + 16 * 2 = 752px` of list width. This is usually fine, but the grouped
  view ignores that and caps at two columns.
- Lowering to `220px` gives three columns at roughly `692px`, which makes the
  pinned sidebar state more resilient without making compact cards unusually
  narrow.
- Keeping max columns at four preserves the current wide-screen upper bound.
  The threshold is content-width based, so ordinary desktop widths should settle
  at two or three columns rather than compressing cards just to reach four.

## Compatibility Notes

- `VirtualizedGrid` currently defaults to `minColumnWidth = 420` and
  `maxColumns = 2` for other callers. Do not change those defaults globally,
  because `PlatformView` relies on the wider card policy.
- If a helper is added for column calculation, preserve the current formula:
  `floor((viewportWidth + columnGap) / (minColumnWidth + columnGap))`, clamped to
  `[1, maxColumns]`.
- Search-active Central Skills view intentionally forces list mode. Preserve
  that behavior.

## Trade-Offs

- A smaller minimum width increases density but gives less room to long names,
  status chips, and action icons. The implementation must verify real card
  rendering with long text before accepting `220px`.
- Keeping `240px` and only removing the grouped hard cap is lower risk but may
  still show two columns on narrower expanded-sidebar windows.
- Allowing four columns at wide desktop width preserves current ungrouped
  behavior. The accepted product direction is "up to four columns, but only
  when the content region is wide enough"; do not implement a forced
  four-column desktop breakpoint.

## Rollback Shape

The change should be easy to revert by restoring the previous grouped class and
Central grid minimum values. No database or persisted state migration is needed.
