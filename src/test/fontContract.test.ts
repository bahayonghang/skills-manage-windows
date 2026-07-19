/// <reference types="node" />

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const indexCss = readFileSync("src/index.css", "utf8");

describe("font contract", () => {
  it("routes global font tokens through display/body preferences", () => {
    expect(indexCss).toContain("--font-heading: var(--font-display);");
    expect(indexCss).toContain("--font-sans: var(--font-body);");
    expect(indexCss).toContain(
      '--font-mono: "JetBrains Mono Variable", ui-monospace, monospace;',
    );
  });

  it("applies font scale to the html root", () => {
    expect(indexCss).toContain("font-size: calc(16px * var(--font-scale));");
  });

  it("does not bundle fallback font assets", () => {
    expect(indexCss).not.toContain("@font-face");
    expect(indexCss).not.toContain("SourceHanSerif");
  });

  it("defines semantic ui-meta and ui-micro sizes in rem on the theme boundary", () => {
    // Tailwind 4 maps --text-* theme vars to text-* utilities. They must use
    // rem so they inherit the root --font-scale (0.875 / 1 / 1.125).
    expect(indexCss).toMatch(/--text-ui-meta:\s*0\.6875rem/);
    expect(indexCss).toMatch(/--text-ui-micro:\s*0\.625rem/);
  });

  it("does not override line-height inside the semantic size tokens", () => {
    // The size tokens only own font-size; layout components keep owning leading.
    const metaLine = indexCss
      .split("\n")
      .find((line) => line.includes("--text-ui-meta"));
    const microLine = indexCss
      .split("\n")
      .find((line) => line.includes("--text-ui-micro"));
    expect(metaLine).toBeDefined();
    expect(microLine).toBeDefined();
    expect(metaLine).not.toContain("line-height");
    expect(microLine).not.toContain("line-height");
  });

  it("keeps the semantic size ladder ordered micro < meta < xs", () => {
    // Static proof that the small-type ladder is monotonic and that every
    // step is rem-based so the root --font-scale drives all of them together.
    const micro = 0.625;
    const meta = 0.6875;
    // text-xs is Tailwind's 0.75rem.
    const xs = 0.75;
    expect(micro).toBeLessThan(meta);
    expect(meta).toBeLessThan(xs);
    expect(indexCss).toMatch(/--text-ui-micro:\s*0\.625rem/);
    expect(indexCss).toMatch(/--text-ui-meta:\s*0\.6875rem/);
  });

  it("drives every semantic size through the root font-scale, not viewport", () => {
    // The root owns scale via calc(16px * var(--font-scale)); the semantic
    // tokens are plain rem, so 0.875 / 1 / 1.125 move them in lockstep.
    expect(indexCss).toContain("font-size: calc(16px * var(--font-scale));");
    // No viewport-driven font-size anywhere in the theme.
    expect(indexCss).not.toMatch(/--text-[a-z-]+:\s*[^;]*v(w|h|min|max)/);
  });
});
