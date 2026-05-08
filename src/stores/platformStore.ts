import { create } from "zustand";
import { invoke, isTauriRuntime } from "@/lib/tauri";
import {
  DEFAULT_PLATFORM_CATEGORY_VISIBILITY,
  PLATFORM_CATEGORY_VISIBILITY_SETTING_KEY,
  resolvePlatformCategoryVisibility,
  type PlatformCategoryKey,
  type PlatformCategoryVisibility,
} from "@/lib/platformVisibility";
import {
  AgentWithStatus,
  BootstrapSnapshot,
  CustomAgentConfig,
  DashboardCentralSummary,
  PlatformPathMap,
  ScanResult,
  ScanState,
  SkillCountsSummary,
  UpdateCustomAgentConfig,
} from "@/types";
import { markAppPerformance } from "@/lib/performance";
import {
  applyPlatformPathsToAgents,
  BROWSER_PLATFORM_PATHS,
  collectPlatformPathsFromAgents,
} from "@/lib/platformPathPolicy";

const BROWSER_FIXTURE_AGENTS: AgentWithStatus[] = [
  {
    id: "claude-code",
    display_name: "Claude Code",
    category: "coding",
    global_skills_dir: "",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
  {
    id: "codex",
    display_name: "Codex CLI",
    category: "coding",
    global_skills_dir: "",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
  {
    id: "gemini-cli",
    display_name: "Gemini CLI",
    category: "coding",
    global_skills_dir: "",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
  {
    id: "opencode",
    display_name: "OpenCode",
    category: "coding",
    global_skills_dir: "",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
  {
    id: "kiro",
    display_name: "Kiro",
    category: "coding",
    global_skills_dir: "",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
  {
    id: "cursor",
    display_name: "Cursor",
    category: "coding",
    global_skills_dir: "",
    is_detected: true,
    is_builtin: true,
    is_enabled: false,
  },
  {
    id: "openclaw",
    display_name: "OpenClaw",
    category: "lobster",
    global_skills_dir: "",
    is_detected: true,
    is_builtin: true,
    is_enabled: false,
  },
  {
    id: "central",
    display_name: "Central Skills",
    category: "central",
    global_skills_dir: "",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
];

const BROWSER_FIXTURE_COUNTS: ScanResult = {
  total_skills: 5,
  agents_scanned: 7,
  skills_by_agent: {
    "claude-code": 1,
    codex: 1,
    "gemini-cli": 1,
    opencode: 1,
    kiro: 1,
    cursor: 0,
    openclaw: 0,
    central: 5,
  },
};

const BROWSER_FIXTURE_DASHBOARD_CENTRAL_SUMMARY: DashboardCentralSummary = {
  centralSkillCount: BROWSER_FIXTURE_COUNTS.skills_by_agent.central,
  updatesAvailable: 0,
  aiReviewCount: 0,
  uncategorizedCount: 0,
  unassignedSourceCount: 0,
  sourceRepositories: [],
};

let initializePromise: Promise<void> | null = null;
let backgroundRefreshPromise: Promise<void> | null = null;
let refreshToken = 0;
let refreshRunId = 0;

function buildAgentCounts(
  agents: AgentWithStatus[],
  cachedCounts: Record<string, number>
): Record<string, number> {
  return agents.reduce<Record<string, number>>((acc, agent) => {
    acc[agent.id] = cachedCounts[agent.id] ?? 0;
    return acc;
  }, {});
}

function applyBootstrapSnapshot(
  snapshot: BootstrapSnapshot,
  platformPaths: PlatformPathMap = {}
): Pick<
  PlatformState,
  | "agents"
  | "platformPaths"
  | "skillsByAgent"
  | "collectionCount"
  | "discoveredCount"
  | "dashboardCentralSummary"
  | "lastScanAt"
  | "scanState"
> {
  const resolvedPlatformPaths = collectPlatformPathsFromAgents(
    snapshot.agents,
    platformPaths
  );
  const agents = applyPlatformPathsToAgents(
    snapshot.agents,
    resolvedPlatformPaths
  );

  return {
    agents,
    platformPaths: resolvedPlatformPaths,
    skillsByAgent: buildAgentCounts(agents, snapshot.cachedSkillCounts),
    collectionCount: snapshot.collectionCount,
    discoveredCount: snapshot.discoveredCount,
    dashboardCentralSummary:
      snapshot.dashboardCentralSummary ?? BROWSER_FIXTURE_DASHBOARD_CENTRAL_SUMMARY,
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
    | "discoveredCount"
    | "dashboardCentralSummary"
    | "lastScanAt"
    | "scanState"
    | "categoryVisibility"
    | "platformPaths"
  >
> {
  const [snapshot, rawCategoryVisibility, platformPaths] = await Promise.all([
    invoke<BootstrapSnapshot>("get_bootstrap_snapshot"),
    invoke<string | null>("get_setting", {
      key: PLATFORM_CATEGORY_VISIBILITY_SETTING_KEY,
    }),
    invoke<PlatformPathMap>("list_platform_paths"),
  ]);
  const bootstrapState = applyBootstrapSnapshot(snapshot, platformPaths);

  return {
    ...bootstrapState,
    categoryVisibility: resolvePlatformCategoryVisibility(
      rawCategoryVisibility,
      bootstrapState.agents
    ),
  };
}

// ─── State ────────────────────────────────────────────────────────────────────

interface PlatformState {
  agents: AgentWithStatus[];
  platformPaths: PlatformPathMap;
  skillsByAgent: Record<string, number>;
  collectionCount: number;
  discoveredCount: number;
  dashboardCentralSummary?: DashboardCentralSummary;
  categoryVisibility: PlatformCategoryVisibility;
  lastScanAt: string | null;
  scanState: ScanState;
  isLoading: boolean;
  isRefreshing: boolean;
  scanGeneration?: number;
  error: string | null;

  // Actions
  initialize: () => Promise<void>;
  hydrateShell: () => Promise<void>;
  refreshScanInBackground: () => Promise<void>;
  rescan: () => Promise<void>;
  refreshCounts: () => Promise<void>;
  resetForTargetChange: () => void;
  setCategoryVisibility: (category: PlatformCategoryKey, visible: boolean) => Promise<void>;
  setAgentEnabled: (agentId: string, enabled: boolean) => Promise<void>;
  applyScanSummary: (summary: SkillCountsSummary) => void;
  setCollectionCount: (count: number) => void;
  setDiscoveredCount: (count: number) => void;
  addCustomAgent: (config: CustomAgentConfig) => Promise<AgentWithStatus>;
  updateCustomAgent: (
    agentId: string,
    config: UpdateCustomAgentConfig
  ) => Promise<AgentWithStatus>;
  removeCustomAgent: (agentId: string) => Promise<void>;
}

// ─── Store ────────────────────────────────────────────────────────────────────

export const usePlatformStore = create<PlatformState>((set, get) => ({
  agents: [],
  platformPaths: {},
  skillsByAgent: {},
  collectionCount: 0,
  discoveredCount: 0,
  dashboardCentralSummary: BROWSER_FIXTURE_DASHBOARD_CENTRAL_SUMMARY,
  categoryVisibility: DEFAULT_PLATFORM_CATEGORY_VISIBILITY,
  lastScanAt: null,
  scanState: "idle",
  isLoading: false,
  isRefreshing: false,
  scanGeneration: 0,
  error: null,

  hydrateShell: async () => {
    const currentRefreshToken = refreshToken;
    set({ isLoading: true, error: null });

    if (!isTauriRuntime()) {
      set((state) => ({
        agents: applyPlatformPathsToAgents(BROWSER_FIXTURE_AGENTS, BROWSER_PLATFORM_PATHS),
        platformPaths: BROWSER_PLATFORM_PATHS,
        skillsByAgent: BROWSER_FIXTURE_COUNTS.skills_by_agent,
        collectionCount: 0,
        discoveredCount: 1,
        dashboardCentralSummary: BROWSER_FIXTURE_DASHBOARD_CENTRAL_SUMMARY,
        categoryVisibility: resolvePlatformCategoryVisibility(
          null,
          applyPlatformPathsToAgents(BROWSER_FIXTURE_AGENTS, BROWSER_PLATFORM_PATHS)
        ),
        lastScanAt: "2026-04-23T00:00:00.000Z",
        scanState: "idle",
        isLoading: false,
        scanGeneration: (state.scanGeneration ?? 0) + 1,
      }));
      return;
    }

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
    if (!isTauriRuntime()) {
      set((state) => ({
        isRefreshing: false,
        scanState: "idle",
        error: null,
        isLoading: state.isLoading,
        scanGeneration: (state.scanGeneration ?? 0) + 1,
      }));
      return;
    }

    if (backgroundRefreshPromise) {
      return backgroundRefreshPromise;
    }

    const currentRefreshToken = refreshToken;
    const currentRefreshRunId = ++refreshRunId;
    set({ isRefreshing: true, scanState: "refreshing", error: null });

    const refreshPromise = (async () => {
      try {
        await invoke<ScanResult>("scan_all_skills");
        const [snapshot, platformPaths] = await Promise.all([
          invoke<BootstrapSnapshot>("get_bootstrap_snapshot"),
          invoke<PlatformPathMap>("list_platform_paths"),
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
        let fallbackState:
          | Awaited<ReturnType<typeof loadBootstrapState>>
          | null = null;
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

    if (!isTauriRuntime()) {
      set((state) => ({
        skillsByAgent: BROWSER_FIXTURE_COUNTS.skills_by_agent,
        isRefreshing: false,
        scanState: "idle",
        collectionCount: state.collectionCount,
        discoveredCount: state.discoveredCount,
        isLoading: state.isLoading,
        scanGeneration: (state.scanGeneration ?? 0) + 1,
      }));
      return;
    }

    try {
      const summary = await invoke<SkillCountsSummary>("get_skill_counts_summary");
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

  resetForTargetChange: () => {
    refreshToken += 1;
    refreshRunId += 1;
    initializePromise = null;
    backgroundRefreshPromise = null;
    set((state) => ({
      agents: [],
      platformPaths: {},
      skillsByAgent: {},
      collectionCount: 0,
      discoveredCount: 0,
      dashboardCentralSummary: BROWSER_FIXTURE_DASHBOARD_CENTRAL_SUMMARY,
      lastScanAt: null,
      scanState: "idle",
      isRefreshing: false,
      isLoading: true,
      error: null,
      scanGeneration: (state.scanGeneration ?? 0) + 1,
    }));
  },

  setCategoryVisibility: async (category, visible) => {
    const previous = get().categoryVisibility;
    const next = {
      ...previous,
      [category]: visible,
    };

    set({ categoryVisibility: next, error: null });

    if (!isTauriRuntime()) {
      return;
    }

    try {
      await invoke("set_setting", {
        key: PLATFORM_CATEGORY_VISIBILITY_SETTING_KEY,
        value: JSON.stringify(next),
      });
    } catch (err) {
      set({ categoryVisibility: previous, error: String(err) });
      throw err;
    }
  },

  setAgentEnabled: async (agentId, enabled) => {
    const previousAgents = get().agents;
    const nextAgents = previousAgents.map((agent) =>
      agent.id === agentId ? { ...agent, is_enabled: enabled } : agent
    );

    set({ agents: nextAgents, error: null });

    if (!isTauriRuntime()) {
      return;
    }

    try {
      const updatedAgent = await invoke<AgentWithStatus>("set_agent_enabled", {
        agentId,
        isEnabled: enabled,
      });
      set((state) => ({
        agents: state.agents.map((agent) =>
          agent.id === updatedAgent.id ? updatedAgent : agent
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

  setDiscoveredCount: (count) => {
    set({ discoveredCount: count });
  },

  addCustomAgent: async (config) => {
    const created = await invoke<AgentWithStatus>("add_custom_agent", { config });
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
    const updated = await invoke<AgentWithStatus>("update_custom_agent", {
      agentId,
      config,
    });
    set((state) => ({
      agents: state.agents.map((agent) =>
        agent.id === updated.id ? updated : agent
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
    await invoke<void>("remove_custom_agent", { agentId });
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
