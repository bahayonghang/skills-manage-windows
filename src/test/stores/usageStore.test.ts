import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";

const { mockTauriListen } = vi.hoisted(() => ({
  mockTauriListen: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: mockTauriListen,
}));

import { UNUSED_REQUEST_THRESHOLD_DAYS, useUsageStore } from "@/stores/usageStore";
import { useTargetStore } from "@/stores/targetStore";
import { ipcFixtureError } from "@/lib/ipc/errors";
import { useUsageBootstrap } from "@/pages/skillUsageBindings";
import {
  ipcInvokeCalls,
  ipcInvokedCommands,
  mockIpcCommand,
  mockIpcCommands,
} from "@/test/support/ipcMock";

vi.mock("sonner", () => ({
  toast: {
    info: vi.fn(),
    error: vi.fn(),
    success: vi.fn(),
  },
}));

import { toast } from "sonner";

const toastInfoMock = vi.mocked(toast.info);
const toastErrorMock = vi.mocked(toast.error);
const initialActions = {
  refresh: useUsageStore.getState().refresh,
  subscribeTargetChanged: useUsageStore.getState().subscribeTargetChanged,
  subscribeScanCompleted: useUsageStore.getState().subscribeScanCompleted,
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
    scanning: false,
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
  toastErrorMock.mockReset();
  mockTauriListen.mockReset();
  mockTauriListen.mockResolvedValue(() => undefined);
  useUsageStore.setState({
    overview: null,
    recent: [],
    providers: [],
    detail: null,
    unused: null,
    unusedLoading: false,
    unusedError: null,
    pendingUnlinkKeys: {},
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
    backgroundScanning: false,
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
  it("unlinks an unused observation, tracks pending state, then refetches the report", async () => {
    const uninstall = deferred<void>();
    mockIpcCommands({
      uninstall_skill_from_agent: () => uninstall.promise,
      usage_get_unused_skills: { central: [], platforms: [] },
    });

    const unlink = useUsageStore
      .getState()
      .unlinkUnusedSkill("loose-skill", "codex", "codex::/skills/loose-skill");

    expect(
      useUsageStore.getState().pendingUnlinkKeys[
        "codex::/skills/loose-skill"
      ],
    ).toBe(true);
    expect(ipcInvokeCalls("uninstall_skill_from_agent")[0].args).toEqual({
      skillId: "loose-skill",
      agentId: "codex",
      rowId: "codex::/skills/loose-skill",
    });

    uninstall.resolve();
    await unlink;

    expect(ipcInvokedCommands()).toEqual([
      "uninstall_skill_from_agent",
      "usage_get_unused_skills",
    ]);
    expect(ipcInvokeCalls("usage_get_unused_skills")[0].args).toEqual({
      source: null,
      thresholdDays: UNUSED_REQUEST_THRESHOLD_DAYS,
    });
    expect(useUsageStore.getState().unused).toEqual({
      central: [],
      platforms: [],
    });
    expect(useUsageStore.getState().pendingUnlinkKeys).toEqual({});
  });

  it("formats unlink failures for the toast and does not refresh stale data", async () => {
    mockIpcCommand("uninstall_skill_from_agent", () =>
      Promise.reject(
        ipcFixtureError(
          "installation.pending_central_recovery",
          "unsafe C:/private/path should not be rendered",
        ),
      ),
    );

    await useUsageStore
      .getState()
      .unlinkUnusedSkill("blocked-skill", "claude-code");

    expect(ipcInvokedCommands()).toEqual(["uninstall_skill_from_agent"]);
    expect(toastErrorMock).toHaveBeenCalledTimes(1);
    const message = String(toastErrorMock.mock.calls[0][0]);
    expect(message).toMatch(/待恢复|pending Central recovery/i);
    expect(message).not.toContain("C:/private/path");
    expect(useUsageStore.getState().pendingUnlinkKeys).toEqual({});
  });

  it("refresh uses the single usage_refresh payload and stores all returned panels", async () => {
    mockIpcCommands({
      usage_refresh: refreshPayload(),
      usage_get_unused_skills: { central: [], platforms: [] },
    });

    const result = await useUsageStore.getState().refresh(true);

    expect(result?.summary.callsWritten).toBe(4);
    expect(ipcInvokeCalls("usage_refresh")[0].args).toEqual({ force: true });

    const state = useUsageStore.getState();
    expect(state.overview?.kpis.totalCalls).toBe(4);
    expect(state.refreshing).toBe(false);
    expect(state.lastRefreshMs).toBe(1700000000000);

    // 成功扫描后自动刷新未使用清单，且只请求最严阈值（视图本地重分类 30/60/90）
    await waitFor(() =>
      expect(ipcInvokeCalls("usage_get_unused_skills")).toHaveLength(1),
    );
    expect(ipcInvokeCalls("usage_get_unused_skills")[0].args).toEqual({
      source: null,
      thresholdDays: UNUSED_REQUEST_THRESHOLD_DAYS,
    });
    expect(useUsageStore.getState().unused).toEqual({
      central: [],
      platforms: [],
    });
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
      Promise.reject(ipcFixtureError("storage.unavailable", "io error")),
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
      usage_get_unused_skills: { central: [], platforms: [] },
    });

    await useUsageStore.getState().selectSource("Claude Code");

    expect(useUsageStore.getState().selectedSource).toBe("Claude Code");
    expect(ipcInvokedCommands()).toEqual([
      "usage_get_overview",
      "usage_get_recent",
      "usage_get_unused_skills",
    ]);
    expect(ipcInvokeCalls("usage_get_overview")[0].args).toEqual({
      topSkillsLimit: 0,
      source: "Claude Code",
    });
    expect(ipcInvokeCalls("usage_get_recent")[0].args).toEqual({
      limit: 20,
      source: "Claude Code",
    });
    // source 切换同样作用于未使用报告
    expect(ipcInvokeCalls("usage_get_unused_skills")[0].args).toEqual({
      source: "Claude Code",
      thresholdDays: UNUSED_REQUEST_THRESHOLD_DAYS,
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
      usage_get_unused_skills: { central: [], platforms: [] },
    });

    await useUsageStore.getState().refresh(true);
    await waitFor(() => expect(ipcInvokeCalls()).toHaveLength(4));

    expect(useUsageStore.getState().selectedSource).toBe("Claude Code");
    expect(ipcInvokeCalls("usage_get_overview")[0].args).toEqual({
      topSkillsLimit: 0,
      source: "Claude Code",
    });
    expect(ipcInvokeCalls("usage_get_recent")[0].args).toEqual({
      limit: 20,
      source: "Claude Code",
    });
    // 未使用报告沿用同一 source 口径
    expect(ipcInvokeCalls("usage_get_unused_skills")[0].args).toEqual({
      source: "Claude Code",
      thresholdDays: UNUSED_REQUEST_THRESHOLD_DAYS,
    });
  });

  it("refresh falls back to all platforms when selected provider has no calls", async () => {
    useUsageStore.setState({ selectedSource: "Claude Code" });
    mockIpcCommands({
      usage_refresh: refreshPayload({
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
      usage_get_unused_skills: { central: [], platforms: [] },
    });

    await useUsageStore.getState().refresh(true);

    expect(useUsageStore.getState().selectedSource).toBeNull();
    expect(ipcInvokeCalls("usage_refresh")).toHaveLength(1);
  });

  it("bootstrap refreshes when active target differs from cached usage scope inside TTL", async () => {
    const refresh = vi.fn(async () => null);
    const subscribeTargetChanged = vi.fn(async () => () => undefined);
    const subscribeScanCompleted = vi.fn(async () => () => undefined);
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
      subscribeScanCompleted,
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
    expect(subscribeScanCompleted).toHaveBeenCalledTimes(1);
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
    mockIpcCommand(
      "usage_get_overview",
      ({ source }: { source: string | null }) =>
        source === "Claude Code"
          ? claudeOverview.promise
          : codexOverview.promise,
    );
    mockIpcCommand(
      "usage_get_recent",
      ({ source }: { source: string | null }) =>
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

  it("clears visible page data and forces a refresh when the target changes", async () => {
    type TargetChangedHandler = (event: { payload: string }) => void;
    let targetChangedHandler: TargetChangedHandler | undefined;
    mockTauriListen.mockImplementation(
      async (_event: string, handler: unknown) => {
        targetChangedHandler = handler as TargetChangedHandler;
        return () => undefined;
      },
    );
    const refresh = vi.fn(async () => null);
    useUsageStore.setState({
      overview: overviewFixture(),
      recent: [
        {
          skill: "review",
          timestampMs: 1,
          project: "/home/demo/repo",
          sessionId: "s1",
          source: "Claude Code",
          matchStatus: "matched",
          resolvedSkillId: "review",
        },
      ],
      providers: [
        {
          providerId: "claude-code",
          displayName: "Claude Code",
          available: true,
          callCount: 4,
          scannedAtMs: 1700000000000,
        },
      ],
      selectedSource: "Claude Code",
      loading: true,
      unused: {
        central: [
          {
            skillId: "legacy-cleanup",
            name: "legacy-cleanup",
            matchStatus: "matched" as const,
            origin: "central" as const,
            agents: [
              {
                agentId: "claude-code",
                linkType: "symlink",
                installedPath: "C:/agents/claude/legacy-cleanup",
                hasPendingRecovery: false,
              },
            ],
            installs: [],
            installedPath: null,
            callCount: 0,
            lastUsedMs: null,
            staticTokenEstimate: 960,
            staticByteCount: 3_840,
            status: "never_used" as const,
          },
        ],
        platforms: [],
      },
      unusedLoading: true,
      backgroundScanning: true,
      refresh,
    });

    await useUsageStore.getState().subscribeTargetChanged();
    expect(targetChangedHandler).toBeDefined();
    targetChangedHandler!({ payload: "ssh-prod" });

    const state = useUsageStore.getState();
    expect(state.overview).toBeNull();
    expect(state.recent).toEqual([]);
    expect(state.providers).toEqual([]);
    expect(state.loading).toBe(false);
    expect(state.selectedSource).toBeNull();
    expect(state.unused).toBeNull();
    expect(state.unusedLoading).toBe(false);
    expect(state.unusedError).toBeNull();
    expect(state.backgroundScanning).toBe(false);
    expect(refresh).toHaveBeenCalledWith(true);
  });

  it("refreshUnused requests the strictest threshold once and commits the report", async () => {
    const report = {
      central: [
        {
          skillId: "legacy-cleanup",
          name: "legacy-cleanup",
          matchStatus: "matched",
          origin: "central",
          agents: [
            {
              agentId: "claude-code",
              linkType: "symlink",
              installedPath: "C:/agents/claude/legacy-cleanup",
              hasPendingRecovery: false,
            },
          ],
          installs: [],
          installedPath: null,
          callCount: 0,
          lastUsedMs: null,
          staticTokenEstimate: 960,
          staticByteCount: 3840,
          status: "never_used",
        },
      ],
      platforms: [],
    };
    mockIpcCommand("usage_get_unused_skills", report);

    await useUsageStore.getState().refreshUnused();

    expect(ipcInvokeCalls("usage_get_unused_skills")[0].args).toEqual({
      source: null,
      thresholdDays: UNUSED_REQUEST_THRESHOLD_DAYS,
    });
    const state = useUsageStore.getState();
    expect(state.unused).toEqual(report);
    expect(state.unusedLoading).toBe(false);
    expect(state.unusedError).toBeNull();
  });

  it("discards an out-of-order unused response", async () => {
    const first = deferred<{ central: never[]; platforms: never[] }>();
    const second = deferred<{
      central: { name: string }[];
      platforms: never[];
    }>();
    let call = 0;
    mockIpcCommand("usage_get_unused_skills", () =>
      call++ === 0 ? first.promise : second.promise,
    );

    const staleRequest = useUsageStore.getState().refreshUnused();
    const latestRequest = useUsageStore.getState().refreshUnused();

    second.resolve({ central: [{ name: "new" }], platforms: [] });
    await latestRequest;
    expect(useUsageStore.getState().unused?.central[0]?.name).toBe("new");

    first.resolve({ central: [], platforms: [] });
    await staleRequest;
    expect(useUsageStore.getState().unused?.central[0]?.name).toBe("new");
    expect(useUsageStore.getState().unusedLoading).toBe(false);
  });

  it("discards unused results when the target changes mid-flight", async () => {
    const pending = deferred<{ central: never[]; platforms: never[] }>();
    mockIpcCommand("usage_get_unused_skills", () => pending.promise);

    const request = useUsageStore.getState().refreshUnused();
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
    pending.resolve({ central: [], platforms: [] });
    await request;

    expect(useUsageStore.getState().unused).toBeNull();
  });

  it("records an unused-specific error without touching the page error", async () => {
    mockIpcCommand("usage_get_unused_skills", () =>
      Promise.reject(ipcFixtureError("storage.unavailable", "unused failed")),
    );

    await useUsageStore.getState().refreshUnused();

    const state = useUsageStore.getState();
    expect(state.unused).toBeNull();
    expect(state.unusedLoading).toBe(false);
    expect(state.unusedError).toContain("unused failed");
    expect(state.error).toBeNull();
  });

  it("marks backgroundScanning when refresh returns a cached page mid-scan", async () => {
    mockIpcCommands({
      usage_refresh: refreshPayload({ scanning: true }),
      usage_get_unused_skills: { central: [], platforms: [] },
    });

    await useUsageStore.getState().refresh(true);

    const state = useUsageStore.getState();
    expect(state.backgroundScanning).toBe(true);
    expect(state.overview?.kpis.totalCalls).toBe(4);
    expect(state.refreshing).toBe(false);
  });

  it("silently refetches page data when the background scan completes", async () => {
    type ScanCompletedHandler = (event: { payload: string }) => void;
    let scanCompletedHandler: ScanCompletedHandler | undefined;
    mockTauriListen.mockImplementation(
      async (event: string, handler: unknown) => {
        if (event === "usage://scan-completed") {
          scanCompletedHandler = handler as ScanCompletedHandler;
        }
        return () => undefined;
      },
    );
    const freshOverview = {
      ...overviewFixture(),
      kpis: { ...overviewFixture().kpis, totalCalls: 42 },
    };
    mockIpcCommands({
      usage_get_overview: freshOverview,
      usage_get_recent: [],
      usage_get_unused_skills: { central: [], platforms: [] },
    });
    useUsageStore.setState({
      overview: overviewFixture(),
      backgroundScanning: true,
      refreshing: false,
      loading: false,
    });

    await useUsageStore.getState().subscribeScanCompleted();
    expect(scanCompletedHandler).toBeDefined();
    scanCompletedHandler!({ payload: "local" });

    await waitFor(() =>
      expect(useUsageStore.getState().overview?.kpis.totalCalls).toBe(42),
    );
    expect(ipcInvokeCalls("usage_get_overview")[0].args).toEqual({
      topSkillsLimit: 0,
      source: null,
    });
    expect(ipcInvokeCalls("usage_get_recent")[0].args).toEqual({
      limit: 20,
      source: null,
    });
    expect(ipcInvokeCalls("usage_get_unused_skills")).toHaveLength(1);
    // 静默更新：无 loading/refreshing 抖动，无 toast，无 error
    const state = useUsageStore.getState();
    expect(state.backgroundScanning).toBe(false);
    expect(state.refreshing).toBe(false);
    expect(state.loading).toBe(false);
    expect(state.error).toBeNull();
    expect(toastInfoMock).not.toHaveBeenCalled();
  });

  it("ignores scan-completed events for other targets", async () => {
    type ScanCompletedHandler = (event: { payload: string }) => void;
    let scanCompletedHandler: ScanCompletedHandler | undefined;
    mockTauriListen.mockImplementation(
      async (event: string, handler: unknown) => {
        if (event === "usage://scan-completed") {
          scanCompletedHandler = handler as ScanCompletedHandler;
        }
        return () => undefined;
      },
    );
    mockIpcCommand("usage_get_overview", overviewFixture());
    useUsageStore.setState({
      overview: overviewFixture(),
      backgroundScanning: true,
    });

    await useUsageStore.getState().subscribeScanCompleted();
    scanCompletedHandler!({ payload: "ssh-prod" });
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(ipcInvokeCalls()).toHaveLength(0);
    expect(useUsageStore.getState().backgroundScanning).toBe(true);
    expect(useUsageStore.getState().overview?.kpis.totalCalls).toBe(4);
  });

  it("discards a silent scan refetch when the source changes mid-flight", async () => {
    type ScanCompletedHandler = (event: { payload: string }) => void;
    let scanCompletedHandler: ScanCompletedHandler | undefined;
    mockTauriListen.mockImplementation(
      async (event: string, handler: unknown) => {
        if (event === "usage://scan-completed") {
          scanCompletedHandler = handler as ScanCompletedHandler;
        }
        return () => undefined;
      },
    );
    const staleOverview = deferred<ReturnType<typeof overviewFixture>>();
    mockIpcCommands({
      usage_get_overview: ({ source }: { source: string | null }) =>
        source === null
          ? staleOverview.promise
          : {
              ...overviewFixture(),
              kpis: { ...overviewFixture().kpis, totalCalls: 7 },
            },
      usage_get_recent: [],
      usage_get_unused_skills: { central: [], platforms: [] },
    });
    useUsageStore.setState({ overview: overviewFixture() });

    await useUsageStore.getState().subscribeScanCompleted();
    scanCompletedHandler!({ payload: "local" });
    await waitFor(() =>
      expect(ipcInvokeCalls("usage_get_overview")).toHaveLength(1),
    );

    // 静默重取尚未返回时用户切换 source：旧响应必须被序列号守卫丢弃
    await useUsageStore.getState().selectSource("Codex CLI");
    expect(useUsageStore.getState().overview?.kpis.totalCalls).toBe(7);

    staleOverview.resolve({
      ...overviewFixture(),
      kpis: { ...overviewFixture().kpis, totalCalls: 99 },
    });
    await waitFor(() =>
      expect(ipcInvokeCalls("usage_get_overview")).toHaveLength(2),
    );
    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(useUsageStore.getState().selectedSource).toBe("Codex CLI");
    expect(useUsageStore.getState().overview?.kpis.totalCalls).toBe(7);
  });
});
