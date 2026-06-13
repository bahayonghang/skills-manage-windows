#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

const PLACEHOLDER_PUBKEY = "TAURI_UPDATER_PUBLIC_KEY_PLACEHOLDER_REPLACE_IN_RELEASE_CONFIG";

export function parseArgs(argv) {
  const args = {
    version: process.env.RELEASE_VERSION || process.env.GITHUB_REF_NAME?.replace(/^v/, "") || "",
    tag: process.env.RELEASE_TAG || process.env.GITHUB_REF_NAME || "",
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
    throw new Error("Missing release version. Pass --version or set RELEASE_VERSION/GITHUB_REF_NAME.");
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
  files.sort();
  const asset = files.at(-1);
  if (!asset) {
    throw new Error(`Windows x64 NSIS asset not found in ${assetDir}.`);
  }
  return asset;
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
  const config = readJson(args.config, "Release updater config");
  if (config?.bundle?.createUpdaterArtifacts !== true) {
    throw new Error("Release updater config must set bundle.createUpdaterArtifacts=true.");
  }
  ensureNonPlaceholderPubkey(config?.plugins?.updater?.pubkey);

  const assetName = findNsisAsset(args.assetDir);
  const signature = readSignature(args.assetDir, assetName);
  const latestJsonPath = path.join(args.assetDir, "latest.json");
  const latest = readJson(latestJsonPath, "latest.json");

  if (latest.version !== args.version) {
    throw new Error(`latest.json version mismatch: expected ${args.version}, got ${latest.version ?? "<missing>"}.`);
  }

  const expectedUrl = `https://github.com/${args.repo}/releases/download/${args.tag}/${assetName}`;
  for (const platformKey of ["windows-x86_64-nsis", "windows-x86_64"]) {
    const platform = latest?.platforms?.[platformKey];
    if (!platform) {
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
  };
}

const entryPath = process.argv[1] ? path.resolve(process.argv[1]) : "";
if (entryPath && import.meta.url === pathToFileURL(entryPath).href) {
  const args = parseArgs(process.argv.slice(2));
  const result = validateReleasePreflight(args);
  console.log(
    `Release preflight passed for ${args.tag}: ${result.assetName} + ${path.relative(process.cwd(), result.latestJsonPath)}`,
  );
}
