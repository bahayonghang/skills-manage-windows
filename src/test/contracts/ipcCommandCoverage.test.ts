/**
 * IPC 命令覆盖率 ratchet：全仓 invoke/invokeRaw 字面量必须登记在
 * IPC_COMMANDS（类型化）或 UNTYPED_IPC_COMMANDS（存量允许清单）之一。
 *
 * - 新增命令：优先加进 commandMap.ts 的 IPC_COMMANDS；确有理由暂缓类型化
 *   时才登记到 UNTYPED_IPC_COMMANDS。
 * - 命令类型化后必须同时从 UNTYPED_IPC_COMMANDS 移除（只减不增）。
 * - 清单里不允许留没有任何调用点的僵尸条目。
 *
 * 扫描范围：src/**\/*.{ts,tsx}，排除 src/test/（测试自身）与 src/lib/ipc/
 * （adapter 内部文档注释会自引用示例命令名）。
 */
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

import {
  HANDWRITTEN_IPC_COMMAND_NAMES,
  TYPED_IPC_COMMAND_NAMES,
  UNTYPED_IPC_COMMANDS,
} from "@/lib/ipc";
import { GENERATED_IPC_COMMAND_NAMES } from "@/lib/ipc/generatedCommandMap";

const SRC_ROOT = join(process.cwd(), "src");
const IPC_ADAPTER_DIR = join(SRC_ROOT, "lib", "ipc");
const TAURI_SOURCE_ROOT = join(process.cwd(), "src-tauri", "src");
const TAURI_COMMAND_ROOT = join(TAURI_SOURCE_ROOT, "commands");
const IPC_REGISTRY_PATH = join(TAURI_SOURCE_ROOT, "ipc_registry.rs");
const GENERATED_ARTIFACT_PATH = join(IPC_ADAPTER_DIR, "generatedCommandMap.ts");
const BACKEND_ONLY_COMMANDS = [
  "detect_agents",
  "get_active_target",
  "get_central_skills_page",
  "read_skill_content",
  "suggest_skill_tags",
  "sync_registry",
  "sync_registry_with_options",
] as const;

function collectSourceFiles(dir: string, out: string[] = []): string[] {
  for (const name of readdirSync(dir)) {
    const full = join(dir, name);
    if (statSync(full).isDirectory()) {
      if (dir === SRC_ROOT && name === "test") continue;
      if (full === IPC_ADAPTER_DIR) continue;
      collectSourceFiles(full, out);
    } else if (/\.(ts|tsx)$/.test(name) && !/\.test\.(ts|tsx)$/.test(name)) {
      out.push(full);
    }
  }
  return out;
}

function collectProductionFiles(
  dir: string,
  extensions: RegExp,
  out: string[] = [],
): string[] {
  for (const name of readdirSync(dir)) {
    const full = join(dir, name);
    if (statSync(full).isDirectory()) {
      if (name === "test" || name === "tests") continue;
      collectProductionFiles(full, extensions, out);
    } else if (extensions.test(name) && !/tests?\.(rs|ts|tsx)$/.test(name)) {
      out.push(full);
    }
  }
  return out;
}

// 支持一层嵌套泛型（如 invoke<Record<string, string | null>>("get_settings")）
const INVOKE_CALL_RE =
  /\binvoke(?:Raw)?(?:<(?:[^<>]|<[^<>]*>)*>)?\(\s*"([a-z0-9_]+)"/gs;

function scanInvokedCommands(): Map<string, string[]> {
  const invoked = new Map<string, string[]>();
  for (const file of collectSourceFiles(SRC_ROOT)) {
    const text = readFileSync(file, "utf8");
    for (const match of text.matchAll(INVOKE_CALL_RE)) {
      const command = match[1];
      const files = invoked.get(command) ?? [];
      files.push(file);
      invoked.set(command, files);
    }
  }
  return invoked;
}

function collectRegistryCommands(macroName: string): string[] {
  const source = readFileSync(IPC_REGISTRY_PATH, "utf8");
  const start = source.indexOf(`macro_rules! ${macroName}`);
  if (start < 0) throw new Error(`missing registry macro ${macroName}`);
  const bodyStart = source.indexOf("$callback! {", start);
  const bodyEnd = source.indexOf("\n        }\n    };", bodyStart);
  if (bodyStart < 0 || bodyEnd < 0) {
    throw new Error(`malformed registry macro ${macroName}`);
  }
  return [
    ...source.slice(bodyStart, bodyEnd).matchAll(/^\s*([a-z0-9_]+)\s*=>/gm),
  ].map((match) => match[1]);
}

function collectTauriCommandSignatures(
  root = TAURI_SOURCE_ROOT,
): Map<string, string | null> {
  const signatures = new Map<string, string | null>();
  const commandPattern =
    /#\[tauri::command\][\s\S]*?\bpub\s+(?:async\s+)?fn\s+([a-z0-9_]+)\s*\([\s\S]*?\)\s*(?:->\s*([^\{]+))?\s*\{/g;
  for (const file of collectProductionFiles(root, /\.rs$/)) {
    const source = readFileSync(file, "utf8");
    for (const match of source.matchAll(commandPattern)) {
      signatures.set(match[1], match[2]?.replace(/\s+/g, " ").trim() ?? null);
    }
  }
  return signatures;
}

describe("ipc command coverage ratchet", () => {
  const invoked = scanInvokedCommands();
  const typed = new Set(TYPED_IPC_COMMAND_NAMES);
  const allowlisted = new Set(UNTYPED_IPC_COMMANDS);

  it("every invoked command literal is registered (typed map or allowlist)", () => {
    const unregistered = [...invoked.entries()]
      .filter(([command]) => !typed.has(command) && !allowlisted.has(command))
      .map(([command, files]) => `${command} <- ${files[0]}`);
    expect(unregistered).toEqual([]);
  });

  it("typed commands are removed from the allowlist when they graduate", () => {
    const overlap = UNTYPED_IPC_COMMANDS.filter((command) =>
      typed.has(command),
    );
    expect(overlap).toEqual([]);
  });

  it("the allowlist carries no zombie entries without call sites", () => {
    const zombies = UNTYPED_IPC_COMMANDS.filter(
      (command) => !invoked.has(command),
    );
    expect(zombies).toEqual([]);
  });

  it("keeps the typed map at or above the design floor (40 commands)", () => {
    expect(TYPED_IPC_COMMAND_NAMES.length).toBeGreaterThanOrEqual(40);
  });

  it("freezes the 155 typed / 47 untyped / 202 frontend migration counts", () => {
    expect(HANDWRITTEN_IPC_COMMAND_NAMES).toHaveLength(94);
    expect(GENERATED_IPC_COMMAND_NAMES).toHaveLength(61);
    expect(TYPED_IPC_COMMAND_NAMES).toHaveLength(155);
    expect(UNTYPED_IPC_COMMANDS).toHaveLength(47);
    expect(
      new Set([...TYPED_IPC_COMMAND_NAMES, ...UNTYPED_IPC_COMMANDS]).size,
    ).toBe(202);
  });

  it("keeps generated and handwritten command maps disjoint", () => {
    const handwritten = new Set(HANDWRITTEN_IPC_COMMAND_NAMES);
    expect(
      GENERATED_IPC_COMMAND_NAMES.filter((name) => handwritten.has(name)),
    ).toEqual([]);
  });

  it("keeps the generated artifact equal to the frozen Rust generated registry", () => {
    const registered = collectRegistryCommands(
      "__skillport_generated_commands",
    );
    expect([...GENERATED_IPC_COMMAND_NAMES].sort()).toEqual(registered.sort());
    expect(registered.every((command) => invoked.has(command))).toBe(true);
  });

  it("keeps generated metadata adapter-only and phase-aware", () => {
    const artifact = readFileSync(GENERATED_ARTIFACT_PATH, "utf8");
    const commandMap = artifact.slice(
      artifact.indexOf("export const GENERATED_IPC_COMMANDS"),
      artifact.indexOf("export const GENERATED_IPC_COMMAND_NAMES"),
    );
    expect(artifact).not.toContain("@tauri-apps/api/core");
    expect(artifact).not.toMatch(/\binvoke\s*\(/);
    expect(artifact).not.toMatch(/:\s*unknown\b/);
    expect(commandMap).toContain(
      "get_github_pat: command<undefined, GitHubPatState_Serialize>()",
    );
    expect(commandMap).toContain(
      "refresh_skill_update_inventory: command<{ scope: SkillRefreshScope; operationId: string }, SkillUpdateInventory_Serialize>()",
    );
    expect(commandMap).not.toMatch(/\b(?:app|state):/);
    expect(commandMap).toContain("request: LocalRemoteSyncPreviewRequest");
    expect(artifact).toContain("repoPath?: string | null");
    expect(commandMap).toContain("LocalRemoteSyncPreview_Serialize");
  });

  it("detects a Rust-derived argument or Serde field rename as byte drift", () => {
    const artifact = readFileSync(GENERATED_ARTIFACT_PATH, "utf8");
    const mutated = artifact.replace(
      "repositoryIds: string[]",
      "renamedRepositoryIds: string[]",
    );
    expect(mutated).not.toBe(artifact);
    expect(Buffer.from(mutated).equals(Buffer.from(artifact))).toBe(false);
  });

  it("keeps runtime, frontend, and backend-only command sets in parity", () => {
    const runtime = collectRegistryCommands("__skillport_runtime_commands");
    const runtimeSet = new Set(runtime);
    const frontend = new Set([
      ...TYPED_IPC_COMMAND_NAMES,
      ...UNTYPED_IPC_COMMANDS,
    ]);
    expect(runtime).toHaveLength(209);
    expect([...frontend].filter((command) => !runtimeSet.has(command))).toEqual(
      [],
    );
    expect(runtime.filter((command) => !frontend.has(command)).sort()).toEqual(
      [...BACKEND_ONLY_COMMANDS].sort(),
    );
  });

  it("keeps one runtime handler registry and the 194 / 4 fallibility boundary", () => {
    const handlerOwners = collectProductionFiles(
      TAURI_SOURCE_ROOT,
      /\.rs$/,
    ).filter((file) =>
      readFileSync(file, "utf8").includes("tauri::generate_handler!"),
    );
    expect(handlerOwners).toEqual([IPC_REGISTRY_PATH]);

    const signatures = collectTauriCommandSignatures(TAURI_COMMAND_ROOT);
    const fallible = [...signatures.entries()].filter(([, result]) =>
      result?.includes("IpcResult<"),
    );
    const infallible = [...signatures.entries()]
      .filter(([, result]) => !result?.includes("IpcResult<"))
      .map(([name]) => name)
      .sort();
    expect(signatures.size).toBe(209);
    expect(fallible).toHaveLength(205);
    expect(infallible).toEqual([
      "exit_startup",
      "get_app_runtime_info",
      "get_startup_status",
      "record_frontend_runtime_log",
    ]);
    expect(
      [...signatures.values()].some(
        (result) => result?.startsWith("Result<") && result.includes("String"),
      ),
    ).toBe(false);
  });

  it("does not expose saved-secret plaintext reveal commands", () => {
    const productionFiles = [
      ...collectProductionFiles(SRC_ROOT, /\.(ts|tsx)$/),
      ...collectProductionFiles(TAURI_SOURCE_ROOT, /\.rs$/),
    ];
    const forbidden = ["reveal_github_pat", "reveal_ai_api_key"];
    const matches = productionFiles.flatMap((file) => {
      const source = readFileSync(file, "utf8");
      return forbidden
        .filter((command) => source.includes(command))
        .map((command) => `${command} <- ${file}`);
    });
    expect(matches).toEqual([]);
  });
});
