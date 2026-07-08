# Implementation Plan

1. Add a focused decision helper for building a cleanup-only `removeDeletedPlatformCopies` payload from the current inventory.
2. Add a confirmed one-click cleanup handler and footer button to `UpdateCenterDialog`, enabled only when leftover paths exist and no apply/refresh/force action is running.
3. Add English and Chinese i18n strings for the button, confirmation, and success/failure toasts.
4. Add tests proving the helper selects all leftover paths while excluding other decisions.
5. Run focused tests, then `just ci`.

## Validation

- `pnpm vitest run src/test/updateCenterDecisionAggregation.test.ts`
- `pnpm vitest run src/test/i18nLocales.test.ts`
- `just ci`
