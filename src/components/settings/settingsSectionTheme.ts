import type { CSSProperties } from "react";

export type SettingsSectionTone =
  | "appearance"
  | "remote-targets"
  | "custom-platforms"
  | "platform-visibility"
  | "github-pat"
  | "ai-provider"
  | "scan-directories"
  | "about";

export interface SettingsSectionTheme {
  accentVar: `--ctp-${string}`;
  tone: SettingsSectionTone;
}

export type SettingsSectionThemeStyle = CSSProperties & {
  "--settings-section-accent": string;
  "--settings-section-accent-soft": string;
  "--settings-section-accent-faint": string;
  "--settings-section-accent-border": string;
};

const DEFAULT_SECTION_TONE: SettingsSectionTheme = {
  accentVar: "--ctp-lavender",
  tone: "appearance",
};

const SETTINGS_SECTION_THEMES: Record<string, SettingsSectionTheme> = {
  appearance: {
    accentVar: "--ctp-mauve",
    tone: "appearance",
  },
  "remote-targets": {
    accentVar: "--ctp-sky",
    tone: "remote-targets",
  },
  "custom-platforms": {
    accentVar: "--ctp-peach",
    tone: "custom-platforms",
  },
  "platform-visibility": {
    accentVar: "--ctp-teal",
    tone: "platform-visibility",
  },
  "github-pat": {
    accentVar: "--ctp-yellow",
    tone: "github-pat",
  },
  "ai-provider": {
    accentVar: "--ctp-lavender",
    tone: "ai-provider",
  },
  "scan-directories": {
    accentVar: "--ctp-green",
    tone: "scan-directories",
  },
  about: {
    accentVar: "--ctp-rosewater",
    tone: "about",
  },
};

function normalizeSettingsSectionId(sectionId: string) {
  if (sectionId === "ai" || sectionId === "ai-section") {
    return "ai-provider";
  }
  if (sectionId.endsWith("-section")) {
    return sectionId.slice(0, "-section".length * -1);
  }
  return sectionId;
}

export function getSettingsSectionTheme(sectionId: string): SettingsSectionTheme {
  const normalizedId = normalizeSettingsSectionId(sectionId);
  return SETTINGS_SECTION_THEMES[normalizedId] ?? DEFAULT_SECTION_TONE;
}

export function getSettingsSectionThemeStyle(
  sectionId: string
): SettingsSectionThemeStyle {
  const { accentVar } = getSettingsSectionTheme(sectionId);
  const accent = `var(${accentVar})`;

  return {
    "--settings-section-accent": accent,
    "--settings-section-accent-soft": `color-mix(in srgb, ${accent} 15%, transparent)`,
    "--settings-section-accent-faint": `color-mix(in srgb, ${accent} 7%, transparent)`,
    "--settings-section-accent-border": `color-mix(in srgb, ${accent} 40%, var(--border))`,
  };
}
