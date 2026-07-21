import { beforeEach, describe, expect, it, vi } from "vitest";

const mockShow = vi.hoisted(() => vi.fn());
const mockTauriInvoke = vi.hoisted(() => vi.fn());
const mockTauriListen = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/api/core", () => ({
  invoke: mockTauriInvoke,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: mockTauriListen,
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    show: mockShow,
  }),
}));

import {
  __resetMainWindowReadyForTest,
  clearIpcFixturesForTest,
  invoke,
  IpcFixtureMissingError,
  isTauriRuntime,
  listen,
  registerIpcFailureRecorder,
  registerIpcFixtures,
  registerUntypedIpcFixture,
  showMainWindowWhenReady,
} from "@/lib/ipc";
import type { ObsidianVault, SkillDetail } from "@/types";

function enableTauriRuntime() {
  Object.defineProperty(window, "__TAURI__", {
    value: {},
    configurable: true,
  });
}

// 编译期断言（永不执行）：双 overload 的类型行为
type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B
  ? 1
  : 2
  ? true
  : false;

const _typeAssertions = () => {
  // 已类型化命令（无上下文类型）：按名推导返回 Promise<SkillDetail>
  const detail = invoke("get_skill_detail", { skillId: "s1" });
  const detailIsTyped: Equal<typeof detail, Promise<SkillDetail>> = true;
  // 未类型化命令：落到兼容 overload，无上下文类型时为 Promise<unknown>
  const untyped = invoke("not_in_map_command");
  const untypedIsUnknown: Equal<typeof untyped, Promise<unknown>> = true;
  // 存量显式泛型写法在命令入 map 后仍兼容（落到第二 overload）
  const legacy: Promise<string> = invoke<string>("get_skill_detail", {
    anything: true,
  });
  return { detail, untyped, legacy, detailIsTyped, untypedIsUnknown };
};
void _typeAssertions;

describe("ipc adapter", () => {
  beforeEach(() => {
    mockShow.mockReset();
    mockTauriInvoke.mockReset();
    mockTauriListen.mockReset();
    registerIpcFailureRecorder(null);
    clearIpcFixturesForTest();
    __resetMainWindowReadyForTest();
    Reflect.deleteProperty(window, "__TAURI__");
    Reflect.deleteProperty(window, "__TAURI_INTERNALS__");
  });

  describe("window readiness", () => {
    it("does not show the main window outside the Tauri runtime", async () => {
      expect(isTauriRuntime()).toBe(false);

      await showMainWindowWhenReady();

      expect(mockShow).not.toHaveBeenCalled();
    });

    it("shows the main window once in the Tauri runtime", async () => {
      enableTauriRuntime();
      mockShow.mockResolvedValue(undefined);

      await showMainWindowWhenReady();
      await showMainWindowWhenReady();

      expect(mockShow).toHaveBeenCalledTimes(1);
    });
  });

  describe("failure recorder", () => {
    it("records IPC failures through the registered diagnostic hook", async () => {
      enableTauriRuntime();
      const recorder = vi.fn();
      const error = new Error("backend failed");
      registerIpcFailureRecorder(recorder);
      mockTauriInvoke.mockRejectedValueOnce(error);

      await expect(
        invoke("dangerous_command", { token: "secret" }),
      ).rejects.toThrow("backend failed");

      expect(recorder).toHaveBeenCalledWith(
        "dangerous_command",
        { token: "secret" },
        error,
      );
    });

    it("does not recursively record runtime-log IPC failures", async () => {
      enableTauriRuntime();
      const recorder = vi.fn();
      registerIpcFailureRecorder(recorder);
      mockTauriInvoke.mockRejectedValueOnce(new Error("logger failed"));

      await expect(
        invoke("record_frontend_runtime_log", { payload: { message: "boom" } }),
      ).rejects.toThrow("logger failed");

      expect(recorder).not.toHaveBeenCalled();
    });

    it("records missing-fixture failures in browser mode", async () => {
      const recorder = vi.fn();
      registerIpcFailureRecorder(recorder);

      await expect(invoke("get_obsidian_vaults")).rejects.toBeInstanceOf(
        IpcFixtureMissingError,
      );

      expect(recorder).toHaveBeenCalledTimes(1);
      expect(recorder.mock.calls[0][0]).toBe("get_obsidian_vaults");
    });
  });

  describe("typed bridge (Tauri runtime)", () => {
    it("routes typed commands through the shared bridge", async () => {
      enableTauriRuntime();
      mockTauriInvoke.mockResolvedValueOnce("skill-content");

      await expect(
        invoke("read_file_by_path", {
          path: "/tmp/skill/SKILL.md",
          skillId: "skill-1",
        }),
      ).resolves.toBe("skill-content");

      expect(mockTauriInvoke).toHaveBeenCalledWith("read_file_by_path", {
        path: "/tmp/skill/SKILL.md",
        skillId: "skill-1",
      });
    });

    it("ignores registered fixtures when running inside Tauri", async () => {
      enableTauriRuntime();
      registerIpcFixtures({
        get_obsidian_vaults: () => [],
      });
      mockTauriInvoke.mockResolvedValueOnce([{ id: "real" }]);

      await expect(invoke("get_obsidian_vaults")).resolves.toEqual([
        { id: "real" },
      ]);
      expect(mockTauriInvoke).toHaveBeenCalledWith("get_obsidian_vaults");
    });
  });

  describe("fixture routing (browser mode)", () => {
    it("serves registered fixtures by command name without touching Tauri", async () => {
      const vault = { id: "vault-1" } as unknown as ObsidianVault;
      registerIpcFixtures({
        get_obsidian_vaults: () => [vault],
        get_obsidian_vault_skills: ({ vaultId }) =>
          vaultId === "vault-1" ? [] : Promise.reject(new Error("unknown")),
      });

      await expect(invoke("get_obsidian_vaults")).resolves.toEqual([vault]);
      await expect(
        invoke("get_obsidian_vault_skills", { vaultId: "vault-1" }),
      ).resolves.toEqual([]);
      expect(mockTauriInvoke).not.toHaveBeenCalled();
    });

    it("supports async handlers and untyped registration", async () => {
      registerUntypedIpcFixture("legacy_command", async (args) => ({
        echoed: args,
      }));

      await expect(
        invoke<{ echoed: unknown }>("legacy_command", { a: 1 }),
      ).resolves.toEqual({ echoed: { a: 1 } });
    });

    it("turns synchronously throwing handlers into rejections", async () => {
      registerUntypedIpcFixture("boom_command", () => {
        throw new Error("fixture boom");
      });

      await expect(invoke("boom_command")).rejects.toThrow("fixture boom");
    });

    it("rejects with IpcFixtureMissingError for unregistered commands", async () => {
      await expect(invoke("nowhere_command")).rejects.toThrow(
        'no browser fixture registered for command "nowhere_command"',
      );
    });
  });

  describe("listen", () => {
    it("returns a no-op unlisten outside the Tauri runtime", async () => {
      const unlisten = await listen("some-event", () => undefined);

      expect(mockTauriListen).not.toHaveBeenCalled();
      expect(() => unlisten()).not.toThrow();
    });

    it("delegates to Tauri listen inside the runtime", async () => {
      enableTauriRuntime();
      const stop = vi.fn();
      mockTauriListen.mockResolvedValueOnce(stop);
      const handler = () => undefined;

      const unlisten = await listen("some-event", handler);

      expect(mockTauriListen).toHaveBeenCalledWith(
        "some-event",
        handler,
        undefined,
      );
      expect(unlisten).toBe(stop);
    });
  });
});
