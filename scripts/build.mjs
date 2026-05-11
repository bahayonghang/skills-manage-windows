import {
  copyFileSync,
  cpSync,
  existsSync,
  mkdirSync,
  readdirSync,
  rmSync,
  statSync,
} from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";
import { spawnSync } from "node:child_process";

const require = createRequire(import.meta.url);
const repoRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const outputDir = join(repoRoot, "outputs");
const bundleRoot = join(repoRoot, "src-tauri", "target", "release", "bundle");
const tauriCliEntry = require.resolve("@tauri-apps/cli/tauri.js");

const bundleConfigs = {
  win32: {
    bundles: "nsis",
    artifacts: [{ dir: "nsis", pattern: /\.exe$/i }],
    staleOutputs: ["skillport.exe"],
  },
  darwin: {
    bundles: "app,dmg",
    artifacts: [
      { dir: "macos", pattern: /\.app$/i },
      { dir: "dmg", pattern: /\.dmg$/i },
    ],
    staleOutputs: [],
  },
  linux: {
    bundles: "appimage,deb",
    artifacts: [
      { dir: "appimage", pattern: /\.AppImage$/ },
      { dir: "deb", pattern: /\.deb$/i },
    ],
    staleOutputs: [],
  },
};

const config = bundleConfigs[process.platform];

if (!config) {
  throw new Error(`Unsupported platform for Tauri build: ${process.platform}`);
}

function run(command, args) {
  const result = spawnSync(command, args, {
    cwd: repoRoot,
    stdio: "inherit",
  });

  if (result.error) {
    throw result.error;
  }

  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

function findLatestArtifact({ dir, pattern }) {
  const artifactDir = join(bundleRoot, dir);

  if (!existsSync(artifactDir)) {
    return null;
  }

  const entries = readdirSync(artifactDir)
    .filter((entry) => pattern.test(entry))
    .map((entry) => {
      const fullPath = join(artifactDir, entry);
      return { fullPath, name: entry, mtimeMs: statSync(fullPath).mtimeMs };
    });

  entries.sort((left, right) => right.mtimeMs - left.mtimeMs);
  return entries[0] ?? null;
}

console.log("[build] preparing output directory");
mkdirSync(outputDir, { recursive: true });

for (const staleOutput of config.staleOutputs) {
  const stalePath = join(outputDir, staleOutput);
  if (existsSync(stalePath)) {
    rmSync(stalePath, { force: true });
  }
}

console.log(`[build] running Tauri build for ${process.platform}: ${config.bundles}`);
run(process.execPath, [tauriCliEntry, "build", "--bundles", config.bundles]);

const copied = [];

for (const artifactConfig of config.artifacts) {
  const artifact = findLatestArtifact(artifactConfig);
  if (!artifact) {
    continue;
  }

  const destination = join(outputDir, artifact.name);
  if (existsSync(destination)) {
    rmSync(destination, { force: true, recursive: true });
  }

  if (statSync(artifact.fullPath).isDirectory()) {
    cpSync(artifact.fullPath, destination, { recursive: true });
  } else {
    copyFileSync(artifact.fullPath, destination);
  }
  copied.push(destination);
}

if (copied.length === 0) {
  const searchedDirs = config.artifacts
    .map(({ dir }) => relative(repoRoot, join(bundleRoot, dir)))
    .join(", ");
  throw new Error(`No build artifacts were found in: ${searchedDirs}`);
}

for (const artifact of copied) {
  console.log(`[build] copied ${relative(repoRoot, artifact)}`);
}
