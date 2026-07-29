#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

const PLACEHOLDER_PUBKEY = "TAURI_UPDATER_PUBLIC_KEY_PLACEHOLDER_REPLACE_IN_RELEASE_CONFIG";

export function parseArgs(argv) {
  const args = {
    version: process.env.RELEASE_VERSION || "",
    tag: process.env.RELEASE_TAG || "",
    assetDir: "release-assets",
    config: "release-updater-config.json",
    repo: process.env.GITHUB_REPOSITORY || "bahayonghang/skills-manage-windows",
  };

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    const next = argv[index + 1];
    if (arg === "--version" && next) {
      args.version = next.replace(/^v/, "");
      index += 1;
    } else if (arg === "--tag" && next) {
      args.tag = next;
      index += 1;
    } else if (arg === "--asset-dir" && next) {
      args.assetDir = next;
      index += 1;
    } else if (arg === "--config" && next) {
      args.config = next;
      index += 1;
    } else if (arg === "--repo" && next) {
      args.repo = next;
      index += 1;
    } else if (arg === "--help" || arg === "-h") {
      console.log(
        "Usage: node scripts/release-preflight.mjs --version <version> --tag <tag> [--config release-updater-config.json] [--asset-dir release-assets] [--repo owner/repo]",
      );
      process.exit(0);
    }
  }

  if (!args.version) {
    throw new Error("Missing release version. Pass --version or set RELEASE_VERSION.");
  }
  if (!args.tag) {
    args.tag = `v${args.version}`;
  }

  return args;
}

function readJson(filePath, label) {
  if (!fs.existsSync(filePath)) {
    throw new Error(`${label} not found: ${filePath}`);
  }
  return JSON.parse(fs.readFileSync(filePath, "utf8"));
}

function ensureNonPlaceholderPubkey(pubkey) {
  if (!pubkey || !pubkey.trim()) {
    throw new Error("Release updater config is missing plugins.updater.pubkey.");
  }
  if (pubkey.trim() === PLACEHOLDER_PUBKEY || /placeholder/i.test(pubkey)) {
    throw new Error("Release updater config still contains the placeholder updater pubkey.");
  }
}

function findNsisAsset(assetDir) {
  const files = fs.readdirSync(assetDir).filter((file) => /windows_x64_nsis\.exe$/i.test(file));
  if (files.length !== 1) {
    if (files.length > 1) {
      throw new Error(`Expected exactly one Windows x64 NSIS asset, found ${files.length}.`);
    }
    throw new Error(`Windows x64 NSIS asset not found in ${assetDir}.`);
  }
  return files[0];
}

function readSignature(assetDir, assetName) {
  const sigPath = path.join(assetDir, `${assetName}.sig`);
  if (!fs.existsSync(sigPath)) {
    throw new Error(`Updater signature not found: ${sigPath}`);
  }
  const signature = fs.readFileSync(sigPath, "utf8").trim();
  if (!signature) {
    throw new Error(`Updater signature is empty: ${sigPath}`);
  }
  return signature;
}

export function validateReleasePreflight(args) {
  if (args.tag !== `v${args.version}`) {
    throw new Error(`Release tag/version mismatch: expected v${args.version}, got ${args.tag}.`);
  }
  const config = readJson(args.config, "Release updater config");
  if (config?.bundle?.createUpdaterArtifacts !== true) {
    throw new Error("Release updater config must set bundle.createUpdaterArtifacts=true.");
  }
  ensureNonPlaceholderPubkey(config?.plugins?.updater?.pubkey);

  const assetName = findNsisAsset(args.assetDir);
  const signature = readSignature(args.assetDir, assetName);
  const latestJsonPath = path.join(args.assetDir, "latest.json");
  const latest = readJson(latestJsonPath, "latest.json");

  if (!latest || typeof latest !== "object" || Array.isArray(latest)) {
    throw new Error("latest.json must contain a JSON object.");
  }
  if (typeof latest.version !== "string" || latest.version !== args.version) {
    throw new Error(`latest.json version mismatch: expected ${args.version}, got ${latest.version ?? "<missing>"}.`);
  }
  if (!latest.platforms || typeof latest.platforms !== "object" || Array.isArray(latest.platforms)) {
    throw new Error("latest.json platforms must be an object.");
  }

  const platformKeys = ["windows-x86_64-nsis", "windows-x86_64"];
  const actualPlatformKeys = Object.keys(latest.platforms).sort();
  if (JSON.stringify(actualPlatformKeys) !== JSON.stringify([...platformKeys].sort())) {
    throw new Error(`latest.json platform keys mismatch: ${actualPlatformKeys.join(", ")}.`);
  }
  const expectedUrl = `https://github.com/${args.repo}/releases/download/${args.tag}/${assetName}`;
  for (const platformKey of platformKeys) {
    const platform = latest?.platforms?.[platformKey];
    if (!platform || typeof platform !== "object" || Array.isArray(platform)) {
      throw new Error(`latest.json is missing platforms.${platformKey}.`);
    }
    if (platform.url !== expectedUrl) {
      throw new Error(
        `latest.json ${platformKey} url mismatch: expected ${expectedUrl}, got ${platform.url ?? "<missing>"}.`,
      );
    }
    if (platform.signature !== signature) {
      throw new Error(`latest.json ${platformKey} signature does not match ${assetName}.sig.`);
    }
  }

  return {
    assetName,
    latestJsonPath,
    publicKey: config.plugins.updater.pubkey,
  };
}

export function verifyUpdaterSignature(args, assetName) {
  const result = spawnSync(
    "cargo",
    [
      "run",
      "--manifest-path",
      "src-tauri/Cargo.toml",
      "--locked",
      "--quiet",
      "--bin",
      "release-signature-verifier",
      "--",
      "--installer",
      path.join(args.assetDir, assetName),
      "--signature",
      path.join(args.assetDir, `${assetName}.sig`),
      "--config",
      args.config,
    ],
    { encoding: "utf8", windowsHide: true },
  );
  if (result.error) {
    throw new Error(`Failed to start release signature verifier: ${result.error.message}`);
  }
  if (result.status !== 0) {
    throw new Error(`Updater signature verification failed: ${result.stderr.trim() || "unknown verifier error"}`);
  }
}

const entryPath = process.argv[1] ? path.resolve(process.argv[1]) : "";
if (entryPath && import.meta.url === pathToFileURL(entryPath).href) {
  const args = parseArgs(process.argv.slice(2));
  const result = validateReleasePreflight(args);
  verifyUpdaterSignature(args, result.assetName);
  console.log(
    `Release preflight passed for ${args.tag}: ${result.assetName} + ${path.relative(process.cwd(), result.latestJsonPath)}`,
  );
}
