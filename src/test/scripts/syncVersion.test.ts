/// <reference types="node" />

import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";

type SyncVersionHelpers = {
  parseSyncVersionArgs: (args: string[]) => { check: boolean };
  syncVersion: (options: {
    check?: boolean;
    log?: (message: string) => void;
    rootDir: string;
  }) => { changedFiles: string[]; version: string };
};

// @ts-expect-error The version synchronizer is an ESM Node script outside the TS source tree.
const { parseSyncVersionArgs, syncVersion } = (await import("../../../scripts/sync-version.mjs")) as SyncVersionHelpers;

const tempDirs: string[] = [];

function createFixture(version = "1.2.3") {
  const rootDir = mkdtempSync(join(tmpdir(), "skillport-version-"));
  tempDirs.push(rootDir);
  mkdirSync(join(rootDir, "src-tauri"), { recursive: true });
  writeFileSync(join(rootDir, "package.json"), JSON.stringify({ version }, null, 2));
  writeFileSync(join(rootDir, "src-tauri", "tauri.conf.json"), '{"version":"0.0.1"}\n');
  writeFileSync(
    join(rootDir, "src-tauri", "Cargo.toml"),
    '[package]\nname = "skillport"\nversion = "0.0.1"\n',
  );
  writeFileSync(
    join(rootDir, "src-tauri", "Cargo.lock"),
    '[[package]]\nname = "skillport"\nversion = "0.0.1"\n',
  );
  return rootDir;
}

function readTargets(rootDir: string) {
  return [
    "src-tauri/tauri.conf.json",
    "src-tauri/Cargo.toml",
    "src-tauri/Cargo.lock",
  ].map((relativePath) => readFileSync(join(rootDir, relativePath), "utf8"));
}

afterEach(() => {
  for (const tempDir of tempDirs.splice(0)) {
    rmSync(tempDir, { recursive: true, force: true });
  }
});

describe("version metadata synchronization", () => {
  it("keeps just ci read-only and exposes explicit check and write recipes", () => {
    const justfile = readFileSync("justfile", "utf8");
    const packageJson = JSON.parse(readFileSync("package.json", "utf8")) as {
      scripts: Record<string, string>;
    };

    expect(justfile).toMatch(/version-check:\r?\n\s+pnpm version:check/);
    expect(justfile).toMatch(/sync-version:\r?\n\s+node scripts\/sync-version\.mjs/);
    expect(justfile).toMatch(/ci:\r?\n\s+node scripts\/run-ci\.mjs/);
    expect(justfile).not.toContain("ci: sync-version");
    expect(packageJson.scripts["version:check"]).toBe("node scripts/sync-version.mjs --check");
  });

  it("reports every drift in check mode without writing files", () => {
    const rootDir = createFixture();
    const before = readTargets(rootDir);

    expect(() => syncVersion({ rootDir, check: true, log: () => undefined })).toThrowError(
      /src-tauri\/tauri\.conf\.json, src-tauri\/Cargo\.toml, src-tauri\/Cargo\.lock/,
    );
    expect(readTargets(rootDir)).toEqual(before);
  });

  it("keeps the explicit write mode compatible", () => {
    const rootDir = createFixture();

    expect(syncVersion({ rootDir, log: () => undefined })).toEqual({
      changedFiles: [
        "src-tauri/tauri.conf.json",
        "src-tauri/Cargo.toml",
        "src-tauri/Cargo.lock",
      ],
      version: "1.2.3",
    });
    expect(readTargets(rootDir)).toEqual([
      '{"version":"1.2.3"}\n',
      '[package]\nname = "skillport"\nversion = "1.2.3"\n',
      '[[package]]\nname = "skillport"\nversion = "1.2.3"\n',
    ]);
  });

  it("passes a clean check without modifying metadata", () => {
    const rootDir = createFixture();
    syncVersion({ rootDir, log: () => undefined });
    const before = readTargets(rootDir);

    expect(syncVersion({ rootDir, check: true, log: () => undefined })).toEqual({
      changedFiles: [],
      version: "1.2.3",
    });
    expect(readTargets(rootDir)).toEqual(before);
  });

  it("parses only the supported check flag", () => {
    expect(parseSyncVersionArgs([])).toEqual({ check: false });
    expect(parseSyncVersionArgs(["--check"])).toEqual({ check: true });
    expect(() => parseSyncVersionArgs(["--write"])).toThrowError(/Unknown argument: --write/);
  });

  it("fails when package.json has no valid version", () => {
    const rootDir = createFixture("  ");
    expect(() => syncVersion({ rootDir, check: true })).toThrowError(
      "package.json is missing a valid version.",
    );
  });
});
