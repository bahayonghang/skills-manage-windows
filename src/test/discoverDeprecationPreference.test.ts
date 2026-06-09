import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@/lib/tauri", () => ({
  invoke: vi.fn(),
  isTauriRuntime: vi.fn(() => true),
}));

import {
  loadDiscoverDeprecationDismissed,
  saveDiscoverDeprecationDismissed,
} from "@/lib/discoverDeprecationPreference";
import { invoke, isTauriRuntime } from "@/lib/tauri";

const SETTING_KEY = "discover_deprecation_dismissed";

describe("discoverDeprecationPreference", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(isTauriRuntime).mockReturnValue(true);
  });

  it("keeps the banner hidden outside Tauri without IPC", async () => {
    vi.mocked(isTauriRuntime).mockReturnValue(false);

    await expect(loadDiscoverDeprecationDismissed()).resolves.toBe(true);
    await saveDiscoverDeprecationDismissed();

    expect(invoke).not.toHaveBeenCalled();
  });

  it("loads the dismissed flag from the existing setting key", async () => {
    vi.mocked(invoke).mockResolvedValueOnce("true");

    await expect(loadDiscoverDeprecationDismissed()).resolves.toBe(true);

    expect(invoke).toHaveBeenCalledWith("get_setting", { key: SETTING_KEY });
  });

  it("persists dismissal using the existing setting key", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(undefined);

    await saveDiscoverDeprecationDismissed();

    expect(invoke).toHaveBeenCalledWith("set_setting", {
      key: SETTING_KEY,
      value: "true",
    });
  });
});
