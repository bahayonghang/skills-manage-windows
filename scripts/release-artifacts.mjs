#!/usr/bin/env node
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

export const checksumFileName = "SHA256SUMS";

export function expectedReleaseAssets(version, includeArm64 = false) {
  const assets = [
    `skillport_${version}_windows_x64_nsis.exe`,
    `skillport_${version}_windows_x64_nsis.exe.sig`,
    `skillport_${version}_windows_x64.msi`,
    `skillport_${version}_windows_x64.zip`,
    "latest.json",
    `skillport_${version}_macos_universal.dmg`,
    `skillport_${version}_macos_universal.zip`,
    `skillport_${version}_macos_universal.tar.gz`,
    `skillport_${version}_linux_x86_64.deb`,
    `skillport_${version}_linux_x86_64.rpm`,
    `skillport_${version}_linux_x86_64.AppImage`,
  ];
  if (includeArm64) {
    assets.push(
      `skillport_${version}_linux_arm64.deb`,
      `skillport_${version}_linux_arm64.rpm`,
      `skillport_${version}_linux_arm64.AppImage`,
    );
  }
  return assets.sort();
}

export function validateArtifactInventory(fileNames, version, { includeChecksums = false } = {}) {
  const normalized = fileNames.map((name) => path.basename(name));
  const duplicates = normalized.filter((name, index) => normalized.indexOf(name) !== index);
  if (duplicates.length > 0) {
    throw new Error(`Duplicate release asset(s): ${[...new Set(duplicates)].sort().join(", ")}.`);
  }

  const arm64 = normalized.filter((name) => name.includes("_linux_arm64."));
  if (arm64.length !== 0 && arm64.length !== 3) {
    throw new Error("Linux arm64 assets are optional but must be provided as a complete DEB/RPM/AppImage set.");
  }

  const expected = expectedReleaseAssets(version, arm64.length === 3);
  if (includeChecksums) expected.push(checksumFileName);
  const actual = [...normalized].sort();
  const missing = expected.filter((name) => !actual.includes(name));
  const unexpected = actual.filter((name) => !expected.includes(name));
  if (missing.length > 0 || unexpected.length > 0) {
    throw new Error(
      `Release artifact inventory mismatch. Missing: ${missing.join(", ") || "none"}. Unexpected: ${unexpected.join(", ") || "none"}.`,
    );
  }
  return actual;
}

function sha256(filePath) {
  return crypto.createHash("sha256").update(fs.readFileSync(filePath)).digest("hex");
}

export function buildChecksumManifest(assetDir, version) {
  const names = fs.readdirSync(assetDir).filter((name) => name !== checksumFileName);
  validateArtifactInventory(names, version);
  return names
    .sort()
    .map((name) => `${sha256(path.join(assetDir, name))}  ${name}`)
    .join("\n") + "\n";
}

export function writeChecksumManifest(assetDir, version) {
  const outputPath = path.join(assetDir, checksumFileName);
  fs.writeFileSync(outputPath, buildChecksumManifest(assetDir, version));
  return outputPath;
}

export function verifyChecksumManifest(assetDir, version) {
  const names = fs.readdirSync(assetDir);
  validateArtifactInventory(names, version, { includeChecksums: true });
  const expected = fs.readFileSync(path.join(assetDir, checksumFileName), "utf8");
  const actual = buildChecksumManifest(assetDir, version);
  if (actual !== expected) {
    throw new Error("SHA256SUMS does not match the release artifact bytes.");
  }
}

const entryPath = process.argv[1] ? path.resolve(process.argv[1]) : "";
if (entryPath && import.meta.url === pathToFileURL(entryPath).href) {
  const versionIndex = process.argv.indexOf("--version");
  const version = versionIndex >= 0 ? process.argv[versionIndex + 1]?.replace(/^v/, "") : process.env.RELEASE_VERSION;
  if (!version) throw new Error("Missing --version.");
  const assetDirIndex = process.argv.indexOf("--asset-dir");
  const assetDir = assetDirIndex >= 0 ? process.argv[assetDirIndex + 1] : "release-assets";
  if (process.argv.includes("--verify")) {
    verifyChecksumManifest(assetDir, version);
    console.log(`Verified ${path.join(assetDir, checksumFileName)}.`);
  } else {
    console.log(`Wrote ${writeChecksumManifest(assetDir, version)}.`);
  }
}
