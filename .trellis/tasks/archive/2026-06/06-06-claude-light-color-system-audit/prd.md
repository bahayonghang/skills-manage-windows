# Claude Light color system audit

## Goal

Create an evidence-backed optimization plan for the app color system, with the
immediate symptom that the Dashboard looks visually wrong under Claude Light.
The plan must preserve SkillPort's theme identity model while making light
themes readable, calmer, and consistent across Dashboard and Settings surfaces.

## Requirements

- Audit all six supported theme flavors: Mocha, Macchiato, Frappé, Latte,
  Claude Light, and Claude Dark.
- Treat Claude Light as a first-class product theme, not a decorative skin:
  Claude Coral remains the signature accent, but the UI must not become washed
  out, pink-foggy, or low-contrast.
- Preserve the existing product design contract:
  - dense developer-tool UI
  - theme identity through semantic tokens and accent usage
  - restrained accent use for actions, focus, current selection, and state
  - Dashboard glass only where it improves the control-room feel
- Fix the shared light-theme problem where `primary` is used as normal text on
  light `background` / `card` surfaces and fails or nearly fails WCAG AA.
- Reduce Dashboard light-theme visual haze caused by combining glass surfaces,
  large radial accent glows, `primary` overlays, and light Claude surfaces.
- Make Settings section accents readable in Claude Light and Latte when section
  colors are used for titles, icons, labels, and selected controls.
- Keep the implementation surgical: prefer token-level and small reusable
  style-rule changes over per-component color overrides.
- Implementation scope is confirmed to focus on the shared Claude Light +
  Latte light-theme token contract, Dashboard, Settings Appearance, and only
  obvious high-risk `text-primary` usage points. A full app-wide `text-primary`
  migration is out of scope for the first implementation pass.
- Do not change backend behavior, data models, i18n copy, platform logic, or
  theme storage semantics unless directly required by the color-system fix.

## Confirmed Facts

- `PRODUCT.md` defines SkillPort as a product UI for heavy AI-skill management
  users; the interface should be dense, keyboard-friendly, and professional.
- `DESIGN.md` defines "theme is identity": Catppuccin 4 flavors plus Claude
  Light/Dark, with Claude Coral (`#cc785c`) as the Claude signature accent.
- `DESIGN.md` also says accent should stay rare and appear mainly in primary
  actions, current selection, focus, and state indicators.
- Theme flavors are implemented in `src/index.css` with `[data-theme="..."]`
  blocks and accent overrides via `[data-accent="..."]`.
- `src/stores/themeStore.ts` persists `catppuccin-flavor` and
  `catppuccin-accent` and already accepts `claude-light` / `claude-dark`.
- Dashboard uses `bg-orbit`, `surface-glass`, `readiness-card`,
  `readiness-score-plaque`, and many `text-primary` / `bg-primary/10` usages.
- Settings uses `settingsSectionTheme.ts` to map sections to fixed
  `--ctp-*` accent variables, then uses those values as text, icon, border, and
  soft background colors.
- Current measured contrast:
  - Claude Light `primary` (`#cc785c`) text on `card` (`#efe9de`) is 2.71:1.
  - Claude Light `primary` text on `background` (`#faf9f5`) is 3.11:1.
  - Claude Light `muted-foreground` on `card` is about 4.48:1, just below AA.
  - Latte `primary` text on `card` is 2.62:1 and on `background` is 2.81:1.
  - Mocha, Macchiato, Frappé, and Claude Dark pass the sampled core pairs.
- Screenshot evidence shows the Claude Light Dashboard has a pale pink wash
  over major panels, low hierarchy in some cards, and muted content close to
  the background.
- Screenshot evidence shows the Appearance flavor picker is compact and
  functional, but its section accent treatment risks being too low-contrast
  under Claude Light.

## Out of Scope

- Replacing the six-theme model.
- Removing user-selectable Catppuccin accents.
- Redesigning the Dashboard layout or Settings information architecture.
- Changing fonts, font scale controls, theme persistence keys, or language
  strings.
- Full visual redesign of non-light themes unless a regression is directly
  caused by the shared token fix.
- Exhaustively migrating every `text-primary` usage across the app in the first
  pass.

## Acceptance Criteria

- [x] A documented implementation plan exists for fixing Claude Light and the
      shared light-theme contrast contract.
- [x] The plan identifies exact likely files and token/component boundaries.
- [x] Claude Light and Latte no longer use the same token for filled primary
      controls and readable accent text when that causes WCAG AA failures.
- [x] Dashboard light-theme glass/radial effects are scoped or toned so the
      first screen reads as a crisp product UI rather than a pink haze.
- [x] Settings section accent text/icons have a readable foreground strategy in
      Claude Light and Latte.
- [x] Contrast checks for the core semantic pairs pass WCAG AA for normal text
      where the token is used as text.
- [x] Targeted tests cover theme initialization/accepted flavors and any new
      light-theme token contract utilities.
- [x] Visual verification includes Claude Light Dashboard and Settings
      Appearance screenshots after implementation.
- [x] Final implementation gate includes at least `pnpm typecheck`,
      `pnpm lint`, targeted Vitest tests, and `just ci`.

## Notes

- This is a complex planning task. `design.md` and `implement.md` are required
  before implementation starts.
- Implementation completed on 2026-06-06. Verification passed with
  `pnpm typecheck`, `pnpm lint`,
  `pnpm test -- src/test/themeContrast.test.ts src/test/themeStore.test.ts`,
  `just ci`, and `git diff --check`.
- Visual evidence:
  - `tmp/color-qa/claude-light-dashboard-updated.png`
  - `tmp/color-qa/claude-light-settings-appearance-updated.png`
  - `tmp/color-qa/latte-dashboard.png`
  - `tmp/color-qa/mocha-dashboard.png`
