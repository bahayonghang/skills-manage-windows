/// <reference types="node" />

import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";

type ReleaseArtifactHelpers = {
  buildChecksumManifest: (assetDir: string, version: string) => string;
  expectedReleaseAssets: (version: string, includeArm64?: boolean) => string[];
  validateArtifactInventory: (names: string[], version: string) => string[];
  verifyChecksumManifest: (assetDir: string, version: string) => void;
  writeChecksumManifest: (assetDir: string, version: string) => string;
};

// @ts-expect-error The release artifact helper is an ESM Node script outside the TS source tree.
const releaseArtifactModule = await import("../../../scripts/release-artifacts.mjs");
const {
  buildChecksumManifest,
  expectedReleaseAssets,
  validateArtifactInventory,
  verifyChecksumManifest,
  writeChecksumManifest,
} = releaseArtifactModule as ReleaseArtifactHelpers;

const roots: string[] = [];

function fixture(names = expectedReleaseAssets("1.2.3")) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "skillport-release-"));
  roots.push(root);
  for (const name of names) fs.writeFileSync(path.join(root, name), `fixture:${name}\n`);
  return root;
}

afterEach(() => {
  for (const root of roots.splice(0)) fs.rmSync(root, { recursive: true, force: true });
});

describe("release artifact inventory", () => {
  it("accepts required assets and a complete optional arm64 set", () => {
    expect(validateArtifactInventory(expectedReleaseAssets("1.2.3"), "1.2.3")).toHaveLength(11);
    expect(validateArtifactInventory(expectedReleaseAssets("1.2.3", true), "1.2.3")).toHaveLength(14);
  });

  it("rejects missing duplicate unexpected and partial optional assets", () => {
    const required = expectedReleaseAssets("1.2.3");
    expect(() => validateArtifactInventory(required.slice(1), "1.2.3")).toThrow(/Missing:/);
    expect(() => validateArtifactInventory([...required, required[0]], "1.2.3")).toThrow(/Duplicate/);
    expect(() => validateArtifactInventory([...required, "stale.zip"], "1.2.3")).toThrow(/Unexpected:/);
    expect(() =>
      validateArtifactInventory([...required, "skillport_1.2.3_linux_arm64.deb"], "1.2.3"),
    ).toThrow(/complete DEB\/RPM\/AppImage set/);
  });

  it("writes deterministic checksums and detects changed bytes", () => {
    const root = fixture();
    const first = buildChecksumManifest(root, "1.2.3");
    writeChecksumManifest(root, "1.2.3");
    expect(() => verifyChecksumManifest(root, "1.2.3")).not.toThrow();
    expect(buildChecksumManifest(root, "1.2.3")).toBe(first);

    fs.appendFileSync(path.join(root, expectedReleaseAssets("1.2.3")[0]), "tampered");
    expect(() => verifyChecksumManifest(root, "1.2.3")).toThrow(/does not match/);
  });
});
