/// <reference types="node" />

import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";

type ReleasePreflightHelpers = {
  validateReleasePreflight: (args: {
    version: string;
    tag: string;
    assetDir: string;
    config: string;
    signingState: string;
    mode: string;
    repo: string;
  }) => { assetName: string };
};

// @ts-expect-error The release preflight helper is an ESM Node script outside the TS source tree.
const releasePreflightModule = await import("../../../scripts/release/release-preflight.mjs");
const { validateReleasePreflight } = releasePreflightModule as ReleasePreflightHelpers;

type LatestMetadata = {
  version: string;
  platforms: Record<string, { url: string; signature: string }>;
};

const roots: string[] = [];

function fixture() {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "skillport-preflight-"));
  roots.push(root);
  const assetName = "skillport_1.2.3_windows_x64_nsis.exe";
  const signature = "wrapped-signature";
  fs.writeFileSync(path.join(root, assetName), "installer");
  fs.writeFileSync(path.join(root, `${assetName}.sig`), signature);
  const signingState = path.join(root, "windows-signing.json");
  fs.writeFileSync(
    signingState,
    JSON.stringify({
      authenticode: "not-configured",
      updaterSignature: "valid",
      files: {
        "skillport.exe": { status: "NotSigned" },
        nsis: { status: "NotSigned" },
        msi: { status: "NotSigned" },
      },
    }),
  );
  const config = path.join(root, "config.json");
  fs.writeFileSync(
    config,
    JSON.stringify({
      bundle: { createUpdaterArtifacts: false },
      plugins: { updater: { pubkey: "real-wrapped-public-key" } },
    }),
  );
  const url = `https://github.com/example/repo/releases/download/v1.2.3/${assetName}`;
  fs.writeFileSync(
    path.join(root, "latest.json"),
    JSON.stringify({
      version: "1.2.3",
      platforms: {
        "windows-x86_64-nsis": { url, signature },
        "windows-x86_64": { url, signature },
      },
    }),
  );
  return {
    root,
    latestPath: path.join(root, "latest.json"),
    args: { version: "1.2.3", tag: "v1.2.3", assetDir: root, config, signingState, mode: "rehearsal", repo: "example/repo" },
  };
}

afterEach(() => {
  for (const root of roots.splice(0)) fs.rmSync(root, { recursive: true, force: true });
});

describe("release preflight metadata", () => {
  it("accepts matching updater metadata", () => {
    const { args } = fixture();
    expect(validateReleasePreflight(args).assetName).toBe("skillport_1.2.3_windows_x64_nsis.exe");
  });

  const invalidCases: Array<[string, (value: LatestMetadata) => void]> = [
    ["wrong version", (value) => { value.version = "9.9.9"; }],
    ["missing platform", (value) => { delete value.platforms["windows-x86_64"]; }],
    ["wrong url", (value) => { value.platforms["windows-x86_64"].url = "https://invalid"; }],
    ["wrong signature", (value) => { value.platforms["windows-x86_64-nsis"].signature = "bad"; }],
    ["unexpected platform", (value) => {
      value.platforms["linux-x86_64"] = { url: "https://invalid", signature: "bad" };
    }],
  ];

  it.each(invalidCases)("rejects %s", (_name, mutate) => {
    const { args, latestPath } = fixture();
    const value = JSON.parse(fs.readFileSync(latestPath, "utf8")) as LatestMetadata;
    mutate(value);
    fs.writeFileSync(latestPath, JSON.stringify(value));
    expect(() => validateReleasePreflight(args)).toThrow();
  });

  it("rejects malformed latest JSON and duplicate NSIS candidates", () => {
    const malformed = fixture();
    fs.writeFileSync(malformed.latestPath, "{");
    expect(() => validateReleasePreflight(malformed.args)).toThrow();

    const duplicate = fixture();
    fs.writeFileSync(path.join(duplicate.root, "duplicate_windows_x64_nsis.exe"), "stale");
    expect(() => validateReleasePreflight(duplicate.args)).toThrow(/exactly one/);
  });
});
