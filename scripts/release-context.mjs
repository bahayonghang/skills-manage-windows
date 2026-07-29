#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

const SEMVER_TAG = /^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$/;

export function parseReleaseTag(tag) {
  const normalized = String(tag ?? "").trim();
  const match = SEMVER_TAG.exec(normalized);
  if (!match) {
    throw new Error(`Release tag must match v<semver>: ${normalized || "<missing>"}.`);
  }
  return { tag: normalized, version: normalized.slice(1) };
}

export function validateVersionSet({ tag, packageVersion, tauriVersion, cargoVersion }) {
  const context = parseReleaseTag(tag);
  for (const [label, actual] of [
    ["package.json", packageVersion],
    ["src-tauri/tauri.conf.json", tauriVersion],
    ["src-tauri/Cargo.toml/Cargo.lock", cargoVersion],
  ]) {
    if (actual !== context.version) {
      throw new Error(`${label} version mismatch: expected ${context.version}, got ${actual ?? "<missing>"}.`);
    }
  }
  return context;
}

function run(command, args, cwd) {
  return execFileSync(command, args, { cwd, encoding: "utf8", windowsHide: true }).trim();
}

export function resolveReleaseTag({ tag, cwd = process.cwd(), exec = run }) {
  const parsedTag = parseReleaseTag(tag);
  const sha = exec("git", ["rev-parse", `${parsedTag.tag}^{commit}`], cwd);
  exec("git", ["merge-base", "--is-ancestor", sha, "origin/main"], cwd);
  return { ...parsedTag, sha };
}

export function resolveReleaseContext({ tag, cwd = process.cwd(), exec = run }) {
  const releaseTag = resolveReleaseTag({ tag, cwd, exec });
  const { version } = releaseTag;
  const packageJson = JSON.parse(fs.readFileSync(path.join(cwd, "package.json"), "utf8"));
  const tauriConfig = JSON.parse(fs.readFileSync(path.join(cwd, "src-tauri", "tauri.conf.json"), "utf8"));

  const metadata = JSON.parse(
    exec(
      "cargo",
      ["metadata", "--manifest-path", "src-tauri/Cargo.toml", "--locked", "--no-deps", "--format-version", "1"],
      cwd,
    ),
  );
  const cargoPackage = metadata.packages.find((candidate) => candidate.name === "skillport");
  if (!cargoPackage) {
    throw new Error("Cargo metadata does not contain the skillport package.");
  }

  validateVersionSet({
    tag: parsedTag.tag,
    packageVersion: packageJson.version,
    tauriVersion: tauriConfig.version,
    cargoVersion: cargoPackage.version,
  });

  return { ...releaseTag, releaseName: `SkillPort v${version}` };
}

export function writeGitHubOutputs(context, outputPath) {
  const lines = [
    `tag=${context.tag}`,
    `version=${context.version}`,
    `sha=${context.sha}`,
    `release_name=${context.releaseName}`,
  ];
  fs.appendFileSync(outputPath, `${lines.join("\n")}\n`);
}

const entryPath = process.argv[1] ? path.resolve(process.argv[1]) : "";
if (entryPath && import.meta.url === pathToFileURL(entryPath).href) {
  const tagIndex = process.argv.indexOf("--tag");
  const tag = tagIndex >= 0 ? process.argv[tagIndex + 1] : process.env.RELEASE_TAG;
  if (process.argv.includes("--resolve-only")) {
    console.log(resolveReleaseTag({ tag }).sha);
    process.exit(0);
  }
  const context = resolveReleaseContext({ tag });
  if (process.env.GITHUB_OUTPUT) {
    writeGitHubOutputs(context, process.env.GITHUB_OUTPUT);
  }
  console.log(`Release context frozen: ${context.tag} -> ${context.sha}.`);
}
