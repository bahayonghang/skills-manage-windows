import { beforeEach, describe, expect, it, vi } from "vitest";

const mockInvoke = vi.hoisted(() => vi.fn());

vi.mock("@tauri-apps/api/core", () => ({
  invoke: mockInvoke,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ show: vi.fn() }),
}));

import { useUpdateCenterStore } from "@/stores/updateCenterStore";
import type { SkillUpdateInventory } from "@/types/skillUpdateInventory";

function emptyInventory(): SkillUpdateInventory {
  return {
    updatable: [],
    remoteAdded: [],
    remoteMissing: [],
    platformDuplicates: [],
    deletedPlatformCopies: [],
    orphans: [],
    failedRepositories: [],
    generatedAt: "2026-06-11T00:00:00.000Z",
  };
}

describe("updateCenterStore", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    Object.defineProperty(window, "__TAURI__", {
      value: {},
      configurable: true,
    });
    useUpdateCenterStore.setState({
      inventory: null,
      isRefreshing: false,
      isApplying: false,
      isForcing: false,
      lastRefreshedAt: null,
      error: null,
    });
  });

  it("bypasses the snapshot cache for manual refresh by default", async () => {
    mockInvoke.mockResolvedValueOnce(emptyInventory());

    await useUpdateCenterStore.getState().refresh({ kind: "all", mode: "sync" });

    expect(mockInvoke).toHaveBeenCalledWith("refresh_skill_update_inventory", {
      scope: { kind: "all", mode: "sync", cachePolicy: "bypass" },
    });
  });
});
