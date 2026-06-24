# Design: Central Skills Interface Polish

## Boundaries

This is a frontend-only polish task for the Central Skills page. It should improve perceived quality, scanability, and tactile feedback while preserving the existing Central Skills workflows and data contracts.

Primary code surfaces:

- `src/components/central/CentralSkillsShell.tsx`
- `src/components/central/CentralSidebar.tsx`
- `src/components/central/CentralTopFilters.tsx`
- `src/components/central/CentralSearchBar.tsx`
- `src/components/central/FacetItem.tsx`
- `src/components/central/FacetSection.tsx`
- `src/components/central/CentralSkillListContent.tsx`
- `src/components/skill/UnifiedSkillCard.tsx`
- `src/components/ui/button-variants.ts`
- `src/index.css` only if shared tokens/utilities are needed

The implementation must keep components prop-driven. Do not move domain data into UI components and do not add direct Tauri calls.

## Visual System Direction

Use a restrained operational-tool polish pass, not a marketing redesign:

- Keep the dense Central Skills workflow visible in the first screen.
- Reduce visual noise from repeated hard borders.
- Keep the dark Catppuccin-style surface identity.
- Make action hierarchy clearer: primary update/check actions, secondary import/update-center actions, and utility menus should have distinct but compatible weights.
- Preserve card density and virtualized grid stability.

## Contracts

- `UnifiedSkillCard` remains the only skill card implementation.
- Central grid/list sizing continues to use `src/lib/centralSkillGrid.ts` and the existing virtualized item heights.
- User-visible strings must remain localized through i18n if any copy changes are needed.
- No `motion` or `framer-motion` dependency is added. Use CSS transitions for interactive polish.
- All added transitions must specify properties; avoid `transition-all`.
- Small interactive controls should aim for at least 40x40px hit areas without overlapping neighboring controls.
- Dynamic Central counts should use tabular numerals consistently.

## Component-Level Design

### Header And Search Chrome

Tune the toolbar as one coherent control band:

- Add compatible radii and height rhythm across header buttons, select trigger, and icon menu.
- Apply optical padding for icon + text buttons.
- Keep the main check action visibly primary without making the entire top row visually heavy.
- Make the path/change-location line less visually noisy while preserving discoverability.

### Sidebar

Improve the expanded sidebar surface and dense controls:

- Rework the collapse-all group control so nested radius, icon slot, and text block align.
- Ensure pinned/overlay sidebar states still work and the overlay shadow is clearer than the regular pinned divider.
- Keep repository search and repository section behavior unchanged.
- Audit section headers and facet rows for tabular counts and minimum hit areas.

### Filter And Search Rows

Make filter rows feel deliberate rather than stacked utility strips:

- Normalize source pills, tag pills, command palette hint, and chip remove targets.
- Use `text-pretty` where chip or hint text can wrap in constrained widths.
- Preserve horizontal scrolling for tag filters.

### Skill Cards

Tune the existing card shell:

- Replace or soften the current hard ring/hover emphasis with a dark-surface shadow/ring recipe.
- Keep selected, update, inventory, and status signals recognizable.
- Improve title/action alignment and card action hit areas.
- Add `text-pretty` to summaries while preserving clamp behavior.
- Keep footer and platform icon behavior unchanged except for spacing and tactile polish.

## Accessibility And Motion

- Maintain visible focus rings.
- Keep real `button` and input controls.
- Respect existing reduced-motion rules in `src/index.css`.
- Add scale-on-press only where it does not cause layout jump in virtualized lists or dense table-like rows.
- For icon state changes, prefer CSS cross-fades with opacity, scale, and blur; skip if it complicates the component without clear value.

## Rollback Shape

This task should be easy to roll back by reverting only UI class/token changes. Avoid changing view model, store, or backend files so behavioral rollback stays simple.
