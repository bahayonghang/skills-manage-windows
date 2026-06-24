# Shortcut Platform Availability Design

## Problem

Shortcut display and shortcut handling are currently separate local decisions. Central Search hard-codes the macOS glyph, TopBar computes platform display locally, and command dialogs register keyboard behavior through two different paths. That lets the UI advertise a shortcut that does not match the user's keyboard.

## Proposed Boundary

Create a small frontend-only shortcut contract under `src/lib/` or `src/hooks/`, then consume it from both display and keyboard handling paths.

Recommended shape:

- `getShortcutPlatform()` or equivalent browser-safe helper that returns `mac` or `nonMac`.
- `formatShortcut("mod+k", platform)` or equivalent that returns:
  - tokens for visual rendering, for example `["Ctrl", "K"]` or `["⌘", "K"]`
  - an accessible label, for example `Ctrl+K` or `Command+K`
- `matchesShortcut(event, "mod+k", platform)` for keyboard tests and `useHotkey`.
- A small `ShortcutKeycap` / `ShortcutHint` component only if it removes duplication between CentralSearchBar and TopBar. If only two call sites need it, keep it compact.

Keep platform detection browser-safe:

- Do not read `navigator` at module load time.
- Prefer an injectable or parameterized platform helper for tests.
- Preserve SSR/jsdom safety with `typeof navigator !== "undefined"` checks.

## Event Semantics

For `mod+k`:

- macOS: require `event.metaKey`, key `k`, and no unexpected `ctrlKey`, `altKey`, or `shiftKey`.
- non-macOS: require `event.ctrlKey`, key `k`, and no unexpected `metaKey`, `altKey`, or `shiftKey`.

This is stricter than the current `CommandPalette.tsx` direct handler, which accepts either modifier on every OS. If implementation discovers existing users rely on broader matching, record the trade-off before preserving it.

## UI Semantics

CentralSearchBar and TopBar should render the same contract for `mod+k`.

Visual options:

- Preferred: separate stable keycap tokens, such as `Ctrl` `K`, with compact spacing. This avoids `CtrlK` looking like a word and avoids macOS-only symbols on Windows.
- Acceptable: one compact text label such as `Ctrl+K` if it fits better in the existing CentralSearchBar chip.

For accessibility, expose a deterministic accessible label/title that includes the normalized shortcut, such as `Open command palette (Ctrl+K)`. Add i18n keys if visible or assistive text changes.

## Tests

Use Vitest and React Testing Library:

- Pure helper tests for platform formatting and event matching.
- Component tests for CentralSearchBar display under mocked platform values.
- Component or hook tests for `useHotkey("mod+k")` on macOS and non-macOS.
- Integration-level test only if existing Central Skills shell support makes it cheap; otherwise focused component/hook tests are enough.

## Compatibility

This is frontend-only. No Rust IPC, database migration, or packaging contract should change.

Do not introduce configurable shortcuts in this task. The goal is to make the existing advertised shortcut truthful and platform-aware.
