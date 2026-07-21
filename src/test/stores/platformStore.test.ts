import { describe, it, expect, beforeEach } from "vitest";
import {
  AgentWithStatus,
  BootstrapSnapshot,
  CentralTopTag,
  DashboardCentralSummary,
  PlatformPathMap,
  SkillCountsSummary,
} from "@/types";
import { usePlatformStore } from "@/stores/platformStore";
import {
  ipcInvokeCalls,
  ipcInvokedCommands,
  mockIpcCommand,
  mockIpcCommands,
} from "@/test/support/ipcMock";

const mockAgents: AgentWithStatus[] = [
  {
    id: "claude-code",
    display_name: "Claude Code",
    category: "coding",
    global_skills_dir: "~/.claude/skills/",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
  {
    id: "openclaw",
    display_name: "OpenClaw",
    category: "lobster",
    global_skills_dir: "~/.openclaw/skills/",
    is_detected: true,
    is_builtin: true,
    is_enabled: false,
  },
  {
    id: "central",
    display_name: "Central Skills",
    category: "central",
    global_skills_dir: "~/.skillsmanage/skills/",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
];

const mockPlatformPaths: PlatformPathMap = Object.fromEntries(
  mockAgents.map((agent) => [
    agent.id,
    {
      global_skills_dir: agent.global_skills_dir,
      project_skills_dir: agent.project_skills_dir ?? null,
    },
  ]),
);

const mockBootstrapSnapshot: BootstrapSnapshot = {
  agents: mockAgents,
  cachedSkillCounts: {
    "claude-code": 5,
    openclaw: 0,
    central: 3,
  },
  collectionCount: 2,
  lastScanAt: "2026-04-23T01:00:00Z",
  scanState: "idle",
};

const refreshedSnapshot: BootstrapSnapshot = {
  ...mockBootstrapSnapshot,
  cachedSkillCounts: {
    "claude-code": 8,
    openclaw: 0,
    central: 4,
  },
  lastScanAt: "2026-04-23T01:05:00Z",
};

const mockCountsSummary: SkillCountsSummary = {
  cachedSkillCounts: {
    "claude-code": 9,
    openclaw: 0,
    central: 4,
  },
  lastScanAt: "2026-04-23T01:06:00Z",
  scanState: "idle",
};

const mockCategoryVisibility = {
  coding: true,
  lobster: true,
};

const allHiddenCategoryVisibility = {
  coding: false,
  lobster: false,
};

const mockScanResult = {
  total_skills: 12,
  agents_scanned: 2,
  skills_by_agent: {},
};

/** 注册 hydrateShell + 后台刷新的完整命令面；bootstrap 快照首查/复查可给不同响应。 */
function mockInitializeCommands({
  firstSnapshot = mockBootstrapSnapshot,
  secondSnapshot = refreshedSnapshot,
  categoryVisibility = JSON.stringify(mockCategoryVisibility),
}: {
  firstSnapshot?: BootstrapSnapshot | Promise<BootstrapSnapshot>;
  secondSnapshot?: BootstrapSnapshot;
  categoryVisibility?: string | null;
} = {}) {
  let snapshotCall = 0;
  mockIpcCommands({
    get_bootstrap_snapshot: () =>
      snapshotCall++ === 0 ? firstSnapshot : secondSnapshot,
    get_setting: categoryVisibility,
    list_platform_paths: mockPlatformPaths,
    scan_all_skills: mockScanResult,
  });
}

describe("platformStore", () => {
  beforeEach(() => {
    usePlatformStore.setState({
      agents: [],
      platformPaths: {},
      skillsByAgent: {},
      collectionCount: 0,
      dashboardCentralSummary: undefined,
      categoryVisibility: {
        coding: true,
        lobster: false,
      },
      lastScanAt: null,
      scanState: "idle",
      isLoading: false,
      isRefreshing: false,
      scanGeneration: 0,
      error: null,
      topTags: [],
      isTopTagsLoading: false,
      topTagsError: null,
    });
  });

  it("has correct initial state", () => {
    const state = usePlatformStore.getState();
    expect(state.agents).toEqual([]);
    expect(state.skillsByAgent).toEqual({});
    expect(state.collectionCount).toBe(0);
    expect(state.categoryVisibility).toEqual({
      coding: true,
      lobster: false,
    });
    expect(state.lastScanAt).toBeNull();
    expect(state.scanState).toBe("idle");
    expect(state.isLoading).toBe(false);
    expect(state.isRefreshing).toBe(false);
    expect(state.scanGeneration).toBe(0);
    expect(state.error).toBeNull();
  });

  it("initialize hydrates the shell first and then refreshes in background", async () => {
    mockInitializeCommands();

    await usePlatformStore.getState().initialize();

    const state = usePlatformStore.getState();
    expect(ipcInvokedCommands()).toEqual([
      "get_bootstrap_snapshot",
      "get_setting",
      "list_platform_paths",
      "scan_all_skills",
      "get_bootstrap_snapshot",
      "list_platform_paths",
    ]);
    expect(ipcInvokeCalls("get_setting")[0].args).toEqual({
      key: "platform_category_visibility",
    });
    expect(state.skillsByAgent).toEqual(refreshedSnapshot.cachedSkillCounts);
    expect(state.collectionCount).toBe(2);
    expect(state.categoryVisibility).toEqual(mockCategoryVisibility);
    expect(state.scanState).toBe("idle");
    expect(state.isLoading).toBe(false);
    expect(state.isRefreshing).toBe(false);
  });

  it("initialize sets isLoading while the bootstrap snapshot is pending", async () => {
    let resolveSnapshot!: (value: BootstrapSnapshot) => void;
    mockInitializeCommands({
      firstSnapshot: new Promise<BootstrapSnapshot>((resolve) => {
        resolveSnapshot = resolve;
      }),
      categoryVisibility: null,
    });

    const initPromise = usePlatformStore.getState().initialize();
    expect(usePlatformStore.getState().isLoading).toBe(true);

    resolveSnapshot(mockBootstrapSnapshot);
    await initPromise;
  });

  it("initialize reuses the in-flight promise and does not trigger duplicate scans", async () => {
    let resolveSnapshot!: (value: BootstrapSnapshot) => void;
    mockInitializeCommands({
      firstSnapshot: new Promise<BootstrapSnapshot>((resolve) => {
        resolveSnapshot = resolve;
      }),
      categoryVisibility: null,
    });

    const firstCall = usePlatformStore.getState().initialize();
    const secondCall = usePlatformStore.getState().initialize();

    resolveSnapshot(mockBootstrapSnapshot);
    await Promise.all([firstCall, secondCall]);

    expect(ipcInvokeCalls()).toHaveLength(6);
  });

  it("sets error and clears isLoading when hydrateShell fails", async () => {
    mockIpcCommands({
      get_bootstrap_snapshot: () =>
        Promise.reject(new Error("bootstrap failed")),
      get_setting: null,
      list_platform_paths: mockPlatformPaths,
    });

    await expect(usePlatformStore.getState().hydrateShell()).rejects.toThrow(
      "bootstrap failed",
    );

    const state = usePlatformStore.getState();
    expect(state.error).toContain("bootstrap failed");
    expect(state.isLoading).toBe(false);
  });

  it("refreshCounts updates cached counts without triggering a scan", async () => {
    usePlatformStore.setState({
      agents: mockAgents,
      platformPaths: {},
      skillsByAgent: mockBootstrapSnapshot.cachedSkillCounts,
      collectionCount: 2,
      categoryVisibility: {
        coding: true,
        lobster: false,
      },
      lastScanAt: mockBootstrapSnapshot.lastScanAt,
      scanState: "idle",
      isLoading: false,
      isRefreshing: false,
      scanGeneration: 1,
      error: null,
    });

    mockIpcCommand("get_skill_counts_summary", mockCountsSummary);

    await usePlatformStore.getState().refreshCounts();

    expect(ipcInvokedCommands()).toEqual(["get_skill_counts_summary"]);
    expect(usePlatformStore.getState().skillsByAgent).toEqual(
      mockCountsSummary.cachedSkillCounts,
    );
    expect(usePlatformStore.getState().lastScanAt).toBe(
      mockCountsSummary.lastScanAt,
    );
    expect(usePlatformStore.getState().scanGeneration).toBe(2);
    expect(usePlatformStore.getState().isLoading).toBe(false);
    expect(usePlatformStore.getState().isRefreshing).toBe(false);
  });

  it("resetForTargetChange clears target-bound cached counts immediately", () => {
    usePlatformStore.setState({
      agents: mockAgents,
      platformPaths: {},
      skillsByAgent: mockBootstrapSnapshot.cachedSkillCounts,
      collectionCount: 2,
      categoryVisibility: {
        coding: true,
        lobster: false,
      },
      lastScanAt: mockBootstrapSnapshot.lastScanAt,
      scanState: "refreshing",
      isLoading: false,
      isRefreshing: true,
      scanGeneration: 1,
      error: "stale",
    });

    usePlatformStore.getState().resetForTargetChange();

    const state = usePlatformStore.getState();
    expect(state.agents).toEqual([]);
    expect(state.skillsByAgent).toEqual({});
    expect(state.collectionCount).toBe(0);
    expect(state.lastScanAt).toBeNull();
    expect(state.scanState).toBe("idle");
    expect(state.isLoading).toBe(true);
    expect(state.isRefreshing).toBe(false);
    expect(state.scanGeneration).toBe(2);
    expect(state.error).toBeNull();
  });

  it("rescan restores cached bootstrap data when the scan fails after a target reset", async () => {
    usePlatformStore.getState().resetForTargetChange();

    mockIpcCommands({
      scan_all_skills: () => Promise.reject(new Error("ssh scan failed")),
      get_bootstrap_snapshot: mockBootstrapSnapshot,
      get_setting: JSON.stringify(mockCategoryVisibility),
      list_platform_paths: mockPlatformPaths,
    });

    await usePlatformStore.getState().rescan();

    const state = usePlatformStore.getState();
    expect(ipcInvokedCommands()).toEqual([
      "scan_all_skills",
      "get_bootstrap_snapshot",
      "get_setting",
      "list_platform_paths",
    ]);
    expect(ipcInvokeCalls("get_setting")[0].args).toEqual({
      key: "platform_category_visibility",
    });
    expect(state.agents).toEqual(mockAgents);
    expect(state.skillsByAgent).toEqual(
      mockBootstrapSnapshot.cachedSkillCounts,
    );
    expect(state.collectionCount).toBe(mockBootstrapSnapshot.collectionCount);
    expect(state.categoryVisibility).toEqual(mockCategoryVisibility);
    expect(state.scanState).toBe("error");
    expect(state.error).toContain("ssh scan failed");
    expect(state.isLoading).toBe(false);
    expect(state.isRefreshing).toBe(false);
  });

  it("hydrateShell applies persisted category visibility", async () => {
    mockInitializeCommands();

    await usePlatformStore.getState().hydrateShell();

    expect(usePlatformStore.getState().categoryVisibility).toEqual(
      mockCategoryVisibility,
    );
  });

  it("setCategoryVisibility persists the setting", async () => {
    usePlatformStore.setState({
      categoryVisibility: {
        coding: true,
        lobster: false,
      },
    });
    mockIpcCommand("set_setting", undefined);

    await usePlatformStore.getState().setCategoryVisibility("lobster", true);

    expect(ipcInvokeCalls("set_setting")[0].args).toEqual({
      key: "platform_category_visibility",
      value: JSON.stringify({ coding: true, lobster: true }),
    });
    expect(usePlatformStore.getState().categoryVisibility).toEqual({
      coding: true,
      lobster: true,
    });
  });

  it("setCategoryVisibility keeps the last visible platform group enabled", async () => {
    usePlatformStore.setState({
      agents: mockAgents,
      platformPaths: {},
      skillsByAgent: {},
      collectionCount: 0,
      categoryVisibility: {
        coding: true,
        lobster: false,
      },
      lastScanAt: null,
      scanState: "idle",
      isLoading: false,
      isRefreshing: false,
      error: null,
    });
    mockIpcCommand("set_setting", undefined);

    await usePlatformStore.getState().setCategoryVisibility("coding", false);

    expect(ipcInvokeCalls("set_setting")[0].args).toEqual({
      key: "platform_category_visibility",
      value: JSON.stringify({ coding: true, lobster: false }),
    });
    expect(usePlatformStore.getState().categoryVisibility).toEqual({
      coding: true,
      lobster: false,
    });
  });

  it("hydrateShell ignores persisted category visibility that hides every platform group", async () => {
    mockInitializeCommands({
      categoryVisibility: JSON.stringify(allHiddenCategoryVisibility),
    });

    await usePlatformStore.getState().hydrateShell();

    expect(usePlatformStore.getState().categoryVisibility).toEqual({
      coding: true,
      lobster: false,
    });
  });

  it("setAgentEnabled updates the target agent", async () => {
    usePlatformStore.setState({
      agents: mockAgents,
      platformPaths: {},
      skillsByAgent: {},
      collectionCount: 0,
      categoryVisibility: {
        coding: true,
        lobster: false,
      },
      lastScanAt: null,
      scanState: "idle",
      isLoading: false,
      isRefreshing: false,
      error: null,
    });

    mockIpcCommand("set_agent_enabled", {
      ...mockAgents[0],
      is_enabled: false,
    });

    await usePlatformStore.getState().setAgentEnabled("claude-code", false);

    expect(ipcInvokeCalls("set_agent_enabled")[0].args).toEqual({
      agentId: "claude-code",
      isEnabled: false,
    });
    expect(
      usePlatformStore
        .getState()
        .agents.find((agent) => agent.id === "claude-code")?.is_enabled,
    ).toBe(false);
  });

  it("hydrateShell derives category visibility from enabled agents when nothing is saved", async () => {
    mockInitializeCommands({ categoryVisibility: null });

    await usePlatformStore.getState().hydrateShell();

    expect(usePlatformStore.getState().categoryVisibility).toEqual({
      coding: true,
      lobster: false,
    });
  });

  // ── addCustomAgent ────────────────────────────────────────────────────────

  it("addCustomAgent calls add_custom_agent and appends the agent to the list", async () => {
    const created: AgentWithStatus = {
      id: "custom-qclaw",
      display_name: "QClaw",
      category: "other",
      global_skills_dir: "~/.qclaw/skills/",
      is_detected: false,
      is_builtin: false,
      is_enabled: true,
    };
    mockIpcCommand("add_custom_agent", created);

    const config = {
      display_name: "QClaw",
      global_skills_dir: "~/.qclaw/skills/",
    };

    const result = await usePlatformStore.getState().addCustomAgent(config);

    expect(result).toEqual(created);
    expect(ipcInvokeCalls("add_custom_agent")[0].args).toEqual({ config });
    expect(usePlatformStore.getState().agents).toContainEqual(created);
    expect(usePlatformStore.getState().skillsByAgent["custom-qclaw"]).toBe(0);
  });

  it("addCustomAgent throws on failure", async () => {
    mockIpcCommand("add_custom_agent", () =>
      Promise.reject(new Error("UNIQUE constraint")),
    );

    await expect(
      usePlatformStore.getState().addCustomAgent({
        display_name: "Dup",
        global_skills_dir: "/dup",
      }),
    ).rejects.toThrow("UNIQUE constraint");
  });

  // ── updateCustomAgent ─────────────────────────────────────────────────────

  it("updateCustomAgent calls update_custom_agent and replaces the agent in the list", async () => {
    usePlatformStore.setState({ agents: [...mockAgents] });

    const updated: AgentWithStatus = {
      ...mockAgents[0],
      display_name: "Claude Code Pro",
    };
    mockIpcCommand("update_custom_agent", updated);

    const config = {
      display_name: "Claude Code Pro",
      global_skills_dir: "~/.claude/skills/",
    };

    const result = await usePlatformStore
      .getState()
      .updateCustomAgent("claude-code", config);

    expect(result).toEqual(updated);
    expect(ipcInvokeCalls("update_custom_agent")[0].args).toEqual({
      agentId: "claude-code",
      config,
    });
    expect(
      usePlatformStore.getState().agents.find((a) => a.id === "claude-code")
        ?.display_name,
    ).toBe("Claude Code Pro");
  });

  // ── removeCustomAgent ─────────────────────────────────────────────────────

  it("removeCustomAgent calls remove_custom_agent and drops the agent from the list", async () => {
    usePlatformStore.setState({
      agents: [...mockAgents],
      platformPaths: {},
      skillsByAgent: { "claude-code": 5, openclaw: 0, central: 3 },
    });
    mockIpcCommand("remove_custom_agent", undefined);

    await usePlatformStore.getState().removeCustomAgent("openclaw");

    expect(ipcInvokeCalls("remove_custom_agent")[0].args).toEqual({
      agentId: "openclaw",
    });
    expect(
      usePlatformStore.getState().agents.find((a) => a.id === "openclaw"),
    ).toBeUndefined();
    expect(usePlatformStore.getState().skillsByAgent.openclaw).toBeUndefined();
  });

  it("removeCustomAgent throws on failure", async () => {
    mockIpcCommand("remove_custom_agent", () =>
      Promise.reject(new Error("Not found")),
    );

    await expect(
      usePlatformStore.getState().removeCustomAgent("nonexistent"),
    ).rejects.toThrow("Not found");
  });

  it("refreshDashboardSummary updates only dashboardCentralSummary", async () => {
    const summary: DashboardCentralSummary = {
      centralSkillCount: 7,
      updatesAvailable: 2,
      aiReviewCount: 1,
      uncategorizedCount: 3,
      unassignedSourceCount: 0,
      readiness: {
        score: 72,
        categorizedRatio: 0.6,
        describedRatio: 0.9,
        sourcedRatio: 0.8,
        installHealthRatio: 0.7,
      },
      sourceRepositories: [],
    };
    mockIpcCommand("get_dashboard_central_summary", summary);

    await usePlatformStore.getState().refreshDashboardSummary();

    expect(ipcInvokeCalls("get_dashboard_central_summary")).toHaveLength(1);
    expect(usePlatformStore.getState().dashboardCentralSummary).toEqual(
      summary,
    );
    // 不触碰 skillsByAgent / scanState 等其它计数字段。
    expect(usePlatformStore.getState().skillsByAgent).toEqual({});
    expect(usePlatformStore.getState().scanState).toBe("idle");
  });

  it("loadTopTags stores tags and surfaces failures for panel retry", async () => {
    const tags: CentralTopTag[] = [{ id: "web", name: "Web", count: 3 }];
    mockIpcCommand("get_central_top_tags", tags);

    await usePlatformStore.getState().loadTopTags(6);

    expect(ipcInvokeCalls("get_central_top_tags")[0].args).toEqual({
      limit: 6,
    });
    expect(usePlatformStore.getState().topTags).toEqual(tags);
    expect(usePlatformStore.getState().topTagsError).toBeNull();

    mockIpcCommand("get_central_top_tags", () =>
      Promise.reject(new Error("tags backend down")),
    );
    await usePlatformStore.getState().loadTopTags(6);

    // 失败保留旧数据，错误供面板展示重试入口。
    expect(usePlatformStore.getState().topTags).toEqual(tags);
    expect(usePlatformStore.getState().topTagsError).toContain(
      "tags backend down",
    );
    expect(usePlatformStore.getState().isTopTagsLoading).toBe(false);
  });

  it("resetForTargetChange clears topTags and discards in-flight responses", async () => {
    let resolvePending!: (value: CentralTopTag[]) => void;
    mockIpcCommand(
      "get_central_top_tags",
      () =>
        new Promise<CentralTopTag[]>((resolve) => {
          resolvePending = resolve;
        }),
    );

    const pending = usePlatformStore.getState().loadTopTags(6);
    usePlatformStore.getState().resetForTargetChange();

    expect(usePlatformStore.getState().topTags).toEqual([]);
    expect(usePlatformStore.getState().topTagsError).toBeNull();

    resolvePending([{ id: "web", name: "Web", count: 3 }]);
    await pending;

    // 在途响应晚于 reset 到达，必须被 latest-wins 令牌丢弃。
    expect(usePlatformStore.getState().topTags).toEqual([]);
  });
});
