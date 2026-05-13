# Frontend

The frontend is a single Vite-bundled React 19 app served inside Tauri's webview. State stays in Zustand stores and IPC calls funnel through them.

## Routing

`src/App.tsx` mounts a single `<AppShell />` route with lazy-loaded pages:

| Path | Page | Layout |
| --- | --- | --- |
| `/dashboard` | `DashboardView` | Operations summary |
| `/central` | `CentralSkillsView` | Two-column skill cards |
| `/platform/:agentId` | `PlatformView` | Two-column skill cards |
| `/skill/:skillId` | `SkillDetailPage` | Markdown + sidebar |
| `/collections` | `CollectionsListView` | Selected card + skill list |
| `/collection/:id` | `CollectionView` | Detail variant |
| `/marketplace` | `MarketplaceView` | Three tabs |
| `/discover` | `DiscoverView` | Project list + skill detail |
| `/discover/:projectPath` | `DiscoverView` | Same view, filtered |
| `/obsidian` / `/obsidian/:vaultId` | `ObsidianVaultView` | Vault list + skills |
| `/logs` | `OperationLogsView` | Filterable log table |
| `/settings` | `SettingsView` | Cards by section |

Lazy imports keep first-paint cost low — the dashboard route does not pull in marketplace HTTP code.

## Stores

Zustand stores in `src/stores/` are the only place that calls `invoke()`. Each store is a single domain so reducer logic stays explicit.

```text
┌──────────────────────────────┬─────────────────────────────────┐
│ Store                        │ Owns                            │
├──────────────────────────────┼─────────────────────────────────┤
│ skillStore                   │ Per-platform skill lists        │
│ centralSkillsStore (split)   │ list + install + metadata +     │
│                              │ update slices                   │
│ skillDetailStore             │ Markdown + file tree + state    │
│ platformStore                │ Agent registry + visibility     │
│ collectionStore              │ Collections + batch install     │
│ marketplaceStore (split)     │ Registries + GitHub import      │
│ discoverStore                │ Project scan roots + results    │
│ obsidianStore                │ Vault list + vault skills       │
│ targetStore                  │ Active target + SSH targets     │
│ operationLogStore            │ Log paging + filters            │
│ settingsStore                │ Key/value settings + scan dirs  │
│ themeStore                   │ Catppuccin variant + accent     │
└──────────────────────────────┴─────────────────────────────────┘
```

The split stores (`centralSkillsStore.*`, `marketplaceStore.*`) decompose larger surfaces into slices so each file stays under the 800-line ceiling defined by the project sizecheck.

## Components

`src/components/` is grouped by domain, not by file type:

```text
components/
├── layout/          AppShell, Sidebar, breadcrumbs
├── skill/           UnifiedSkillCard, detail panes, file tree
├── central/         install/manage drawers, dialogs
├── collections/     collection cards and dialogs
├── platform/        PlatformIcon (LobeHub + monogram fallback)
├── marketplace/     tabs, search, GitHub import drawer
├── settings/        section cards
└── ui/              shadcn primitives wrapped to the design system
```

`UnifiedSkillCard` is the single skill-card surface across central / platform / discover / marketplace / collection contexts; new pages must use it via props rather than rebuilding inline cards.

## Theming

Catppuccin Mocha / Frappé / Latte plus 14 accent colors are toggled by the `data-theme` and `data-accent` attributes on `<html>`. The palette is derived from `tailwind.config.ts` tokens; component CSS uses tokens, not literal hex values.

## i18n

`src/i18n/locales/en.json` and `zh.json` are the single source for user-visible strings. Components read via `useTranslation()`; tests assert on translation keys rather than rendered strings to avoid locale-flipping flakes.

## IPC Boundary Rule

> Components never call `invoke()` directly.

This rule keeps the test surface small: `vitest` mocks `window.__TAURI_INTERNALS__.invoke` once and every store cooperates. Look at `src/stores/*.test.ts` for the pattern.

Last reviewed: 2026-05-04
