/**
 * Central registry of user-facing keyboard shortcuts, rendered by the
 * ShortcutsSheet overlay. Keep every entry in sync with the actual behavior:
 *
 * - mod+k        → GlobalSearchDialog.tsx / CommandPalette.tsx (useHotkey)
 * - ? / mod+/    → ShortcutsSheet.tsx (useHotkey)
 * - Logs keys    → components/logs/useLogsKeyboard.ts
 * - Targets Esc  → components/layout/TargetQuickSwitcher.tsx
 */
export interface ShortcutEntry {
  id: string;
  /** Key combos in keyboardShortcuts.ts syntax; alternatives are rendered as "A or B". */
  combos: string[];
  /** i18n key describing what the shortcut does. */
  descriptionKey: string;
  /** Optional i18n key describing when the shortcut applies (scope/condition). */
  scopeKey?: string;
}

export interface ShortcutGroup {
  id: string;
  titleKey: string;
  entries: ShortcutEntry[];
}

export const SHORTCUT_GROUPS: ShortcutGroup[] = [
  {
    id: "global",
    titleKey: "shortcuts.groups.global",
    entries: [
      {
        id: "global-search",
        combos: ["mod+k"],
        descriptionKey: "shortcuts.entries.globalSearch",
      },
      {
        id: "shortcuts-sheet",
        combos: ["?", "mod+/"],
        descriptionKey: "shortcuts.entries.shortcutsSheet",
        scopeKey: "shortcuts.scopes.questionNotWhileTyping",
      },
    ],
  },
  {
    id: "central",
    titleKey: "shortcuts.groups.central",
    entries: [
      {
        id: "central-command-palette",
        combos: ["mod+k"],
        descriptionKey: "shortcuts.entries.commandPalette",
        scopeKey: "shortcuts.scopes.sharedWithGlobalSearch",
      },
    ],
  },
  {
    id: "logs",
    titleKey: "shortcuts.groups.logs",
    entries: [
      {
        id: "logs-focus-search",
        combos: ["/"],
        descriptionKey: "shortcuts.entries.logsFocusSearch",
        scopeKey: "shortcuts.scopes.notWhileTyping",
      },
      {
        id: "logs-refresh",
        combos: ["r"],
        descriptionKey: "shortcuts.entries.logsRefresh",
        scopeKey: "shortcuts.scopes.notWhileTyping",
      },
      {
        id: "logs-export",
        combos: ["e"],
        descriptionKey: "shortcuts.entries.logsExport",
        scopeKey: "shortcuts.scopes.notWhileTyping",
      },
      {
        id: "logs-move-focus",
        combos: ["arrowup", "arrowdown"],
        descriptionKey: "shortcuts.entries.logsMoveFocus",
        scopeKey: "shortcuts.scopes.notWhileTyping",
      },
    ],
  },
  {
    id: "targets",
    titleKey: "shortcuts.groups.targets",
    entries: [
      {
        id: "targets-close-menu",
        combos: ["esc"],
        descriptionKey: "shortcuts.entries.targetsCloseMenu",
        scopeKey: "shortcuts.scopes.menuOpen",
      },
    ],
  },
];
