/// <reference types="node" />

import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const indexCss = readFileSync("src/index.css", "utf8");

describe("font contract", () => {
  it("routes global font tokens through display/body preferences", () => {
    expect(indexCss).toContain("--font-heading: var(--font-display);");
    expect(indexCss).toContain("--font-sans: var(--font-body);");
    expect(indexCss).toContain(
      "--font-mono: 'JetBrains Mono Variable', ui-monospace, monospace;"
    );
  });

  it("applies font scale to the html root", () => {
    expect(indexCss).toContain("font-size: calc(16px * var(--font-scale));");
  });
});
