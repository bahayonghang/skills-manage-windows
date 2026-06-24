# Implementation Plan

## Success Criteria

The app no longer displays a macOS shortcut glyph on Windows, and tests prove the displayed `mod+k` hint matches the keyboard event that actually opens the relevant command/search UI.

## Checklist

1. Inspect existing tests around `CentralSearchBar`, `TopBar`, `GlobalSearchDialog`, `CommandPalette`, and `useHotkey`.
   - Verify: identify the narrowest existing test files to extend.

2. Add a shared shortcut contract.
   - Likely files: `src/lib/keyboardShortcuts.ts` plus `src/test/keyboardShortcuts.test.ts`.
   - Include platform detection, display formatting, accessible label text, and event matching for `mod+k`.
   - Verify: pure helper tests cover macOS and non-macOS.

3. Route `useHotkey("mod+k")` through the shared matcher.
   - File: `src/hooks/useHotkey.ts`.
   - Verify: hook or consumer test proves `Ctrl+K` works on non-macOS and `Meta+K` works on macOS.

4. Replace hard-coded and local shortcut display.
   - Files: `src/components/central/CentralSearchBar.tsx`, `src/components/layout/TopBar.tsx`.
   - Prefer a tiny shared `ShortcutHint` component only if it keeps markup cleaner than duplicated token rendering.
   - Verify: component tests assert CentralSearchBar and TopBar render the same non-mac label under Windows mocks.

5. Align `CommandPalette.tsx` with the shared handler or shared matcher.
   - Avoid keeping a second permissive `(metaKey || ctrlKey)` implementation.
   - Verify: command palette keyboard behavior is covered directly or through the hook.

6. Update i18n only if accessible labels or visible shortcut-related text changes.
   - Files: `src/i18n/locales/zh.json`, `src/i18n/locales/en.json`.
   - Verify: no hard-coded user-visible strings are introduced.

7. Run validation.
   - Narrow: `pnpm test -- --run <relevant test files>` or project-equivalent Vitest invocation.
   - Frontend: `pnpm typecheck`, `pnpm lint`, `pnpm test`.
   - Final: `just ci`.

## Risk Areas

- `navigator.platform` can be awkward to override in jsdom; use helper-level injection where possible instead of brittle global mutation in every test.
- Current `useHotkey` logic appears internally inconsistent for `mod+k` because it sets both `needsMeta` and `needsCtrl`. Do not preserve that shape blindly if tests show it blocks expected macOS behavior.
- `CommandPalette.tsx` and `GlobalSearchDialog.tsx` may both react to `mod+k` depending on mounted state. Confirm which dialog the Central Search button owns before changing event scope.
- Keep layout stable: the shortcut chip is inside an input overlay with `pr-24`; longer `Ctrl+K` text must not overlap clear button or search text.

## Rollback

All changes should be frontend-only. If a shared shortcut helper causes broader behavior changes, revert consumers one-by-one back to local handling while keeping the CentralSearchBar display fix and tests as the minimal fallback.
