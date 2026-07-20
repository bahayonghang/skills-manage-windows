import { create } from "zustand";
import { invoke } from "@/lib/ipc";
import {
  DEFAULT_PLATFORM_CATEGORY_VISIBILITY,
  ensureAtLeastOnePlatformCategoryVisible,
  PLATFORM_CATEGORY_VISIBILITY_SETTING_KEY,
  resolvePlatformCategoryVisibility,
  type PlatformCategoryKey,
  type PlatformCategoryVisibility,
} from "@/lib/platformVisibility";
import {
  AgentWithStatus,
  BootstrapSnapshot,
  CentralTopTag,
  CustomAgentConfig,
  DashboardCentralSummary,
  PlatformPathMap,
  ScanState,
  SkillCountsSummary,
  UpdateCustomAgentConfig,
} from "@/types";
import { markAppPerformance } from "@/lib/performance";
import {
  applyPlatformPathsToAgents,
  collectPlatformPathsFromAgents,
} from "@/lib/platformPathPolicy";

/** 加载前/重置时的空 dashboard 概要（真实数据由 bootstrap 快照带回）。 */
const DEFAULT_DASHBOARD_CENTRAL_SUMMARY: DashboardCentralSummary = {
  centralSkillCount: 0,
  updatesAvailable: 0,
  aiReviewCount: 0,
  uncategorizedCount: 0,
  unassignedSourceCount: 0,
  readiness: {
    score: 0,
    categorizedRatio: 0,
    describedRatio: 0,
    sourcedRatio: 0,
    installHealthRatio: 0,
  },
  sourceRepositories: [],
};

let initializePromise: Promise<void> | null = null;
let backgroundRefreshPromise: Promise<void> | null = null;
let refreshToken = 0;
let refreshRunId = 0;
// latest-wins 令牌：summary / topTags 各自独立，旧响应不得覆盖新结果；
// resetForTargetChange 时自增使在途请求失效。
let dashboardSummaryToken = 0;
let topTagsToken = 0;

function buildAgentCounts(
  agents: AgentWithStatus[],
  cachedCounts: Record<string, number>,
): Record<string, number> {
  return agents.reduce<Record<string, number>>((acc, agent) => {
    acc[agent.id] = cachedCounts[agent.id] ?? 0;
    return acc;
  }, {});
}

function applyBootstrapSnapshot(
  snapshot: BootstrapSnapshot,
  platformPaths: PlatformPathMap = {},
): Pick<
  PlatformState,
  | "agents"
  | "platformPaths"
  | "skillsByAgent"
  | "collectionCount"
  | "dashboardCentralSummary"
  | "lastScanAt"
  | "scanState"
> {
  const resolvedPlatformPaths = collectPlatformPathsFromAgents(
    snapshot.agents,
    platformPaths,
  );
  const agents = applyPlatformPathsToAgents(
    snapshot.agents,
    resolvedPlatformPaths,
  );

  return {
    agents,
    platformPaths: resolvedPlatformPaths,
    skillsByAgent: buildAgentCounts(agents, snapshot.cachedSkillCounts),
    collectionCount: snapshot.collectionCount,
    dashboardCentralSummary:
      snapshot.dashboardCentralSummary ?? DEFAULT_DASHBOARD_CENTRAL_SUMMARY,
    lastScanAt: snapshot.lastScanAt,
    scanState: snapshot.scanState,
  };
}

async function loadBootstrapState(): Promise<
  Pick<
    PlatformState,
    | "agents"
    | "skillsByAgent"
    | "collectionCount"
    | "dashboardCentralSummary"
    | "lastScanAt"
    | "scanState"
    | "categoryVisibility"
    | "platformPaths"
  >
> {
  const [snapshot, rawCategoryVisibility, platformPaths] = await Promise.all([
    invoke("get_bootstrap_snapshot"),
    invoke("get_setting", {
      key: PLATFORM_CATEGORY_VISIBILITY_SETTING_KEY,
    }),
    invoke("list_platform_paths"),
  ]);
  const bootstrapState = applyBootstrapSnapshot(snapshot, platformPaths);

  return {
    ...bootstrapState,
    categoryVisibility: resolvePlatformCategoryVisibility(
      rawCategoryVisibility,
      bootstrapState.agents,
    ),
  };
}

// ─── State ────────────────────────────────────────────────────────────────────

interface PlatformState {
  agents: AgentWithStatus[];
  platformPaths: PlatformPathMap;
  skillsByAgent: Record<string, number>;
  collectionCount: number;
  dashboardCentralSummary?: DashboardCentralSummary;
  categoryVisibility: PlatformCategoryVisibility;
  lastScanAt: string | null;
  scanState: ScanState;
  isLoading: boolean;
  isRefreshing: boolean;
  scanGeneration?: number;
  error: string | null;
  /** Central 技能 Top tags（随 target 切换重置，重扫后需重载）。 */
  topTags: CentralTopTag[];
  isTopTagsLoading: boolean;
  topTagsError: string | null;

  // Actions
  initialize: () => Promise<void>;
  hydrateShell: () => Promise<void>;
  refreshScanInBackground: () => Promise<void>;
  rescan: () => Promise<void>;
  refreshCounts: () => Promise<void>;
  /** 轻量刷新 dashboardCentralSummary（挂载 / scanGeneration 变化 / 更新检查完成后调用）。 */
  refreshDashboardSummary: () => Promise<void>;
  /** 加载 Central Top tags（latest-wins，失败写 topTagsError 供面板重试）。 */
  loadTopTags: (limit?: number) => Promise<void>;
  resetForTargetChange: () => void;
  setCategoryVisibility: (
    category: PlatformCategoryKey,
    visible: boolean,
  ) => Promise<void>;
  setAgentEnabled: (agentId: string, enabled: boolean) => Promise<void>;
  applyScanSummary: (summary: SkillCountsSummary) => void;
  setCollectionCount: (count: number) => void;
  addCustomAgent: (config: CustomAgentConfig) => Promise<AgentWithStatus>;
  updateCustomAgent: (
    agentId: string,
    config: UpdateCustomAgentConfig,
  ) => Promise<AgentWithStatus>;
  removeCustomAgent: (agentId: string) => Promise<void>;
}

// ─── Store ────────────────────────────────────────────────────────────────────

export const usePlatformStore = create<PlatformState>((set, get) => ({
  agents: [],
  platformPaths: {},
  skillsByAgent: {},
  collectionCount: 0,
  dashboardCentralSummary: DEFAULT_DASHBOARD_CENTRAL_SUMMARY,
  categoryVisibility: DEFAULT_PLATFORM_CATEGORY_VISIBILITY,
  lastScanAt: null,
  scanState: "idle",
  isLoading: false,
  isRefreshing: false,
  scanGeneration: 0,
  error: null,
  topTags: [],
  isTopTagsLoading: false,
  topTagsError: null,

  hydrateShell: async () => {
    const currentRefreshToken = refreshToken;
    set({ isLoading: true, error: null });

    try {
      const bootstrapState = await loadBootstrapState();
      if (currentRefreshToken === refreshToken) {
        set((state) => ({
          ...bootstrapState,
          isLoading: false,
          scanGeneration: (state.scanGeneration ?? 0) + 1,
        }));
      }
      markAppPerformance("shell_ready");
    } catch (err) {
      if (currentRefreshToken === refreshToken) {
        set({ error: String(err), isLoading: false });
      }
      throw err;
    }
  },

  initialize: async () => {
    if (get().agents.length > 0) {
      return get().refreshScanInBackground();
    }

    if (initializePromise) {
      return initializePromise;
    }

    initializePromise = get()
      .hydrateShell()
      .then(() => get().refreshScanInBackground())
      .finally(() => {
        initializePromise = null;
      });

    return initializePromise;
  },

  refreshScanInBackground: async () => {
    if (backgroundRefreshPromise) {
      return backgroundRefreshPromise;
    }

    const currentRefreshToken = refreshToken;
    const currentRefreshRunId = ++refreshRunId;
    set({ isRefreshing: true, scanState: "refreshing", error: null });

    const refreshPromise = (async () => {
      try {
        await invoke("scan_all_skills");
        const [snapshot, platformPaths] = await Promise.all([
          invoke("get_bootstrap_snapshot"),
          invoke("list_platform_paths"),
        ]);
        if (currentRefreshToken === refreshToken) {
          set((state) => ({
            ...applyBootstrapSnapshot(snapshot, platformPaths),
            isRefreshing: false,
            isLoading: state.isLoading,
            error: null,
          }));
        }
        markAppPerformance("scan_finished");
      } catch (err) {
        let fallbackState: Awaited<
          ReturnType<typeof loadBootstrapState>
        > | null = null;
        try {
          fallbackState = await loadBootstrapState();
        } catch {
          fallbackState = null;
        }

        if (currentRefreshToken === refreshToken) {
          set((state) => ({
            ...(fallbackState ?? {}),
            isRefreshing: false,
            scanState: "error",
            error: String(err),
            isLoading: state.isLoading,
            scanGeneration: (state.scanGeneration ?? 0) + 1,
          }));
        }
        throw err;
      } finally {
        if (currentRefreshRunId === refreshRunId) {
          backgroundRefreshPromise = null;
        }
      }
    })();

    backgroundRefreshPromise = refreshPromise;
    return backgroundRefreshPromise;
  },

  rescan: async () => {
    const currentRefreshToken = refreshToken;
    set({ isLoading: true, error: null });
    try {
      await get().refreshScanInBackground();
      if (currentRefreshToken === refreshToken) {
        set((state) => ({
          isLoading: false,
          scanGeneration: (state.scanGeneration ?? 0) + 1,
        }));
      }
    } catch (err) {
      if (currentRefreshToken === refreshToken) {
        set({ error: String(err), isLoading: false });
      }
    }
  },

  refreshCounts: async () => {
    set({ isRefreshing: true, error: null });

    try {
      const summary = await invoke("get_skill_counts_summary");
      get().applyScanSummary(summary);
      set((state) => ({
        isRefreshing: false,
        error: null,
        isLoading: state.isLoading,
        scanGeneration: (state.scanGeneration ?? 0) + 1,
      }));
    } catch (err) {
      set({ error: String(err), isRefreshing: false, scanState: "error" });
      throw err;
    }
  },

  refreshDashboardSummary: async () => {
    const currentToken = ++dashboardSummaryToken;
    try {
      const summary = await invoke("get_dashboard_central_summary");
      if (currentToken === dashboardSummaryToken) {
        set({ dashboardCentralSummary: summary });
      }
    } catch {
      // 后台静默刷新：失败保留旧 summary，IPC failure recorder 已记录；
      // 下一次挂载 / scanGeneration 变化 / 更新完成会重试。
    }
  },

  loadTopTags: async (limit = 6) => {
    const currentToken = ++topTagsToken;
    set({ isTopTagsLoading: true, topTagsError: null });
    try {
      const topTags = await invoke("get_central_top_tags", { limit });
      if (currentToken === topTagsToken) {
        set({ topTags: topTags ?? [], isTopTagsLoading: false });
      }
    } catch (err) {
      if (currentToken === topTagsToken) {
        set({ topTagsError: String(err), isTopTagsLoading: false });
      }
    }
  },

  resetForTargetChange: () => {
    refreshToken += 1;
    refreshRunId += 1;
    dashboardSummaryToken += 1;
    topTagsToken += 1;
    initializePromise = null;
    backgroundRefreshPromise = null;
    set((state) => ({
      agents: [],
      platformPaths: {},
      skillsByAgent: {},
      collectionCount: 0,
      dashboardCentralSummary: DEFAULT_DASHBOARD_CENTRAL_SUMMARY,
      lastScanAt: null,
      scanState: "idle",
      isRefreshing: false,
      isLoading: true,
      error: null,
      topTags: [],
      isTopTagsLoading: false,
      topTagsError: null,
      scanGeneration: (state.scanGeneration ?? 0) + 1,
    }));
  },

  setCategoryVisibility: async (category, visible) => {
    const previous = get().categoryVisibility;
    const next = {
      ...previous,
      [category]: visible,
    };
    const guardedNext = ensureAtLeastOnePlatformCategoryVisible(
      next,
      previous,
      get().agents,
    );

    set({ categoryVisibility: guardedNext, error: null });

    try {
      await invoke("set_setting", {
        key: PLATFORM_CATEGORY_VISIBILITY_SETTING_KEY,
        value: JSON.stringify(guardedNext),
      });
    } catch (err) {
      set({ categoryVisibility: previous, error: String(err) });
      throw err;
    }
  },

  setAgentEnabled: async (agentId, enabled) => {
    const previousAgents = get().agents;
    const nextAgents = previousAgents.map((agent) =>
      agent.id === agentId ? { ...agent, is_enabled: enabled } : agent,
    );

    set({ agents: nextAgents, error: null });

    try {
      const updatedAgent = await invoke("set_agent_enabled", {
        agentId,
        isEnabled: enabled,
      });
      set((state) => ({
        agents: state.agents.map((agent) =>
          agent.id === updatedAgent.id ? updatedAgent : agent,
        ),
      }));
    } catch (err) {
      set({ agents: previousAgents, error: String(err) });
      throw err;
    }
  },

  applyScanSummary: (summary) => {
    set((state) => ({
      skillsByAgent: buildAgentCounts(state.agents, summary.cachedSkillCounts),
      lastScanAt: summary.lastScanAt,
      scanState: summary.scanState,
    }));
  },

  setCollectionCount: (count) => {
    set({ collectionCount: count });
  },

  addCustomAgent: async (config) => {
    const created = await invoke("add_custom_agent", { config });
    set((state) => ({
      agents: [...state.agents, created],
      platformPaths: {
        ...state.platformPaths,
        [created.id]: {
          global_skills_dir: created.global_skills_dir,
          project_skills_dir: created.project_skills_dir ?? null,
        },
      },
      skillsByAgent: { ...state.skillsByAgent, [created.id]: 0 },
    }));
    return created;
  },

  updateCustomAgent: async (agentId, config) => {
    const updated = await invoke("update_custom_agent", {
      agentId,
      config,
    });
    set((state) => ({
      agents: state.agents.map((agent) =>
        agent.id === updated.id ? updated : agent,
      ),
      platformPaths: {
        ...state.platformPaths,
        [updated.id]: {
          global_skills_dir: updated.global_skills_dir,
          project_skills_dir: updated.project_skills_dir ?? null,
        },
      },
    }));
    return updated;
  },

  removeCustomAgent: async (agentId) => {
    await invoke("remove_custom_agent", { agentId });
    set((state) => {
      const nextSkillsByAgent = { ...state.skillsByAgent };
      const nextPlatformPaths = { ...state.platformPaths };
      delete nextSkillsByAgent[agentId];
      delete nextPlatformPaths[agentId];
      return {
        agents: state.agents.filter((agent) => agent.id !== agentId),
        platformPaths: nextPlatformPaths,
        skillsByAgent: nextSkillsByAgent,
      };
    });
  },
}));
