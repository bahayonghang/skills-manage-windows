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
import type { FontThemeMode } from "@/stores/themeStore";

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

export interface FontProfile {
  display: DisplayFontKey;
  displayCustom: string;
  displayChineseFallback: ChineseFallbackKey;
  displayChineseFallbackCustom: string;
  body: BodyFontKey;
  bodyCustom: string;
  bodyChineseFallback: ChineseFallbackKey;
  bodyChineseFallbackCustom: string;
}

export interface FontPreferences extends FontProfile {
  scale: number;
}

export interface ThemedFontPreferences {
  light: FontProfile;
  dark: FontProfile;
  scale: number;
}

export const DEFAULT_FONT_PROFILE: FontProfile = {
  display: "geist",
  displayCustom: "",
  displayChineseFallback: "system",
  displayChineseFallbackCustom: "",
  body: "jetbrains",
  bodyCustom: "",
  bodyChineseFallback: "system",
  bodyChineseFallbackCustom: "",
};

export const DEFAULT_FONT_PREFERENCES: FontPreferences = {
  ...DEFAULT_FONT_PROFILE,
  scale: 1,
};

export const DEFAULT_THEMED_FONT_PREFERENCES: ThemedFontPreferences = {
  light: { ...DEFAULT_FONT_PROFILE },
  dark: { ...DEFAULT_FONT_PROFILE },
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

const LEGACY_PROFILE_STORAGE_KEYS: Record<keyof FontProfile, string> = {
  display: STORAGE_KEYS.display,
  displayCustom: STORAGE_KEYS.displayCustom,
  displayChineseFallback: STORAGE_KEYS.displayChineseFallback,
  displayChineseFallbackCustom: STORAGE_KEYS.displayChineseFallbackCustom,
  body: STORAGE_KEYS.body,
  bodyCustom: STORAGE_KEYS.bodyCustom,
  bodyChineseFallback: STORAGE_KEYS.bodyChineseFallback,
  bodyChineseFallbackCustom: STORAGE_KEYS.bodyChineseFallbackCustom,
};

const THEMED_STORAGE_KEYS: Record<
  FontThemeMode,
  Record<keyof FontProfile, string>
> = {
  light: {
    display: "display_font_light_v2",
    displayCustom: "display_font_custom_light_v2",
    displayChineseFallback: "display_chinese_fallback_light_v2",
    displayChineseFallbackCustom:
      "display_chinese_fallback_custom_light_v2",
    body: "body_font_light_v2",
    bodyCustom: "body_font_custom_light_v2",
    bodyChineseFallback: "body_chinese_fallback_light_v2",
    bodyChineseFallbackCustom: "body_chinese_fallback_custom_light_v2",
  },
  dark: {
    display: "display_font_dark_v2",
    displayCustom: "display_font_custom_dark_v2",
    displayChineseFallback: "display_chinese_fallback_dark_v2",
    displayChineseFallbackCustom:
      "display_chinese_fallback_custom_dark_v2",
    body: "body_font_dark_v2",
    bodyCustom: "body_font_custom_dark_v2",
    bodyChineseFallback: "body_chinese_fallback_dark_v2",
    bodyChineseFallbackCustom: "body_chinese_fallback_custom_dark_v2",
  },
};

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

function resolveFontFamily(
  chain: string,
  chineseFallback: ChineseFallbackKey,
  chineseFallbackCustom: string,
): string {
  const fallback = chineseFallbackFamily(
    chineseFallback,
    chineseFallbackCustom,
  );
  return fallback ? insertBeforeTail(chain, fallback) : chain;
}

export function resolveDisplayFontFamily(profile: FontProfile): string {
  return resolveFontFamily(
    displayFontChain(profile.display, profile.displayCustom),
    profile.displayChineseFallback,
    profile.displayChineseFallbackCustom,
  );
}

export function resolveBodyFontFamily(profile: FontProfile): string {
  return resolveFontFamily(
    bodyFontChain(profile.body, profile.bodyCustom),
    profile.bodyChineseFallback,
    profile.bodyChineseFallbackCustom,
  );
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

  const hasCustomPrimary = key === "custom" && Boolean(customFamily.trim());
  const hasCustomFallback = Boolean(
    chineseFallbackFamily(chineseFallback, chineseFallbackCustom),
  );
  if (!hasCustomPrimary && !hasCustomFallback) {
    html.style.removeProperty("--font-display");
    return;
  }

  html.style.setProperty(
    "--font-display",
    resolveFontFamily(
      displayFontChain(key, customFamily),
      chineseFallback,
      chineseFallbackCustom,
    ),
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

  const hasCustomPrimary = key === "custom" && Boolean(customFamily.trim());
  const hasCustomFallback = Boolean(
    chineseFallbackFamily(chineseFallback, chineseFallbackCustom),
  );
  if (!hasCustomPrimary && !hasCustomFallback) {
    html.style.removeProperty("--font-body");
    return;
  }

  html.style.setProperty(
    "--font-body",
    resolveFontFamily(
      bodyFontChain(key, customFamily),
      chineseFallback,
      chineseFallbackCustom,
    ),
  );
}

let appliedFontScale = DEFAULT_THEMED_FONT_PREFERENCES.scale;
const fontScaleListeners = new Set<() => void>();

export function getAppliedFontScale(): number {
  return appliedFontScale;
}

export function subscribeAppliedFontScale(listener: () => void): () => void {
  fontScaleListeners.add(listener);
  return () => fontScaleListeners.delete(listener);
}

export function applyFontScale(scale: number) {
  const html = getHtml();
  if (!html) return;
  const clamped = Math.min(Math.max(scale, 0.75), 1.5);
  html.style.setProperty("--font-scale", String(clamped));
  if (clamped === appliedFontScale) return;
  appliedFontScale = clamped;
  for (const listener of fontScaleListeners) listener();
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

export function applyFontProfile(profile: FontProfile) {
  applyDisplayFont(
    profile.display,
    profile.displayCustom,
    profile.displayChineseFallback,
    profile.displayChineseFallbackCustom,
  );
  applyBodyFont(
    profile.body,
    profile.bodyCustom,
    profile.bodyChineseFallback,
    profile.bodyChineseFallbackCustom,
  );
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

function isDisplayFontKey(value: unknown): value is DisplayFontKey {
  return (
    typeof value === "string" &&
    DISPLAY_FONT_KEYS.has(value as DisplayFontKey)
  );
}

function isBodyFontKey(value: unknown): value is BodyFontKey {
  return (
    typeof value === "string" && BODY_FONT_KEYS.has(value as BodyFontKey)
  );
}

function isChineseFallbackKey(
  value: unknown,
): value is ChineseFallbackKey {
  return (
    typeof value === "string" &&
    CHINESE_FALLBACK_KEYS.has(value as ChineseFallbackKey)
  );
}

type RawFontProfile = {
  [Key in keyof FontProfile]: string | null;
};

async function readSetting(key: string): Promise<string | null> {
  try {
    return await invoke("get_setting", { key });
  } catch {
    return null;
  }
}

async function readRawFontProfile(
  keys: Record<keyof FontProfile, string>,
): Promise<RawFontProfile> {
  const fields = Object.keys(keys) as (keyof FontProfile)[];
  const values = await Promise.all(
    fields.map((field) => readSetting(keys[field])),
  );
  return Object.fromEntries(
    fields.map((field, index) => [field, values[index]]),
  ) as RawFontProfile;
}

function resolveFontProfile(
  raw: RawFontProfile,
  fallback: FontProfile,
): FontProfile {
  return {
    display: isDisplayFontKey(raw.display) ? raw.display : fallback.display,
    displayCustom: raw.displayCustom ?? fallback.displayCustom,
    displayChineseFallback: isChineseFallbackKey(raw.displayChineseFallback)
      ? raw.displayChineseFallback
      : fallback.displayChineseFallback,
    displayChineseFallbackCustom:
      raw.displayChineseFallbackCustom ??
      fallback.displayChineseFallbackCustom,
    body: isBodyFontKey(raw.body) ? raw.body : fallback.body,
    bodyCustom: raw.bodyCustom ?? fallback.bodyCustom,
    bodyChineseFallback: isChineseFallbackKey(raw.bodyChineseFallback)
      ? raw.bodyChineseFallback
      : fallback.bodyChineseFallback,
    bodyChineseFallbackCustom:
      raw.bodyChineseFallbackCustom ?? fallback.bodyChineseFallbackCustom,
  };
}

function rawProfileNeedsMigration(raw: RawFontProfile): boolean {
  return (
    !isDisplayFontKey(raw.display) ||
    raw.displayCustom === null ||
    !isChineseFallbackKey(raw.displayChineseFallback) ||
    raw.displayChineseFallbackCustom === null ||
    !isBodyFontKey(raw.body) ||
    raw.bodyCustom === null ||
    !isChineseFallbackKey(raw.bodyChineseFallback) ||
    raw.bodyChineseFallbackCustom === null
  );
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

let themedLoadPromise: Promise<ThemedFontPreferences> | null = null;
let cachedThemedPreferences: ThemedFontPreferences = {
  light: { ...DEFAULT_FONT_PROFILE },
  dark: { ...DEFAULT_FONT_PROFILE },
  scale: DEFAULT_THEMED_FONT_PREFERENCES.scale,
};
let activeFontTheme: FontThemeMode = "dark";
const FONT_PROFILE_FIELDS = Object.keys(
  DEFAULT_FONT_PROFILE,
) as (keyof FontProfile)[];
const profileRevisions: Record<
  FontThemeMode,
  Record<keyof FontProfile, number>
> = {
  light: Object.fromEntries(
    FONT_PROFILE_FIELDS.map((field) => [field, 0]),
  ) as Record<keyof FontProfile, number>,
  dark: Object.fromEntries(
    FONT_PROFILE_FIELDS.map((field) => [field, 0]),
  ) as Record<keyof FontProfile, number>,
};
let scaleRevision = 0;

function cloneThemedFontPreferences(
  preferences: ThemedFontPreferences,
): ThemedFontPreferences {
  return {
    light: { ...preferences.light },
    dark: { ...preferences.dark },
    scale: preferences.scale,
  };
}

export function applyThemedFontPreferences(
  preferences: ThemedFontPreferences,
  mode: FontThemeMode,
) {
  cachedThemedPreferences = cloneThemedFontPreferences(preferences);
  applyFontScale(preferences.scale);
  activateFontTheme(mode);
}

export function activateFontTheme(mode: FontThemeMode) {
  activeFontTheme = mode;
  applyFontProfile(cachedThemedPreferences[mode]);
}

function updateCachedFontProfile(
  mode: FontThemeMode,
  patch: Partial<FontProfile>,
): FontProfile {
  for (const field of Object.keys(patch) as (keyof FontProfile)[]) {
    profileRevisions[mode][field] += 1;
  }
  const profile = { ...cachedThemedPreferences[mode], ...patch };
  cachedThemedPreferences = {
    ...cachedThemedPreferences,
    [mode]: profile,
  };
  if (mode === activeFontTheme) {
    applyFontProfile(profile);
  }
  return profile;
}

async function writeFontProfile(
  mode: FontThemeMode,
  profile: FontProfile,
): Promise<void> {
  const keys = THEMED_STORAGE_KEYS[mode];
  await Promise.all(
    FONT_PROFILE_FIELDS.map((field) =>
      writeSetting(keys[field], profile[field]),
    ),
  );
}

function mergeLoadedFontProfile(
  mode: FontThemeMode,
  loaded: FontProfile,
  revisionsAtStart: Record<keyof FontProfile, number>,
): FontProfile {
  const merged = { ...loaded };
  for (const field of FONT_PROFILE_FIELDS) {
    if (profileRevisions[mode][field] !== revisionsAtStart[field]) {
      Object.assign(merged, { [field]: cachedThemedPreferences[mode][field] });
    }
  }
  return merged;
}

async function performThemedFontPreferencesLoad(): Promise<ThemedFontPreferences> {
  const revisionsAtStart = {
    light: { ...profileRevisions.light },
    dark: { ...profileRevisions.dark },
    scale: scaleRevision,
  };
  const [legacyRaw, lightRaw, darkRaw, scale] = await Promise.all([
    readRawFontProfile(LEGACY_PROFILE_STORAGE_KEYS),
    readRawFontProfile(THEMED_STORAGE_KEYS.light),
    readRawFontProfile(THEMED_STORAGE_KEYS.dark),
    readSetting(STORAGE_KEYS.scale),
  ]);
  const legacy = resolveFontProfile(legacyRaw, DEFAULT_FONT_PROFILE);
  const loadedLight = resolveFontProfile(lightRaw, legacy);
  const loadedDark = resolveFontProfile(darkRaw, legacy);
  const preferences: ThemedFontPreferences = {
    light: mergeLoadedFontProfile(
      "light",
      loadedLight,
      revisionsAtStart.light,
    ),
    dark: mergeLoadedFontProfile("dark", loadedDark, revisionsAtStart.dark),
    scale:
      scaleRevision === revisionsAtStart.scale
        ? coerceScale(scale)
        : cachedThemedPreferences.scale,
  };
  cachedThemedPreferences = cloneThemedFontPreferences(preferences);

  await Promise.all([
    rawProfileNeedsMigration(lightRaw)
      ? writeFontProfile("light", preferences.light)
      : Promise.resolve(),
    rawProfileNeedsMigration(darkRaw)
      ? writeFontProfile("dark", preferences.dark)
      : Promise.resolve(),
  ]);
  return cloneThemedFontPreferences(cachedThemedPreferences);
}

export function loadThemedFontPreferences(): Promise<ThemedFontPreferences> {
  if (!themedLoadPromise) {
    themedLoadPromise = performThemedFontPreferencesLoad().finally(() => {
      themedLoadPromise = null;
    });
  }
  return themedLoadPromise;
}

const settingWriteChains = new Map<string, Promise<void>>();

function writeSetting(key: string, value: string): Promise<void> {
  const previous = settingWriteChains.get(key) ?? Promise.resolve();
  const next = previous.then(async () => {
    try {
      await invoke("set_setting", { key, value });
    } catch {
      // 偏好类设置写入失败不应中断 UI；fall through 静默
    }
  });
  settingWriteChains.set(key, next);
  return next.then(() => {
    if (settingWriteChains.get(key) === next) {
      settingWriteChains.delete(key);
    }
  });
}

export async function saveDisplayFont(
  mode: FontThemeMode,
  key: DisplayFontKey,
  customFamily = "",
): Promise<void> {
  updateCachedFontProfile(mode, {
    display: key,
    displayCustom: customFamily,
  });
  const keys = THEMED_STORAGE_KEYS[mode];
  await Promise.all([
    writeSetting(keys.display, key),
    writeSetting(keys.displayCustom, customFamily),
  ]);
}

export async function saveBodyFont(
  mode: FontThemeMode,
  key: BodyFontKey,
  customFamily = "",
): Promise<void> {
  updateCachedFontProfile(mode, {
    body: key,
    bodyCustom: customFamily,
  });
  const keys = THEMED_STORAGE_KEYS[mode];
  await Promise.all([
    writeSetting(keys.body, key),
    writeSetting(keys.bodyCustom, customFamily),
  ]);
}

export async function saveDisplayChineseFallback(
  mode: FontThemeMode,
  key: ChineseFallbackKey,
  customFamily: string,
): Promise<void> {
  updateCachedFontProfile(mode, {
    displayChineseFallback: key,
    displayChineseFallbackCustom: customFamily,
  });
  const keys = THEMED_STORAGE_KEYS[mode];
  await Promise.all([
    writeSetting(keys.displayChineseFallback, key),
    writeSetting(keys.displayChineseFallbackCustom, customFamily),
  ]);
}

export async function saveBodyChineseFallback(
  mode: FontThemeMode,
  key: ChineseFallbackKey,
  customFamily: string,
): Promise<void> {
  updateCachedFontProfile(mode, {
    bodyChineseFallback: key,
    bodyChineseFallbackCustom: customFamily,
  });
  const keys = THEMED_STORAGE_KEYS[mode];
  await Promise.all([
    writeSetting(keys.bodyChineseFallback, key),
    writeSetting(keys.bodyChineseFallbackCustom, customFamily),
  ]);
}

export async function saveFontScale(scale: number): Promise<void> {
  scaleRevision += 1;
  cachedThemedPreferences = { ...cachedThemedPreferences, scale };
  applyFontScale(scale);
  await writeSetting(STORAGE_KEYS.scale, String(scale));
}
