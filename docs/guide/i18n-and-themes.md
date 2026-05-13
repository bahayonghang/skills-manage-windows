# i18n and Themes

SkillPort ships in two languages and four Catppuccin flavors. All toggles live in the top bar and can be changed at any time without losing state.

## Language

| Locale | Source files |
|--------|--------------|
| English | `src/i18n/locales/en.json` |
| 简体中文 | `src/i18n/locales/zh.json` |

The active locale is detected on first launch via `i18next-browser-languagedetector` and persisted afterwards. All user-facing strings flow through `react-i18next`; raw English text in source files is treated as a bug.

## Themes

The Catppuccin palette is built into the app:

| Flavor | Variant |
|--------|---------|
| Latte | Light |
| Frappé | Mid-dark |
| Macchiato | Dark |
| Mocha | Deepest dark |

Switch flavors from the top bar. The selection is persisted as `data-theme` on the root HTML element so CSS variables update instantly.

## Accent colors

There are 14 accent colors derived from the Catppuccin palette. Pick one from the appearance panel; the choice is saved as `data-accent`. Accents only repaint primary actions, focus rings, and a few highlights — they do not change the underlying flavor.

## Customizing

| Goal | Where to look |
|------|---------------|
| Add a new translation key | Edit both `en.json` and `zh.json` together; PRs that update only one will be flagged. |
| Add a new accent | Update `tokens.css` and the accent picker; tests cover the contract. |
| Override the default flavor | Set the `data-theme` attribute on the root before app boot (advanced; usually unnecessary). |

## Documentation site language

This documentation site mirrors the same convention. The English root lives at `/`, the Chinese mirror lives under `/zh/`. The language switcher in the top-right of the docs nav follows the same locale routing as the desktop app.

---

Last reviewed: 2026-05-04
