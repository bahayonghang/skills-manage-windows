import { useEffect } from "react";

import { matchesShortcut } from "@/lib/keyboardShortcuts";

/**
 * Registers a global keyboard shortcut.
 * @param key - The key combo (e.g. "mod+k"). "mod" maps to Meta on Mac, Ctrl elsewhere.
 * @param callback - Function to call when the shortcut fires.
 */
export function useHotkey(key: string, callback: () => void) {
  useEffect(() => {
    if (typeof window === "undefined") return;

    function handler(e: KeyboardEvent) {
      if (matchesShortcut(e, key)) {
        e.preventDefault();
        callback();
      }
    }

    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [key, callback]);
}
