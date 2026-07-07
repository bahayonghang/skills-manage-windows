export type ShortcutPlatform = "mac" | "nonMac";

export interface ShortcutDescriptor {
  tokens: string[];
  ariaLabel: string;
}

const MODIFIER_KEYS = new Set(["mod", "meta", "ctrl", "shift", "alt"]);

// Display labels for named (non-character) keys used by the shortcut registry.
const KEY_TOKEN_LABELS: Record<string, string> = {
  esc: "Esc",
  escape: "Esc",
  arrowup: "↑",
  arrowdown: "↓",
};

const KEY_ARIA_LABELS: Record<string, string> = {
  esc: "Escape",
  escape: "Escape",
  arrowup: "Arrow Up",
  arrowdown: "Arrow Down",
};

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

  // Punctuation keys like "?" or "/" may or may not require Shift depending on
  // the keyboard layout, so Shift state is ignored for them unless the
  // shortcut explicitly asks for it.
  const shiftAgnostic =
    !wantsShift && parsed.key.length === 1 && !/[a-z0-9]/.test(parsed.key);

  return (
    event.metaKey === wantsMeta &&
    event.ctrlKey === wantsCtrl &&
    (shiftAgnostic || event.shiftKey === wantsShift) &&
    event.altKey === wantsAlt
  );
}

/** Whether the event target is a text-entry surface (input/textarea/select/contenteditable). */
export function isEditableEventTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return true;
  // jsdom does not implement isContentEditable; fall back to the reflected property.
  return target.isContentEditable || target.contentEditable === "true";
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
  if (part in KEY_TOKEN_LABELS) return KEY_TOKEN_LABELS[part];
  return part.length === 1 ? part.toUpperCase() : part;
}

function formatShortcutAriaPart(part: string, platform: ShortcutPlatform): string {
  if (part === "mod") return platform === "mac" ? "Command" : "Ctrl";
  if (part === "meta") return platform === "mac" ? "Command" : "Meta";
  if (part === "ctrl") return "Ctrl";
  if (part === "shift") return "Shift";
  if (part === "alt") return platform === "mac" ? "Option" : "Alt";
  if (part in KEY_ARIA_LABELS) return KEY_ARIA_LABELS[part];
  return part.length === 1 ? part.toUpperCase() : part;
}
