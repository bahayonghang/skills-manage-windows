import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";

import { useUsageStore } from "../stores/usageStore";
import { useTargetStore } from "../stores/targetStore";
import { useUsageBootstrap } from "../pages/skillUsageBindings";

vi.mock("sonner", () => ({
  toast: {
    info: vi.fn(),
    error: vi.fn(),
    success: vi.fn(),
  },
}));

vi.mock("@/lib/tauri", () => ({
  invoke: vi.fn(),
  listen: vi.fn(),
  isTauriRuntime: vi.fn(() => true),
}));

import { invoke, isTauriRuntime } from "@/lib/tauri";
import { toast } from "sonner";

const invokeMock = vi.mocked(invoke);
const runtimeMock = vi.mocked(isTauriRuntime);
const toastInfoMock = vi.mocked(toast.info);
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
      uniqueSessions: 2,
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
  toastInfoMock.mockReset();
  useUsageStore.setState({
    overview: null,
    recent: [],
    providers: [],
    detail: null,
    scope: null,
    loading: false,
    refreshing: false,
    error: null,
    selectedSource: null,
    lastRefreshMs: null,
    ...initialActions,
  });
  useTargetStore.setState({
    targets: [{ id: "local", kind: "local", label: "Local", isActive: true }],
    activeTarget: { id: "local", kind: "local", label: "Local", isActive: true },
  });
});

describe("usageStore", () => {
  it("refresh uses the single usage_refresh payload and stores all returned panels", async () => {
    invokeMock.mockResolvedValue({
      summary: {
        cached: false,
        callsWritten: 4,
        providersAvailable: 1,
        scannedAtMs: 1700000000000,
      },
      overview: overviewFixture(),
      recent: [],
      providers: [],
      scope: {
        targetId: "local",
        label: "Local",
        isRemote: false,
        remoteReachable: false,
      },
      usedCachedData: false,
      refreshError: null,
    });

    const result = await useUsageStore.getState().refresh(true);

    expect(result?.summary.callsWritten).toBe(4);
    expect(invokeMock).toHaveBeenCalledWith("usage_refresh", { force: true });
    expect(invokeMock).toHaveBeenCalledTimes(1);

    const state = useUsageStore.getState();
    expect(state.overview?.kpis.totalCalls).toBe(4);
    expect(state.refreshing).toBe(false);
    expect(state.lastRefreshMs).toBe(1700000000000);
  });

  it("deduplicates concurrent refresh calls for the same active target", async () => {
    type RefreshPayload = {
      summary: {
        cached: boolean;
        callsWritten: number;
        providersAvailable: number;
        scannedAtMs: number;
      };
      overview: ReturnType<typeof overviewFixture>;
      recent: [];
      providers: [];
      scope: {
        targetId: string;
        label: string;
        isRemote: boolean;
        remoteReachable: boolean;
      };
      usedCachedData: boolean;
      refreshError: null;
    };
    let resolveRefresh: (value: RefreshPayload) => void = () => {
      throw new Error("refresh resolver missing");
    };
    invokeMock.mockImplementation(
      () =>
        new Promise<RefreshPayload>((resolve) => {
          resolveRefresh = resolve;
        })
    );

    const first = useUsageStore.getState().refresh(false);
    const second = useUsageStore.getState().refresh(true);

    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(second).not.toBeNull();

    resolveRefresh({
      summary: {
        cached: false,
        callsWritten: 4,
        providersAvailable: 1,
        scannedAtMs: 1700000000000,
      },
      overview: overviewFixture(),
      recent: [],
      providers: [],
      scope: {
        targetId: "local",
        label: "Local",
        isRemote: false,
        remoteReachable: false,
      },
      usedCachedData: false,
      refreshError: null,
    });

    const [firstResult, secondResult] = await Promise.all([first, second]);
    expect(firstResult?.summary.callsWritten).toBe(4);
    expect(secondResult?.summary.callsWritten).toBe(4);
  });

  it("keeps cached payload on remote refresh failure and shows cached-data toast", async () => {
    invokeMock.mockResolvedValue({
      summary: {
        cached: true,
        callsWritten: 0,
        providersAvailable: 0,
        scannedAtMs: 1700000000000,
      },
      overview: overviewFixture(),
      recent: [],
      providers: [],
      scope: {
        targetId: "ssh-prod",
        label: "alice@prod",
        isRemote: true,
        remoteReachable: false,
      },
      usedCachedData: true,
      refreshError: "ssh timeout",
    });
    useTargetStore.setState({
      targets: [{ id: "ssh-prod", kind: "ssh", label: "alice@prod", isActive: true }],
      activeTarget: { id: "ssh-prod", kind: "ssh", label: "alice@prod", isActive: true },
    });

    const result = await useUsageStore.getState().refresh(true);

    expect(result?.usedCachedData).toBe(true);
    expect(result?.refreshError).toContain("ssh timeout");
    expect(useUsageStore.getState().overview?.kpis.totalCalls).toBe(4);
    expect(useUsageStore.getState().scope?.remoteReachable).toBe(false);
    expect(useUsageStore.getState().error).toBeNull();
    expect(toastInfoMock).toHaveBeenCalledTimes(1);
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

  it("selectSource reloads overview and recent with the selected source", async () => {
    invokeMock.mockResolvedValueOnce({
      ...overviewFixture(),
      kpis: {
        ...overviewFixture().kpis,
        totalCalls: 2,
        uniqueSessions: 2,
      },
    });
    invokeMock.mockResolvedValueOnce([]);

    await useUsageStore.getState().selectSource("Claude Code");

    expect(useUsageStore.getState().selectedSource).toBe("Claude Code");
    expect(invokeMock).toHaveBeenNthCalledWith(1, "usage_get_overview", {
      topSkillsLimit: 50,
      source: "Claude Code",
    });
    expect(invokeMock).toHaveBeenNthCalledWith(2, "usage_get_recent", {
      limit: 20,
      source: "Claude Code",
    });
  });

  it("refresh preserves selected source when the provider still has calls", async () => {
    useUsageStore.setState({ selectedSource: "Claude Code" });
    invokeMock.mockResolvedValueOnce({
      summary: {
        cached: false,
        callsWritten: 4,
        providersAvailable: 1,
        scannedAtMs: 1700000000000,
      },
      overview: overviewFixture(),
      recent: [],
      providers: [
        {
          providerId: "claude-code",
          displayName: "Claude Code",
          available: true,
          callCount: 4,
          scannedAtMs: 1700000000000,
        },
      ],
      scope: {
        targetId: "local",
        label: "Local",
        isRemote: false,
        remoteReachable: false,
      },
      usedCachedData: false,
      refreshError: null,
    });
    invokeMock.mockResolvedValueOnce({
      ...overviewFixture(),
      kpis: {
        ...overviewFixture().kpis,
        totalCalls: 4,
        uniqueSessions: 2,
      },
    });
    invokeMock.mockResolvedValueOnce([]);

    await useUsageStore.getState().refresh(true);
    await waitFor(() => expect(invokeMock).toHaveBeenCalledTimes(3));

    expect(useUsageStore.getState().selectedSource).toBe("Claude Code");
    expect(invokeMock).toHaveBeenNthCalledWith(2, "usage_get_overview", {
      topSkillsLimit: 50,
      source: "Claude Code",
    });
    expect(invokeMock).toHaveBeenNthCalledWith(3, "usage_get_recent", {
      limit: 20,
      source: "Claude Code",
    });
  });

  it("refresh falls back to all platforms when selected provider has no calls", async () => {
    useUsageStore.setState({ selectedSource: "Claude Code" });
    invokeMock.mockResolvedValue({
      summary: {
        cached: false,
        callsWritten: 0,
        providersAvailable: 0,
        scannedAtMs: 1700000000000,
      },
      overview: overviewFixture(),
      recent: [],
      providers: [
        {
          providerId: "claude-code",
          displayName: "Claude Code",
          available: true,
          callCount: 0,
          scannedAtMs: 1700000000000,
        },
      ],
      scope: {
        targetId: "local",
        label: "Local",
        isRemote: false,
        remoteReachable: false,
      },
      usedCachedData: false,
      refreshError: null,
    });

    await useUsageStore.getState().refresh(true);

    expect(useUsageStore.getState().selectedSource).toBeNull();
    expect(invokeMock).toHaveBeenCalledTimes(1);
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

  it("bootstrap cleans up late listener registration after unmount", async () => {
    let resolveUnlisten: (value: () => void) => void = () => {
      throw new Error("listener resolver missing");
    };
    const unlisten = vi.fn();
    const subscribeTargetChanged = vi.fn(
      () =>
        new Promise<() => void>((resolve) => {
          resolveUnlisten = resolve;
        })
    );
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
      refresh: vi.fn(async () => null),
      subscribeTargetChanged,
    });

    const { unmount } = renderHook(() => useUsageBootstrap());
    unmount();
    resolveUnlisten(unlisten);

    await waitFor(() => expect(unlisten).toHaveBeenCalledTimes(1));
  });

  it("resolveSkillId returns null on backend failure", async () => {
    invokeMock.mockRejectedValue(new Error("nope"));
    const id = await useUsageStore.getState().resolveSkillId("review");
    expect(id).toBeNull();
  });
});
