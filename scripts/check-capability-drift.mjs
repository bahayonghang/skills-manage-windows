import { spawnSync } from "node:child_process";
import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, extname, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import ts from "typescript";

const repoRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const inventoryPath = resolve(repoRoot, "docs/reference/ipc-capability-inventory.md");
const CONTRACT_START = "<!-- capability-contract:start -->";
const CONTRACT_END = "<!-- capability-contract:end -->";
const TABLE_START = "<!-- capability-table:start -->";
const TABLE_END = "<!-- capability-table:end -->";

const SURFACES = [
  ["capabilityPermissions", "Main-window permissions"],
  ["frontendPluginImports", "Frontend plugin imports"],
  ["frontendPluginDependencies", "Frontend plugin dependencies"],
  ["rustPluginDependencies", "Rust plugin dependencies"],
  ["rustPluginInitializers", "Rust plugin initializers"],
];

function sorted(values) {
  return [...new Set(values)].sort((left, right) =>
    left < right ? -1 : left > right ? 1 : 0,
  );
}

function listSourceFiles(directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const path = resolve(directory, entry.name);
    if (entry.isDirectory()) return listSourceFiles(path);
    return [".ts", ".tsx"].includes(extname(entry.name)) ? [path] : [];
  });
}

function collectPluginImports(sourceRoot) {
  const imports = [];
  for (const path of listSourceFiles(sourceRoot)) {
    const source = readFileSync(path, "utf8");
    const sourceFile = ts.createSourceFile(
      path,
      source,
      ts.ScriptTarget.Latest,
      true,
      path.endsWith(".tsx") ? ts.ScriptKind.TSX : ts.ScriptKind.TS,
    );
    const visit = (node) => {
      let specifier = null;
      if (ts.isImportDeclaration(node) && ts.isStringLiteral(node.moduleSpecifier)) {
        specifier = node.moduleSpecifier.text;
      } else if (
        ts.isCallExpression(node) &&
        node.expression.kind === ts.SyntaxKind.ImportKeyword &&
        node.arguments.length === 1 &&
        ts.isStringLiteral(node.arguments[0])
      ) {
        specifier = node.arguments[0].text;
      }
      if (specifier?.startsWith("@tauri-apps/plugin-")) imports.push(specifier);
      ts.forEachChild(node, visit);
    };
    visit(sourceFile);
  }
  return sorted(imports);
}

function cargoMetadata() {
  const cargo = spawnSync(
    "cargo",
    [
      "metadata",
      "--manifest-path",
      "src-tauri/Cargo.toml",
      "--no-deps",
      "--locked",
      "--format-version",
      "1",
    ],
    { cwd: repoRoot, encoding: "utf8", windowsHide: true },
  );
  if (cargo.error) throw new Error(`Failed to start cargo metadata: ${cargo.error.message}`);
  if (cargo.status !== 0) {
    throw new Error(`cargo metadata failed: ${cargo.stderr.trim() || `exit ${cargo.status}`}`);
  }
  return JSON.parse(cargo.stdout);
}

export function collectObservedCapabilityState() {
  const capability = JSON.parse(
    readFileSync(resolve(repoRoot, "src-tauri/capabilities/default.json"), "utf8"),
  );
  const packageJson = JSON.parse(readFileSync(resolve(repoRoot, "package.json"), "utf8"));
  const metadata = cargoMetadata();
  const skillport = metadata.packages.find((entry) => entry.name === "skillport");
  if (!skillport) throw new Error("Cargo package 'skillport' was not found.");
  const rustEntrypoint = readFileSync(resolve(repoRoot, "src-tauri/src/lib.rs"), "utf8");
  const initializerMatches = rustEntrypoint.matchAll(
    /tauri_plugin_([a-z0-9_]+)::(?:init|Builder)/g,
  );

  return {
    capabilityPermissions: sorted(
      capability.permissions.map((permission) =>
        typeof permission === "string" ? permission : permission.identifier,
      ),
    ),
    frontendPluginImports: collectPluginImports(resolve(repoRoot, "src")),
    frontendPluginDependencies: sorted(
      Object.keys({ ...packageJson.dependencies, ...packageJson.devDependencies }).filter((name) =>
        name.startsWith("@tauri-apps/plugin-"),
      ),
    ),
    rustPluginDependencies: sorted(
      skillport.dependencies
        .map((dependency) => dependency.name)
        .filter((name) => name.startsWith("tauri-plugin-")),
    ),
    rustPluginInitializers: sorted(
      [...initializerMatches].map((match) => `tauri-plugin-${match[1].replaceAll("_", "-")}`),
    ),
  };
}

function extractMarkedContent(document, startMarker, endMarker) {
  const start = document.indexOf(startMarker);
  const end = document.indexOf(endMarker);
  if (start < 0 || end < 0 || end <= start) {
    throw new Error(`Missing or invalid marker pair: ${startMarker} / ${endMarker}`);
  }
  return document.slice(start + startMarker.length, end).trim();
}

export function parseCapabilityContract(document) {
  const block = extractMarkedContent(document, CONTRACT_START, CONTRACT_END);
  const match = block.match(/^```json\s*([\s\S]*?)\s*```$/);
  if (!match) throw new Error("Capability contract marker must contain one JSON code block.");
  const contract = JSON.parse(match[1]);
  for (const [key] of SURFACES) {
    if (!Array.isArray(contract[key])) throw new Error(`Contract field '${key}' must be an array.`);
    contract[key] = sorted(contract[key]);
  }
  return contract;
}

export function renderCapabilityTable(contract) {
  const rows = SURFACES.map(([key, label]) => {
    const values = contract[key].length > 0 ? contract[key].map((value) => `\`${value}\``).join("<br>") : "none";
    return `| ${label} | ${values} |`;
  });
  return ["| Surface | Exact values |", "|---|---|", ...rows].join("\n");
}

export function validateCapabilityContract(observed, contract) {
  const errors = [];
  for (const [key] of SURFACES) {
    const actual = sorted(observed[key]);
    const expected = sorted(contract[key]);
    if (JSON.stringify(actual) !== JSON.stringify(expected)) {
      errors.push(`${key} drifted. expected=${JSON.stringify(expected)} actual=${JSON.stringify(actual)}`);
    }
  }

  for (const pluginImport of observed.frontendPluginImports) {
    const pluginName = pluginImport.replace("@tauri-apps/", "tauri-");
    const permissionPrefix = pluginImport.replace("@tauri-apps/plugin-", "");
    if (!observed.frontendPluginDependencies.includes(pluginImport)) {
      errors.push(`Frontend import '${pluginImport}' has no package dependency.`);
    }
    if (!observed.rustPluginDependencies.includes(pluginName)) {
      errors.push(`Frontend import '${pluginImport}' has no Rust plugin dependency.`);
    }
    if (!observed.rustPluginInitializers.includes(pluginName)) {
      errors.push(`Frontend import '${pluginImport}' has no Rust plugin initializer.`);
    }
    if (!observed.capabilityPermissions.some((permission) => permission.startsWith(`${permissionPrefix}:`))) {
      errors.push(`Frontend import '${pluginImport}' has no main-window permission.`);
    }
  }
  return errors;
}

export function validateRenderedTable(document, contract) {
  const current = extractMarkedContent(document, TABLE_START, TABLE_END);
  const expected = renderCapabilityTable(contract);
  return current === expected ? [] : ["Human-readable capability table is stale; run pnpm capabilitycheck -- --update."];
}

function replaceRenderedTable(document, contract) {
  const start = document.indexOf(TABLE_START);
  const end = document.indexOf(TABLE_END);
  if (start < 0 || end < 0 || end <= start) {
    throw new Error(`Missing or invalid marker pair: ${TABLE_START} / ${TABLE_END}`);
  }
  return `${document.slice(0, start + TABLE_START.length)}\n${renderCapabilityTable(contract)}\n${document.slice(end)}`;
}

export function runCapabilityCheck({ update = false } = {}) {
  let document = readFileSync(inventoryPath, "utf8");
  const contract = parseCapabilityContract(document);
  if (update) {
    document = replaceRenderedTable(document, contract);
    writeFileSync(inventoryPath, document, "utf8");
  }
  const errors = [
    ...validateCapabilityContract(collectObservedCapabilityState(), contract),
    ...validateRenderedTable(document, contract),
  ];
  if (errors.length > 0) throw new Error(errors.join("\n"));
  return contract;
}

const invokedPath = process.argv[1] ? pathToFileURL(resolve(process.argv[1])).href : null;
if (invokedPath === import.meta.url) {
  try {
    const contract = runCapabilityCheck({ update: process.argv.includes("--update") });
    console.log(
      `[capabilitycheck] Passed. ${contract.capabilityPermissions.length} permissions, ${contract.frontendPluginImports.length} frontend plugin imports.`,
    );
  } catch (error) {
    console.error(`[capabilitycheck] ${error.message}`);
    process.exit(1);
  }
}
