export type ShortcutPlatform = "mac" | "nonMac";

export interface ShortcutDescriptor {
  tokens: string[];
  ariaLabel: string;
}

const MODIFIER_KEYS = new Set(["mod", "meta", "ctrl", "shift", "alt"]);

export function getShortcutPlatform(navigatorLike: Pick<Navigator, "platform"> | undefined = getNavigator()): ShortcutPlatform {
  return navigatorLike?.platform.toUpperCase().includes("MAC") ? "mac" : "nonMac";
}

export function formatShortcut(
  shortcut: string,
  platform: ShortcutPlatform = getShortcutPlatform(),
): ShortcutDescriptor {
  const parsed = parseShortcut(shortcut);
  const tokens = parsed.parts.map((part) => formatShortcutPart(part, platform));
  const ariaTokens = parsed.parts.map((part) => formatShortcutAriaPart(part, platform));
  return {
    tokens,
    ariaLabel: ariaTokens.join("+"),
  };
}

export function matchesShortcut(
  event: Pick<KeyboardEvent, "key" | "metaKey" | "ctrlKey" | "shiftKey" | "altKey">,
  shortcut: string,
  platform: ShortcutPlatform = getShortcutPlatform(),
): boolean {
  const parsed = parseShortcut(shortcut);
  if (event.key.toLowerCase() !== parsed.key) return false;

  const wantsMeta = parsed.parts.includes("meta") || (parsed.parts.includes("mod") && platform === "mac");
  const wantsCtrl = parsed.parts.includes("ctrl") || (parsed.parts.includes("mod") && platform === "nonMac");
  const wantsShift = parsed.parts.includes("shift");
  const wantsAlt = parsed.parts.includes("alt");

  return (
    event.metaKey === wantsMeta &&
    event.ctrlKey === wantsCtrl &&
    event.shiftKey === wantsShift &&
    event.altKey === wantsAlt
  );
}

function getNavigator(): Pick<Navigator, "platform"> | undefined {
  return typeof navigator === "undefined" ? undefined : navigator;
}

function parseShortcut(shortcut: string): { parts: string[]; key: string } {
  const parts = shortcut
    .toLowerCase()
    .split("+")
    .map((part) => part.trim())
    .filter(Boolean);
  const key = parts.find((part) => !MODIFIER_KEYS.has(part));
  if (!key) {
    throw new Error(`Shortcut "${shortcut}" is missing a key.`);
  }
  return { parts, key };
}

function formatShortcutPart(part: string, platform: ShortcutPlatform): string {
  if (part === "mod") return platform === "mac" ? "⌘" : "Ctrl";
  if (part === "meta") return platform === "mac" ? "⌘" : "Meta";
  if (part === "ctrl") return "Ctrl";
  if (part === "shift") return "Shift";
  if (part === "alt") return platform === "mac" ? "Option" : "Alt";
  return part.length === 1 ? part.toUpperCase() : part;
}

function formatShortcutAriaPart(part: string, platform: ShortcutPlatform): string {
  if (part === "mod") return platform === "mac" ? "Command" : "Ctrl";
  if (part === "meta") return platform === "mac" ? "Command" : "Meta";
  if (part === "ctrl") return "Ctrl";
  if (part === "shift") return "Shift";
  if (part === "alt") return platform === "mac" ? "Option" : "Alt";
  return part.length === 1 ? part.toUpperCase() : part;
}
