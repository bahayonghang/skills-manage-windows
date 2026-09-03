/// <reference types="node" />

import { readdirSync, readFileSync } from "node:fs";
import { extname, join, relative, resolve } from "node:path";

export const REPO_ROOT = resolve(process.cwd());
export const SRC_TAURI_SRC = join(REPO_ROOT, "src-tauri", "src");
export const SERVICES_ROOT = join(SRC_TAURI_SRC, "services");
export const REPOS_ROOT = join(SRC_TAURI_SRC, "db", "repos");

export const MIGRATION_TREES = [
  "src-tauri/src/services/central_updates",
  "src-tauri/src/services/skills_cli",
  "src-tauri/src/services/installation",
] as const;

export const SCAN_ROOTS = {
  servicesCommands: "src-tauri/src/services",
  wideRepoFns: "src-tauri/src",
  repoFunctionOwners: "src-tauri/src/db/repos/*_repo.rs",
} as const;

export const TEST_FILE_ALLOWLIST = {
  directorySegments: ["tests"],
  exactBasenames: ["tests.rs"],
  basenameSuffixes: ["_tests.rs"],
  basenamePrefixes: ["test_"],
} as const;

export const FACADE_AND_OWNER_EXCLUSIONS = [
  "src-tauri/src/db/mod.rs",
  "src-tauri/src/db/repos/**",
] as const;

export type ServicesCommandHit = {
  file: string;
  line: number;
  text: string;
};

export type WideRepoFnHit = {
  file: string;
  line: number;
  name: string;
  owner: string;
  kind: "use-list" | "use-path" | "crate-path" | "db-module";
};

export type BoundaryScan = {
  repoFunctionCount: number;
  servicesCommandsHits: ServicesCommandHit[];
  migrationHits: WideRepoFnHit[];
  historicalHits: WideRepoFnHit[];
};

export function toRepoPath(file: string): string {
  return relative(REPO_ROOT, file).replace(/\\/g, "/");
}

export function walkRsFiles(dir: string): string[] {
  const entries: string[] = [];
  const stack = [dir];
  while (stack.length > 0) {
    const current = stack.pop()!;
    for (const entry of readdirSync(current, { withFileTypes: true })) {
      const full = join(current, entry.name);
      if (entry.isDirectory()) {
        stack.push(full);
        continue;
      }
      if (entry.isFile() && extname(entry.name) === ".rs") {
        entries.push(full);
      }
    }
  }
  return entries.sort();
}

export function isAllowlistedTestFile(repoPath: string): boolean {
  const parts = repoPath.split("/");
  const base = parts[parts.length - 1] ?? "";
  if (parts.some((part) => (TEST_FILE_ALLOWLIST.directorySegments as readonly string[]).includes(part))) {
    return true;
  }
  if ((TEST_FILE_ALLOWLIST.exactBasenames as readonly string[]).includes(base)) {
    return true;
  }
  if (TEST_FILE_ALLOWLIST.basenameSuffixes.some((suffix) => base.endsWith(suffix))) {
    return true;
  }
  if (TEST_FILE_ALLOWLIST.basenamePrefixes.some((prefix) => base.startsWith(prefix))) {
    return true;
  }
  return false;
}

export function inMigrationTree(repoPath: string): boolean {
  return MIGRATION_TREES.some((tree) => repoPath === tree || repoPath.startsWith(`${tree}/`));
}

export function isFacadeOrOwnerPath(repoPath: string): boolean {
  return repoPath === "src-tauri/src/db/mod.rs" || repoPath.startsWith("src-tauri/src/db/repos/");
}

export function stripRustCommentsAndStrings(source: string): string {
  let out = "";
  let i = 0;
  const n = source.length;
  while (i < n) {
    const c = source[i];
    const next = i + 1 < n ? source[i + 1] : "";
    if (c === "/" && next === "/") {
      out += "  ";
      i += 2;
      while (i < n && source[i] !== "\n") {
        out += source[i] === "\r" ? "\r" : " ";
        i += 1;
      }
      continue;
    }
    if (c === "/" && next === "*") {
      out += "  ";
      i += 2;
      let depth = 1;
      while (i < n && depth > 0) {
        if (source[i] === "/" && i + 1 < n && source[i + 1] === "*") {
          out += "  ";
          i += 2;
          depth += 1;
          continue;
        }
        if (source[i] === "*" && i + 1 < n && source[i + 1] === "/") {
          out += "  ";
          i += 2;
          depth -= 1;
          continue;
        }
        out += source[i] === "\n" || source[i] === "\r" ? source[i] : " ";
        i += 1;
      }
      continue;
    }
    if (c === "b" && next === "'") {
      out += "b";
      i += 1;
      continue;
    }
    if (
      c === "b" &&
      (next === '"' ||
        (next === "r" && i + 2 < n && (source[i + 2] === '"' || source[i + 2] === "#")))
    ) {
      out += "b";
      i += 1;
      continue;
    }
    if (c === "r" && (next === '"' || next === "#")) {
      let hashes = 0;
      let j = i + 1;
      while (j < n && source[j] === "#") {
        hashes += 1;
        j += 1;
      }
      if (j < n && source[j] === '"') {
        out += " ".repeat(j - i + 1);
        i = j + 1;
        const closer = `"${"#".repeat(hashes)}`;
        while (i < n) {
          if (source.slice(i, i + closer.length) === closer) {
            out += " ".repeat(closer.length);
            i += closer.length;
            break;
          }
          out += source[i] === "\n" || source[i] === "\r" ? source[i] : " ";
          i += 1;
        }
        continue;
      }
    }
    if (c === '"') {
      out += " ";
      i += 1;
      while (i < n) {
        if (source[i] === "\\") {
          out += "  ";
          i += 2;
          continue;
        }
        if (source[i] === '"') {
          out += " ";
          i += 1;
          break;
        }
        out += source[i] === "\n" || source[i] === "\r" ? source[i] : " ";
        i += 1;
      }
      continue;
    }
    if (c === "'") {
      out += " ";
      i += 1;
      if (i < n && source[i] === "\\") {
        out += "  ";
        i += 2;
      } else if (i < n) {
        out += " ";
        i += 1;
      }
      if (i < n && source[i] === "'") {
        out += " ";
        i += 1;
      }
      continue;
    }
    out += c;
    i += 1;
  }
  return out;
}

function lineNumberAt(source: string, index: number): number {
  let line = 1;
  for (let i = 0; i < index && i < source.length; i += 1) {
    if (source[i] === "\n") {
      line += 1;
    }
  }
  return line;
}

export function loadRepoFunctions(): Map<string, string> {
  const fnToOwner = new Map<string, string>();
  const files = walkRsFiles(REPOS_ROOT).filter((file) => toRepoPath(file).endsWith("_repo.rs"));
  const fnRe = /(?:^|\n)\s*pub(?:\(crate\))?\s+(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)/g;
  for (const file of files) {
    const repoPath = toRepoPath(file);
    const owner = repoPath.split("/").pop()?.replace(/\.rs$/, "") ?? "";
    const source = readFileSync(file, "utf8");
    for (const match of source.matchAll(fnRe)) {
      const name = match[1];
      if (name && !fnToOwner.has(name)) {
        fnToOwner.set(name, owner);
      }
    }
  }
  return fnToOwner;
}

function collectBraceList(code: string, braceStart: number): { text: string; end: number } {
  let depth = 0;
  let i = braceStart;
  while (i < code.length) {
    if (code[i] === "{") {
      depth += 1;
    } else if (code[i] === "}") {
      depth -= 1;
      if (depth === 0) {
        return { text: code.slice(braceStart + 1, i), end: i + 1 };
      }
    }
    i += 1;
  }
  return { text: "", end: braceStart };
}

export function scanWideRepoFnHits(
  repoPath: string,
  source: string,
  fnToOwner: Map<string, string>,
): WideRepoFnHit[] {
  const code = stripRustCommentsAndStrings(source);
  const hits: WideRepoFnHit[] = [];
  const seen = new Set<string>();
  const addHit = (index: number, name: string, kind: WideRepoFnHit["kind"]) => {
    const owner = fnToOwner.get(name);
    if (!owner) {
      return;
    }
    const key = `${index}:${name}:${kind}`;
    if (seen.has(key)) {
      return;
    }
    seen.add(key);
    hits.push({
      file: repoPath,
      line: lineNumberAt(source, index),
      name,
      owner,
      kind,
    });
  };

  const useRe = /use\s+crate::db\s*::/g;
  let useMatch: RegExpExecArray | null;
  while ((useMatch = useRe.exec(code)) !== null) {
    let i = useRe.lastIndex;
    while (i < code.length && /\s/.test(code[i] ?? "")) {
      i += 1;
    }
    if (code[i] === "{") {
      const { text, end } = collectBraceList(code, i);
      useRe.lastIndex = end;
      const identRe = /[A-Za-z_][A-Za-z0-9_]*/g;
      let identMatch: RegExpExecArray | null;
      while ((identMatch = identRe.exec(text)) !== null) {
        const ident = identMatch[0];
        if (ident === "self" || ident === "super" || ident === "crate" || ident === "as") {
          continue;
        }
        if (fnToOwner.has(ident)) {
          addHit(useMatch.index + identMatch.index, ident, "use-list");
        }
      }
      continue;
    }
    const rest = code.slice(i);
    const single = rest.match(/^([A-Za-z_][A-Za-z0-9_]*)/);
    if (!single?.[1] || single[1] === "repos") {
      continue;
    }
    if (fnToOwner.has(single[1])) {
      addHit(i, single[1], "use-path");
    }
  }

  const cratePathRe = /crate::db::([A-Za-z_][A-Za-z0-9_]*)/g;
  let crateMatch: RegExpExecArray | null;
  while ((crateMatch = cratePathRe.exec(code)) !== null) {
    const ident = crateMatch[1];
    if (!ident || ident === "repos") {
      continue;
    }
    const prefix = code.slice(Math.max(0, crateMatch.index - 12), crateMatch.index);
    if (/\b(?:pub\s+)?use\s+$/.test(prefix)) {
      continue;
    }
    addHit(crateMatch.index, ident, "crate-path");
  }

  const dbPathRe = /db::([A-Za-z_][A-Za-z0-9_]*)/g;
  let dbMatch: RegExpExecArray | null;
  while ((dbMatch = dbPathRe.exec(code)) !== null) {
    const prev = dbMatch.index === 0 ? "" : code[dbMatch.index - 1];
    if (prev === ":" || /[A-Za-z0-9_]/.test(prev ?? "")) {
      continue;
    }
    addHit(dbMatch.index, dbMatch[1] ?? "", "db-module");
  }

  hits.sort(
    (a, b) => a.line - b.line || a.name.localeCompare(b.name) || a.kind.localeCompare(b.kind),
  );
  return hits;
}

export function scanServicesCommandsHits(repoPath: string, source: string): ServicesCommandHit[] {
  const code = stripRustCommentsAndStrings(source);
  const hits: ServicesCommandHit[] = [];
  const re = /crate::commands\b|commands::APP_USER_AGENT\b/g;
  let match: RegExpExecArray | null;
  while ((match = re.exec(code)) !== null) {
    hits.push({
      file: repoPath,
      line: lineNumberAt(source, match.index),
      text: match[0],
    });
  }
  return hits;
}

export function formatHits(hits: Array<{ file: string; line: number; text?: string; name?: string }>): string {
  if (hits.length === 0) {
    return "(none)";
  }
  return hits
    .map((hit) => `${hit.file}:${hit.line} ${hit.text ?? hit.name ?? ""}`.trim())
    .join("\n");
}

export function scanRustBoundaries(): BoundaryScan {
  const fnToOwner = loadRepoFunctions();
  const servicesCommandsHits: ServicesCommandHit[] = [];
  for (const file of walkRsFiles(SERVICES_ROOT)) {
    const repoPath = toRepoPath(file);
    if (isAllowlistedTestFile(repoPath)) {
      continue;
    }
    servicesCommandsHits.push(...scanServicesCommandsHits(repoPath, readFileSync(file, "utf8")));
  }

  const migrationHits: WideRepoFnHit[] = [];
  const historicalHits: WideRepoFnHit[] = [];
  for (const file of walkRsFiles(SRC_TAURI_SRC)) {
    const repoPath = toRepoPath(file);
    if (isAllowlistedTestFile(repoPath) || isFacadeOrOwnerPath(repoPath)) {
      continue;
    }
    const hits = scanWideRepoFnHits(repoPath, readFileSync(file, "utf8"), fnToOwner);
    if (inMigrationTree(repoPath)) {
      migrationHits.push(...hits);
    } else {
      historicalHits.push(...hits);
    }
  }

  return {
    repoFunctionCount: fnToOwner.size,
    servicesCommandsHits,
    migrationHits,
    historicalHits,
  };
}
