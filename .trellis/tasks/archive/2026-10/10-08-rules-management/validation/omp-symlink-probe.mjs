// Run with Bun and OMP_PACKAGE_ROOT pointing to the installed OMP package.
// The probe uses temporary synthetic rules and makes no model requests.
import assert from "node:assert/strict";
import { mkdtemp, mkdir, writeFile, symlink, readFile, readlink, lstat } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

const packageRoot = process.env.OMP_PACKAGE_ROOT;
assert(packageRoot, "OMP_PACKAGE_ROOT is required");
const installed = JSON.parse(await readFile(join(packageRoot, "package.json"), "utf8"));
const helpers = await import(pathToFileURL(join(packageRoot, "src/discovery/helpers.ts")).href);
const { bucketRules } = await import(pathToFileURL(join(packageRoot, "src/capability/rule-buckets.ts")).href);
const { TtsrManager } = await import(pathToFileURL(join(packageRoot, "src/export/ttsr.ts")).href);
const root = await mkdtemp(join(tmpdir(), "skillport-rules-discovery-"));
const central = join(root, ".skillport/rules");
const rulesDir = join(root, ".omp/agent/rules");
await mkdir(central, { recursive: true });
await mkdir(rulesDir, { recursive: true });
const name = "中文 rule.md";
const source = join(central, name);
const destination = join(rulesDir, name);
const text = "---\nalwaysApply: true\ndescription: synthetic probe\n---\n\n# Rule\nUse exact names.\n";
await writeFile(source, text);
await symlink("../../../.skillport/rules/" + name, destination, "file");
assert((await lstat(destination)).isSymbolicLink());
assert.equal(await readFile(destination, "utf8"), text);
const result = await helpers.loadFilesFromDir(
  { cwd: root, home: root, repoRoot: null, agentDir: join(root, ".omp/agent") },
  rulesDir,
  "omp",
  "user",
  { extensions: ["md", "mdc"], transform: helpers.discoverRuleFromMarkdown },
);
assert.equal(result.warnings?.length ?? 0, 0);
assert.equal(result.items.length, 1, "Installed OMP must discover the file symlink");
assert.equal(result.items[0].alwaysApply, true);
const buckets = bucketRules(result.items, new TtsrManager(), { agentName: "main" });
assert.equal(buckets.alwaysApplyRules.length, 1);
assert.equal(buckets.rulebookRules.length, 0);
console.log(JSON.stringify({
  result: "PASS", installedVersion: installed.version,
  boundary: "installed OMP discovery helper, parser, and session rule bucketing; no model request",
  fileSymlink: true, relativeTarget: await readlink(destination),
  discovered: result.items.length, alwaysApply: buckets.alwaysApplyRules.length,
  temporaryRoot: root,
}));
