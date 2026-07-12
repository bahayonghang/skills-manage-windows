import { describe, it, expect, beforeEach, afterEach } from "vitest";

import {
  applyDisplayFont,
  applyBodyFont,
  applyFontScale,
  applyFontPreferences,
  DEFAULT_FONT_PREFERENCES,
  DISPLAY_FONT_OPTIONS,
  BODY_FONT_OPTIONS,
  CHINESE_FALLBACK_OPTIONS,
  FONT_SCALE_OPTIONS,
} from "../lib/displayFont";

function resetHtml() {
  const html = document.documentElement;
  delete html.dataset.displayFont;
  delete html.dataset.bodyFont;
  html.style.removeProperty("--font-display");
  html.style.removeProperty("--font-body");
  html.style.removeProperty("--font-scale");
}

describe("displayFont", () => {
  beforeEach(() => resetHtml());
  afterEach(() => resetHtml());

  it("exposes 6 display font options and 5 body font options", () => {
    expect(DISPLAY_FONT_OPTIONS).toHaveLength(6);
    expect(BODY_FONT_OPTIONS).toHaveLength(5);
    expect(CHINESE_FALLBACK_OPTIONS.map((option) => option.key)).toEqual([
      "system",
      "sourceHanSerif",
      "custom",
    ]);
    expect(FONT_SCALE_OPTIONS.map((o) => o.value)).toEqual([0.875, 1, 1.125]);
  });

  it("applies preset display font via data attribute and clears inline style", () => {
    document.documentElement.style.setProperty("--font-display", "stale");
    applyDisplayFont("serif");
    expect(document.documentElement.dataset.displayFont).toBe("serif");
    expect(document.documentElement.style.getPropertyValue("--font-display"))
      .toBe("");
  });

  it("falls back to geist when custom display font has empty family", () => {
    applyDisplayFont("custom", "   ");
    expect(document.documentElement.dataset.displayFont).toBe("geist");
    expect(document.documentElement.style.getPropertyValue("--font-display"))
      .toBe("");
  });

  it("injects inline --font-display when custom display font has a name", () => {
    applyDisplayFont("custom", "Atkinson Hyperlegible");
    expect(document.documentElement.dataset.displayFont).toBe("custom");
    expect(
      document.documentElement.style.getPropertyValue("--font-display"),
    ).toContain("Atkinson Hyperlegible");
  });

  it("adds Source Han after the primary font without bundling an asset", () => {
    applyDisplayFont("geist", "", "sourceHanSerif");
    expect(
      document.documentElement.style.getPropertyValue("--font-display"),
    ).toBe(
      '"Geist Variable", "Source Han Serif SC VF", "思源宋体 VF", ui-sans-serif, system-ui, sans-serif',
    );
  });

  it("quotes a custom Chinese fallback as one family name", () => {
    applyBodyFont("jetbrains", "", "custom", 'Noto "Serif" CJK');
    expect(
      document.documentElement.style.getPropertyValue("--font-body"),
    ).toContain('"Noto \\"Serif\\" CJK"');
  });

  it("sanitizes control characters in custom family names", () => {
    applyDisplayFont("custom", 'Line\nBreak" Font');
    const value = document.documentElement.style.getPropertyValue("--font-display");
    expect(value).toContain('"Line Break\\" Font"');
    expect(value).not.toContain("\n");
  });

  it("treats an empty custom Chinese fallback as System", () => {
    applyDisplayFont("inter", "", "custom", "   ");
    expect(document.documentElement.dataset.displayFont).toBe("inter");
    expect(document.documentElement.style.getPropertyValue("--font-display"))
      .toBe("");
  });

  it("applies preset body font and custom fallback symmetrically", () => {
    applyBodyFont("inter");
    expect(document.documentElement.dataset.bodyFont).toBe("inter");

    applyBodyFont("custom", "Comic Sans MS");
    expect(document.documentElement.dataset.bodyFont).toBe("custom");
    expect(
      document.documentElement.style.getPropertyValue("--font-body"),
    ).toContain("Comic Sans MS");
  });

  it("clamps font scale into [0.75, 1.5]", () => {
    applyFontScale(2);
    expect(document.documentElement.style.getPropertyValue("--font-scale"))
      .toBe("1.5");
    applyFontScale(0);
    expect(document.documentElement.style.getPropertyValue("--font-scale"))
      .toBe("0.75");
    applyFontScale(1);
    expect(document.documentElement.style.getPropertyValue("--font-scale"))
      .toBe("1");
  });

  it("applyFontPreferences walks all three axes", () => {
    applyFontPreferences({
      ...DEFAULT_FONT_PREFERENCES,
      display: "inter",
      body: "geist",
      scale: 1.125,
    });
    expect(document.documentElement.dataset.displayFont).toBe("inter");
    expect(document.documentElement.dataset.bodyFont).toBe("geist");
    expect(document.documentElement.style.getPropertyValue("--font-scale"))
      .toBe("1.125");
  });

  it("keeps both Chinese fallback roles on System by default", () => {
    expect(DEFAULT_FONT_PREFERENCES).toMatchObject({
      display: "geist",
      displayChineseFallback: "system",
      displayChineseFallbackCustom: "",
      body: "jetbrains",
      bodyChineseFallback: "system",
      bodyChineseFallbackCustom: "",
    });
  });
});
