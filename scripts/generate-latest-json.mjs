#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

export function parseArgs(argv) {
  const args = {
    version: process.env.RELEASE_VERSION || process.env.GITHUB_REF_NAME?.replace(/^v/, "") || "",
    tag: process.env.RELEASE_TAG || process.env.GITHUB_REF_NAME || "",
    assetDir: "release-assets",
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
    } else if (arg === "--repo" && next) {
      args.repo = next;
      index += 1;
    } else if (arg === "--help" || arg === "-h") {
      console.log(`Usage: node scripts/generate-latest-json.mjs --version <version> --tag <tag> [--asset-dir release-assets] [--repo owner/repo]`);
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

export function readSignature(assetPath) {
  const sigPath = `${assetPath}.sig`;
  if (!fs.existsSync(sigPath)) {
    throw new Error(`Updater signature not found: ${sigPath}`);
  }
  return fs.readFileSync(sigPath, "utf8").trim();
}

export function findAsset(assetDir, pattern, label) {
  const files = fs.readdirSync(assetDir).filter((file) => pattern.test(file));
  files.sort();
  const asset = files.at(-1);
  if (!asset) {
    throw new Error(`${label} not found in ${assetDir}.`);
  }
  return path.join(assetDir, asset);
}

export function buildLatestMetadata(args) {
  const nsisAsset = findAsset(args.assetDir, /windows_x64_nsis\.exe$/i, "Windows x64 NSIS asset");
  const signature = readSignature(nsisAsset);
  const assetName = path.basename(nsisAsset);
  const notes = fs.existsSync("release-body.md") ? fs.readFileSync("release-body.md", "utf8") : undefined;
  const pubDate = process.env.RELEASE_PUB_DATE || new Date().toISOString();
  const endpoint = `https://github.com/${args.repo}/releases/download/${args.tag}/${assetName}`;
  const windowsPlatform = {
    signature,
    url: endpoint,
  };

  return {
    version: args.version,
    notes,
    pub_date: pubDate,
    platforms: {
      "windows-x86_64-nsis": windowsPlatform,
      "windows-x86_64": windowsPlatform,
    },
  };
}

export function writeLatestJson(args) {
  const metadata = buildLatestMetadata(args);
  const outputPath = path.join(args.assetDir, "latest.json");
  fs.writeFileSync(outputPath, `${JSON.stringify(metadata, null, 2)}\n`);
  return outputPath;
}

const entryPath = process.argv[1] ? path.resolve(process.argv[1]) : "";
if (entryPath && import.meta.url === pathToFileURL(entryPath).href) {
  const args = parseArgs(process.argv.slice(2));
  const outputPath = writeLatestJson(args);
  console.log(`Wrote ${outputPath}`);
}
