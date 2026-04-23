import { create } from "zustand";
import { invoke, isTauriRuntime } from "@/lib/tauri";
import {
  AgentWithStatus,
  BootstrapSnapshot,
  ScanResult,
  ScanState,
  SkillCountsSummary,
} from "@/types";
import { markAppPerformance } from "@/lib/performance";

const BROWSER_FIXTURE_AGENTS: AgentWithStatus[] = [
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
    id: "cursor",
    display_name: "Cursor",
    category: "coding",
    global_skills_dir: "~/.cursor/skills/",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
  {
    id: "central",
    display_name: "Central Skills",
    category: "central",
    global_skills_dir: "~/.agents/skills/",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
];

const BROWSER_FIXTURE_COUNTS: ScanResult = {
  total_skills: 1,
  agents_scanned: 3,
  skills_by_agent: {
    "claude-code": 1,
    cursor: 1,
    central: 1,
  },
};

let initializePromise: Promise<void> | null = null;
let backgroundRefreshPromise: Promise<void> | null = null;

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
  snapshot: BootstrapSnapshot
): Pick<
  PlatformState,
  | "agents"
  | "skillsByAgent"
  | "collectionCount"
  | "discoveredCount"
  | "lastScanAt"
  | "scanState"
> {
  return {
    agents: snapshot.agents,
    skillsByAgent: buildAgentCounts(snapshot.agents, snapshot.cachedSkillCounts),
    collectionCount: snapshot.collectionCount,
    discoveredCount: snapshot.discoveredCount,
    lastScanAt: snapshot.lastScanAt,
    scanState: snapshot.scanState,
  };
}

// ─── State ────────────────────────────────────────────────────────────────────

interface PlatformState {
  agents: AgentWithStatus[];
  skillsByAgent: Record<string, number>;
  collectionCount: number;
  discoveredCount: number;
  lastScanAt: string | null;
  scanState: ScanState;
  isLoading: boolean;
  isRefreshing: boolean;
  error: string | null;

  // Actions
  initialize: () => Promise<void>;
  hydrateShell: () => Promise<void>;
  refreshScanInBackground: () => Promise<void>;
  rescan: () => Promise<void>;
  refreshCounts: () => Promise<void>;
  applyScanSummary: (summary: SkillCountsSummary) => void;
  setCollectionCount: (count: number) => void;
  setDiscoveredCount: (count: number) => void;
}

// ─── Store ────────────────────────────────────────────────────────────────────

export const usePlatformStore = create<PlatformState>((set, get) => ({
  agents: [],
  skillsByAgent: {},
  collectionCount: 0,
  discoveredCount: 0,
  lastScanAt: null,
  scanState: "idle",
  isLoading: false,
  isRefreshing: false,
  error: null,

  hydrateShell: async () => {
    set({ isLoading: true, error: null });

    if (!isTauriRuntime()) {
      set({
        agents: BROWSER_FIXTURE_AGENTS,
        skillsByAgent: BROWSER_FIXTURE_COUNTS.skills_by_agent,
        collectionCount: 0,
        discoveredCount: 1,
        lastScanAt: "2026-04-23T00:00:00.000Z",
        scanState: "idle",
        isLoading: false,
      });
      return;
    }

    try {
      const snapshot = await invoke<BootstrapSnapshot>("get_bootstrap_snapshot");
      set({
        ...applyBootstrapSnapshot(snapshot),
        isLoading: false,
      });
      markAppPerformance("shell_ready");
    } catch (err) {
      set({ error: String(err), isLoading: false });
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
      }));
      return;
    }

    if (backgroundRefreshPromise) {
      return backgroundRefreshPromise;
    }

    set({ isRefreshing: true, scanState: "refreshing", error: null });

    backgroundRefreshPromise = (async () => {
      try {
        await invoke<ScanResult>("scan_all_skills");
        const snapshot = await invoke<BootstrapSnapshot>("get_bootstrap_snapshot");
        set((state) => ({
          ...applyBootstrapSnapshot(snapshot),
          isRefreshing: false,
          isLoading: state.isLoading,
          error: null,
        }));
        markAppPerformance("scan_finished");
      } catch (err) {
        set({
          isRefreshing: false,
          scanState: "error",
          error: String(err),
        });
        throw err;
      } finally {
        backgroundRefreshPromise = null;
      }
    })();

    return backgroundRefreshPromise;
  },

  rescan: async () => {
    set({ isLoading: true, error: null });
    try {
      await get().refreshScanInBackground();
      set({ isLoading: false });
    } catch (err) {
      set({ error: String(err), isLoading: false });
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
      }));
    } catch (err) {
      set({ error: String(err), isRefreshing: false, scanState: "error" });
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
}));
