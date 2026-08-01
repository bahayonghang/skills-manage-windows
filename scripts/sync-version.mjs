import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const defaultRepoRoot = dirname(dirname(fileURLToPath(import.meta.url)));

const targets = [
  {
    relativePath: "src-tauri/tauri.conf.json",
    pattern: /("version"\s*:\s*")[^"]+(")/,
    replacement: (version) => `$1${version}$2`,
    missingMessage: "tauri.conf.json version was not found.",
  },
  {
    relativePath: "src-tauri/Cargo.toml",
    pattern: /(\[package\]\s*name\s*=\s*"skillport"\s*version\s*=\s*")[^"]+(")/ms,
    replacement: (version) => `$1${version}$2`,
    missingMessage: "Cargo.toml package version was not found.",
  },
  {
    relativePath: "src-tauri/Cargo.lock",
    pattern: /(\[\[package\]\]\s*name\s*=\s*"skillport"\s*version\s*=\s*")[^"]+(")/ms,
    replacement: (version) => `$1${version}$2`,
    missingMessage: "Cargo.lock package version was not found.",
  },
];

function replaceFirstOrFail(content, pattern, replacement, errorMessage) {
  if (!pattern.test(content)) {
    throw new Error(errorMessage);
  }

  return content.replace(pattern, replacement);
}

export function parseSyncVersionArgs(args) {
  if (args.length === 0) {
    return { check: false };
  }

  if (args.length === 1 && args[0] === "--check") {
    return { check: true };
  }

  throw new Error(`Unknown argument: ${args[0] ?? ""}`);
}

export function syncVersion({
  rootDir = defaultRepoRoot,
  check = false,
  log = console.log,
} = {}) {
  const packageJsonPath = join(rootDir, "package.json");
  const packageJson = JSON.parse(readFileSync(packageJsonPath, "utf8"));
  const version = String(packageJson.version ?? "").trim();

  if (!version) {
    throw new Error("package.json is missing a valid version.");
  }

  const updates = targets.map((target) => {
    const filePath = join(rootDir, ...target.relativePath.split("/"));
    const currentContent = readFileSync(filePath, "utf8");
    const nextContent = replaceFirstOrFail(
      currentContent,
      target.pattern,
      target.replacement(version),
      target.missingMessage,
    );
    return { ...target, filePath, currentContent, nextContent };
  });
  const changed = updates.filter((target) => target.currentContent !== target.nextContent);
  const changedFiles = changed.map((target) => target.relativePath);

  if (check && changedFiles.length > 0) {
    throw new Error(
      `[version] metadata is not synced to ${version}: ${changedFiles.join(", ")}. Run just sync-version.`,
    );
  }

  if (!check) {
    for (const target of changed) {
      writeFileSync(target.filePath, target.nextContent);
    }
  }

  if (changedFiles.length === 0) {
    log(`[version] already synced to ${version}`);
  } else {
    log(`[version] synced to ${version}: ${changedFiles.join(", ")}`);
  }

  return { version, changedFiles };
}

function isDirectExecution() {
  return process.argv[1] !== undefined
    && pathToFileURL(resolve(process.argv[1])).href === import.meta.url;
}

if (isDirectExecution()) {
  try {
    syncVersion(parseSyncVersionArgs(process.argv.slice(2)));
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
