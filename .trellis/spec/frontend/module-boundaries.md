# Frontend Module Boundaries

## 1. Scope / Trigger

Apply this contract when moving shared frontend types/functions, deleting unused
UI modules, changing `@/lib/ipc` type re-exports, adding production
`@/types` barrel imports, or putting browser-fixture copy in Settings /
Marketplace.

## 2. Signatures

```ts
export type UpdateCheckMode = "regular" | "sync";
export const CENTRAL_UPDATE_CHECK_MODE_SETTING_KEY = "central_update_check_mode_v1";
export const DEFAULT_UPDATE_CHECK_MODE: UpdateCheckMode;
export function normalizeUpdateCheckMode(value: unknown): UpdateCheckMode;

import type { UnlistenFn } from "@/lib/ipc";

t("settings.aiTestBrowserUnavailable");
t("marketplace.previewUnavailableTitle");
t("marketplace.previewUnavailableDetail");
```

Canonical owner for the update-check symbols is `src/lib/updateCheckMode.ts`.
Page modules may consume them; they must not own the type, setting key, default,
or normalize function.

## 3. Contracts

- Dependency direction is `neutral lib -> store/adapter -> page/component`.
  `src/lib/**` and `src/stores/**` must not import `src/pages/**` for
  update-center types or helpers.
- `src/lib/updateCheckMode.ts` must not import `src/pages/**` or `src/stores/**`.
  `src/pages/centralUpdateCheckMode.ts` may keep view-specific scope builders.
- Production `@tauri-apps/api/event` imports match exactly `src/lib/ipc/invoke.ts`.
  Consumers take `UnlistenFn` from `@/lib/ipc`. See `ipc-adapter.md`.
- Deleted unreachable modules are not restored as deprecated wrappers:
  `CollectionView.tsx`, `SkillPreviewDialog.tsx`,
  `DuplicatePlatformSkillsDialog.tsx`, `SkillDetailPanelShell.tsx`.
  Live `/collections` remains `CollectionsListView`.
- Browser-fixture user-visible copy goes through `src/i18n/locales/{en,zh}.json`.
  Store/lib may keep a stable status code; the renderer calls `t(...)`.
- Root `@/types` barrel production importers are scanned from `src/**/*.{ts,tsx}`
  excluding `src/test/**` and `src/types/**`. Count after sort+dedupe. The
  no-growth baseline is the Step 0 remasurement (199), never the audit's 193.
  New production files in an unrelated domain must not add a root-barrel import
  if a narrow entry exists.

## 4. Validation & Error Matrix

| Condition | Required result |
| --- | --- |
| lib/store imports `@/pages/centralUpdateCheckMode` | `frontendArchitectureContract` fails |
| `UpdateCheckMode` redefined in page/store | contract fails; setting key/default drift |
| production file other than `invoke.ts` imports `@tauri-apps/api/event` | contract fails |
| deleted dead module reappears or is re-imported | contract fails |
| finding hardcoded EN/ZH fixture sentences return in production TS | contract fails |
| `@/types` production importer count > 199 | contract fails; do not raise the baseline |
| locale key added to only one of `en.json` / `zh.json` | `i18nLocales.test.ts` fails |

## 5. Good / Base / Bad Cases

- Good: `updateCenterStore` imports `UpdateCheckMode` from `@/lib/updateCheckMode`.
- Base: an untouched domain still imports `@/types`; this task does not migrate it.
- Bad: lib imports a page module for a type-only cycle, or restore
  `CollectionView` behind a wrapper because tests used to mount it.

## 6. Tests Required

- `pnpm exec vitest run src/test/contracts/frontendArchitectureContract.test.ts`
  - owner file exists and is page/store-free
  - zero update-center lib/store → pages edges
  - event allowlist exact match
  - dead production + isolated test paths absent
  - barrel importers `<= 199`
  - finding fixture sentences absent from production TS
- `pnpm exec vitest run src/test/contracts/i18nLocales.test.ts` for key parity
- Keep `CollectionsListView`, Marketplace drawer, and Skill detail drawer tests;
  do not resurrect `CollectionView.test.tsx` or `SkillPreviewDialog.test.tsx`

## 7. Wrong vs Correct

```ts
// Wrong: page owns the shared mode; lib/store import the page.
import { type UpdateCheckMode, normalizeUpdateCheckMode } from "@/pages/centralUpdateCheckMode";
import type { UnlistenFn } from "@tauri-apps/api/event";
throw new Error("AI connection is unavailable in the browser fixture.");

// Correct
import {
  type UpdateCheckMode,
  normalizeUpdateCheckMode,
} from "@/lib/updateCheckMode";
import type { UnlistenFn } from "@/lib/ipc";
t("settings.aiTestBrowserUnavailable");
```
