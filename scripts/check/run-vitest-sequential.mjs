import { spawnSync } from "node:child_process";
import { mkdtempSync, rmSync, readdirSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { resolveRepoRoot } from "../lib/repo-root.mjs";

const repoRoot = resolveRepoRoot(import.meta.url);
const testDir = join(repoRoot, "src", "test");
const vitestEntry = join(repoRoot, "node_modules", "vitest", "vitest.mjs");
const TEST_FILE_PATTERN = /\.(test|spec)\.(js|jsx|ts|tsx)$/;

function normalizePath(filePath) {
  return filePath.replace(/\\/g, "/");
}

function walkFiles(dirPath) {
  return readdirSync(dirPath, { withFileTypes: true }).flatMap((entry) => {
    const fullPath = join(dirPath, entry.name);
    return entry.isDirectory() ? walkFiles(fullPath) : [fullPath];
  });
}

export function collectTestFiles(rootDir = testDir, baseDir = repoRoot) {
  return walkFiles(rootDir)
    .filter((filePath) => TEST_FILE_PATTERN.test(filePath))
    .map((filePath) => normalizePath(relative(baseDir, filePath)))
    .sort();
}

function isDirectExecution() {
  return process.argv[1] !== undefined
    && resolve(process.argv[1]) === fileURLToPath(import.meta.url);
}

function main() {
  const requestedFiles = process.argv
    .slice(2)
    .filter((arg) => arg !== "--")
    .map(normalizePath);
  const testFiles = requestedFiles.length > 0 ? requestedFiles : collectTestFiles();

  if (testFiles.length === 0) {
    console.error("[test] No Vitest files found under src/test.");
    process.exit(1);
  }

  const tempDir = mkdtempSync(join(tmpdir(), "skillport-vitest-"));
  const inheritedNodeOptions = process.env.NODE_OPTIONS;
  let exitCode = 0;

  try {
    for (const testFile of testFiles) {
      const localStorageFile = join(
        tempDir,
        `${testFile.replace(/[^a-zA-Z0-9._-]/g, "_")}.localstorage.json`
      );
      const nodeOptions = [
        inheritedNodeOptions || "--max-old-space-size=4096",
        `--localstorage-file=${localStorageFile}`,
      ].join(" ");

      console.log(`[test] running ${testFile}`);
      const result = spawnSync(
        process.execPath,
        [vitestEntry, "run", "--maxWorkers=1", testFile],
        {
          cwd: repoRoot,
          stdio: "inherit",
          env: {
            ...process.env,
            NODE_OPTIONS: nodeOptions,
          },
        }
      );

      if (result.error) {
        throw result.error;
      }

      if (result.status !== 0) {
        exitCode = result.status ?? 1;
        break;
      }
    }

    if (exitCode === 0) {
      console.log(`[test] completed ${testFiles.length} Vitest files sequentially.`);
    }
  } finally {
    rmSync(tempDir, { recursive: true, force: true });
  }

  process.exit(exitCode);
}

if (isDirectExecution()) {
  main();
}
