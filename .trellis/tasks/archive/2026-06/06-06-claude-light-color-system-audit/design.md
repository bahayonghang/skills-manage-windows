# Claude Light Color System Design

## Problem Statement

Claude Light currently treats `--primary` as both a filled-control color and a
general accent text color. That is workable in dark themes, but fails in light
themes where the accent is close in luminance to `--background` and `--card`.
The result is visible in the Dashboard screenshot: large glass panels inherit
coral/pink overlays, cards lose hierarchy, and accent text can become too weak.

This is not only a Claude Light issue. Latte has the same structural problem
with Catppuccin Lavender used as text on light surfaces.

## Evidence

Measured contrast from current `src/index.css` semantic tokens:

| Theme | Pair | Ratio | Result |
| --- | --- | ---: | --- |
| Claude Light | `primary` on `card` | 2.71:1 | Fail |
| Claude Light | `primary` on `background` | 3.11:1 | Large text only |
| Claude Light | `muted-foreground` on `card` | 4.48:1 | Borderline fail |
| Latte | `primary` on `card` | 2.62:1 | Fail |
| Latte | `primary` on `background` | 2.81:1 | Fail |
| Claude Dark | `primary` on `card` | 4.78:1 | Pass |
| Mocha/Macchiato/Frappé | sampled primary text pairs | >= 6.72:1 | Pass |

Dashboard evidence:

- `DashboardShell.tsx` wraps the page in `.bg-orbit`.
- `HeroSection.tsx` uses `.surface-glass` and an additional primary radial glow.
- `HealthOrbit.tsx` uses `.readiness-card`, `.readiness-score-plaque`, and
  multiple primary/chart gradients.
- `DashboardPanels.tsx` and dashboard sections use `text-primary` and
  `bg-primary/10` as both state and decorative emphasis.

Settings evidence:

- `settingsSectionTheme.ts` maps sections to `--ctp-*` colors and exposes
  `--settings-section-accent`.
- `SettingsCollapsibleCard.tsx` and `AppearanceSettingsSection.tsx` use that
  accent directly for titles, icons, labels, active states, and soft backgrounds.
- In Claude Light, the `--ctp-*` palette is remapped to Anthropic-like light
  tones, so some section accents become too soft for text usage.

## Design Direction

### 1. Split Filled Accent From Text Accent

Keep `--primary` as the filled-control accent because existing Button and
selected-state components rely on it. Introduce or standardize a separate
semantic role for accent text on light surfaces.

Preferred contract:

- `--primary`: filled primary control / active filled indicator.
- `--primary-foreground`: text on `--primary`.
- `--accent-text` or `--primary-text`: readable accent text on `background`,
  `card`, and `popover`.
- `--accent-soft`: low-emphasis tinted background.
- `--accent-border`: border/ring color mixed with `--border`.

If adding new global tokens is too broad for Tailwind theme mapping, use CSS
custom properties and bracket-value classes first, then optionally add Tailwind
aliases later.

Suggested Claude Light values:

- Keep signature coral fill: `--primary: #cc785c`.
- Use darker coral/brown for accent text: around `#a35038`, because it reaches
  about 4.62:1 on Claude Light `card` and 5.30:1 on `background`.
- Use light foreground on darker accent text only when the darker value becomes
  a filled chip. Keep current black-ish `--primary-foreground` for the
  existing coral fill.

Suggested Latte direction:

- Preserve Catppuccin Lavender as filled primary.
- Use a darker `--primary-text` for text/icon/link roles in light mode.
- Keep the special foreground overrides for filled Latte red/mauve/blue where
  they remain necessary.

### 2. Make Dashboard Light Theme Less Hazy

Do not remove the Dashboard control-room material. Scope its intensity by theme:

- For `[data-theme="claude-light"]` and `[data-theme="latte"]`, reduce
  `.bg-orbit` primary/ring radial alpha and distance.
- Reduce `.surface-glass` / `.surface-glass-strong` light-theme blur and
  transparent blending so panels read as distinct surfaces, not fog.
- Replace Dashboard `text-primary` usages that are normal text/icons with the
  readable accent text token.
- Keep `bg-primary/10` for selected or state surfaces only when paired with
  readable text and adequate contrast.
- Avoid new decorative gradients or extra glass; the fix should calm the
  existing material.

### 3. Make Settings Section Accent Readable

Settings section colors should keep personality without becoming low-contrast
labels.

Add section-level semantic variables:

- `--settings-section-accent`: filled/accent swatch color.
- `--settings-section-accent-text`: readable text/icon color.
- `--settings-section-accent-soft`: tinted background.
- `--settings-section-accent-faint`: very soft wash.
- `--settings-section-accent-border`: border mix.

Then update Settings components that render text/icons to use
`--settings-section-accent-text`; keep `--settings-section-accent` for dots,
swatches, filled micro-shapes, and soft backgrounds.

### 4. Keep Accent Overrides Compatible

The current `[data-accent="..."]` selectors globally override `--primary`,
`--ring`, `--sidebar-primary`, and `--sidebar-ring`. The repair should preserve
that user-facing behavior. If `--primary-text` is introduced, accent override
selectors should also set it for light themes to a readable value.

For Claude Light specifically, the default `lavender` accent maps to Claude
Coral through `--ctp-lavender`. That behavior can remain, but its text role must
not be the same luminance as the fill role.

## Files and Boundaries

Likely files:

- `src/index.css`
  - add readable accent text token(s)
  - adjust Claude Light and Latte text/soft color contracts
  - add light-theme-specific Dashboard material tuning
  - clean mojibake comments only where touched if needed
- `src/components/settings/settingsSectionTheme.ts`
  - emit `--settings-section-accent-text`
  - optionally theme-condition via token rather than component logic
- `src/components/settings/SettingsCollapsibleCard.tsx`
  - switch title/icon/toggle text from accent fill to accent text
- `src/components/settings/AppearanceSettingsSection.tsx`
  - switch labels/icons/checkmark foregrounds that use section accent as text
- Dashboard section files
  - prefer token-level CSS, but update `text-primary` usages to
    bracket-token classes only where they render normal text/icons on light
    panels
- `src/test/themeStore.test.ts` or a new token-contract test
  - keep flavor acceptance tests
  - add a small contrast/token contract test if practical in Vitest

Avoid touching:

- Tauri/Rust backend
- settings store persistence keys
- i18n strings
- Dashboard data/view-model logic
- layout structure unless visual verification reveals overflow or overlap
- unrelated `text-primary` usages outside Dashboard / Settings unless they are
  clearly normal-size accent text on light `card` or `background` surfaces and
  are cheap to correct without changing behavior

## Risks and Trade-Offs

- Adding new semantic tokens is cleaner than darkening `--primary` globally,
  because darkening Claude Coral enough for text would make filled buttons need
  a different foreground and would change the brand feel.
- Per-component overrides are faster but will miss future surfaces. A token
  contract plus a few high-risk component updates is the better durable fix.
- Dashboard glass is part of the design system, so removing it would solve the
  screenshot symptom at the cost of product identity. The plan keeps it but
  reduces intensity in light themes.
- Existing accent color overrides may make some light-theme accents fail as
  text. The implementation must either compute/set readable `--primary-text`
  values for all accent selectors or avoid using accent fill as text in normal
  UI.
- The first pass intentionally does not migrate every app-wide `text-primary`
  usage. This keeps the review small and focused, with a follow-up available if
  visual QA later exposes more light-theme contrast failures.

## Visual QA Targets

Required screenshots after implementation:

- Dashboard at 1920x1080 or current desktop size with `claude-light`.
- Settings Appearance section with `claude-light`.
- Dashboard with `latte` to confirm shared light-theme repair.
- One dark theme, preferably `mocha` or `claude-dark`, to confirm no regression.
