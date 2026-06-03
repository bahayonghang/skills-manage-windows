import { beforeEach, describe, expect, it, vi } from "vitest";

const mockShow = vi.hoisted(() => vi.fn());
const mockTauriInvoke = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/api/core", () => ({
  invoke: mockTauriInvoke,
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    show: mockShow,
  }),
}));

import {
  __resetMainWindowReadyForTest,
  invoke,
  isTauriRuntime,
  registerIpcFailureRecorder,
  showMainWindowWhenReady,
} from "@/lib/tauri";

describe("tauri helpers", () => {
  beforeEach(() => {
    mockShow.mockReset();
    mockTauriInvoke.mockReset();
    registerIpcFailureRecorder(null);
    __resetMainWindowReadyForTest();
    Reflect.deleteProperty(window, "__TAURI__");
    Reflect.deleteProperty(window, "__TAURI_INTERNALS__");
  });

  it("does not show the main window outside the Tauri runtime", async () => {
    expect(isTauriRuntime()).toBe(false);

    await showMainWindowWhenReady();

    expect(mockShow).not.toHaveBeenCalled();
  });

  it("shows the main window once in the Tauri runtime", async () => {
    Object.defineProperty(window, "__TAURI__", {
      value: {},
      configurable: true,
    });
    mockShow.mockResolvedValue(undefined);

    await showMainWindowWhenReady();
    await showMainWindowWhenReady();

    expect(mockShow).toHaveBeenCalledTimes(1);
  });

  it("records IPC failures through the registered diagnostic hook", async () => {
    const recorder = vi.fn();
    const error = new Error("backend failed");
    registerIpcFailureRecorder(recorder);
    mockTauriInvoke.mockRejectedValueOnce(error);

    await expect(invoke("dangerous_command", { token: "secret" })).rejects.toThrow(
      "backend failed"
    );

    expect(recorder).toHaveBeenCalledWith(
      "dangerous_command",
      { token: "secret" },
      error
    );
  });

  it("does not recursively record runtime-log IPC failures", async () => {
    const recorder = vi.fn();
    registerIpcFailureRecorder(recorder);
    mockTauriInvoke.mockRejectedValueOnce(new Error("logger failed"));

    await expect(
      invoke("record_frontend_runtime_log", { payload: { message: "boom" } })
    ).rejects.toThrow("logger failed");

    expect(recorder).not.toHaveBeenCalled();
  });
});
