/// <reference types="node" />

import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";

type GenerateLatestJsonHelpers = {
  parseArgs: (argv: string[]) => {
    version: string;
    tag: string;
    assetDir: string;
    repo: string;
  };
  writeLatestJson: (args: {
    version: string;
    tag: string;
    assetDir: string;
    repo: string;
  }) => string;
};

type PrepareReleaseBodyHelpers = {
  parseArgs: (argv: string[]) => { version: string; output: string };
  resolveReleaseBody: (version: string) => { body: string; source: string };
  writeReleaseBody: (args: { version: string; output: string }) => string;
};

// @ts-expect-error The release helper is an ESM Node script outside the TS source tree.
const generateLatestJsonModule = await import("../../../scripts/release/generate-latest-json.mjs");
const { parseArgs: parseLatestJsonArgs, writeLatestJson } =
  generateLatestJsonModule as GenerateLatestJsonHelpers;

// @ts-expect-error The release helper is an ESM Node script outside the TS source tree.
const prepareReleaseBodyModule = await import("../../../scripts/release/prepare-release-body.mjs");
const { parseArgs, resolveReleaseBody, writeReleaseBody } =
  prepareReleaseBodyModule as PrepareReleaseBodyHelpers;

const roots: string[] = [];
const originalCwd = process.cwd();

function ownedDir(prefix: string) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), prefix));
  roots.push(root);
  return root;
}

afterEach(() => {
  process.chdir(originalCwd);
  for (const root of roots.splice(0)) fs.rmSync(root, { recursive: true, force: true });
});

describe("generate-latest-json fail-closed", () => {
  const argsFor = (assetDir: string) => ({
    version: "1.2.3",
    tag: "v1.2.3",
    assetDir,
    repo: "example/repo",
  });

  it("rejects a missing NSIS asset and does not write latest.json", () => {
    const assetDir = ownedDir("skillport-latest-missing-");
    expect(() => writeLatestJson(argsFor(assetDir))).toThrow(/not found/);
    expect(fs.existsSync(path.join(assetDir, "latest.json"))).toBe(false);
  });

  it("rejects more than one NSIS match and does not write latest.json", () => {
    const assetDir = ownedDir("skillport-latest-dup-");
    fs.writeFileSync(path.join(assetDir, "skillport_1.2.3_windows_x64_nsis.exe"), "a");
    fs.writeFileSync(path.join(assetDir, "skillport_1.2.4_windows_x64_nsis.exe"), "b");
    fs.writeFileSync(path.join(assetDir, "skillport_1.2.3_windows_x64_nsis.exe.sig"), "sig");
    fs.writeFileSync(path.join(assetDir, "skillport_1.2.4_windows_x64_nsis.exe.sig"), "sig");
    expect(() => writeLatestJson(argsFor(assetDir))).toThrow(/matched 2 files/);
    expect(fs.existsSync(path.join(assetDir, "latest.json"))).toBe(false);
  });

  it("rejects a missing .sig and does not write latest.json", () => {
    const assetDir = ownedDir("skillport-latest-nosig-");
    fs.writeFileSync(path.join(assetDir, "skillport_1.2.3_windows_x64_nsis.exe"), "installer");
    expect(() => writeLatestJson(argsFor(assetDir))).toThrow(/Updater signature not found/);
    expect(fs.existsSync(path.join(assetDir, "latest.json"))).toBe(false);
  });

  it("rejects an empty .sig and does not write latest.json", () => {
    const assetDir = ownedDir("skillport-latest-emptysig-");
    const nsis = path.join(assetDir, "skillport_1.2.3_windows_x64_nsis.exe");
    fs.writeFileSync(nsis, "installer");
    fs.writeFileSync(`${nsis}.sig`, "  \n");
    expect(() => writeLatestJson(argsFor(assetDir))).toThrow(/Updater signature is empty/);
    expect(fs.existsSync(path.join(assetDir, "latest.json"))).toBe(false);
  });

  it("strips a v prefix, defaults the tag, and writes dual Windows updater keys", () => {
    const assetDir = ownedDir("skillport-latest-ok-");
    const nsisName = "skillport_1.2.3_windows_x64_nsis.exe";
    const nsis = path.join(assetDir, nsisName);
    fs.writeFileSync(nsis, "installer");
    fs.writeFileSync(`${nsis}.sig`, "  SIGNATURE  \n");
    const previousEnv = {
      RELEASE_PUB_DATE: process.env.RELEASE_PUB_DATE,
      RELEASE_VERSION: process.env.RELEASE_VERSION,
      RELEASE_TAG: process.env.RELEASE_TAG,
      GITHUB_REPOSITORY: process.env.GITHUB_REPOSITORY,
    };
    process.env.RELEASE_PUB_DATE = "2026-08-31T00:00:00.000Z";
    delete process.env.RELEASE_VERSION;
    delete process.env.RELEASE_TAG;
    delete process.env.GITHUB_REPOSITORY;

    try {
      const args = parseLatestJsonArgs([
        "--version",
        "v1.2.3",
        "--asset-dir",
        assetDir,
        "--repo",
        "example/repo",
      ]);
      expect(args).toEqual({
        version: "1.2.3",
        tag: "v1.2.3",
        assetDir,
        repo: "example/repo",
      });

      const output = writeLatestJson(args);
      expect(output).toBe(path.join(assetDir, "latest.json"));
      const metadata = JSON.parse(fs.readFileSync(output, "utf8")) as {
        version: string;
        pub_date: string;
        platforms: Record<string, { signature: string; url: string }>;
      };
      const windowsPlatform = {
        signature: "SIGNATURE",
        url: `https://github.com/example/repo/releases/download/v1.2.3/${nsisName}`,
      };
      expect(metadata.version).toBe("1.2.3");
      expect(metadata.pub_date).toBe("2026-08-31T00:00:00.000Z");
      expect(metadata.platforms["windows-x86_64-nsis"]).toEqual(windowsPlatform);
      expect(metadata.platforms["windows-x86_64"]).toEqual(windowsPlatform);
    } finally {
      for (const [key, value] of Object.entries(previousEnv)) {
        if (value === undefined) {
          delete process.env[key];
        } else {
          process.env[key] = value;
        }
      }
    }
  });
});

describe("prepare-release-body", () => {
  it("prefers exact version notes over the series file", () => {
    const root = ownedDir("skillport-notes-exact-");
    fs.mkdirSync(path.join(root, "release-notes"));
    fs.writeFileSync(path.join(root, "release-notes", "1.2.md"), "series notes\n");
    fs.writeFileSync(path.join(root, "release-notes", "1.2.3.md"), "exact version notes\n");
    process.chdir(root);

    const resolved = resolveReleaseBody("1.2.3");
    expect(resolved.source).toBe(path.join("release-notes", "1.2.3.md"));
    expect(resolved.body).toBe("exact version notes\n");
  });

  it("falls back to series major.minor.md when the exact file is missing", () => {
    const root = ownedDir("skillport-notes-series-");
    fs.mkdirSync(path.join(root, "release-notes"));
    fs.writeFileSync(path.join(root, "release-notes", "1.2.md"), "series notes\n");
    process.chdir(root);

    const resolved = resolveReleaseBody("1.2.3");
    expect(resolved.source).toBe(path.join("release-notes", "1.2.md"));
    expect(resolved.body).toBe("series notes\n");
  });

  it("uses generated fallback when neither notes file exists", () => {
    const root = ownedDir("skillport-notes-fallback-");
    process.chdir(root);

    const resolved = resolveReleaseBody("1.2.3");
    expect(resolved.source).toBe("generated fallback");
    expect(resolved.body).toContain("SkillPort 1.2.3");
    expect(resolved.body).toContain("release-notes");
  });

  it("writes to the --output path", () => {
    const root = ownedDir("skillport-notes-output-");
    fs.mkdirSync(path.join(root, "release-notes"));
    fs.writeFileSync(path.join(root, "release-notes", "1.2.3.md"), "exact version notes\n");
    process.chdir(root);
    const output = path.join(root, "custom-body.md");

    expect(parseArgs(["--version", "1.2.3", "--output", output])).toEqual({
      version: "1.2.3",
      output,
    });
    const source = writeReleaseBody({ version: "1.2.3", output });
    expect(source).toBe(path.join("release-notes", "1.2.3.md"));
    expect(fs.readFileSync(output, "utf8")).toBe("exact version notes\n");
  });
});
