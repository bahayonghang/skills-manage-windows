/**
 * 字体偏好管理。
 *
 * Display 字体（标题）与 Body 字体（正文）独立持久化，缩放系数也单独保存。
 * 统一经 `get_setting` / `set_setting` IPC 持久化；浏览器演示态由 settings
 * fixture 供数（get_setting→null 回落默认值，set_setting 静默成功）。
 *
 * DOM 落地形式：
 *   - 预设：`html[data-display-font='geist']` 切换 CSS 变量
 *   - 自定义：`html.style.setProperty('--font-display', "'My Font', fallback...")`
 */
import { invoke } from "@/lib/ipc";

export type DisplayFontKey =
  "geist" | "jetbrains" | "inter" | "serif" | "system" | "custom";

export type BodyFontKey = "jetbrains" | "geist" | "inter" | "system" | "custom";

export type ChineseFallbackKey = "system" | "sourceHanSerif" | "custom";

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

export interface ChineseFallbackOption {
  key: ChineseFallbackKey;
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

export const CHINESE_FALLBACK_OPTIONS: readonly ChineseFallbackOption[] = [
  { key: "system", labelKey: "system" },
  { key: "sourceHanSerif", labelKey: "sourceHanSerif" },
  { key: "custom", labelKey: "custom" },
];

export interface FontPreferences {
  display: DisplayFontKey;
  displayCustom: string;
  displayChineseFallback: ChineseFallbackKey;
  displayChineseFallbackCustom: string;
  body: BodyFontKey;
  bodyCustom: string;
  bodyChineseFallback: ChineseFallbackKey;
  bodyChineseFallbackCustom: string;
  scale: number;
}

export const DEFAULT_FONT_PREFERENCES: FontPreferences = {
  display: "geist",
  displayCustom: "",
  displayChineseFallback: "system",
  displayChineseFallbackCustom: "",
  body: "jetbrains",
  bodyCustom: "",
  bodyChineseFallback: "system",
  bodyChineseFallbackCustom: "",
  scale: 1,
};

const STORAGE_KEYS = {
  display: "display_font_v1",
  displayCustom: "display_font_custom_v1",
  displayChineseFallback: "display_chinese_fallback_v1",
  displayChineseFallbackCustom: "display_chinese_fallback_custom_v1",
  body: "body_font_v1",
  bodyCustom: "body_font_custom_v1",
  bodyChineseFallback: "body_chinese_fallback_v1",
  bodyChineseFallbackCustom: "body_chinese_fallback_custom_v1",
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

const CHINESE_FALLBACK_KEYS = new Set<ChineseFallbackKey>([
  "system",
  "sourceHanSerif",
  "custom",
]);

const DISPLAY_FONT_CHAINS: Record<Exclude<DisplayFontKey, "custom">, string> = {
  geist: '"Geist Variable", ui-sans-serif, system-ui, sans-serif',
  jetbrains: '"JetBrains Mono Variable", ui-monospace, monospace',
  inter: '"Inter Variable", ui-sans-serif, system-ui, sans-serif',
  serif: '"Instrument Serif", Georgia, "Times New Roman", serif',
  system: 'ui-sans-serif, system-ui, -apple-system, "Segoe UI", sans-serif',
};

const BODY_FONT_CHAINS: Record<Exclude<BodyFontKey, "custom">, string> = {
  jetbrains: '"JetBrains Mono Variable", ui-monospace, monospace',
  geist: '"Geist Variable", ui-sans-serif, system-ui, sans-serif',
  inter: '"Inter Variable", ui-sans-serif, system-ui, sans-serif',
  system: 'ui-sans-serif, system-ui, -apple-system, "Segoe UI", sans-serif',
};

function fallbackChain(): string {
  return "ui-sans-serif, system-ui, -apple-system, 'Segoe UI', sans-serif, 'JetBrains Mono Variable', ui-monospace, monospace";
}

function quoteFontFamily(family: string): string {
  const sanitized = family.replace(/[\u0000-\u001f\u007f]/g, " ");
  return `"${sanitized.replace(/\\/g, "\\\\").replace(/"/g, '\\"')}"`;
}

function chineseFallbackFamily(
  key: ChineseFallbackKey,
  customFamily: string,
): string | null {
  if (key === "sourceHanSerif") {
    return '"Source Han Serif SC VF", "思源宋体 VF"';
  }
  if (key === "custom" && customFamily.trim()) {
    return quoteFontFamily(customFamily.trim());
  }
  return null;
}

function insertBeforeTail(chain: string, fallback: string): string {
  const separator = chain.indexOf(",");
  if (separator === -1) return `${chain}, ${fallback}`;
  return `${chain.slice(0, separator)}, ${fallback}${chain.slice(separator)}`;
}

function displayFontChain(key: DisplayFontKey, customFamily: string): string {
  if (key === "custom" && customFamily.trim()) {
    return `${quoteFontFamily(customFamily.trim())}, ${fallbackChain()}`;
  }
  return DISPLAY_FONT_CHAINS[key === "custom" ? "geist" : key];
}

function bodyFontChain(key: BodyFontKey, customFamily: string): string {
  if (key === "custom" && customFamily.trim()) {
    return `${quoteFontFamily(customFamily.trim())}, ${fallbackChain()}`;
  }
  return BODY_FONT_CHAINS[key === "custom" ? "jetbrains" : key];
}

function getHtml(): HTMLElement | null {
  if (typeof document === "undefined") return null;
  return document.documentElement;
}

export function applyDisplayFont(
  key: DisplayFontKey,
  customFamily = "",
  chineseFallback: ChineseFallbackKey = "system",
  chineseFallbackCustom = "",
) {
  const html = getHtml();
  if (!html) return;

  html.dataset.displayFont = key === "custom" ? "geist" : key;
  if (key === "custom" && customFamily.trim()) {
    html.dataset.displayFont = "custom";
  }

  const fallback = chineseFallbackFamily(
    chineseFallback,
    chineseFallbackCustom,
  );
  const hasCustomPrimary = key === "custom" && Boolean(customFamily.trim());
  if (!hasCustomPrimary && !fallback) {
    html.style.removeProperty("--font-display");
    return;
  }

  const chain = displayFontChain(key, customFamily);
  html.style.setProperty(
    "--font-display",
    fallback ? insertBeforeTail(chain, fallback) : chain,
  );
}

export function applyBodyFont(
  key: BodyFontKey,
  customFamily = "",
  chineseFallback: ChineseFallbackKey = "system",
  chineseFallbackCustom = "",
) {
  const html = getHtml();
  if (!html) return;

  html.dataset.bodyFont = key === "custom" ? "jetbrains" : key;
  if (key === "custom" && customFamily.trim()) {
    html.dataset.bodyFont = "custom";
  }

  const fallback = chineseFallbackFamily(
    chineseFallback,
    chineseFallbackCustom,
  );
  const hasCustomPrimary = key === "custom" && Boolean(customFamily.trim());
  if (!hasCustomPrimary && !fallback) {
    html.style.removeProperty("--font-body");
    return;
  }

  const chain = bodyFontChain(key, customFamily);
  html.style.setProperty(
    "--font-body",
    fallback ? insertBeforeTail(chain, fallback) : chain,
  );
}

export function applyFontScale(scale: number) {
  const html = getHtml();
  if (!html) return;
  const clamped = Math.min(Math.max(scale, 0.75), 1.5);
  html.style.setProperty("--font-scale", String(clamped));
}

export function applyFontPreferences(prefs: FontPreferences) {
  applyDisplayFont(
    prefs.display,
    prefs.displayCustom,
    prefs.displayChineseFallback,
    prefs.displayChineseFallbackCustom,
  );
  applyBodyFont(
    prefs.body,
    prefs.bodyCustom,
    prefs.bodyChineseFallback,
    prefs.bodyChineseFallbackCustom,
  );
  applyFontScale(prefs.scale);
}

function coerceDisplay(value: unknown): DisplayFontKey {
  if (
    typeof value === "string" &&
    DISPLAY_FONT_KEYS.has(value as DisplayFontKey)
  ) {
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

function coerceChineseFallback(value: unknown): ChineseFallbackKey {
  if (
    typeof value === "string" &&
    CHINESE_FALLBACK_KEYS.has(value as ChineseFallbackKey)
  ) {
    return value as ChineseFallbackKey;
  }
  return "system";
}

function coerceScale(value: unknown): number {
  if (typeof value !== "string") return DEFAULT_FONT_PREFERENCES.scale;
  const parsed = Number.parseFloat(value);
  if (!Number.isFinite(parsed)) return DEFAULT_FONT_PREFERENCES.scale;
  return Math.min(Math.max(parsed, 0.75), 1.5);
}

async function readSetting(key: string): Promise<string | null> {
  try {
    return await invoke("get_setting", { key });
  } catch {
    return null;
  }
}

export async function loadFontPreferences(): Promise<FontPreferences> {
  const [
    display,
    displayCustom,
    displayChineseFallback,
    displayChineseFallbackCustom,
    body,
    bodyCustom,
    bodyChineseFallback,
    bodyChineseFallbackCustom,
    scale,
  ] = await Promise.all([
    readSetting(STORAGE_KEYS.display),
    readSetting(STORAGE_KEYS.displayCustom),
    readSetting(STORAGE_KEYS.displayChineseFallback),
    readSetting(STORAGE_KEYS.displayChineseFallbackCustom),
    readSetting(STORAGE_KEYS.body),
    readSetting(STORAGE_KEYS.bodyCustom),
    readSetting(STORAGE_KEYS.bodyChineseFallback),
    readSetting(STORAGE_KEYS.bodyChineseFallbackCustom),
    readSetting(STORAGE_KEYS.scale),
  ]);

  return {
    display: coerceDisplay(display),
    displayCustom: displayCustom ?? "",
    displayChineseFallback: coerceChineseFallback(displayChineseFallback),
    displayChineseFallbackCustom: displayChineseFallbackCustom ?? "",
    body: coerceBody(body),
    bodyCustom: bodyCustom ?? "",
    bodyChineseFallback: coerceChineseFallback(bodyChineseFallback),
    bodyChineseFallbackCustom: bodyChineseFallbackCustom ?? "",
    scale: coerceScale(scale),
  };
}

async function writeSetting(key: string, value: string): Promise<void> {
  try {
    await invoke("set_setting", { key, value });
  } catch {
    // 偏好类设置写入失败不应中断 UI；fall through 静默
  }
}

export async function saveDisplayFont(
  key: DisplayFontKey,
  customFamily = "",
  chineseFallback: ChineseFallbackKey = "system",
  chineseFallbackCustom = "",
): Promise<void> {
  applyDisplayFont(
    key,
    customFamily,
    chineseFallback,
    chineseFallbackCustom,
  );
  await Promise.all([
    writeSetting(STORAGE_KEYS.display, key),
    writeSetting(STORAGE_KEYS.displayCustom, customFamily),
  ]);
}

export async function saveBodyFont(
  key: BodyFontKey,
  customFamily = "",
  chineseFallback: ChineseFallbackKey = "system",
  chineseFallbackCustom = "",
): Promise<void> {
  applyBodyFont(key, customFamily, chineseFallback, chineseFallbackCustom);
  await Promise.all([
    writeSetting(STORAGE_KEYS.body, key),
    writeSetting(STORAGE_KEYS.bodyCustom, customFamily),
  ]);
}

export async function saveDisplayChineseFallback(
  key: ChineseFallbackKey,
  customFamily: string,
  display: DisplayFontKey,
  displayCustom: string,
): Promise<void> {
  applyDisplayFont(display, displayCustom, key, customFamily);
  await Promise.all([
    writeSetting(STORAGE_KEYS.displayChineseFallback, key),
    writeSetting(STORAGE_KEYS.displayChineseFallbackCustom, customFamily),
  ]);
}

export async function saveBodyChineseFallback(
  key: ChineseFallbackKey,
  customFamily: string,
  body: BodyFontKey,
  bodyCustom: string,
): Promise<void> {
  applyBodyFont(body, bodyCustom, key, customFamily);
  await Promise.all([
    writeSetting(STORAGE_KEYS.bodyChineseFallback, key),
    writeSetting(STORAGE_KEYS.bodyChineseFallbackCustom, customFamily),
  ]);
}

export async function saveFontScale(scale: number): Promise<void> {
  applyFontScale(scale);
  await writeSetting(STORAGE_KEYS.scale, String(scale));
}
