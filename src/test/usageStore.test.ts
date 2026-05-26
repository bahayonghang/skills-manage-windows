import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";

import { useUsageStore } from "../stores/usageStore";
import { useTargetStore } from "../stores/targetStore";
import { useUsageBootstrap } from "../pages/skillUsageBindings";

vi.mock("@/lib/tauri", () => ({
  invoke: vi.fn(),
  listen: vi.fn(),
  isTauriRuntime: vi.fn(() => true),
}));

import { invoke, isTauriRuntime } from "@/lib/tauri";

const invokeMock = vi.mocked(invoke);
const runtimeMock = vi.mocked(isTauriRuntime);
const initialActions = {
  refresh: useUsageStore.getState().refresh,
  subscribeTargetChanged: useUsageStore.getState().subscribeTargetChanged,
};

function overviewFixture() {
  return {
    kpis: {
      totalCalls: 4,
      uniqueSkills: 2,
      uniqueProjects: 1,
      uniqueSources: 1,
    },
    topSkills: [],
    heatmap: [],
    lastScanMs: 1700000000000,
  };
}

beforeEach(() => {
  invokeMock.mockReset();
  runtimeMock.mockReset();
  runtimeMock.mockReturnValue(true);
  useUsageStore.setState({
    overview: null,
    recent: [],
    providers: [],
    detail: null,
    scope: null,
    loading: false,
    refreshing: false,
    error: null,
    lastRefreshMs: null,
    ...initialActions,
  });
  useTargetStore.setState({
    targets: [{ id: "local", kind: "local", label: "Local", isActive: true }],
    activeTarget: { id: "local", kind: "local", label: "Local", isActive: true },
  });
});

describe("usageStore", () => {
  it("refresh dispatches usage_refresh + 4 follow-up loads", async () => {
    invokeMock.mockImplementation(async (cmd: string) => {
      if (cmd === "usage_refresh") {
        return {
          cached: false,
          callsWritten: 4,
          providersAvailable: 1,
          scannedAtMs: 1700000000000,
        };
      }
      if (cmd === "usage_get_overview") {
        return overviewFixture();
      }
      if (cmd === "usage_get_recent") return [];
      if (cmd === "usage_get_providers") return [];
      if (cmd === "usage_get_scope_info") {
        return {
          targetId: "local",
          label: "Local",
          isRemote: false,
          remoteReachable: false,
        };
      }
      throw new Error(`unexpected command: ${cmd}`);
    });

    const summary = await useUsageStore.getState().refresh(true);

    expect(summary?.callsWritten).toBe(4);
    expect(invokeMock).toHaveBeenCalledWith("usage_refresh", { force: true });
    expect(invokeMock).toHaveBeenCalledWith("usage_get_overview", {
      topSkillsLimit: 50,
    });
    expect(invokeMock).toHaveBeenCalledWith("usage_get_recent", { limit: 20 });
    expect(invokeMock).toHaveBeenCalledWith("usage_get_providers");
    expect(invokeMock).toHaveBeenCalledWith("usage_get_scope_info");

    const state = useUsageStore.getState();
    expect(state.overview?.kpis.totalCalls).toBe(4);
    expect(state.refreshing).toBe(false);
    expect(state.lastRefreshMs).not.toBeNull();
  });

  it("refresh writes error on backend failure and stops refreshing", async () => {
    invokeMock.mockRejectedValue(new Error("io error"));
    const summary = await useUsageStore.getState().refresh(false);
    expect(summary).toBeNull();
    const state = useUsageStore.getState();
    expect(state.error).toContain("io error");
    expect(state.refreshing).toBe(false);
  });

  it("falls back to fixture overview when not running in Tauri", async () => {
    runtimeMock.mockReturnValue(false);
    await useUsageStore.getState().refresh(false);
    const state = useUsageStore.getState();
    expect(state.overview?.heatmap.length).toBe(16 * 7);
    expect(state.providers.length).toBe(8);
  });

  it("bootstrap refreshes when active target differs from cached usage scope inside TTL", async () => {
    const refresh = vi.fn(async () => null);
    const subscribeTargetChanged = vi.fn(async () => () => undefined);
    useUsageStore.setState({
      overview: overviewFixture(),
      recent: [],
      providers: [],
      scope: {
        targetId: "local",
        label: "Local",
        isRemote: false,
        remoteReachable: false,
      },
      lastRefreshMs: Date.now(),
      refresh,
      subscribeTargetChanged,
    });
    useTargetStore.setState({
      targets: [
        { id: "local", kind: "local", label: "Local", isActive: false },
        { id: "ssh-prod", kind: "ssh", label: "prod", isActive: true },
      ],
      activeTarget: { id: "ssh-prod", kind: "ssh", label: "prod", isActive: true },
    });

    renderHook(() => useUsageBootstrap());

    await waitFor(() => expect(refresh).toHaveBeenCalledWith(false));
    expect(subscribeTargetChanged).toHaveBeenCalledTimes(1);
  });

  it("resolveSkillId returns null on backend failure", async () => {
    invokeMock.mockRejectedValue(new Error("nope"));
    const id = await useUsageStore.getState().resolveSkillId("review");
    expect(id).toBeNull();
  });
});
