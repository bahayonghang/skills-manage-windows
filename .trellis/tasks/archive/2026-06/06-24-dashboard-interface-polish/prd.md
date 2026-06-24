# Dashboard interface polish audit

## Goal

Improve the Dashboard first-viewport polish so it feels like a calm, work-focused SkillPort cockpit instead of a heavy landing-style composition. The optimization should preserve the current product intent: users should immediately see Central health, review queues, enabled targets, and safe next actions without triggering background network work.

This task is based on the screenshot supplied in chat on 2026-06-24 and the `make-interfaces-feel-better` review principles.

## Assumptions

- Scope is the Dashboard view shown in the screenshot, not a global redesign of the whole app shell.
- The screenshot is a wide desktop dark-theme state with meaningful production-like counts: 229 Central skills, 30 sources, 4 enabled agents, and a readiness score of 65.
- Implementation should keep all user-visible text in existing i18n resources if any text changes are needed.
- Implementation should not change Dashboard data loading, registry sync behavior, Central skill semantics, or target selection behavior.

## Confirmed Facts

- The screenshot maps to `src/pages/DashboardView.tsx` and the composed `DashboardShell`.
- First-viewport components are split across:
  - `src/components/dashboard/DashboardShell.tsx`
  - `src/components/dashboard/sections/HeroSection.tsx`
  - `src/components/dashboard/sections/HealthOrbit.tsx`
  - `src/components/dashboard/sections/MetricStrip.tsx`
  - `src/components/dashboard/sections/ProgressBreakdown.tsx`
  - `src/components/dashboard/DashboardPanels.tsx`
  - Dashboard-specific CSS in `src/index.css`
- Existing code already uses several good details:
  - Hero heading has `text-balance`.
  - Hero body has `text-pretty`.
  - Dynamic dashboard numbers mostly use `tabular-nums`.
  - Dashboard avoids background network operations during quick navigation, covered by `src/test/DashboardView.test.tsx`.
- `package.json` does not include `motion` or `framer-motion`, so any icon or press transitions should use CSS rather than adding a motion dependency.

## Requirements

### Screenshot Audit

#### Typography and Hierarchy

| Before | After |
| --- | --- |
| Hero headline visually dominates the first viewport and wraps into a dense 4-line block in the screenshot. | Reduce hero dominance enough that the readiness panel and action strip remain first-scan peers; preserve a strong Dashboard title but avoid landing-page scale. |
| Dashboard uses a display/mono-heavy look in the screenshot, making dense operational text feel heavier than needed. | Keep the user's font settings, but tune Dashboard scale, weight, and line-height so operational copy scans cleanly at desktop sizes. |
| Some panel descriptions and metric descriptions rely on truncation or tight wrapping. | Use `text-pretty` where descriptions are short UI copy, and avoid truncating information that should be scannable in the first viewport. |

#### Concentric Radius and Surfaces

| Before | After |
| --- | --- |
| `surface-glass rounded-3xl` panels, `rounded-3xl` score plaques, `rounded-2xl` rails, and `rounded-md` metric tiles create an uneven radius hierarchy. | Establish a Dashboard radius ladder: outer panels, inner plaques/tiles, and controls should read as deliberately nested surfaces. |
| The readiness score plaque uses a large rounded surface inside another large rounded surface, which reads blobby in the screenshot. | Make the score plaque feel like an inset metric, not a second hero card, using smaller inner radius and calmer depth. |
| Metric strip cards are flatter and sharper than the large glass panels above them. | Bring metric tiles into the same surface system without making them look like nested cards inside cards. |

#### Shadows, Borders, and Background Weight

| Before | After |
| --- | --- |
| The orbit/glass background, hero glow, readiness glow, and readiness grid compete for attention in dark mode. | Reduce decorative intensity so data and next actions are primary; retain the SkillPort visual identity without heavy background noise. |
| Several Dashboard containers use visible borders for depth on already complex backgrounds. | Prefer subtle shadow/ring depth on dashboard cards and buttons where it improves depth; keep true dividers as borders. |
| The readiness card's decorative glow can make the top-right area feel heavier than the actual score and factors. | Make the score, status, and factor rails visually lead the card, with decoration subordinate. |

#### Interactions, Motion, and Hit Areas

| Before | After |
| --- | --- |
| Dashboard hero action buttons and stat tiles mostly transition color only; press feedback is inconsistent with the `scale(0.96)` polish rule. | Add tactile `active:scale-[0.96]` press feedback to Dashboard-specific controls where it will not distract. |
| Shared `Button` default height is 32px, and Dashboard hero CTAs appear compact for primary first-screen actions. | Dashboard first-screen action hit areas should be at least 40px high or have an equivalent non-overlapping hit target. |
| Future Dashboard edits could accidentally use Tailwind `transition` / `transition-all`. | Dashboard polish must use explicit transition properties such as `transition-[scale,box-shadow,border-color,background-color,color]` or `transition-transform`. |

#### Numeric Stability

| Before | After |
| --- | --- |
| Most Dashboard counters already use `tabular-nums`, but acceptance is implicit. | Keep all dynamic Dashboard counts, percentages, and score values on tabular numerals after the polish pass. |

### Functional Requirements

- Preserve all Dashboard navigation targets and store behavior currently covered by tests.
- Preserve first-screen information architecture:
  - hero next actions
  - readiness score and factor rails
  - metric strip
  - Central library health
  - enabled platforms
- Keep the app shell sidebar and top bar out of scope unless a Dashboard change visibly breaks their alignment.
- Do not add a new animation library.
- Do not add new marketing copy or an explanatory landing page.

## Acceptance Criteria

- [ ] At desktop width, the hero, readiness panel, and metric strip form a clear hierarchy without the hero headline or decorative glows overpowering operational data.
- [ ] Dashboard nested surfaces follow a consistent radius hierarchy; the readiness score plaque and metric tiles no longer feel like mismatched rounded islands.
- [ ] Dashboard-specific interactive controls that are visually button-like have interruptible transitions and use `active:scale-[0.96]` where appropriate.
- [ ] Dashboard first-screen action hit areas are at least 40px high or have an equivalent non-overlapping hit target.
- [ ] No new Dashboard code uses `transition-all` or Tailwind `transition` shorthand for changed properties.
- [ ] Dynamic Dashboard numbers, percentages, and score values keep `tabular-nums`.
- [ ] Existing Dashboard navigation and no-background-sync behaviors still pass.
- [ ] Verification includes:
  - `pnpm test -- DashboardView.test.tsx`
  - `pnpm typecheck`
  - `pnpm lint`
  - `just ci` before final completion
- [ ] Visual verification captures at least one dark-theme desktop screenshot and one narrower viewport screenshot before declaring implementation complete.

## Out of Scope

- Changing Central skill counts, readiness formulas, source mapping, or install health semantics.
- Redesigning the sidebar, top search bar, Settings, Central Skills, Marketplace, or app-wide navigation.
- Adding new Dashboard panels or new product workflows.
- Changing global font settings, theme tokens, or shared button behavior unless the implementation proves a Dashboard-only fix is insufficient.
- Windows installer or release packaging work.

## Open Product Decision

- Recommended: preserve the current orbit/glass visual identity, but reduce intensity and tighten hierarchy.
- Alternative: flatten the Dashboard into a more utilitarian operations console. This would likely require broader theme and layout work and should be split into a separate design task.

## Notes

- This is a planning task only until the user explicitly approves implementation.
