/// <reference types="node" />

import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";

type SequentialTestHelpers = {
  collectTestFiles: (rootDir: string, baseDir?: string) => string[];
};

// @ts-expect-error The sequential runner is an ESM Node script outside the TS source tree.
const { collectTestFiles } = (await import("../../../scripts/run-vitest-sequential.mjs")) as SequentialTestHelpers;

const tempDirs: string[] = [];

afterEach(() => {
  for (const tempDir of tempDirs.splice(0)) {
    rmSync(tempDir, { recursive: true, force: true });
  }
});

describe("sequential Vitest discovery", () => {
  it("recursively finds test files with normalized, deterministic paths", () => {
    const rootDir = mkdtempSync(join(tmpdir(), "skillport-test-discovery-"));
    tempDirs.push(rootDir);
    mkdirSync(join(rootDir, "nested", "deeper"), { recursive: true });
    writeFileSync(join(rootDir, "z.spec.ts"), "");
    writeFileSync(join(rootDir, "nested", "a.test.tsx"), "");
    writeFileSync(join(rootDir, "nested", "deeper", "b.test.js"), "");
    writeFileSync(join(rootDir, "nested", "helper.ts"), "");

    expect(collectTestFiles(rootDir, rootDir)).toEqual([
      "nested/a.test.tsx",
      "nested/deeper/b.test.js",
      "z.spec.ts",
    ]);
  });
});
