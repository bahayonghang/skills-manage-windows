# Polish Central Skills interface

## Goal

Improve the Central Skills page shown in the user-provided screenshot so it feels more deliberate, polished, and easier to scan without changing Central Skills workflows, data behavior, or information architecture.

## Requirements

- Scope is the Central Skills library screen only: header toolbar, search/filter rows, left sidebar, and skill card grid/list surfaces.
- Preserve the existing Central Skills feature behavior: search syntax, source/tag/repository filters, saved views, update center, check updates, GitHub import, card selection, card actions, and platform toggles.
- Keep `UnifiedSkillCard` as the only skill card implementation.
- Keep Central card grid sizing aligned with `src/lib/centralSkillGrid.ts` and existing virtualized grid/list constraints.
- Follow the `$make-interfaces-feel-better` audit findings captured in `research/screenshot-interface-audit.md`.
- Use existing React + TypeScript + Tailwind patterns and existing theme tokens.
- Do not add `motion`, `framer-motion`, or other animation dependencies.
- Do not introduce direct Tauri IPC calls in UI components.
- Do not redesign the global theme, routing, stores, or backend behavior.

## Acceptance Criteria

- [ ] Header controls have consistent visual rhythm: compatible heights, radii, icon alignment, and action hierarchy.
- [ ] Sidebar expanded and overlay states feel intentionally layered, with improved collapse-all control radius/alignment and clearer facet row hit areas.
- [ ] Search bar, filter chips, source pills, tag pills, and "More" menu look like one filter system rather than unrelated strips.
- [ ] Skill cards have a calmer dark-surface treatment, stable selected/update/status states, better title/action alignment, and readable clamped summaries.
- [ ] Dynamic counts visible on the Central Skills screen consistently use tabular numerals.
- [ ] Small icon buttons, chip remove buttons, checkboxes, and dense row actions meet or approximate a 40x40px hit area without overlapping neighboring controls.
- [ ] Any added interaction animation uses explicit transition properties and avoids `transition-all`.
- [ ] Text does not overlap or overflow incoherently on desktop and narrower viewport checks.
- [ ] `pnpm typecheck`, `pnpm lint`, focused Central/UnifiedSkillCard tests, and `just ci` pass before implementation is marked complete.
- [ ] Final implementation is visually verified with a screenshot of `/central`.

## Notes

- Current task status is planning only. Implementation should not start until these artifacts are reviewed and `task.py start 06-24-central-skills-interface-polish` is run after user approval.
- Code inspection confirmed likely implementation targets under `src/components/central/`, `src/components/skill/UnifiedSkillCard.tsx`, and possibly `src/components/ui/button-variants.ts` / `src/index.css`.
