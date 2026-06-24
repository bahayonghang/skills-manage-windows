import { describe, expect, it } from "vitest";

import {
  formatShortcut,
  getShortcutPlatform,
  matchesShortcut,
} from "@/lib/keyboardShortcuts";

describe("keyboardShortcuts", () => {
  it("detects macOS from navigator platform", () => {
    expect(getShortcutPlatform({ platform: "MacIntel" })).toBe("mac");
    expect(getShortcutPlatform({ platform: "Win32" })).toBe("nonMac");
  });

  it("formats mod shortcuts for macOS and non-macOS", () => {
    expect(formatShortcut("mod+k", "mac")).toEqual({
      tokens: ["⌘", "K"],
      ariaLabel: "Command+K",
    });
    expect(formatShortcut("mod+k", "nonMac")).toEqual({
      tokens: ["Ctrl", "K"],
      ariaLabel: "Ctrl+K",
    });
  });

  it("matches Ctrl+K only on non-macOS", () => {
    expect(
      matchesShortcut(
        { key: "k", ctrlKey: true, metaKey: false, shiftKey: false, altKey: false },
        "mod+k",
        "nonMac",
      ),
    ).toBe(true);
    expect(
      matchesShortcut(
        { key: "k", ctrlKey: false, metaKey: true, shiftKey: false, altKey: false },
        "mod+k",
        "nonMac",
      ),
    ).toBe(false);
  });

  it("matches Command+K only on macOS", () => {
    expect(
      matchesShortcut(
        { key: "K", ctrlKey: false, metaKey: true, shiftKey: false, altKey: false },
        "mod+k",
        "mac",
      ),
    ).toBe(true);
    expect(
      matchesShortcut(
        { key: "k", ctrlKey: true, metaKey: false, shiftKey: false, altKey: false },
        "mod+k",
        "mac",
      ),
    ).toBe(false);
  });

  it("rejects unexpected modifier combinations", () => {
    expect(
      matchesShortcut(
        { key: "k", ctrlKey: true, metaKey: false, shiftKey: true, altKey: false },
        "mod+k",
        "nonMac",
      ),
    ).toBe(false);
    expect(
      matchesShortcut(
        { key: "k", ctrlKey: true, metaKey: false, shiftKey: false, altKey: true },
        "mod+k",
        "nonMac",
      ),
    ).toBe(false);
  });
});
