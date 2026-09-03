#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

const SEMVER_TAG = /^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$/;
const COMMIT_SHA = /^[0-9a-f]{40}$/i;
export const RESOLVER_SUBPROCESS_TIMEOUT_MS = 120_000;
export const RESOLVER_OUTPUT_CAP_BYTES = 1_048_576;

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

function isTimeoutError(error) {
  if (!error || typeof error !== "object") {
    return false;
  }
  return error.code === "ETIMEDOUT" || error.error?.code === "ETIMEDOUT";
}

function isMaxBufferError(error) {
  if (!error || typeof error !== "object") {
    return false;
  }
  const code = error.code ?? error.error?.code;
  return code === "ERR_CHILD_PROCESS_STDIO_MAXBUFFER" || code === "ENOBUFS";
}

export function run(command, args, cwd, options = {}) {
  const timeoutMs = options.timeoutMs ?? RESOLVER_SUBPROCESS_TIMEOUT_MS;
  const maxBuffer = options.maxBuffer ?? RESOLVER_OUTPUT_CAP_BYTES;
  try {
    return execFileSync(command, args, {
      cwd,
      encoding: "utf8",
      windowsHide: true,
      timeout: timeoutMs,
      maxBuffer,
    }).trim();
  } catch (error) {
    if (isTimeoutError(error)) {
      throw new Error(`release-context: ${command} timed out after ${timeoutMs}ms.`);
    }
    if (isMaxBufferError(error)) {
      throw new Error(`release-context: ${command} output exceeded ${maxBuffer} bytes.`);
    }
    throw error;
  }
}

export function resolveReleaseTag({ tag, cwd = process.cwd(), exec = run }) {
  const parsedTag = parseReleaseTag(tag);
  const sha = exec("git", ["rev-parse", `${parsedTag.tag}^{commit}`], cwd);
  exec("git", ["merge-base", "--is-ancestor", sha, "origin/main"], cwd);
  return { ...parsedTag, sha };
}

export function resolveRehearsalRef({ sha, cwd = process.cwd(), exec = run }) {
  const normalized = String(sha ?? "").trim();
  if (!COMMIT_SHA.test(normalized)) {
    throw new Error("Rehearsal ref must be an exact 40-character commit SHA.");
  }
  const resolved = exec("git", ["rev-parse", `${normalized}^{commit}`], cwd);
  if (resolved.toLowerCase() !== normalized.toLowerCase()) {
    throw new Error("Rehearsal ref did not resolve to the requested commit SHA.");
  }
  exec("git", ["merge-base", "--is-ancestor", resolved, "origin/main"], cwd);
  return { sha: resolved };
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
    tag: releaseTag.tag,
    packageVersion: packageJson.version,
    tauriVersion: tauriConfig.version,
    cargoVersion: cargoPackage.version,
  });

  return { ...releaseTag, releaseName: `SkillPort v${version}` };
}

export function resolveRehearsalContext({ sha, cwd = process.cwd(), exec = run }) {
  const rehearsal = resolveRehearsalRef({ sha, cwd, exec });
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
  if (!cargoPackage) throw new Error("Cargo metadata does not contain the skillport package.");
  const tag = `v${packageJson.version}`;
  validateVersionSet({
    tag,
    packageVersion: packageJson.version,
    tauriVersion: tauriConfig.version,
    cargoVersion: cargoPackage.version,
  });
  return { tag, version: packageJson.version, sha: rehearsal.sha, releaseName: `SkillPort v${packageJson.version}` };
}

export function writeGitHubOutputs(context, outputPath) {
  const lines = [
    `tag=${context.tag}`,
    `version=${context.version}`,
    `sha=${context.sha}`,
    `release_name=${context.releaseName}`,
    `mode=${context.mode}`,
  ];
  fs.appendFileSync(outputPath, `${lines.join("\n")}\n`);
}

const entryPath = process.argv[1] ? path.resolve(process.argv[1]) : "";
if (entryPath && import.meta.url === pathToFileURL(entryPath).href) {
  const tagIndex = process.argv.indexOf("--tag");
  const tag = tagIndex >= 0 ? process.argv[tagIndex + 1] : process.env.RELEASE_TAG;
  const shaIndex = process.argv.indexOf("--sha");
  const sha = shaIndex >= 0 ? process.argv[shaIndex + 1] : process.env.REHEARSAL_REF;
  const modeIndex = process.argv.indexOf("--mode");
  const mode = modeIndex >= 0 ? process.argv[modeIndex + 1] : process.env.RELEASE_MODE || "publish";
  if (!new Set(["rehearsal", "publish"]).has(mode)) throw new Error("Release mode must be rehearsal or publish.");
  if (process.argv.includes("--resolve-only")) {
    console.log(mode === "rehearsal" ? resolveRehearsalRef({ sha }).sha : resolveReleaseTag({ tag }).sha);
    process.exit(0);
  }
  const context = mode === "rehearsal" ? resolveRehearsalContext({ sha }) : resolveReleaseContext({ tag });
  context.mode = mode;
  if (process.env.GITHUB_OUTPUT) {
    writeGitHubOutputs(context, process.env.GITHUB_OUTPUT);
  }
  console.log(`Release context frozen: ${context.tag} -> ${context.sha}.`);
}
