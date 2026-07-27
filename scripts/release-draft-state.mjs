#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";
import { checksumFileName, expectedReleaseAssets } from "./release-artifacts.mjs";

export function validateReleaseState(release, { tag, version, expectDraft = true }) {
  if (release.draft !== expectDraft) {
    throw new Error(`Release draft state mismatch: expected ${expectDraft}, got ${release.draft}.`);
  }
  if (release.tag_name !== tag) {
    throw new Error(`Release tag mismatch: expected ${tag}, got ${release.tag_name ?? "<missing>"}.`);
  }
  const names = (release.assets ?? []).map((asset) => asset.name);
  const hasArm64 = names.some((name) => name.includes("_linux_arm64."));
  const expected = [...expectedReleaseAssets(version, hasArm64), checksumFileName].sort();
  if (new Set(names).size !== names.length || names.some((name) => !name)) {
    throw new Error("Release API returned duplicate or unnamed assets.");
  }
  const actual = [...names].sort();
  if (JSON.stringify(actual) !== JSON.stringify(expected)) {
    throw new Error(`Release asset inventory mismatch. Expected ${expected.join(", ")}; got ${actual.join(", ")}.`);
  }
  const empty = (release.assets ?? []).filter((asset) => !Number.isFinite(asset.size) || asset.size <= 0);
  if (empty.length > 0) {
    throw new Error(`Release contains empty asset(s): ${empty.map((asset) => asset.name).join(", ")}.`);
  }
  return release.id;
}

const entryPath = process.argv[1] ? path.resolve(process.argv[1]) : "";
if (entryPath && import.meta.url === pathToFileURL(entryPath).href) {
  const value = (flag) => {
    const index = process.argv.indexOf(flag);
    return index >= 0 ? process.argv[index + 1] : undefined;
  };
  const jsonPath = value("--release-json");
  const tag = value("--tag");
  const version = value("--version")?.replace(/^v/, "");
  if (!jsonPath || !tag || !version) {
    throw new Error("Usage: release-draft-state.mjs --release-json <path> --tag <tag> --version <version> [--public]");
  }
  const release = JSON.parse(fs.readFileSync(jsonPath, "utf8"));
  const id = validateReleaseState(release, {
    tag,
    version,
    expectDraft: !process.argv.includes("--public"),
  });
  if (process.env.GITHUB_OUTPUT) {
    fs.appendFileSync(process.env.GITHUB_OUTPUT, `release_id=${id}\n`);
  }
  console.log(`Release ${id} state and asset inventory verified.`);
}
