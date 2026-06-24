# Implementation Plan: Central Skills Interface Polish

## Success Criteria

- Central Skills page keeps existing workflows and data behavior.
- Screenshot-driven polish issues are addressed in the header, sidebar, filter/search rows, and skill cards.
- No new card component is introduced.
- No new animation dependency is added.
- `pnpm typecheck`, `pnpm lint`, relevant Central/UnifiedSkillCard tests, and `just ci` pass before completion.
- Final verification includes desktop screenshot review of `/central`; mobile/narrow viewport should not show overlapping text or controls.

## Ordered Checklist

1. Load current frontend specs.
   - Read `.trellis/spec/ref/skillshare/frontend/index.md`.
   - Read component and quality guideline files referenced by the index.
   - Verify Central grid sizing rules before touching list/card layout.

2. Add shared polish primitives only if they remove real duplication.
   - Prefer local class changes first.
   - If several components need the same dark-surface ring/shadow or tap-scale pattern, add a small utility/token in `src/index.css` or a local helper constant.
   - Verify no `transition-all` is introduced.

3. Polish header and toolbar in `CentralSkillsShell.tsx`.
   - Normalize action heights, icon sizes, icon-side padding, and radii.
   - Improve the Central path/change-location row without changing actions.
   - Keep update center and check update button behavior unchanged.
   - Verify via `CentralSkillsView.shell.test.tsx` if class/test assumptions are affected.

4. Polish `CentralSidebar.tsx`, `FacetSection.tsx`, and `FacetItem.tsx`.
   - Tune collapse-all control nested radius and surface treatment.
   - Improve overlay vs pinned sidebar depth.
   - Normalize facet row count badges and hit areas.
   - Keep repository search, pin persistence, expansion signals, and repo actions unchanged.

5. Polish `CentralTopFilters.tsx` and `CentralSearchBar.tsx`.
   - Normalize pills and chip remove targets.
   - Use tabular counts and pretty wrapping where appropriate.
   - Keep source/tag/query semantics unchanged.
   - Confirm tests for top filters and search still pass.

6. Polish `UnifiedSkillCard.tsx`.
   - Tune `cardShellClass` dark-surface shadow/ring and selected state.
   - Align title actions and minimum hit areas.
   - Add text wrapping improvements to summary text without changing clamp behavior.
   - Add tactile press feedback only to controls where it does not shift layout.
   - Update `UnifiedSkillCard.test.tsx` only if assertions need to reflect intentional class changes.

7. Run focused validation.
   - `pnpm typecheck`
   - `pnpm lint`
   - `pnpm test -- CentralSkillsView.shell CentralTopFilters CentralSearchBar UnifiedSkillCard`

8. Run full repository gate.
   - `just ci`

9. Visual verification.
   - Start the app/dev server using the project’s normal workflow.
   - Capture desktop screenshot of `/central` and compare against the audit criteria.
   - Capture a narrower viewport if feasible to verify no overlapping text/buttons.

## Risk Notes

- Virtualized grid item heights are sensitive. Do not increase card content height unless `CentralSkillListContent` heights are updated deliberately.
- Global `Button` changes affect the whole app; prefer targeted Central classes unless the change is clearly a design-system fix.
- The app uses multiple themes. Avoid hard-coded hex colors in component classes; use theme tokens and `color-mix` only where the repo already does.
- `.trellis/` task state may be local-only in this repo; treat it as workflow bookkeeping unless the user explicitly asks to commit Trellis artifacts.
