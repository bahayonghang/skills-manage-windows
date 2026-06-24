# Dashboard Interface Polish Design

## Scope

This task should make surgical Dashboard-only changes. The likely edit surface is:

- `src/components/dashboard/DashboardShell.tsx`
- `src/components/dashboard/sections/HeroSection.tsx`
- `src/components/dashboard/sections/HealthOrbit.tsx`
- `src/components/dashboard/sections/MetricStrip.tsx`
- `src/components/dashboard/sections/ProgressBreakdown.tsx`
- `src/components/dashboard/DashboardPanels.tsx`
- the Dashboard CSS block in `src/index.css`

Avoid touching stores, Tauri commands, database code, or Central skill logic.

## Design Direction

Keep the Dashboard as a work surface, not a landing page. The current screenshot has a strong identity, but the first viewport is visually expensive: the hero type, glass surfaces, orbit glows, and readiness decoration all compete. The design should keep the dark SkillPort character while making operational data calmer and easier to scan.

## Surface System

Use a consistent radius hierarchy:

- Outer Dashboard panels: large but not excessive, roughly `rounded-2xl` to `rounded-3xl` depending on existing layout.
- Inner plaques and tiles: smaller than their parent, usually `rounded-xl` or `rounded-2xl`.
- Buttons and small controls: `rounded-lg` or equivalent, with enough height for target size.

Do not put new visual cards inside existing cards. If an inner element is only a metric plaque, make it read as an inset tile through scale, radius, and shadow, not as another full card.

Prefer subtle shadow or ring depth for cards and tiles on complex backgrounds. Keep borders for real separators such as panel headers and list dividers.

## Typography

The code already uses `text-balance` for the hero heading and `text-pretty` for hero body copy. Preserve that direction.

Tune Dashboard typography by scale and density:

- Reduce hero dominance rather than rewriting copy.
- Keep counters tabular.
- Use `text-pretty` for short descriptions where wrapping matters.
- Avoid relying on truncation for first-viewport operational descriptions unless space is genuinely constrained.

## Interaction and Motion

The project does not depend on `motion` or `framer-motion`. Use CSS transitions only.

Dashboard-specific controls should use:

- `active:scale-[0.96]` for tactile press feedback where it does not distract.
- explicit transition properties, never `transition` or `transition-all`.
- 40px or larger hit areas for first-screen primary actions and other button-like controls.

Do not add motion to static navigation icons. If future state-changing icon swaps are added, use the CSS cross-fade pattern from the `make-interfaces-feel-better` skill.

## Responsive Behavior

Validate at:

- wide desktop, matching the screenshot class of viewport
- narrower desktop/tablet width where the hero and readiness panel stack or compress

The goal is not pixel-perfect parity with the screenshot. The goal is stable hierarchy, no overlapping text, and no dynamic content resizing that makes tiles jump.

## Compatibility

Preserve existing tests that prove Dashboard does not trigger marketplace sync while navigating. Keep all existing `data-testid` attributes unless a test is intentionally updated.

All user-visible copy remains in i18n files if copy changes are necessary.

## Risks

- Changing shared `Button` variants would affect the whole app. Prefer Dashboard-specific classes first.
- Reducing decorative glows too far could erase the current brand feel. Favor intensity tuning over removal.
- CSS polish can regress light themes. Any CSS variable change should be checked against at least one light theme if implementation touches shared Dashboard CSS.
