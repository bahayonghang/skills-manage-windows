#!/usr/bin/env node
// scripts/docs/build-ipc-dict.mjs
//
// 扫描 src-tauri/src/commands 下所有 .rs 文件中的 `#[tauri::command]` 函数，
// 解析函数签名（名称、参数、返回类型、所属模块、首段 doc 注释），
// 生成 docs/architecture/_generated/ipc-commands.md。
// VitePress 通过 markdown 引入此文件，避免手工同步 80+ IPC 命令清单。

import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative, resolve, sep } from "node:path";
import { pathToFileURL } from "node:url";
import { writeOrCheckGeneratedFile } from "./generated-doc-file.mjs";
import { resolveRepoRoot } from "../lib/repo-root.mjs";

const repoRoot = resolveRepoRoot(import.meta.url);
const srcDir = join(repoRoot, "src-tauri", "src");
const outDir = join(repoRoot, "docs", "architecture", "_generated");
const outFile = join(outDir, "ipc-commands.md");

function collectRustFiles(dir) {
  const out = [];
  for (const entry of readdirSync(dir).sort()) {
    const full = join(dir, entry);
    const stat = statSync(full);
    if (stat.isDirectory()) {
      out.push(...collectRustFiles(full));
    } else if (entry.endsWith(".rs")) {
      out.push(full);
    }
  }
  return out;
}

function shortPath(absolute, rootDir = repoRoot) {
  return relative(rootDir, absolute).split(sep).join("/");
}

function extractFnName(signature) {
  const match = signature.match(/fn\s+([a-zA-Z_][a-zA-Z0-9_]*)/);
  return match ? match[1] : null;
}

function findParameterBounds(signature) {
  const fnIndex = signature.search(/\bfn\s+[a-zA-Z_][a-zA-Z0-9_]*/);
  const open = signature.indexOf("(", fnIndex);
  if (fnIndex < 0 || open < 0) return null;

  let depth = 0;
  for (let index = open; index < signature.length; index++) {
    if (signature[index] === "(") depth++;
    else if (signature[index] === ")") depth--;
    if (depth === 0) return { open, close: index };
  }
  return null;
}

function extractReturn(signature) {
  const bounds = findParameterBounds(signature);
  const returnClause = bounds ? signature.slice(bounds.close + 1) : signature;
  const arrow = returnClause.indexOf("->");
  if (arrow < 0) return "unit";
  const after = returnClause.slice(arrow + 2);
  const brace = after.indexOf("{");
  const slice = brace >= 0 ? after.slice(0, brace) : after;
  return slice.trim() || "unit";
}

function splitTopLevel(input, sep) {
  const out = [];
  let depth = 0;
  let current = "";
  for (const ch of input) {
    if (ch === "<" || ch === "(" || ch === "[") depth++;
    else if (ch === ">" || ch === ")" || ch === "]") depth--;
    if (ch === sep && depth === 0) {
      out.push(current);
      current = "";
    } else {
      current += ch;
    }
  }
  if (current.length) out.push(current);
  return out;
}

function extractParams(signature) {
  const bounds = findParameterBounds(signature);
  if (!bounds) return [];
  const inside = signature.slice(bounds.open + 1, bounds.close);
  const params = splitTopLevel(inside, ",");
  const business = [];
  for (const raw of params) {
    const trimmed = raw.trim();
    if (!trimmed) continue;
    if (/State<|Window|AppHandle|Emitter|Manager/.test(trimmed)) continue;
    if (trimmed.startsWith("_")) continue;
    business.push(trimmed);
  }
  return business;
}

function extractDocComment(lines, attributeIndex) {
  const docs = [];
  let i = attributeIndex - 1;
  while (i >= 0) {
    const line = lines[i].trim();
    if (line.startsWith("///")) {
      docs.unshift(line.replace(/^\/\/\/\s?/, ""));
      i--;
      continue;
    }
    if (line.startsWith("#[")) {
      i--;
      continue;
    }
    break;
  }
  return docs.join(" ").trim();
}

function deriveModule(filePath, sourceDir = srcDir) {
  const rel = relative(sourceDir, filePath).split(sep);
  if (rel[rel.length - 1] === "mod.rs") rel.pop();
  else rel[rel.length - 1] = rel[rel.length - 1].replace(/\.rs$/, "");
  return rel.join("::");
}

function parseFile(filePath, sourceDir = srcDir, rootDir = repoRoot) {
  const text = readFileSync(filePath, "utf8");
  const lines = text.split(/\r?\n/);
  const commands = [];
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i].trim();
    if (!line.startsWith("#[tauri::command")) continue;

    let cursor = i + 1;
    while (cursor < lines.length && lines[cursor].trim().startsWith("#["))
      cursor++;
    if (cursor >= lines.length) continue;
    const headerLine = cursor;

    let buffer = "";
    while (cursor < lines.length) {
      buffer += " " + lines[cursor];
      if (lines[cursor].includes("{") || lines[cursor].includes(";")) break;
      cursor++;
    }
    const signature = buffer.replace(/\s+/g, " ").trim();
    const name = extractFnName(signature);
    if (!name) continue;

    commands.push({
      name,
      module: deriveModule(filePath, sourceDir),
      sourcePath: shortPath(filePath, rootDir),
      sourceLine: headerLine + 1,
      params: extractParams(signature),
      returnType: extractReturn(signature),
      doc: extractDocComment(lines, i),
    });
  }
  return commands;
}

function groupCommands(commands) {
  const groups = new Map();
  for (const cmd of commands) {
    if (!groups.has(cmd.module)) groups.set(cmd.module, []);
    groups.get(cmd.module).push(cmd);
  }
  for (const list of groups.values()) {
    list.sort((a, b) => a.name.localeCompare(b.name));
  }
  return [...groups.entries()].sort(([a], [b]) => a.localeCompare(b));
}

function parseCommandPolicies(registryFile) {
  if (!existsSync(registryFile)) return null;

  const source = readFileSync(registryFile, "utf8");
  const macroStart = source.indexOf(
    "macro_rules! __skillport_runtime_commands",
  );
  const bodyStart = source.indexOf("$callback! {", macroStart);
  const bodyEnd = source.indexOf("\n        }\n    };", bodyStart);
  if (macroStart < 0 || bodyStart < 0 || bodyEnd < 0) {
    throw new Error("[build-ipc-dict] malformed runtime command registry");
  }

  const policies = new Map();
  const entryPattern =
    /^\s*([a-z0-9_]+)\s*=>\s*\((commands::[a-z0-9_:]+),\s*(operation\(([^)]*)\)|runtime_only\(([^)]*)\)|excluded\(([^)]*)\))\),/gm;
  const body = source.slice(bodyStart, bodyEnd);
  for (const match of body.matchAll(entryPattern)) {
    const [
      ,
      name,
      handler,
      declaration,
      operationArgs,
      runtimeReason,
      exclusionReason,
    ] = match;
    if (policies.has(name)) {
      throw new Error(`[build-ipc-dict] duplicate command policy: ${name}`);
    }
    if (operationArgs) {
      const [category, phase, lifecycle] = operationArgs
        .split(",")
        .map((value) => value.trim());
      if (!category || !phase || !lifecycle) {
        throw new Error(`[build-ipc-dict] malformed operation policy: ${name}`);
      }
      policies.set(name, {
        handler,
        kind: "operation",
        category,
        phase,
        lifecycle,
        reason: null,
      });
    } else {
      policies.set(name, {
        handler,
        kind: declaration.startsWith("runtime_only")
          ? "runtime-only"
          : "excluded",
        category: null,
        phase: null,
        lifecycle: null,
        reason: (runtimeReason || exclusionReason).trim(),
      });
    }
  }
  if (!policies.size) {
    throw new Error("[build-ipc-dict] no command policies found");
  }
  return policies;
}

function escapePipe(text) {
  return String(text || "").replace(/\|/g, "\\|");
}

function renderMarkdown(groups, includePolicies) {
  const total = groups.reduce((acc, [, list]) => acc + list.length, 0);
  const out = [];
  out.push("<!-- AUTOGENERATED — do not edit by hand. -->");
  out.push("<!-- regenerate via: pnpm docs:gen -->");
  out.push("");
  out.push(`> Source modules: ${groups.length}　·　Commands: ${total}`);
  out.push("");

  for (const [module, list] of groups) {
    out.push(`## \`${module}\``);
    out.push("");
    out.push(`Source: \`${list[0].sourcePath}\``);
    out.push("");
    if (includePolicies) {
      out.push(
        "| Command / action | Log policy | Category | Phase | Lifecycle / reason | Audit evidence | Inputs | Returns | Doc |",
      );
      out.push("| --- | --- | --- | --- | --- | --- | --- | --- | --- |");
    } else {
      out.push("| Command | Inputs | Returns | Doc |");
      out.push("| --- | --- | --- | --- |");
    }
    for (const cmd of list) {
      const inputs = cmd.params.length
        ? cmd.params.map((p) => `\`${escapePipe(p)}\``).join("<br>")
        : "—";
      const ret = `\`${escapePipe(cmd.returnType)}\``;
      const doc = cmd.doc ? escapePipe(cmd.doc) : "—";
      if (includePolicies) {
        const policy = cmd.policy;
        const auditEvidence =
          policy.kind === "operation"
            ? "success + failure; conditional partial/cancel; typed target; reviewed code; safe fields"
            : policy.kind === "runtime-only"
              ? "safe backend rejection only"
              : "none; recursion/readiness exclusion";
        out.push(
          `| \`${cmd.name}\` | ${policy.kind} | ${policy.category || "—"} | ${policy.phase || "—"} | ${policy.lifecycle || policy.reason} | ${auditEvidence} | ${inputs} | ${ret} | ${doc} |`,
        );
      } else {
        out.push(`| \`${cmd.name}\` | ${inputs} | ${ret} | ${doc} |`);
      }
    }
    out.push("");
  }

  return out.join("\n");
}

export function generateIpcDocs({
  check = false,
  log = console.log,
  outputFile = outFile,
  rootDir = repoRoot,
  sourceDir = srcDir,
} = {}) {
  const scanDir =
    sourceDir === srcDir ? join(sourceDir, "commands") : sourceDir;
  const files = collectRustFiles(scanDir);
  let commands = files.flatMap((file) => parseFile(file, sourceDir, rootDir));
  if (!commands.length) {
    throw new Error(
      "[build-ipc-dict] no #[tauri::command] entries found - refusing to overwrite",
    );
  }
  const policies = parseCommandPolicies(join(sourceDir, "ipc_registry.rs"));
  if (policies) {
    const commandNames = new Set(commands.map((command) => command.name));
    const missingPolicies = commands.filter(
      (command) => !policies.has(command.name),
    );
    const stalePolicies = [...policies.keys()].filter(
      (name) => !commandNames.has(name),
    );
    if (missingPolicies.length || stalePolicies.length) {
      throw new Error(
        `[build-ipc-dict] command/policy drift: missing=${missingPolicies.map((command) => command.name).join(",") || "none"} stale=${stalePolicies.join(",") || "none"}`,
      );
    }
    commands = commands.map((command) => ({
      ...command,
      policy: policies.get(command.name),
    }));
  }
  const groups = groupCommands(commands);
  const markdown = renderMarkdown(groups, Boolean(policies));
  const displayPath = shortPath(outputFile, rootDir);
  writeOrCheckGeneratedFile({
    check,
    content: markdown,
    displayPath,
    outputFile,
  });
  log(
    check
      ? `[build-ipc-dict] up to date: ${displayPath}`
      : `[build-ipc-dict] wrote ${commands.length} commands -> ${displayPath}`,
  );
  return markdown;
}

function runCli(args) {
  if (args.some((arg) => arg !== "--check")) {
    throw new Error(
      "[build-ipc-dict] usage: node scripts/docs/build-ipc-dict.mjs [--check]",
    );
  }
  generateIpcDocs({ check: args.includes("--check") });
}

if (
  process.argv[1] &&
  pathToFileURL(resolve(process.argv[1])).href === import.meta.url
) {
  try {
    runCli(process.argv.slice(2));
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
