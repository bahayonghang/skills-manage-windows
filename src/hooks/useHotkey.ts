import { useEffect } from "react";

import {
  isEditableEventTarget,
  matchesShortcut,
} from "@/lib/keyboardShortcuts";

export interface UseHotkeyOptions {
  /**
   * When false, the hotkey is ignored (and not preventDefault-ed) while an
   * input/textarea/select/contenteditable is focused. Defaults to true.
   */
  allowInEditable?: boolean;
}

/**
 * Registers a global keyboard shortcut.
 * @param key - The key combo (e.g. "mod+k"). "mod" maps to Meta on Mac, Ctrl elsewhere.
 * @param callback - Function to call when the shortcut fires.
 * @param options - Optional behavior tweaks (see UseHotkeyOptions).
 */
export function useHotkey(
  key: string,
  callback: () => void,
  options: UseHotkeyOptions = {},
) {
  const { allowInEditable = true } = options;

  useEffect(() => {
    if (typeof window === "undefined") return;

    function handler(e: KeyboardEvent) {
      if (!allowInEditable && isEditableEventTarget(e.target)) return;
      if (matchesShortcut(e, key)) {
        e.preventDefault();
        callback();
      }
    }

    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [key, callback, allowInEditable]);
}
