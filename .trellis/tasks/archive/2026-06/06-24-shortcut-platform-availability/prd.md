# 修复跨平台快捷键显示与可用性

## Goal

Make keyboard shortcut hints match the user's actual operating system and ensure the advertised shortcut is genuinely usable in the Tauri app, with focused tests that prevent macOS-only symbols from leaking into Windows UI again.

The user-reported symptom is visible in the Central Skills search field: on Windows, the inline shortcut chip shows `⌘K`, which is a macOS command-key hint. The fork's project goal prioritizes Windows packaging and Windows usability, so this is not just a cosmetic issue. The displayed hint must match the real shortcut users can press.

Root-cause hypothesis from source inspection:

- `src/components/central/CentralSearchBar.tsx` hard-codes `⌘K` in the command palette open button.
- `src/components/layout/TopBar.tsx` has a separate platform check and renders `{isMac ? "⌘" : "Ctrl"}K`.
- `src/components/central/CommandPalette.tsx` independently listens for `(metaKey || ctrlKey) + K`.
- `src/components/layout/GlobalSearchDialog.tsx` uses `useHotkey("mod+k")`, while `src/hooks/useHotkey.ts` contains its own `mod` mapping logic.
- These separate display and event paths mean a symbol can look platform-correct in one place but be wrong or untested in another.

## Requirements

- Central Skills search shortcut hint must render Windows/Linux-friendly text on non-macOS systems, expected as `Ctrl K` or `Ctrl+K` according to the final shared component design.
- macOS may still render a command-key style hint, but only when the runtime platform detection is macOS.
- Shortcut display must come from a shared helper or component so Central Search and TopBar do not drift.
- The displayed shortcut and active keyboard binding must share one contract for `mod+k`.
- The actual shortcut must open/toggle the intended dialog on Windows via `Ctrl+K`.
- The actual shortcut must open/toggle the intended dialog on macOS via `Meta/Command+K`.
- Keyboard handlers should avoid false positives from unrelated modifiers where practical, especially `Alt+Ctrl+K` or `Shift+Ctrl+K` unless deliberately supported.
- User-visible shortcut labels and accessible names must remain testable and not require backend IPC.
- Do not broaden scope into a full keyboard shortcut settings system.
- Do not change unrelated Central Skills search behavior, filtering, command palette actions, or global search result logic.

## Acceptance Criteria

- [x] On a mocked Windows/non-macOS environment, the Central Skills search bar no longer renders `⌘K`; it renders the same non-mac shortcut label as the shared shortcut contract.
- [x] On a mocked macOS environment, the same shortcut display renders the macOS command-key label.
- [x] TopBar and CentralSearchBar use the same shortcut display helper/component for `mod+k`.
- [x] A focused test proves `Ctrl+K` opens/toggles the relevant command/search dialog on non-macOS.
- [x] A focused test proves `Meta/Command+K` opens/toggles the relevant command/search dialog on macOS.
- [x] A focused test or helper test proves the shortcut matching logic rejects unintended modifier combinations unless explicitly accepted by the design.
- [ ] `pnpm typecheck`, `pnpm lint`, and the relevant Vitest tests pass before implementation is considered complete.
- [ ] Final full gate `just ci` passes before task archive/finish.

## Evidence

- Screenshot from 2026-06-24 shows `⌘K` in the Central Skills search box on a Windows app window.
- `src/components/central/CentralSearchBar.tsx` currently hard-codes `⌘K`.
- `src/components/layout/TopBar.tsx` currently computes `isMac` locally and renders `{isMac ? "⌘" : "Ctrl"}K`.
- `src/hooks/useHotkey.ts` currently maps `mod` to Meta on macOS and Ctrl elsewhere.
- `src/components/central/CommandPalette.tsx` currently bypasses `useHotkey` and directly accepts either Meta or Ctrl.

## Open Questions

- Should the non-macOS visual format be compact `Ctrl K`, `Ctrl+K`, or two separate keycaps `Ctrl` and `K`? Recommendation: use two keycap tokens where the layout has space, but expose a compact accessible label like `Ctrl+K`.

## Implementation Notes

- Implemented shared shortcut contract in `src/lib/keyboardShortcuts.ts`.
- Implemented shared display in `src/components/ui/shortcut-hint.tsx`.
- Routed `useHotkey("mod+k")` and Central command palette through the shared matcher.
- Replaced the Central search hard-coded `⌘K` and TopBar local platform check with `ShortcutHint`.
- Added focused tests:
  - `src/test/keyboardShortcuts.test.ts`
  - `src/test/useHotkey.test.tsx`
  - `src/test/CentralSearchBar.test.tsx`
  - `src/test/TopBar.test.tsx`
- Verified:
  - `pnpm typecheck` passed.
  - `pnpm lint` passed.
  - `pnpm sizecheck` passed.
  - scoped `git diff --check` for touched files passed.
- Blocked verification:
  - `pnpm test ...` cannot start Vitest in the current sandbox because Vite config loading fails while loading `@tailwindcss/oxide-win32-x64-msvc` and `spawn EPERM`.
  - `just ci` cannot start its parallel child processes in the current sandbox due to `spawn EPERM`.
  - global `git diff --check` still reports a pre-existing `TODO.md` trailing-space change outside this task.
