/**
 * 字体偏好管理。
 *
 * Display 字体（标题）与 Body 字体（正文）独立持久化，缩放系数也单独保存。
 * Tauri 环境通过 `get_setting` / `set_setting` IPC 持久化；浏览器/测试环境
 * 跳过 IPC 仅操作 DOM，保证 vitest 与 SSR 不报错。
 *
 * DOM 落地形式：
 *   - 预设：`html[data-display-font='geist']` 切换 CSS 变量
 *   - 自定义：`html.style.setProperty('--font-display', "'My Font', fallback...")`
 */
import { invoke, isTauriRuntime } from "@/lib/ipc";

export type DisplayFontKey =
  | "geist"
  | "jetbrains"
  | "inter"
  | "serif"
  | "system"
  | "custom";

export type BodyFontKey =
  | "jetbrains"
  | "geist"
  | "inter"
  | "system"
  | "custom";

export interface DisplayFontOption {
  key: DisplayFontKey;
  /** i18n key under `settings.appearance.displayFontOption.<key>` */
  labelKey: string;
  /** 渲染按钮预览时的样本字符（不进 i18n，留作视觉示意）。 */
  sample: string;
}

export interface BodyFontOption {
  key: BodyFontKey;
  labelKey: string;
}

export interface FontScaleOption {
  value: number;
  labelKey: string;
}

export const DISPLAY_FONT_OPTIONS: readonly DisplayFontOption[] = [
  { key: "geist", labelKey: "geist", sample: "Aa" },
  { key: "jetbrains", labelKey: "jetbrains", sample: "Aa" },
  { key: "inter", labelKey: "inter", sample: "Aa" },
  { key: "serif", labelKey: "serif", sample: "Aa" },
  { key: "system", labelKey: "system", sample: "Aa" },
  { key: "custom", labelKey: "custom", sample: "Aa" },
];

export const BODY_FONT_OPTIONS: readonly BodyFontOption[] = [
  { key: "jetbrains", labelKey: "jetbrains" },
  { key: "geist", labelKey: "geist" },
  { key: "inter", labelKey: "inter" },
  { key: "system", labelKey: "system" },
  { key: "custom", labelKey: "custom" },
];

export const FONT_SCALE_OPTIONS: readonly FontScaleOption[] = [
  { value: 0.875, labelKey: "scaleCompact" },
  { value: 1, labelKey: "scaleNormal" },
  { value: 1.125, labelKey: "scaleSpacious" },
];

export interface FontPreferences {
  display: DisplayFontKey;
  displayCustom: string;
  body: BodyFontKey;
  bodyCustom: string;
  scale: number;
}

export const DEFAULT_FONT_PREFERENCES: FontPreferences = {
  display: "geist",
  displayCustom: "",
  body: "jetbrains",
  bodyCustom: "",
  scale: 1,
};

const STORAGE_KEYS = {
  display: "display_font_v1",
  displayCustom: "display_font_custom_v1",
  body: "body_font_v1",
  bodyCustom: "body_font_custom_v1",
  scale: "font_scale_v1",
} as const;

const DISPLAY_FONT_KEYS = new Set<DisplayFontKey>([
  "geist",
  "jetbrains",
  "inter",
  "serif",
  "system",
  "custom",
]);

const BODY_FONT_KEYS = new Set<BodyFontKey>([
  "jetbrains",
  "geist",
  "inter",
  "system",
  "custom",
]);

function fallbackChain(): string {
  return "ui-sans-serif, system-ui, -apple-system, 'Segoe UI', sans-serif, 'JetBrains Mono Variable', ui-monospace, monospace";
}

function getHtml(): HTMLElement | null {
  if (typeof document === "undefined") return null;
  return document.documentElement;
}

export function applyDisplayFont(key: DisplayFontKey, customFamily = "") {
  const html = getHtml();
  if (!html) return;

  if (key === "custom" && customFamily.trim()) {
    html.dataset.displayFont = "custom";
    html.style.setProperty(
      "--font-display",
      `'${customFamily.trim()}', ${fallbackChain()}`,
    );
    return;
  }

  html.dataset.displayFont = key === "custom" ? "geist" : key;
  html.style.removeProperty("--font-display");
}

export function applyBodyFont(key: BodyFontKey, customFamily = "") {
  const html = getHtml();
  if (!html) return;

  if (key === "custom" && customFamily.trim()) {
    html.dataset.bodyFont = "custom";
    html.style.setProperty(
      "--font-body",
      `'${customFamily.trim()}', ${fallbackChain()}`,
    );
    return;
  }

  html.dataset.bodyFont = key === "custom" ? "jetbrains" : key;
  html.style.removeProperty("--font-body");
}

export function applyFontScale(scale: number) {
  const html = getHtml();
  if (!html) return;
  const clamped = Math.min(Math.max(scale, 0.75), 1.5);
  html.style.setProperty("--font-scale", String(clamped));
}

export function applyFontPreferences(prefs: FontPreferences) {
  applyDisplayFont(prefs.display, prefs.displayCustom);
  applyBodyFont(prefs.body, prefs.bodyCustom);
  applyFontScale(prefs.scale);
}

function coerceDisplay(value: unknown): DisplayFontKey {
  if (typeof value === "string" && DISPLAY_FONT_KEYS.has(value as DisplayFontKey)) {
    return value as DisplayFontKey;
  }
  return DEFAULT_FONT_PREFERENCES.display;
}

function coerceBody(value: unknown): BodyFontKey {
  if (typeof value === "string" && BODY_FONT_KEYS.has(value as BodyFontKey)) {
    return value as BodyFontKey;
  }
  return DEFAULT_FONT_PREFERENCES.body;
}

function coerceScale(value: unknown): number {
  if (typeof value !== "string") return DEFAULT_FONT_PREFERENCES.scale;
  const parsed = Number.parseFloat(value);
  if (!Number.isFinite(parsed)) return DEFAULT_FONT_PREFERENCES.scale;
  return Math.min(Math.max(parsed, 0.75), 1.5);
}

async function readSetting(key: string): Promise<string | null> {
  try {
    return await invoke<string | null>("get_setting", { key });
  } catch {
    return null;
  }
}

export async function loadFontPreferences(): Promise<FontPreferences> {
  if (!isTauriRuntime()) {
    return { ...DEFAULT_FONT_PREFERENCES };
  }

  const [display, displayCustom, body, bodyCustom, scale] = await Promise.all([
    readSetting(STORAGE_KEYS.display),
    readSetting(STORAGE_KEYS.displayCustom),
    readSetting(STORAGE_KEYS.body),
    readSetting(STORAGE_KEYS.bodyCustom),
    readSetting(STORAGE_KEYS.scale),
  ]);

  return {
    display: coerceDisplay(display),
    displayCustom: displayCustom ?? "",
    body: coerceBody(body),
    bodyCustom: bodyCustom ?? "",
    scale: coerceScale(scale),
  };
}

async function writeSetting(key: string, value: string): Promise<void> {
  if (!isTauriRuntime()) return;
  try {
    await invoke("set_setting", { key, value });
  } catch {
    // 偏好类设置写入失败不应中断 UI；fall through 静默
  }
}

export async function saveDisplayFont(
  key: DisplayFontKey,
  customFamily = "",
): Promise<void> {
  applyDisplayFont(key, customFamily);
  await Promise.all([
    writeSetting(STORAGE_KEYS.display, key),
    writeSetting(STORAGE_KEYS.displayCustom, customFamily),
  ]);
}

export async function saveBodyFont(
  key: BodyFontKey,
  customFamily = "",
): Promise<void> {
  applyBodyFont(key, customFamily);
  await Promise.all([
    writeSetting(STORAGE_KEYS.body, key),
    writeSetting(STORAGE_KEYS.bodyCustom, customFamily),
  ]);
}

export async function saveFontScale(scale: number): Promise<void> {
  applyFontScale(scale);
  await writeSetting(STORAGE_KEYS.scale, String(scale));
}
