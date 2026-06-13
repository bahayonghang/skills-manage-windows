import { existsSync, readdirSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

if (process.platform !== "win32") {
  console.error("[install] just install is only supported on Windows NSIS builds.");
  process.exit(1);
}

const repoRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const outputDir = join(repoRoot, "outputs");

if (!existsSync(outputDir)) {
  throw new Error(`Output directory was not found: ${outputDir}`);
}

const installer = readdirSync(outputDir)
  .filter((entry) => /-setup\.exe$/i.test(entry))
  .map((entry) => {
    const fullPath = join(outputDir, entry);
    return { fullPath, mtimeMs: statSync(fullPath).mtimeMs };
  })
  .sort((left, right) => right.mtimeMs - left.mtimeMs)[0];

if (!installer) {
  throw new Error(`No NSIS installer was found in: ${outputDir}`);
}

console.log(`[install] installing ${installer.fullPath}`);
const result = spawnSync(installer.fullPath, ["/P"], {
  cwd: repoRoot,
  stdio: "inherit",
});

if (result.error) {
  throw result.error;
}

process.exit(result.status ?? 0);
