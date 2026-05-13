import { spawnSync } from "node:child_process";
import { readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const testDir = join(repoRoot, "src", "test");
const vitestEntry = join(repoRoot, "node_modules", "vitest", "vitest.mjs");
const TEST_FILE_PATTERN = /\.(test|spec)\.(js|jsx|ts|tsx)$/;

const testFiles = readdirSync(testDir, { withFileTypes: true })
  .filter((entry) => entry.isFile() && TEST_FILE_PATTERN.test(entry.name))
  .map((entry) => `src/test/${entry.name}`)
  .sort();

if (testFiles.length === 0) {
  console.error("[test] No Vitest files found under src/test.");
  process.exit(1);
}

for (const testFile of testFiles) {
  console.log(`[test] running ${testFile}`);
  const result = spawnSync(
    process.execPath,
    [vitestEntry, "run", "--maxWorkers=1", testFile],
    {
      cwd: repoRoot,
      stdio: "inherit",
      env: {
        ...process.env,
        NODE_OPTIONS: process.env.NODE_OPTIONS ?? "--max-old-space-size=4096",
      },
    }
  );

  if (result.error) {
    throw result.error;
  }

  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

console.log(`[test] completed ${testFiles.length} Vitest files sequentially.`);
