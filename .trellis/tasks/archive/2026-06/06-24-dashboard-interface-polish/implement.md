# Dashboard Interface Polish Implementation Plan

Do not start this checklist until the user approves implementation and the task is started with `task.py start`.

## Checklist

1. Capture baseline
   - Run the app or inspect the current Dashboard with the same desktop-like viewport as the supplied screenshot.
   - Save or note a baseline screenshot for comparison.

2. Tune Dashboard background and surfaces
   - Reduce `bg-orbit`, `dashboard-hero-glow`, `readiness-card::before`, and `readiness-card::after` visual competition.
   - Adjust dashboard surface border/shadow balance without changing true panel dividers.
   - Verify dark theme remains dimensional and light themes are not washed out.

3. Refine hero hierarchy
   - Reduce hero text dominance through scale, width, line-height, or spacing.
   - Keep `text-balance` and `text-pretty`.
   - Give hero action controls at least 40px effective hit height.
   - Add Dashboard-local press feedback and explicit transition properties.

4. Refine readiness panel
   - Make the score plaque read as an inset metric rather than a nested hero card.
   - Align plaque, status badge, factor rails, and mini stats under one radius/depth system.
   - Keep all score and percentage values tabular.

5. Refine metric and lower panel controls
   - Update `StatButton`, `QueueRow`, and other Dashboard-specific button-like controls to use consistent radius, explicit transitions, and `active:scale-[0.96]` where appropriate.
   - Avoid broad changes to shared `Button` unless a Dashboard-only approach cannot satisfy hit area or interaction requirements.

6. Guard against regressions
   - Preserve all Dashboard test ids used by `DashboardView.test.tsx`.
   - Search for newly introduced `transition-all` or Tailwind `transition` shorthand in Dashboard files.
   - Check that first-viewport text does not overlap or clip at desktop and narrower widths.

## Validation Commands

```powershell
pnpm test -- DashboardView.test.tsx
pnpm typecheck
pnpm lint
just ci
```

## Visual Verification

- Capture a dark-theme desktop screenshot after the polish pass.
- Capture a narrower viewport screenshot after the polish pass.
- Compare against the supplied screenshot for hierarchy, hit area, and surface consistency rather than pixel parity.

## Rollback Points

- Dashboard CSS block in `src/index.css` can be reverted independently if theme-level changes cause broad regressions.
- Dashboard component class changes should stay isolated under `src/components/dashboard/**`.
- Avoid modifying shared `Button` variants unless the implementation explicitly records why the Dashboard-local approach failed.
