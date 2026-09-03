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
  IpcInvokeError,
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

// 编译期断言（永不执行）：typed-only invoke 拒绝错误 name/参数/返回类型
type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B
  ? 1
  : 2
  ? true
  : false;

function compileTimeTypedUsage() {
  const detail = invoke("get_skill_detail", { skillId: "s1" });
  const detailIsTyped: Equal<typeof detail, Promise<SkillDetail>> = true;

  // @ts-expect-error wrong command name
  void invoke("not_in_map_command");

  // @ts-expect-error missing args
  void invoke("get_skill_detail");

  // @ts-expect-error extra args
  void invoke("get_obsidian_vaults", { extra: true });

  // @ts-expect-error wrong arg type
  void invoke("get_skill_detail", { skillId: 1 });

  // @ts-expect-error wrong result assignment
  const wrongResult: Promise<number> = invoke("get_obsidian_vaults");

  return { detail, detailIsTyped, wrongResult };
}
void compileTimeTypedUsage;

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
      const error = {
        code: "storage.unavailable",
        message: "Backend failed",
        retryable: false,
      };
      registerIpcFailureRecorder(recorder);
      mockTauriInvoke.mockRejectedValueOnce(error);

      await expect(
        invoke("get_skill_detail", { skillId: "secret" }),
      ).rejects.toThrow("Backend failed");

      expect(recorder).toHaveBeenCalledWith(
        "get_skill_detail",
        { skillId: "[REDACTED]" },
        expect.objectContaining({
          code: "storage.unavailable",
          message: "Backend failed",
          retryable: false,
        }),
      );
    });

    it("records only the public archive redirect code and message", async () => {
      enableTauriRuntime();
      const recorder = vi.fn();
      const seed = "ghp_secret https://example.invalid/private C:\\private\\SKILL.md";
      registerIpcFailureRecorder(recorder);
      mockTauriInvoke.mockRejectedValueOnce({
        code: "github_import.archive_redirect_rejected",
        message: "GitHub repository archive redirect was rejected.",
        retryable: false,
        details: seed,
      });

      await expect(invoke("get_obsidian_vaults")).rejects.toThrow(
        "GitHub repository archive redirect was rejected.",
      );

      expect(recorder).toHaveBeenCalledWith(
        "get_obsidian_vaults",
        undefined,
        expect.objectContaining({
          code: "github_import.archive_redirect_rejected",
          message: "GitHub repository archive redirect was rejected.",
          retryable: false,
        }),
      );
      expect(JSON.stringify(recorder.mock.calls)).not.toContain(seed);
    });

    it("does not recursively record runtime-log IPC failures", async () => {
      enableTauriRuntime();
      const recorder = vi.fn();
      registerIpcFailureRecorder(recorder);
      mockTauriInvoke.mockRejectedValueOnce(new Error("logger failed"));

      await expect(
        invoke("record_frontend_runtime_log", { payload: { message: "boom" } }),
      ).rejects.toThrow("The operation failed. See runtime logs for details.");

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

  describe("error normalization", () => {
    it("passes an existing IpcInvokeError through unchanged", async () => {
      enableTauriRuntime();
      const rejection = new IpcInvokeError({
        code: "operation.cancelled",
        message: "The operation was cancelled.",
        retryable: false,
      });
      mockTauriInvoke.mockRejectedValueOnce(rejection);

      const error = await invoke("get_obsidian_vaults").catch((value) => value);

      expect(error).toBe(rejection);
    });

    it("wraps structured payloads and preserves message stringification", async () => {
      enableTauriRuntime();
      mockTauriInvoke.mockRejectedValueOnce({
        code: "operation.cancelled",
        message: "Historical cancellation message",
        retryable: false,
      });

      const error = (await invoke("get_obsidian_vaults").catch(
        (value) => value,
      )) as IpcInvokeError;

      expect(error).toBeInstanceOf(IpcInvokeError);
      expect(error.code).toBe("operation.cancelled");
      expect(error.retryable).toBe(false);
      expect(String(error)).toBe("Historical cancellation message");
      expect(String(error)).not.toContain("[object Object]");
      expect(String(error)).not.toContain("IpcInvokeError:");

      const direct = new IpcInvokeError({
        code: "operation.cancelled",
        message: "Historical cancellation message",
        retryable: false,
      });
      expect(String(direct)).toBe("Historical cancellation message");
    });

    it("preserves a validated correlation id and rejects an unsafe one", async () => {
      enableTauriRuntime();
      const correlationId = "123e4567-e89b-42d3-a456-426614174000";
      mockTauriInvoke.mockRejectedValueOnce({
        code: "operation.cancelled",
        message: "The operation was cancelled.",
        retryable: false,
        correlationId,
      });

      const correlated = (await invoke("get_obsidian_vaults").catch(
        (value) => value,
      )) as IpcInvokeError;
      expect(correlated.correlationId).toBe(correlationId);

      mockTauriInvoke.mockRejectedValueOnce({
        code: "operation.cancelled",
        message: "The operation was cancelled.",
        retryable: false,
        correlationId: "C:\\Users\\alice\\private.log",
      });
      const rejected = (await invoke("get_obsidian_vaults").catch(
        (value) => value,
      )) as IpcInvokeError;
      expect(rejected.code).toBe("internal.unexpected");
      expect(rejected.correlationId).toBeUndefined();
    });

    it.each([
      [
        "legacy coded string",
        "ai.rate_limit:Try again",
        "ai.rate_limit",
        "The AI provider request failed.",
      ],
      [
        "plain string",
        "Legacy failure",
        "internal.unexpected",
        "The operation failed. See runtime logs for details.",
      ],
      [
        "archive redirect coded string",
        "github_import.archive_redirect_rejected:ghp_secret https://example.invalid/private",
        "github_import.archive_redirect_rejected",
        "GitHub repository archive redirect was rejected.",
      ],
      [
        "no importable skills coded string",
        "github_import.no_importable_skills:No importable skills found in this repository.",
        "github_import.no_importable_skills",
        "This GitHub repository does not contain an importable skill.",
      ],
      [
        "JavaScript Error",
        new Error("Transport failure"),
        "internal.unexpected",
        "The operation failed. See runtime logs for details.",
      ],
    ])("normalizes %s", async (_label, rejection, code, message) => {
      enableTauriRuntime();
      mockTauriInvoke.mockRejectedValueOnce(rejection);

      const error = (await invoke("get_obsidian_vaults").catch(
        (value) => value,
      )) as IpcInvokeError;

      expect(error).toBeInstanceOf(IpcInvokeError);
      expect(error.code).toBe(code);
      expect(String(error)).toBe(message);
    });

    it("uses a fixed safe message for unknown transport values", async () => {
      enableTauriRuntime();
      mockTauriInvoke.mockRejectedValueOnce({ raw: "transport detail" });

      await expect(invoke("get_obsidian_vaults")).rejects.toThrow(
        "The operation failed. See runtime logs for details.",
      );
    });

    it("redacts sensitive messages before recording or display", async () => {
      enableTauriRuntime();
      const recorder = vi.fn();
      registerIpcFailureRecorder(recorder);
      mockTauriInvoke.mockRejectedValueOnce(
        "failed at C:\\Users\\alice\\skill.md token=ghp_secret",
      );

      const error = await invoke("get_skill_detail", {
        skillId: "hunter2",
        agentId: "C:\\Users\\alice\\skill.md",
      }).catch((value) => value);

      expect(String(error)).not.toContain("alice");
      expect(String(error)).not.toContain("ghp_secret");
      expect(recorder).toHaveBeenCalledWith(
        "get_skill_detail",
        { skillId: "[REDACTED]", agentId: "[REDACTED]" },
        error,
      );
    });

    it.each([
      String.raw`C:\Users\alice\private\skill.md`,
      "C:/Users/alice/private/skill.md",
      String.raw`..\alice\private\skill.md`,
      "../alice/private/skill.md",
      "/home/alice/private/skill.md",
      "ssh -i private.pem host -- command",
      "stdout: first line\nstderr: second line",
      "ghp_super_secret",
      "sk-live-secret",
      "-----BEGIN PRIVATE KEY-----",
      "file content: private thesis text",
      "https://example.invalid/path?token=secret",
      "quoted data: 'private value'",
      "UNKNOWN_ENV=private-value",
    ])("never exposes an adversarial raw rejection seed %s", async (seed) => {
      enableTauriRuntime();
      const recorder = vi.fn();
      registerIpcFailureRecorder(recorder);
      mockTauriInvoke.mockRejectedValueOnce(seed);

      const error = await invoke("get_skill_detail", {
        skillId: seed,
      }).catch((value) => value);
      const recorded = JSON.stringify(recorder.mock.calls);

      expect(String(error)).not.toContain(seed);
      expect(recorded).not.toContain(seed);
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
        // @ts-expect-error untyped fixture command name
        invoke("legacy_command", { a: 1 }),
      ).resolves.toEqual({ echoed: { a: 1 } });
    });

    it("turns synchronously throwing handlers into rejections", async () => {
      registerUntypedIpcFixture("boom_command", () => {
        throw new Error("fixture boom");
      });

      await expect(
        // @ts-expect-error untyped fixture command name
        invoke("boom_command"),
      ).rejects.toThrow(
        "The operation failed. See runtime logs for details.",
      );
    });

    it("rejects with IpcFixtureMissingError for unregistered commands", async () => {
      await expect(
        // @ts-expect-error untyped fixture command name
        invoke("nowhere_command"),
      ).rejects.toThrow(
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
