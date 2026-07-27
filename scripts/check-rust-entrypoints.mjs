import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const manifestPath = join(repoRoot, "src-tauri", "Cargo.toml");
const expectedDefaultRun = "skillport";
const expectedBins = ["skillport", "skillport-cli", "release-signature-verifier"];

const result = spawnSync(
  "cargo",
  [
    "metadata",
    "--manifest-path",
    manifestPath,
    "--no-deps",
    "--format-version",
    "1",
  ],
  {
    cwd: repoRoot,
    encoding: "utf8",
    windowsHide: true,
  },
);

if (result.error) {
  console.error(`[entrypointcheck] Failed to start cargo: ${result.error.message}`);
  process.exit(1);
}

if (result.status !== 0) {
  process.stderr.write(result.stderr);
  console.error(`[entrypointcheck] cargo metadata failed with exit code ${result.status ?? 1}.`);
  process.exit(result.status ?? 1);
}

let metadata;
try {
  metadata = JSON.parse(result.stdout);
} catch (error) {
  console.error(`[entrypointcheck] Invalid cargo metadata JSON: ${error.message}`);
  process.exit(1);
}

const skillportPackage = metadata.packages.find(({ name }) => name === "skillport");
if (!skillportPackage) {
  console.error("[entrypointcheck] Cargo package 'skillport' was not found.");
  process.exit(1);
}

if (skillportPackage.default_run !== expectedDefaultRun) {
  console.error(
    `[entrypointcheck] Expected default-run '${expectedDefaultRun}', got '${skillportPackage.default_run ?? "unset"}'.`,
  );
  process.exit(1);
}

const binaryTargets = skillportPackage.targets
  .filter(({ kind }) => kind.includes("bin"))
  .map(({ name }) => name);
const missingBins = expectedBins.filter((name) => !binaryTargets.includes(name));

if (missingBins.length > 0) {
  console.error(`[entrypointcheck] Missing binary target(s): ${missingBins.join(", ")}.`);
  process.exit(1);
}

console.log(
  `[entrypointcheck] Passed. default-run=${expectedDefaultRun}; bins=${expectedBins.join(", ")}.`,
);
