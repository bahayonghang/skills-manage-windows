/// <reference types="node" />

import { readdirSync, readFileSync } from "node:fs";
import { extname, join, relative, resolve } from "node:path";

import { describe, expect, it } from "vitest";

const ROOT = resolve(process.cwd(), "src");
const TEST_ROOT = resolve(ROOT, "test");

function walk(dir: string): string[] {
  const entries: string[] = [];
  const stack = [dir];
  while (stack.length > 0) {
    const current = stack.pop()!;
    let dirents: import("node:fs").Dirent[];
    try {
      dirents = readdirSync(current, {
        withFileTypes: true,
      }) as import("node:fs").Dirent[];
    } catch {
      continue;
    }
    for (const entry of dirents) {
      const full = join(current, entry.name);
      if (entry.isDirectory()) {
        stack.push(full);
      } else if (entry.isFile()) {
        const ext = extname(entry.name);
        if (ext === ".ts" || ext === ".tsx") {
          entries.push(full);
        }
      }
    }
  }
  return entries.sort();
}

const PRODUCTION_FILES = walk(ROOT).filter(
  (file) => !resolve(file).startsWith(TEST_ROOT)
);

function readProductionFile(file: string) {
  return {
    path: relative(process.cwd(), file).replace(/\\/g, "/"),
    content: readFileSync(file, "utf8"),
  };
}

// Match any arbitrary text-size or text-color utility emitted by Tailwind's
// arbitrary value syntax, including px/rem/em/calc/clamp and alpha-prefixed
// colors. The goal is to forbid the entire `text-[...]` family in production
// TS/TSX, so that future drift cannot bypass the semantic type ladder via
// arbitrary rem/color values.
const ARBITRARY_TEXT_UTILITY = /text-\[[^\]]+\]/;

// Focused sub-patterns for clearer failure messages.
const ARBITRARY_TEXT_SIZE = /text-\[(?:[0-9]+(?:\.[0-9]+)?(?:px|rem|em)|0?\.[0-9]+(?:px|rem|em)|calc\([^)]*\)|clamp\([^)]*\))\]/;
const ARBITRARY_TEXT_COLOR = /text-\[#[0-9a-fA-F]+\]/;

describe("typography contract", () => {
  it("scans at least one production TS/TSX file", () => {
    expect(PRODUCTION_FILES.length).toBeGreaterThan(0);
  });

  it("forbids arbitrary text-[...] utilities in production TS/TSX", () => {
    const offenders: string[] = [];
    for (const file of PRODUCTION_FILES) {
      const { path, content } = readProductionFile(file);
      const matches = content.match(new RegExp(ARBITRARY_TEXT_UTILITY, "g"));
      if (matches && matches.length > 0) {
        offenders.push(`${path}: ${matches.length} hit(s)`);
      }
    }
    expect(
      offenders,
      `Expected zero text-[...] hits, found:\n${offenders.join("\n")}`
    ).toEqual([]);
  });

  it("forbids arbitrary text-size utilities (px/rem/em/calc/clamp)", () => {
    const offenders: string[] = [];
    for (const file of PRODUCTION_FILES) {
      const { path, content } = readProductionFile(file);
      const matches = content.match(new RegExp(ARBITRARY_TEXT_SIZE, "g"));
      if (matches && matches.length > 0) {
        offenders.push(`${path}: ${matches.join(", ")}`);
      }
    }
    expect(
      offenders,
      `Expected zero arbitrary text-size hits, found:\n${offenders.join("\n")}`
    ).toEqual([]);
  });

  it("forbids arbitrary text-color utilities", () => {
    const offenders: string[] = [];
    for (const file of PRODUCTION_FILES) {
      const { path, content } = readProductionFile(file);
      const matches = content.match(new RegExp(ARBITRARY_TEXT_COLOR, "g"));
      if (matches && matches.length > 0) {
        offenders.push(`${path}: ${matches.join(", ")}`);
      }
    }
    expect(
      offenders,
      `Expected zero arbitrary text-color hits, found:\n${offenders.join("\n")}`
    ).toEqual([]);
  });

  it("uses semantic ui-meta and ui-micro tokens instead of arbitrary sizes", () => {
    const indexCss = readFileSync(
      resolve(process.cwd(), "src/index.css"),
      "utf8"
    );
    expect(indexCss).toContain("--text-ui-meta");
    expect(indexCss).toContain("--text-ui-micro");
  });

  it("does not introduce viewport-driven font sizes", () => {
    const indexCss = readFileSync(
      resolve(process.cwd(), "src/index.css"),
      "utf8"
    );
    // viewport units (vw/vh/vmin/vmax) are not part of the root-scale contract
    expect(indexCss).not.toMatch(/font-size:\s*[^;]*v(w|h|min|max)/);
  });
});


