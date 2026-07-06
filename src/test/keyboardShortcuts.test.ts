import { describe, expect, it } from "vitest";

import {
  formatShortcut,
  getShortcutPlatform,
  isEditableEventTarget,
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

  it("matches punctuation keys regardless of Shift state (layout dependent)", () => {
    // "?" is produced with Shift on US layouts; the key itself is enough.
    expect(
      matchesShortcut(
        { key: "?", ctrlKey: false, metaKey: false, shiftKey: true, altKey: false },
        "?",
        "nonMac",
      ),
    ).toBe(true);
    expect(
      matchesShortcut(
        { key: "?", ctrlKey: false, metaKey: false, shiftKey: false, altKey: false },
        "?",
        "nonMac",
      ),
    ).toBe(true);
    // Other modifiers still have to match exactly.
    expect(
      matchesShortcut(
        { key: "?", ctrlKey: true, metaKey: false, shiftKey: true, altKey: false },
        "?",
        "nonMac",
      ),
    ).toBe(false);
    // Shift-agnosticism also applies within combos like mod+/.
    expect(
      matchesShortcut(
        { key: "/", ctrlKey: true, metaKey: false, shiftKey: true, altKey: false },
        "mod+/",
        "nonMac",
      ),
    ).toBe(true);
    // Letter keys keep exact Shift matching.
    expect(
      matchesShortcut(
        { key: "k", ctrlKey: true, metaKey: false, shiftKey: true, altKey: false },
        "mod+k",
        "nonMac",
      ),
    ).toBe(false);
  });

  it("formats named keys with display labels", () => {
    expect(formatShortcut("esc", "nonMac")).toEqual({
      tokens: ["Esc"],
      ariaLabel: "Escape",
    });
    expect(formatShortcut("arrowup", "nonMac")).toEqual({
      tokens: ["↑"],
      ariaLabel: "Arrow Up",
    });
    expect(formatShortcut("arrowdown", "nonMac")).toEqual({
      tokens: ["↓"],
      ariaLabel: "Arrow Down",
    });
  });

  it("detects editable event targets", () => {
    const input = document.createElement("input");
    const textarea = document.createElement("textarea");
    const select = document.createElement("select");
    const div = document.createElement("div");
    const editableDiv = document.createElement("div");
    editableDiv.contentEditable = "true";
    document.body.append(input, textarea, select, div, editableDiv);

    expect(isEditableEventTarget(input)).toBe(true);
    expect(isEditableEventTarget(textarea)).toBe(true);
    expect(isEditableEventTarget(select)).toBe(true);
    expect(isEditableEventTarget(div)).toBe(false);
    expect(isEditableEventTarget(editableDiv)).toBe(true);
    expect(isEditableEventTarget(null)).toBe(false);
    expect(isEditableEventTarget(window)).toBe(false);

    input.remove();
    textarea.remove();
    select.remove();
    div.remove();
    editableDiv.remove();
  });
});
