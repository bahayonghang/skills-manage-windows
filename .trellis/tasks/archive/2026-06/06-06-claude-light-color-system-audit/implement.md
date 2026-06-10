# Claude Light Color System Implementation Plan

## Success Criteria

- Claude Light Dashboard no longer has the washed-out pink haze shown in the
  provided screenshot.
- Normal-size accent text/icons meet WCAG AA on `background` and `card` in
  Claude Light and Latte.
- Filled primary buttons remain visually branded and readable.
- Dark themes keep their current contrast and identity.
- Changes are mostly token-level and reusable, with only targeted component
  updates where components currently use fill accents as text.

## Checklist

1. Add readable accent text semantics.
   - Update `src/index.css` theme blocks with a readable token such as
     `--primary-text` or `--accent-text`.
   - For Claude Light, keep `--primary: #cc785c` and use a darker text coral
     around `#a35038`.
   - For Latte, use a darker readable accent text while preserving the filled
     Catppuccin primary.
   - Extend `[data-accent="..."]` behavior so selected accents keep readable
     text roles, especially in light themes.
   - Verify: run the contrast script or a checked-in test against core pairs.

2. Repair Settings section accent semantics.
   - Update `settingsSectionTheme.ts` to output
     `--settings-section-accent-text`.
   - Use `--settings-section-accent-text` for text/icons in
     `SettingsCollapsibleCard.tsx` and `AppearanceSettingsSection.tsx`.
   - Keep `--settings-section-accent` for dots, swatches, filled tiny badges,
     and soft-background generation.
   - Verify: Appearance section in Claude Light still shows the active flavor
     clearly, and labels do not wash out.

3. Tune Dashboard material for light themes.
   - Add `[data-theme="claude-light"]` and `[data-theme="latte"]` overrides for
     `.bg-orbit`, `.surface-glass`, `.surface-glass-strong`,
     `.readiness-card::before`, and `.readiness-score-plaque` as needed.
   - Reduce radial primary opacity and blur-heavy haze in light themes.
   - Preserve panel separation with border/ring and surface contrast.
   - Prefer CSS token overrides over changing layout/components.
   - Verify: Dashboard hero, readiness panel, metric strip, and lower panels
     have clear hierarchy on Claude Light.

4. Replace high-risk accent-as-text usages.
   - Search for `text-primary`, `text-primary/`, and
     `text-[color:var(--settings-section-accent)]`.
   - Update only normal text/icon/link usages that sit on light `card` or
     `background` surfaces to use the readable accent text token.
   - Prioritize Dashboard and Settings Appearance. Do not perform a mechanical
     app-wide migration in this first pass.
   - Leave filled buttons, focus rings, active filled controls, progress fills,
     and state indicators on the fill token.
   - Verify: no broad restyling of unrelated pages.

5. Add targeted tests.
   - Keep `themeStore` flavor persistence tests intact.
   - Add a lightweight semantic contrast contract test if feasible, using the
     actual token values or a small exported map if one exists.
   - At minimum, add tests for any helper introduced to derive readable accent
     text values.
   - Verify: targeted Vitest tests pass.

6. Run visual verification.
   - Start the dev server.
   - Open the app and set `catppuccin-flavor=claude-light` and default accent.
   - Capture Dashboard and Settings Appearance screenshots.
   - Repeat Dashboard for Latte and one dark theme.
   - Check for text overlap, washed surfaces, and unreadable accent labels.

7. Run gates.
   - `pnpm typecheck`
   - `pnpm lint`
   - targeted Vitest tests, likely `pnpm test -- src/test/themeStore.test.ts`
     plus any new test file
   - `just ci`

## Suggested Contrast Contract

Core token pairs to keep above 4.5:1 when used as normal text:

- `foreground` on `background`
- `card-foreground` on `card`
- `muted-foreground` on `background`
- `muted-foreground` on `card`
- readable accent text token on `background`
- readable accent text token on `card`
- `primary-foreground` on `primary`
- `sidebar-foreground` on `sidebar`
- `sidebar-primary-foreground` on `sidebar-primary`

If a token is intended only for large display text or non-text decoration, do
not use it in normal-size text classes.

## Rollback Points

- If the new text token causes too many component edits, keep the token but
  scope the first implementation to Dashboard and Settings, then follow up with
  a broader accent-text migration.
- If accent override selectors become too large, first repair default Claude
  Light and Latte, then add a separate audited pass for all 14 light-theme
  accent overrides.
- If visual QA shows dark themes changed, revert dark-theme token edits and
  keep light-theme overrides isolated under `[data-theme="latte"]` and
  `[data-theme="claude-light"]`.

## Implementation Approval Gate

Do not start implementation until the user approves this plan or explicitly
asks to begin implementation.
