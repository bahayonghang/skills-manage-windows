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

import { TYPED_IPC_COMMAND_NAMES, UNTYPED_IPC_COMMANDS } from "@/lib/ipc";

const SRC_ROOT = join(process.cwd(), "src");
const IPC_ADAPTER_DIR = join(SRC_ROOT, "lib", "ipc");
const TAURI_SOURCE_ROOT = join(process.cwd(), "src-tauri", "src");

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

function collectProductionFiles(dir: string, extensions: RegExp, out: string[] = []): string[] {
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
