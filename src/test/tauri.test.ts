import { beforeEach, describe, expect, it, vi } from "vitest";

const mockShow = vi.fn();

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    show: mockShow,
  }),
}));

import {
  __resetMainWindowReadyForTest,
  isTauriRuntime,
  showMainWindowWhenReady,
} from "@/lib/tauri";

describe("tauri helpers", () => {
  beforeEach(() => {
    mockShow.mockReset();
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
});
