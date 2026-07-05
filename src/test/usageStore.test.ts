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

beforeEach(() => {
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
      topSkillsLimit: 50,
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
      topSkillsLimit: 50,
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

  it("resolveSkillId returns null on backend failure", async () => {
    mockIpcCommand("usage_resolve_skill_id", () =>
      Promise.reject(new Error("nope")),
    );
    const id = await useUsageStore.getState().resolveSkillId("review");
    expect(id).toBeNull();
  });
});
