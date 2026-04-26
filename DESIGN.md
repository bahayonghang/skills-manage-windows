---
name: SkillPort
description: Local-first desktop operations console for AI coding skill libraries.
colors:
  mocha-base: "#1e1e2e"
  mocha-mantle: "#181825"
  mocha-surface-0: "#313244"
  mocha-surface-1: "#45475a"
  mocha-text: "#cdd6f4"
  latte-base: "#eff1f5"
  latte-mantle: "#e6e9ef"
  latte-surface-0: "#ccd0da"
  latte-text: "#4c4f69"
  primary-lavender: "#b4befe"
  primary-latte-lavender: "#7287fd"
  primary-foreground-dark: "#181825"
  primary-foreground-latte-dark: "#11111b"
  primary-foreground-latte-light: "#fbfaff"
  destructive-red: "#f38ba8"
  latte-destructive-red: "#d20f39"
  accent-rosewater: "#dc8a78"
  accent-flamingo: "#dd7878"
  accent-pink: "#ea76cb"
  accent-mauve: "#8839ef"
  accent-maroon: "#e64553"
  accent-peach: "#fe640b"
  accent-yellow: "#df8e1d"
  accent-green: "#40a02b"
  accent-teal: "#179299"
  accent-sky: "#04a5e5"
  accent-sapphire: "#209fb5"
  accent-blue: "#1e66f5"
typography:
  display:
    fontFamily: "JetBrains Mono Variable, Geist Variable, monospace"
    fontSize: "1.25rem"
    fontWeight: 600
    lineHeight: 1.2
  headline:
    fontFamily: "JetBrains Mono Variable, Geist Variable, monospace"
    fontSize: "1.125rem"
    fontWeight: 600
    lineHeight: 1.3
  title:
    fontFamily: "JetBrains Mono Variable, Geist Variable, monospace"
    fontSize: "1rem"
    fontWeight: 600
    lineHeight: 1.4
  body:
    fontFamily: "JetBrains Mono Variable, Geist Variable, monospace"
    fontSize: "0.875rem"
    fontWeight: 400
    lineHeight: 1.65
  label:
    fontFamily: "JetBrains Mono Variable, Geist Variable, monospace"
    fontSize: "0.75rem"
    fontWeight: 500
    lineHeight: 1.4
rounded:
  sm: "6px"
  md: "8px"
  lg: "10px"
  xl: "14px"
  2xl: "18px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "12px"
  lg: "16px"
  xl: "24px"
components:
  button-primary:
    backgroundColor: "{colors.primary-lavender}"
    textColor: "{colors.primary-foreground-dark}"
    rounded: "{rounded.md}"
    padding: "6px 10px"
    height: "32px"
  button-primary-latte:
    backgroundColor: "{colors.primary-latte-lavender}"
    textColor: "{colors.primary-foreground-latte-dark}"
    rounded: "{rounded.md}"
    padding: "6px 10px"
    height: "32px"
  button-outline:
    backgroundColor: "{colors.latte-base}"
    textColor: "{colors.latte-text}"
    rounded: "{rounded.md}"
    padding: "6px 10px"
    height: "32px"
  input-default:
    backgroundColor: "{colors.latte-base}"
    textColor: "{colors.latte-text}"
    rounded: "{rounded.md}"
    padding: "4px 10px"
    height: "32px"
  card-default:
    backgroundColor: "{colors.mocha-mantle}"
    textColor: "{colors.mocha-text}"
    rounded: "{rounded.xl}"
    padding: "12px"
---

# Design System: SkillPort

## 1. Overview

**Creative North Star: "The Local Operations Console"**

SkillPort should feel like a calm control surface for local filesystem work. The interface is dense, explicit, and reviewable. Visual polish serves trust; it never hides where a skill lives, which target is active, or which action will touch disk or network.

The product uses a restrained Catppuccin console language: tinted neutral surfaces, one active accent, precise borders, and compact controls. It rejects generic SaaS dashboard polish, marketplace landing-page spectacle, decorative AI command-center effects, ornamental gradients, glass cards, vague empty states, and hidden automation.

**Key Characteristics:**

- Local-first and operational, not promotional.
- Compact enough for daily expert workflows.
- State-rich through tokens, not raw color patches.
- Familiar desktop controls with clear focus and disabled states.
- Windows-first, with local data boundaries made visible.

## 2. Colors

The palette is Catppuccin by structure, used as an operations palette: surfaces carry calm, accents mark selection and action.

### Primary

- **Default Lavender Accent**: The active action and selection color. Use it for primary buttons, current navigation, selected filters, focus rings, and progress indicators.
- **Latte Lavender Accent**: The light-theme primary accent. Its readable foreground is dark by default, with light text only for the darker Latte accents.

### Secondary

- **Semantic Red**: Destructive and error states. Keep it scoped to destructive actions, failed jobs, and invalid fields.

### Neutral

- **Mocha Console Base**: Dark application background for low-light desktop use.
- **Mocha Mantle Surface**: Sidebar, cards, popovers, and nested panels.
- **Latte Paper Base**: Light application background for daylight use.
- **Latte Mantle Surface**: Light sidebar, cards, popovers, and panels.
- **Text Ink**: Muted Catppuccin text, never pure black or pure white.

### Named Rules

**The One Accent Rule.** One chosen Catppuccin accent owns primary action, selection, focus, and progress. Do not add a second decorative accent to the same surface.

**The Contrast Before Preference Rule.** In Latte, every primary accent must meet WCAG AA text contrast before it can appear on a filled control.

**The Token Only Rule.** Use semantic tokens for UI color. Raw black, raw white, raw rgba overlays, and one-off stripe accents are forbidden.

## 3. Typography

**Display Font:** JetBrains Mono Variable with Geist Variable and monospace fallback
**Body Font:** JetBrains Mono Variable with Geist Variable and monospace fallback
**Label/Mono Font:** JetBrains Mono Variable

**Character:** The typography is technical and compact. It should read like a dependable filesystem console, not a marketing site.

### Hierarchy

- **Display** (600, 1.25rem, 1.2): Page titles and major settings headers.
- **Headline** (600, 1.125rem, 1.3): Dialog titles and large section headers.
- **Title** (600, 1rem, 1.4): Card titles and dense panel headings.
- **Body** (400, 0.875rem, 1.65): Descriptions, markdown preview, and operational guidance. Cap prose at roughly 65 to 75 characters where layout allows.
- **Label** (500, 0.75rem, 1.4): Form labels, chips, counters, and compact metadata.

### Named Rules

**The Console Legibility Rule.** Use weight and spacing for hierarchy. Do not introduce display fonts or oversized marketing type.

## 4. Elevation

Depth is mostly tonal. Borders, muted backgrounds, and subtle rings separate surfaces. Shadows are shallow and structural, used for cards, drawers, popovers, and active selection only.

### Shadow Vocabulary

- **Card Rest** (`shadow-sm`): Default card lift, enough to separate panels without visual drama.
- **Panel Lift** (`shadow-lg`): Sticky action bars and elevated panels.
- **Drawer Lift** (`shadow-2xl`): Full-height detail drawers that must sit above the app shell.

### Named Rules

**The Flat At Rest Rule.** Surfaces are flat at rest. Use borders and tonal layers first; shadows only explain stacking or interaction state.

## 5. Components

### Buttons

- **Shape:** Gently rounded rectangles (8px to 10px radius).
- **Primary:** Filled semantic primary, primary foreground, compact horizontal padding, 32px default height.
- **Hover / Focus:** Color transitions only. Focus uses the semantic ring and never removes the outline path.
- **Secondary / Ghost / Outline:** Neutral surfaces with border or muted hover states. They must not compete with primary action.

### Chips

- **Style:** Rounded pills with border and muted backgrounds.
- **State:** Selected chips use a light primary tint, primary border, and font weight shift. Unselected chips stay neutral.

### Cards / Containers

- **Corner Style:** Rounded panels (14px to 18px radius) for main cards, 8px to 10px for controls.
- **Background:** Card and muted tokens, never raw white or raw black.
- **Shadow Strategy:** Subtle at rest, stronger only for drawers or sticky panels.
- **Border:** Semantic border always present on operational panels.
- **Internal Padding:** 12px to 24px depending on density.

### Inputs / Fields

- **Style:** 32px height, 8px radius, semantic input border, transparent or token-backed background.
- **Focus:** Border shifts to ring color with a visible ring.
- **Error / Disabled:** Destructive ring for invalid fields, muted disabled background, no hidden focus loss.

### Navigation

- **Style:** Sidebar navigation uses semantic sidebar tokens. Active routes use filled sidebar primary, count badges inherit the active foreground, and hover uses sidebar accent.
- **Mobile treatment:** Compact controls expand touch target size on narrow screens before returning to dense desktop size at medium widths.

### Progress Bars

- **Style:** Muted track with primary fill.
- **Motion:** Fill changes use transform scaling from the left edge, not width animation.

## 6. Do's and Don'ts

### Do:

- **Do** keep filesystem paths, targets, install destinations, and network side effects visible before action.
- **Do** use Catppuccin semantic tokens for backgrounds, foregrounds, borders, rings, and actions.
- **Do** keep expert density, but preserve touchable controls on narrow windows.
- **Do** use full borders, background tint, icon state, or weight changes for selected list items.
- **Do** keep animations short, stateful, and transform or opacity based.
- **Do** use i18n for every user-visible string.

### Don't:

- **Don't** look like a generic SaaS dashboard, a marketplace landing page, or a decorative AI-themed command center.
- **Don't** use hero metrics, ornamental gradients, glass cards, vague empty states, or hidden background automation for reviewable operations.
- **Don't** use side-stripe borders as card, list, callout, or alert accents.
- **Don't** use raw black, raw white, or raw rgba overlays in product UI.
- **Don't** animate width, height, max-width, left, right, top, or bottom.
- **Don't** rely on color alone for status. Pair it with labels, icons, or structure.
