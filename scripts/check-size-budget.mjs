import { readdirSync, readFileSync } from "node:fs";
import { extname, join, relative, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const MAX_LINES = 800;
const SOURCE_ROOTS = ["src", "src-tauri/src"];
const SOURCE_EXTENSIONS = new Set([".ts", ".tsx", ".js", ".jsx", ".rs"]);

const BASELINE_ALLOWLIST = new Map([
  ["src-tauri/src/services/central_updates/core.rs", 861],
  ["src-tauri/src/commands/collections.rs", 1033],
  ["src-tauri/src/db/seed.rs", 810],
  ["src/pages/CentralSkillsView.tsx", 865],
]);

function normalizePath(path) {
  return path.replace(/\\/g, "/");
}

function countLines(filePath) {
  const text = readFileSync(filePath, "utf8");
  if (text.length === 0) {
    return 0;
  }

  const lines = text.split(/\r\n|\n|\r/);
  if (text.endsWith("\n") || text.endsWith("\r")) {
    lines.pop();
  }
  return lines.length;
}

function isIgnoredSourceFile(relativePath) {
  const normalized = normalizePath(relativePath);
  return (
    normalized.startsWith("src/test/") ||
    normalized.startsWith("src/data/") ||
    normalized.startsWith("src/i18n/locales/") ||
    normalized === "src/index.css" ||
    normalized.includes("/__tests__/") ||
    /\.(test|spec)\.(js|jsx|ts|tsx)$/.test(normalized) ||
    normalized.endsWith("/tests.rs")
  );
}

function walkFiles(dirPath) {
  const entries = readdirSync(dirPath, { withFileTypes: true });
  const files = [];

  for (const entry of entries) {
    const fullPath = join(dirPath, entry.name);
    if (entry.isDirectory()) {
      files.push(...walkFiles(fullPath));
      continue;
    }

    if (!entry.isFile()) {
      continue;
    }

    files.push(fullPath);
  }

  return files;
}

const sourceFiles = SOURCE_ROOTS.flatMap((root) =>
  walkFiles(join(repoRoot, root))
    .map((filePath) => normalizePath(relative(repoRoot, filePath)))
    .filter((relativePath) => SOURCE_EXTENSIONS.has(extname(relativePath)))
    .filter((relativePath) => !isIgnoredSourceFile(relativePath)),
);

const violations = [];
const allowlistStillOversized = [];

for (const relativePath of sourceFiles) {
  const absolutePath = join(repoRoot, relativePath);
  const lines = countLines(absolutePath);

  if (lines <= MAX_LINES) {
    continue;
  }

  const baselineLimit = BASELINE_ALLOWLIST.get(relativePath);
  if (baselineLimit !== undefined) {
    allowlistStillOversized.push({ relativePath, lines, baselineLimit });
    if (lines > baselineLimit) {
      violations.push(
        `${relativePath}: ${lines} lines exceeds frozen baseline ${baselineLimit} lines`,
      );
    }
    continue;
  }

  violations.push(
    `${relativePath}: ${lines} lines exceeds budget ${MAX_LINES} lines`,
  );
}

if (violations.length > 0) {
  console.error("[sizecheck] Size budget violations detected:");
  for (const violation of violations) {
    console.error(`- ${violation}`);
  }
  process.exit(1);
}

console.log(
  `[sizecheck] Passed. Checked ${sourceFiles.length} production source files with max ${MAX_LINES} lines.`,
);

if (allowlistStillOversized.length > 0) {
  console.log(
    "[sizecheck] Frozen baseline exceptions (must not grow until refactored):",
  );
  for (const item of allowlistStillOversized) {
    console.log(`- ${item.relativePath}: ${item.lines}/${item.baselineLimit}`);
  }
}
