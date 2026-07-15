import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";

import { useUsageStore } from "../stores/usageStore";
import { useTargetStore } from "../stores/targetStore";
import { useUsageBootstrap } from "../pages/skillUsageBindings";
import {
  ipcInvokeCalls,
  ipcInvokedCommands,
  mockIpcCommand,
  mockIpcCommands,
} from "./ipcMock";

vi.mock("sonner", () => ({
  toast: {
    info: vi.fn(),
    error: vi.fn(),
    success: vi.fn(),
  },
}));

import { toast } from "sonner";

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

function refreshPayload(overrides: Record<string, unknown> = {}) {
  return {
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
    ...overrides,
  };
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

beforeEach(() => {
  toastInfoMock.mockReset();
  useUsageStore.setState({
    overview: null,
    recent: [],
    providers: [],
    detail: null,
    scope: null,
    selectedSkill: null,
    loading: false,
    refreshing: false,
    detailLoading: false,
    error: null,
    refreshError: null,
    usedCachedData: false,
    selectedSource: null,
    lastRefreshMs: null,
    ...initialActions,
  });
  useTargetStore.setState({
    targets: [{ id: "local", kind: "local", label: "Local", isActive: true }],
    activeTarget: {
      id: "local",
      kind: "local",
      label: "Local",
      isActive: true,
    },
  });
});

describe("usageStore", () => {
  it("refresh uses the single usage_refresh payload and stores all returned panels", async () => {
    mockIpcCommand("usage_refresh", refreshPayload());

    const result = await useUsageStore.getState().refresh(true);

    expect(result?.summary.callsWritten).toBe(4);
    expect(ipcInvokeCalls("usage_refresh")[0].args).toEqual({ force: true });
    expect(ipcInvokeCalls()).toHaveLength(1);

    const state = useUsageStore.getState();
    expect(state.overview?.kpis.totalCalls).toBe(4);
    expect(state.refreshing).toBe(false);
    expect(state.lastRefreshMs).toBe(1700000000000);
  });

  it("deduplicates concurrent refresh calls for the same active target", async () => {
    type RefreshPayload = ReturnType<typeof refreshPayload>;
    let resolveRefresh: (value: RefreshPayload) => void = () => {
      throw new Error("refresh resolver missing");
    };
    mockIpcCommand(
      "usage_refresh",
      () =>
        new Promise<RefreshPayload>((resolve) => {
          resolveRefresh = resolve;
        }),
    );

    const first = useUsageStore.getState().refresh(false);
    const second = useUsageStore.getState().refresh(true);

    expect(ipcInvokeCalls("usage_refresh")).toHaveLength(1);
    expect(second).not.toBeNull();

    resolveRefresh(refreshPayload());

    const [firstResult, secondResult] = await Promise.all([first, second]);
    expect(firstResult?.summary.callsWritten).toBe(4);
    expect(secondResult?.summary.callsWritten).toBe(4);
  });

  it("keeps cached payload on remote refresh failure and shows cached-data toast", async () => {
    mockIpcCommand(
      "usage_refresh",
      refreshPayload({
        summary: {
          cached: true,
          callsWritten: 0,
          providersAvailable: 0,
          scannedAtMs: 1700000000000,
        },
        scope: {
          targetId: "ssh-prod",
          label: "alice@prod",
          isRemote: true,
          remoteReachable: false,
        },
        usedCachedData: true,
        refreshError: "ssh timeout",
      }),
    );
    useTargetStore.setState({
      targets: [
        { id: "ssh-prod", kind: "ssh", label: "alice@prod", isActive: true },
      ],
      activeTarget: {
        id: "ssh-prod",
        kind: "ssh",
        label: "alice@prod",
        isActive: true,
      },
    });

    const result = await useUsageStore.getState().refresh(true);

    expect(result?.usedCachedData).toBe(true);
    expect(result?.refreshError).toContain("ssh timeout");
    expect(useUsageStore.getState().overview?.kpis.totalCalls).toBe(4);
    expect(useUsageStore.getState().scope?.remoteReachable).toBe(false);
    expect(useUsageStore.getState().error).toBeNull();
    expect(useUsageStore.getState().usedCachedData).toBe(true);
    expect(useUsageStore.getState().refreshError).toContain("ssh timeout");
    expect(toastInfoMock).toHaveBeenCalledTimes(1);
  });

  it("refresh writes error on backend failure and stops refreshing", async () => {
    mockIpcCommand("usage_refresh", () =>
      Promise.reject(new Error("io error")),
    );
    const summary = await useUsageStore.getState().refresh(false);
    expect(summary).toBeNull();
    const state = useUsageStore.getState();
    expect(state.error).toContain("io error");
    expect(state.refreshing).toBe(false);
  });

  it("selectSource reloads overview and recent with the selected source", async () => {
    mockIpcCommands({
      usage_get_overview: {
        ...overviewFixture(),
        kpis: {
          ...overviewFixture().kpis,
          totalCalls: 2,
          uniqueSessions: 2,
        },
      },
      usage_get_recent: [],
    });

    await useUsageStore.getState().selectSource("Claude Code");

    expect(useUsageStore.getState().selectedSource).toBe("Claude Code");
    expect(ipcInvokedCommands()).toEqual([
      "usage_get_overview",
      "usage_get_recent",
    ]);
    expect(ipcInvokeCalls("usage_get_overview")[0].args).toEqual({
      topSkillsLimit: 0,
      source: "Claude Code",
    });
    expect(ipcInvokeCalls("usage_get_recent")[0].args).toEqual({
      limit: 20,
      source: "Claude Code",
    });
  });

  it("refresh preserves selected source when the provider still has calls", async () => {
    useUsageStore.setState({ selectedSource: "Claude Code" });
    mockIpcCommands({
      usage_refresh: refreshPayload({
        providers: [
          {
            providerId: "claude-code",
            displayName: "Claude Code",
            available: true,
            callCount: 4,
            scannedAtMs: 1700000000000,
          },
        ],
      }),
      usage_get_overview: {
        ...overviewFixture(),
        kpis: {
          ...overviewFixture().kpis,
          totalCalls: 4,
          uniqueSessions: 2,
        },
      },
      usage_get_recent: [],
    });

    await useUsageStore.getState().refresh(true);
    await waitFor(() => expect(ipcInvokeCalls()).toHaveLength(3));

    expect(useUsageStore.getState().selectedSource).toBe("Claude Code");
    expect(ipcInvokeCalls("usage_get_overview")[0].args).toEqual({
      topSkillsLimit: 0,
      source: "Claude Code",
    });
    expect(ipcInvokeCalls("usage_get_recent")[0].args).toEqual({
      limit: 20,
      source: "Claude Code",
    });
  });

  it("refresh falls back to all platforms when selected provider has no calls", async () => {
    useUsageStore.setState({ selectedSource: "Claude Code" });
    mockIpcCommand(
      "usage_refresh",
      refreshPayload({
        summary: {
          cached: false,
          callsWritten: 0,
          providersAvailable: 0,
          scannedAtMs: 1700000000000,
        },
        providers: [
          {
            providerId: "claude-code",
            displayName: "Claude Code",
            available: true,
            callCount: 0,
            scannedAtMs: 1700000000000,
          },
        ],
      }),
    );

    await useUsageStore.getState().refresh(true);

    expect(useUsageStore.getState().selectedSource).toBeNull();
    expect(ipcInvokeCalls()).toHaveLength(1);
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
      activeTarget: {
        id: "ssh-prod",
        kind: "ssh",
        label: "prod",
        isActive: true,
      },
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
        }),
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

  it("prevents a slow source request from overwriting the latest selection", async () => {
    const claudeOverview = deferred<ReturnType<typeof overviewFixture>>();
    const claudeRecent = deferred<never[]>();
    const codexOverview = deferred<ReturnType<typeof overviewFixture>>();
    const codexRecent = deferred<never[]>();
    mockIpcCommand("usage_get_overview", ({ source }: { source: string | null }) =>
      source === "Claude Code" ? claudeOverview.promise : codexOverview.promise,
    );
    mockIpcCommand("usage_get_recent", ({ source }: { source: string | null }) =>
      source === "Claude Code" ? claudeRecent.promise : codexRecent.promise,
    );

    const first = useUsageStore.getState().selectSource("Claude Code");
    const second = useUsageStore.getState().selectSource("Codex CLI");
    codexOverview.resolve({
      ...overviewFixture(),
      kpis: { ...overviewFixture().kpis, totalCalls: 2 },
    });
    codexRecent.resolve([]);
    await second;
    expect(useUsageStore.getState().selectedSource).toBe("Codex CLI");
    expect(useUsageStore.getState().overview?.kpis.totalCalls).toBe(2);

    claudeOverview.resolve({
      ...overviewFixture(),
      kpis: { ...overviewFixture().kpis, totalCalls: 99 },
    });
    claudeRecent.resolve([]);
    await first;
    expect(useUsageStore.getState().selectedSource).toBe("Codex CLI");
    expect(useUsageStore.getState().overview?.kpis.totalCalls).toBe(2);
  });

  it("keeps filtered data visible during refresh and commits the refetch atomically", async () => {
    useUsageStore.setState({
      selectedSource: "Claude Code",
      overview: overviewFixture(),
      recent: [],
    });
    const filteredOverview = deferred<ReturnType<typeof overviewFixture>>();
    const filteredRecent = deferred<never[]>();
    mockIpcCommands({
      usage_refresh: refreshPayload({
        overview: {
          ...overviewFixture(),
          kpis: { ...overviewFixture().kpis, totalCalls: 100 },
        },
        providers: [
          {
            providerId: "claude-code",
            displayName: "Claude Code",
            available: true,
            callCount: 4,
            scannedAtMs: 1_700_000_000_000,
          },
        ],
      }),
      usage_get_overview: () => filteredOverview.promise,
      usage_get_recent: () => filteredRecent.promise,
    });

    const refresh = useUsageStore.getState().refresh(true);
    await waitFor(() =>
      expect(ipcInvokeCalls("usage_get_overview")).toHaveLength(1),
    );
    expect(useUsageStore.getState().overview?.kpis.totalCalls).toBe(4);

    filteredOverview.resolve({
      ...overviewFixture(),
      kpis: { ...overviewFixture().kpis, totalCalls: 3 },
    });
    filteredRecent.resolve([]);
    await refresh;
    expect(useUsageStore.getState().overview?.kpis.totalCalls).toBe(3);
    expect(useUsageStore.getState().selectedSource).toBe("Claude Code");
  });

  it("passes the selected source to detail and ignores stale detail results", async () => {
    const detailRequest = deferred<{
      skill: string;
      count: number;
      sessions: number;
      firstUsedMs: number;
      lastUsedMs: number;
      byProject: never[];
      weekly: never[];
      matchStatus: "unmatched";
      resolvedSkillId: null;
      staticTokenEstimate: null;
      staticByteCount: null;
    }>();
    useUsageStore.setState({ selectedSource: "Claude Code" });
    mockIpcCommand("usage_get_skill_detail", () => detailRequest.promise);

    const loading = useUsageStore.getState().loadDetail("review");
    expect(ipcInvokeCalls("usage_get_skill_detail")[0].args).toEqual({
      skill: "review",
      source: "Claude Code",
    });
    useUsageStore.getState().clearDetail();
    detailRequest.resolve({
      skill: "review",
      count: 1,
      sessions: 1,
      firstUsedMs: 1,
      lastUsedMs: 1,
      byProject: [],
      weekly: [],
      matchStatus: "unmatched",
      resolvedSkillId: null,
      staticTokenEstimate: null,
      staticByteCount: null,
    });
    await loading;
    expect(useUsageStore.getState().detail).toBeNull();
    expect(useUsageStore.getState().selectedSkill).toBeNull();
  });
});
