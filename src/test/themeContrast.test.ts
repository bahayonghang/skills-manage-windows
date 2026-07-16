import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { describe, expect, it } from "vitest";

const INDEX_CSS = readFileSync(resolve(process.cwd(), "src/index.css"), "utf8");
const MIN_NORMAL_TEXT_CONTRAST = 4.5;
const THEMES = [
  "mocha",
  "macchiato",
  "frappe",
  "latte",
  "claude-light",
  "claude-dark",
] as const;
const LIGHT_THEMES = ["latte", "claude-light"] as const;
const ACCENTS = [
  "rosewater",
  "flamingo",
  "pink",
  "mauve",
  "red",
  "maroon",
  "peach",
  "yellow",
  "green",
  "teal",
  "sky",
  "sapphire",
  "blue",
  "lavender",
] as const;

type ThemeName = (typeof THEMES)[number];
type AccentName = (typeof ACCENTS)[number];

function cssBlock(selector: string) {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const match = INDEX_CSS.match(new RegExp(`${escaped}\\s*\\{(?<body>[^}]*)\\}`, "m"));
  if (!match?.groups?.body) {
    throw new Error(`Missing CSS selector: ${selector}`);
  }
  return match.groups.body;
}

function cssVariable(block: string, name: string) {
  const match = block.match(
    new RegExp(`${name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\s*:\\s*(#[0-9a-fA-F]{6})\\s*;`)
  );
  if (!match?.[1]) {
    throw new Error(`Missing hex variable ${name}`);
  }
  return match[1].toLowerCase();
}

function resolvedCssVariable(block: string, name: string): string {
  const escaped = name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const match = block.match(new RegExp(`${escaped}\\s*:\\s*([^;]+)\\s*;`));
  const value = match?.[1]?.trim();
  if (!value) {
    throw new Error(`Missing variable ${name}`);
  }
  if (/^#[0-9a-fA-F]{6}$/.test(value)) {
    return value.toLowerCase();
  }
  const reference = value.match(/^var\((--[^)]+)\)$/)?.[1];
  if (!reference) {
    throw new Error(`Unsupported variable value ${name}: ${value}`);
  }
  return resolvedCssVariable(block, reference);
}

function hexToRgb(hex: string) {
  return [0, 2, 4].map((index) =>
    Number.parseInt(hex.slice(1 + index, 3 + index), 16) / 255
  ) as [number, number, number];
}

function linearize(channel: number) {
  return channel <= 0.04045
    ? channel / 12.92
    : ((channel + 0.055) / 1.055) ** 2.4;
}

function relativeLuminance(hex: string) {
  const [red, green, blue] = hexToRgb(hex).map(linearize);
  return red * 0.2126 + green * 0.7152 + blue * 0.0722;
}

function contrastRatio(foreground: string, background: string) {
  const lighter = Math.max(
    relativeLuminance(foreground),
    relativeLuminance(background)
  );
  const darker = Math.min(
    relativeLuminance(foreground),
    relativeLuminance(background)
  );
  return (lighter + 0.05) / (darker + 0.05);
}

function expectContrast(
  foreground: string,
  background: string,
  label: string,
  min = MIN_NORMAL_TEXT_CONTRAST
) {
  expect(contrastRatio(foreground, background), label).toBeGreaterThanOrEqual(
    min
  );
}

function themeTokens(theme: ThemeName) {
  const block = cssBlock(`[data-theme="${theme}"]`);
  return {
    background: cssVariable(block, "--background"),
    foreground: cssVariable(block, "--foreground"),
    card: cssVariable(block, "--card"),
    cardForeground: cssVariable(block, "--card-foreground"),
    mutedForeground: cssVariable(block, "--muted-foreground"),
    primary: cssVariable(block, "--primary"),
    primaryText: cssVariable(block, "--primary-text"),
    primaryForeground: cssVariable(block, "--primary-foreground"),
    successForeground: resolvedCssVariable(block, "--success-foreground"),
    warningForeground: resolvedCssVariable(block, "--warning-foreground"),
    infoForeground: resolvedCssVariable(block, "--info-foreground"),
  };
}

function accentPrimaryText(theme: ThemeName, accent: AccentName) {
  return cssVariable(
    cssBlock(`[data-theme="${theme}"][data-accent="${accent}"]`),
    "--primary-text"
  );
}

describe("theme contrast tokens", () => {
  it("maps the readable primary text token into Tailwind", () => {
    expect(cssBlock("@theme inline")).toContain(
      "--color-primary-text: var(--primary-text);"
    );
  });

  it.each(THEMES)(
    "%s keeps inspector labels and semantic states readable",
    (theme) => {
      const tokens = themeTokens(theme);

      expectContrast(
        tokens.foreground,
        tokens.background,
        `${theme} foreground on background`
      );
      expectContrast(
        tokens.cardForeground,
        tokens.card,
        `${theme} card foreground on card`
      );
      expectContrast(
        tokens.mutedForeground,
        tokens.background,
        `${theme} muted foreground on background`
      );
      expectContrast(
        tokens.mutedForeground,
        tokens.card,
        `${theme} muted foreground on card`
      );
      expectContrast(
        tokens.primaryText,
        tokens.background,
        `${theme} primary text on background`
      );
      expectContrast(
        tokens.primaryText,
        tokens.card,
        `${theme} primary text on card`
      );
      expectContrast(
        tokens.primaryForeground,
        tokens.primary,
        `${theme} primary foreground on primary`
      );
      for (const [name, color] of [
        ["success", tokens.successForeground],
        ["warning", tokens.warningForeground],
        ["info", tokens.infoForeground],
      ] as const) {
        expectContrast(color, tokens.background, `${theme} ${name} on background`);
        expectContrast(color, tokens.card, `${theme} ${name} on card`);
      }
    }
  );

  it.each(LIGHT_THEMES)(
    "%s accent overrides keep readable primary text on light surfaces",
    (theme) => {
      const { background, card } = themeTokens(theme);

      for (const accent of ACCENTS) {
        const primaryText = accentPrimaryText(theme, accent);
        expectContrast(
          primaryText,
          background,
          `${theme}/${accent} primary text on background`
        );
        expectContrast(
          primaryText,
          card,
          `${theme}/${accent} primary text on card`
        );
      }
    }
  );
});
