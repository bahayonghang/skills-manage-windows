# Observability Console 实施计划

## Steps

1. Add pure view models for diagnostic envelope, operation ID/source grouping, next-action i18n and legacy fallback.
2. Extend operation/runtime stores with exact ID filter and reversible cross-mode navigation state.
3. Rename/rebuild detail drawer as centered Dialog with structured hierarchy and collapsed JSON.
4. Add started/interrupted/source visual semantics and accessible copy/jump actions.
5. Harden loading/empty/error/invalid/long/narrow states and en/zh parity.
6. Run component/page/store tests, then one bounded Windows visual pass and at most one confirmation pass.

## Validation

```powershell
pnpm exec vitest run src/test/components/logs src/test/pages/OperationLogsView.test.tsx src/test/stores/operationLogStore.test.ts src/test/stores/runtimeLogStore.test.ts src/test/contracts/i18nLocales.test.ts
pnpm typecheck
pnpm lint
pnpm sizecheck
git diff --check
```

Manual: Windows Tauri at 100/125/150%, narrow window, keyboard-only, long en/zh and real Operation/Runtime rows. Report
unrun native checks as `UNVERIFIED`.

## Rollback

Keep store DTO compatibility; centered Dialog and correlation navigation can be reverted independently. Do not remove or
rewrite user logs. If grouping fails, show separate source rows rather than hiding evidence.
