import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@/lib/ipc", async (importOriginal) => {
  const actual = await importOriginal<typeof import("@/lib/ipc")>();
  return {
    ...actual,
    invokeRaw: vi.fn(),
    registerIpcFailureRecorder: vi.fn(),
  };
});

import {
  IpcInvokeError,
  invokeRaw,
  isSafeCorrelationId,
  registerIpcFailureRecorder,
} from "@/lib/ipc";
import {
  __resetRuntimeLoggerForTest,
  emitFrontendRuntimeLog,
  installRuntimeLogger,
  recordFrontendRuntimeLog,
  recordIpcFailure,
} from "@/lib/runtimeLogger";

function enableTauriRuntime() {
  Object.defineProperty(window, "__TAURI__", {
    value: {},
    configurable: true,
  });
}

function recordedPayload(index = 0) {
  const call = vi.mocked(invokeRaw).mock.calls[index];
  return (call?.[1] as { payload?: Record<string, unknown> })?.payload;
}

describe("runtimeLogger", () => {
  beforeEach(() => {
    __resetRuntimeLoggerForTest();
    vi.clearAllMocks();
    Reflect.deleteProperty(window, "__TAURI__");
    Reflect.deleteProperty(window, "__TAURI_INTERNALS__");
    vi.mocked(invokeRaw).mockResolvedValue(undefined);
  });

  it("records global window errors with only fixed diagnostics", async () => {
    enableTauriRuntime();
    installRuntimeLogger();
    const planted =
      "C:\\Users\\alice\\private.log https://example.invalid/?token=secret";
    const error = Object.assign(new Error(planted), { code: "secret.value" });

    window.dispatchEvent(
      new ErrorEvent("error", {
        message: planted,
        filename: planted,
        lineno: 10,
        colno: 2,
        error,
      }),
    );

    await vi.waitFor(() => expect(invokeRaw).toHaveBeenCalledTimes(1));
    expect(recordedPayload()).toEqual(
      expect.objectContaining({
        level: "error",
        source: "window.error",
        message: "A window error occurred",
        details: { errorName: "Error", line: 10, column: 2 },
      }),
    );
    expect(JSON.stringify(vi.mocked(invokeRaw).mock.calls)).not.toContain(
      planted,
    );
    expect(JSON.stringify(vi.mocked(invokeRaw).mock.calls)).not.toContain(
      "secret.value",
    );
  });

  it("redacts both keys and values in explicit frontend events", async () => {
    enableTauriRuntime();
    installRuntimeLogger();
    const planted =
      "C:\\Users\\alice\\private.log https://example.invalid/?token=secret";

    emitFrontendRuntimeLog({
      level: "warn",
      source: "frontend.runtime",
      message: planted,
      details: {
        [planted]: planted,
        nested: { [planted]: "sk-test", visible: "ok" },
      },
    });

    await vi.waitFor(() => expect(invokeRaw).toHaveBeenCalledTimes(1));
    expect(recordedPayload()).toEqual(
      expect.objectContaining({
        level: "warn",
        source: "frontend.runtime",
        message: "Frontend runtime event",
        details: {
          field_0: "[REDACTED]",
          field_1: {
            field_0: "[REDACTED]",
            field_1: "[REDACTED]",
          },
        },
      }),
    );
    expect(JSON.stringify(vi.mocked(invokeRaw).mock.calls)).not.toContain(
      planted,
    );
  });

  it("records unhandled rejections without raw reason text", async () => {
    enableTauriRuntime();
    installRuntimeLogger();
    const planted = "ghp_secret C:\\private\\SKILL.md";
    const rejection = new Event("unhandledrejection") as PromiseRejectionEvent;
    Object.defineProperty(rejection, "reason", {
      value: Object.assign(new TypeError(planted), { code: "internal.host" }),
    });
    window.dispatchEvent(rejection);

    await vi.waitFor(() => expect(invokeRaw).toHaveBeenCalledTimes(1));
    expect(recordedPayload()).toEqual(
      expect.objectContaining({
        source: "window.unhandledrejection",
        message: "An unhandled promise rejection occurred",
        details: { errorName: "TypeError" },
      }),
    );
    expect(JSON.stringify(vi.mocked(invokeRaw).mock.calls)).not.toContain(
      planted,
    );
  });

  it("keeps backend correlation and a stable IPC failure code", async () => {
    enableTauriRuntime();
    installRuntimeLogger();
    const operationId = "123e4567-e89b-42d3-a456-426614174000";

    recordIpcFailure(
      "set_ai_api_key",
      { apiKey: "sk-test", provider: "openai" },
      new IpcInvokeError({
        code: "storage.unavailable",
        message: "Storage is unavailable.",
        retryable: false,
        correlationId: operationId,
      }),
    );

    await vi.waitFor(() => expect(invokeRaw).toHaveBeenCalledTimes(1));
    expect(registerIpcFailureRecorder).toHaveBeenCalledWith(recordIpcFailure);
    expect(recordedPayload()).toEqual(
      expect.objectContaining({
        source: "ipc.failure",
        message: "IPC command failed",
        operationId,
        details: {
          command: "set_ai_api_key",
          args: { field_0: "[REDACTED]", field_1: "[REDACTED]" },
          error: { code: "storage.unavailable", retryable: false },
          correlationOrigin: "backend",
        },
      }),
    );
  });

  it("replaces an unreviewed IPC code before invoking the Rust recorder", async () => {
    enableTauriRuntime();
    const operationId = "123e4567-e89b-42d3-a456-426614174000";

    recordIpcFailure(
      "get_central_skills",
      undefined,
      new IpcInvokeError({
        code: "secret.value",
        message: "private",
        retryable: true,
        correlationId: operationId,
      }),
    );

    await vi.waitFor(() => expect(invokeRaw).toHaveBeenCalledTimes(1));
    expect(recordedPayload()).toEqual(
      expect.objectContaining({
        operationId,
        details: expect.objectContaining({
          error: { code: "internal.unexpected", retryable: true },
        }),
      }),
    );
    expect(JSON.stringify(vi.mocked(invokeRaw).mock.calls)).not.toContain(
      "secret.value",
    );
  });

  it("preserves a reviewed direct-construction IPC code", async () => {
    enableTauriRuntime();

    recordIpcFailure(
      "install_marketplace_skill",
      undefined,
      new IpcInvokeError({
        code: "marketplace.install_failed",
        message: "The Marketplace skill could not be installed.",
        retryable: false,
      }),
    );

    await vi.waitFor(() => expect(invokeRaw).toHaveBeenCalledTimes(1));
    expect(recordedPayload()?.details).toEqual(
      expect.objectContaining({
        error: { code: "marketplace.install_failed", retryable: false },
      }),
    );
  });

  it("generates frontend correlation for legacy IPC rejections", async () => {
    enableTauriRuntime();
    recordIpcFailure(
      "get_central_skills",
      undefined,
      new IpcInvokeError({
        code: "storage.unavailable",
        message: "Storage is unavailable.",
        retryable: false,
      }),
    );

    await vi.waitFor(() => expect(invokeRaw).toHaveBeenCalledTimes(1));
    const payload = recordedPayload();
    expect(isSafeCorrelationId(payload?.operationId)).toBe(true);
    expect(payload?.details).toEqual(
      expect.objectContaining({ correlationOrigin: "frontend" }),
    );
  });

  it("generates frontend correlation for unknown failures", async () => {
    enableTauriRuntime();
    recordIpcFailure("get_central_skills", undefined, new Error("private"));

    await vi.waitFor(() => expect(invokeRaw).toHaveBeenCalledTimes(1));
    const payload = recordedPayload();
    expect(isSafeCorrelationId(payload?.operationId)).toBe(true);
    expect(payload?.details).toEqual(
      expect.objectContaining({
        error: { code: "internal.unexpected", retryable: false },
        correlationOrigin: "frontend",
      }),
    );
  });

  it("does not drop concurrent events while the recorder is pending", () => {
    enableTauriRuntime();
    vi.mocked(invokeRaw).mockImplementation(() => new Promise(() => undefined));

    recordIpcFailure("get_central_skills", undefined, new Error("first"));
    recordIpcFailure("list_targets", undefined, new Error("second"));

    expect(invokeRaw).toHaveBeenCalledTimes(2);
  });

  it("continues after a recorder write fails", async () => {
    enableTauriRuntime();
    vi.mocked(invokeRaw)
      .mockRejectedValueOnce(new Error("logger unavailable"))
      .mockResolvedValueOnce(undefined);

    await expect(
      recordFrontendRuntimeLog({ source: "frontend.runtime" }),
    ).resolves.toBeUndefined();
    await expect(
      recordFrontendRuntimeLog({ source: "frontend.runtime" }),
    ).resolves.toBeUndefined();
    expect(invokeRaw).toHaveBeenCalledTimes(2);
  });

  it("drops invalid source, message, and correlation fields", async () => {
    enableTauriRuntime();
    const planted = "https://private.invalid/?token=secret";
    await recordFrontendRuntimeLog({
      level: "error",
      source: planted as "frontend.runtime",
      message: planted,
      operationId: planted,
      details: planted,
    });

    expect(recordedPayload()).toEqual(
      expect.objectContaining({
        source: "frontend.runtime",
        message: "Frontend runtime event",
        details: "[REDACTED]",
        operationId: undefined,
      }),
    );
    expect(JSON.stringify(vi.mocked(invokeRaw).mock.calls)).not.toContain(
      planted,
    );
  });

  it("does not proxy console calls", async () => {
    enableTauriRuntime();
    installRuntimeLogger();
    const consoleErrorSpy = vi
      .spyOn(console, "error")
      .mockImplementation(() => undefined);

    console.error("console noise should stay local");
    await Promise.resolve();

    expect(consoleErrorSpy).toHaveBeenCalledWith(
      "console noise should stay local",
    );
    expect(invokeRaw).not.toHaveBeenCalled();
  });
});
