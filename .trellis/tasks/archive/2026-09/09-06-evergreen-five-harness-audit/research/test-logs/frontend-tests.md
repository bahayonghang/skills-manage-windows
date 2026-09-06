# Frontend/local Node checks — 2026-09-06

The canonical `pnpm` wrapper was blocked by the local package-manager bootstrap. These commands use only binaries already present in `node_modules`; they provide logic/test evidence but do not make `node scripts/check/run-ci.mjs` a pass.

## Vitest

- Command: `rtk node node_modules/vitest/vitest.mjs run`
- Exit code: `0`
- Result: 178 files passed; 2057 tests passed, 1 skipped (2058 total).

```text
Test Files  178 passed (178)
Tests       2057 passed | 1 skipped (2058)
Duration    44.37s
```

## Static/generated checks

| Check | Direct local command | Exit | Evidence |
| --- | --- | ---: | --- |
| TypeScript | `rtk node node_modules/typescript/bin/tsc --noEmit` | 0 | no diagnostics |
| ESLint | `rtk node node_modules/eslint/bin/eslint.js 'src/**/*.{ts,tsx}' --cache --cache-location node_modules/.cache/eslint/ --report-unused-disable-directives --max-warnings 0` | 0 | no diagnostics |
| Version drift | `rtk node scripts/check/sync-version.mjs --check` | 0 | version already synced to 1.0.2 |
| IPC docs drift | `rtk node scripts/docs/build-ipc-dict.mjs --check` | 0 | generated IPC Markdown up to date |
| Schema docs drift | `rtk node scripts/docs/build-schema-table.mjs --check` | 0 | generated data model up to date |
| Capability drift | `rtk node scripts/check/check-capability-drift.mjs` | 0 | 8 permissions, 4 frontend plugin imports |
| Size budget | `rtk node scripts/check/check-size-budget.mjs` | 0 | 732 production files, max 800 lines |
| Rust entrypoint contract | `rtk node scripts/check/check-rust-entrypoints.mjs` | 0 | default-run and three expected bins present |

## Builds

| Build | Direct local command | Exit | Evidence |
| --- | --- | ---: | --- |
| Renderer production | `rtk node node_modules/vite/bin/vite.js build` | 0 | 2841 modules transformed; built in 6.19 s |
| VitePress site | `rtk node node_modules/vitepress/bin/vitepress.js build docs` | 0 | client/server bundles and pages rendered; 7.79 s |

The one Vitest skip is `src/test/pages/CentralSkillsView.repositories-and-installs.test.tsx:541`. Its own comment says the old quick-filter entrypoints were removed and the test still needs rewriting for `central-toolbar-view-installed-*` plus BulkActionBar. Existing tests assert menu/bulk controls are present and cover batch-install outcomes, but no enabled test executes installed-filter selection through the current toolbar and then selects/installs the filtered results. This is a real interaction-coverage gap, not a runtime failure.
