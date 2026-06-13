import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = dirname(dirname(fileURLToPath(import.meta.url)));

const packageJsonPath = join(repoRoot, "package.json");
const cargoTomlPath = join(repoRoot, "src-tauri", "Cargo.toml");
const cargoLockPath = join(repoRoot, "src-tauri", "Cargo.lock");
const tauriConfigPath = join(repoRoot, "src-tauri", "tauri.conf.json");

const packageJson = JSON.parse(readFileSync(packageJsonPath, "utf8"));
const version = String(packageJson.version ?? "").trim();

if (!version) {
  throw new Error("package.json is missing a valid version.");
}

function replaceFirstOrFail(content, pattern, replacement, errorMessage) {
  if (!pattern.test(content)) {
    throw new Error(errorMessage);
  }

  return content.replace(pattern, replacement);
}

function writeIfChanged(filePath, nextContent) {
  const currentContent = readFileSync(filePath, "utf8");

  if (currentContent === nextContent) {
    return false;
  }

  writeFileSync(filePath, nextContent);
  return true;
}

const changedFiles = [];

const tauriConfigContent = readFileSync(tauriConfigPath, "utf8");
if (writeIfChanged(
  tauriConfigPath,
  replaceFirstOrFail(
    tauriConfigContent,
    /("version"\s*:\s*")[^"]+(")/,
    `$1${version}$2`,
    "tauri.conf.json version was not found.",
  ),
)) {
  changedFiles.push("src-tauri/tauri.conf.json");
}

const cargoTomlContent = readFileSync(cargoTomlPath, "utf8");
if (writeIfChanged(
  cargoTomlPath,
  replaceFirstOrFail(
    cargoTomlContent,
    /(\[package\]\s*name\s*=\s*"skillport"\s*version\s*=\s*")[^"]+(")/ms,
    `$1${version}$2`,
    "Cargo.toml package version was not found.",
  ),
)) {
  changedFiles.push("src-tauri/Cargo.toml");
}

const cargoLockContent = readFileSync(cargoLockPath, "utf8");
if (writeIfChanged(
  cargoLockPath,
  replaceFirstOrFail(
    cargoLockContent,
    /(\[\[package\]\]\s*name\s*=\s*"skillport"\s*version\s*=\s*")[^"]+(")/ms,
    `$1${version}$2`,
    "Cargo.lock package version was not found.",
  ),
)) {
  changedFiles.push("src-tauri/Cargo.lock");
}

if (changedFiles.length === 0) {
  console.log(`[version] already synced to ${version}`);
} else {
  console.log(`[version] synced to ${version}: ${changedFiles.join(", ")}`);
}
