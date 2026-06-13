import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@/lib/tauri", () => ({
  invokeRaw: vi.fn(),
  registerIpcFailureRecorder: vi.fn(),
}));

import { invokeRaw, registerIpcFailureRecorder } from "@/lib/tauri";
import {
  __resetRuntimeLoggerForTest,
  emitFrontendRuntimeLog,
  installRuntimeLogger,
  recordIpcFailure,
} from "@/lib/runtimeLogger";

function enableTauriRuntime() {
  Object.defineProperty(window, "__TAURI__", {
    value: {},
    configurable: true,
  });
}

describe("runtimeLogger", () => {
  beforeEach(() => {
    __resetRuntimeLoggerForTest();
    vi.clearAllMocks();
    Reflect.deleteProperty(window, "__TAURI__");
    Reflect.deleteProperty(window, "__TAURI_INTERNALS__");
    vi.mocked(invokeRaw).mockResolvedValue(undefined);
  });

  it("records global window errors", async () => {
    enableTauriRuntime();
    installRuntimeLogger();

    window.dispatchEvent(
      new ErrorEvent("error", {
        message: "Render exploded",
        filename: "OperationLogsView.tsx",
        lineno: 10,
        colno: 2,
        error: new Error("Render exploded"),
      })
    );

    await vi.waitFor(() => {
      expect(invokeRaw).toHaveBeenCalledWith("record_frontend_runtime_log", {
        payload: expect.objectContaining({
          level: "error",
          source: "window.error",
          message: "Render exploded",
        }),
      });
    });
  });

  it("records explicit frontend runtime events with local redaction", async () => {
    enableTauriRuntime();
    installRuntimeLogger();

    emitFrontendRuntimeLog({
      level: "warn",
      source: "frontend.runtime",
      message: "Manual diagnostic",
      details: {
        token: "secret-token",
        nested: { apiKey: "sk-test", visible: "ok" },
      },
    });

    await vi.waitFor(() => {
      expect(invokeRaw).toHaveBeenCalledWith("record_frontend_runtime_log", {
        payload: expect.objectContaining({
          level: "warn",
          source: "frontend.runtime",
          details: {
            token: "[REDACTED]",
            nested: { apiKey: "[REDACTED]", visible: "ok" },
          },
        }),
      });
    });
  });

  it("registers and records IPC failures without leaking sensitive args", async () => {
    enableTauriRuntime();
    installRuntimeLogger();

    expect(registerIpcFailureRecorder).toHaveBeenCalledWith(recordIpcFailure);

    recordIpcFailure(
      "set_ai_api_key",
      { apiKey: "sk-test", provider: "openai" },
      new Error("Backend rejected key")
    );

    await vi.waitFor(() => {
      expect(invokeRaw).toHaveBeenCalledWith("record_frontend_runtime_log", {
        payload: expect.objectContaining({
          level: "error",
          source: "ipc.failure",
          message: "IPC command failed: set_ai_api_key",
          details: expect.objectContaining({
            command: "set_ai_api_key",
            args: { apiKey: "[REDACTED]", provider: "openai" },
          }),
        }),
      });
    });
  });

  it("does not proxy console calls", async () => {
    enableTauriRuntime();
    installRuntimeLogger();

    const consoleErrorSpy = vi.spyOn(console, "error").mockImplementation(() => undefined);

    console.error("console noise should stay local");
    await Promise.resolve();

    expect(consoleErrorSpy).toHaveBeenCalledWith("console noise should stay local");
    expect(invokeRaw).not.toHaveBeenCalled();
  });
});
