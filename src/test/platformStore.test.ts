import { describe, it, expect, vi, beforeEach } from "vitest";
import {
  AgentWithStatus,
  BootstrapSnapshot,
  SkillCountsSummary,
} from "../types";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import { usePlatformStore } from "../stores/platformStore";

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

const mockBootstrapSnapshot: BootstrapSnapshot = {
  agents: mockAgents,
  cachedSkillCounts: {
    "claude-code": 5,
    openclaw: 0,
    central: 3,
  },
  collectionCount: 2,
  discoveredCount: 7,
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
  coding: false,
  lobster: true,
};

describe("platformStore", () => {
  beforeEach(() => {
    usePlatformStore.setState({
      agents: [],
      skillsByAgent: {},
      collectionCount: 0,
      discoveredCount: 0,
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
    });
    vi.clearAllMocks();
  });

  it("has correct initial state", () => {
    const state = usePlatformStore.getState();
    expect(state.agents).toEqual([]);
    expect(state.skillsByAgent).toEqual({});
    expect(state.collectionCount).toBe(0);
    expect(state.discoveredCount).toBe(0);
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
    vi.mocked(invoke)
      .mockResolvedValueOnce(mockBootstrapSnapshot)
      .mockResolvedValueOnce(JSON.stringify(mockCategoryVisibility))
      .mockResolvedValueOnce({ total_skills: 12, agents_scanned: 2, skills_by_agent: {} })
      .mockResolvedValueOnce(refreshedSnapshot);

    await usePlatformStore.getState().initialize();

    const state = usePlatformStore.getState();
    expect(invoke).toHaveBeenNthCalledWith(1, "get_bootstrap_snapshot");
    expect(invoke).toHaveBeenNthCalledWith(2, "get_setting", {
      key: "platform_category_visibility",
    });
    expect(invoke).toHaveBeenNthCalledWith(3, "scan_all_skills");
    expect(invoke).toHaveBeenNthCalledWith(4, "get_bootstrap_snapshot");
    expect(state.skillsByAgent).toEqual(refreshedSnapshot.cachedSkillCounts);
    expect(state.collectionCount).toBe(2);
    expect(state.discoveredCount).toBe(7);
    expect(state.categoryVisibility).toEqual(mockCategoryVisibility);
    expect(state.scanState).toBe("idle");
    expect(state.isLoading).toBe(false);
    expect(state.isRefreshing).toBe(false);
  });

  it("initialize sets isLoading while the bootstrap snapshot is pending", async () => {
    let resolveSnapshot!: (value: BootstrapSnapshot) => void;

    vi.mocked(invoke)
      .mockReturnValueOnce(new Promise<BootstrapSnapshot>((resolve) => {
        resolveSnapshot = resolve;
      }))
      .mockResolvedValueOnce(null)
      .mockResolvedValueOnce({ total_skills: 12, agents_scanned: 2, skills_by_agent: {} })
      .mockResolvedValueOnce(refreshedSnapshot);

    const initPromise = usePlatformStore.getState().initialize();
    expect(usePlatformStore.getState().isLoading).toBe(true);

    resolveSnapshot(mockBootstrapSnapshot);
    await initPromise;
  });

  it("initialize reuses the in-flight promise and does not trigger duplicate scans", async () => {
    let resolveSnapshot!: (value: BootstrapSnapshot) => void;

    vi.mocked(invoke)
      .mockReturnValueOnce(new Promise<BootstrapSnapshot>((resolve) => {
        resolveSnapshot = resolve;
      }))
      .mockResolvedValueOnce(null)
      .mockResolvedValueOnce({ total_skills: 12, agents_scanned: 2, skills_by_agent: {} })
      .mockResolvedValueOnce(refreshedSnapshot);

    const firstCall = usePlatformStore.getState().initialize();
    const secondCall = usePlatformStore.getState().initialize();

    resolveSnapshot(mockBootstrapSnapshot);
    await Promise.all([firstCall, secondCall]);

    expect(invoke).toHaveBeenCalledTimes(4);
  });

  it("sets error and clears isLoading when hydrateShell fails", async () => {
    vi.mocked(invoke).mockRejectedValueOnce(new Error("bootstrap failed"));

    await expect(usePlatformStore.getState().hydrateShell()).rejects.toThrow("bootstrap failed");

    const state = usePlatformStore.getState();
    expect(state.error).toContain("bootstrap failed");
    expect(state.isLoading).toBe(false);
  });

  it("refreshCounts updates cached counts without triggering a scan", async () => {
    usePlatformStore.setState({
      agents: mockAgents,
      skillsByAgent: mockBootstrapSnapshot.cachedSkillCounts,
      collectionCount: 2,
      discoveredCount: 7,
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

    vi.mocked(invoke).mockResolvedValueOnce(mockCountsSummary);

    await usePlatformStore.getState().refreshCounts();

    expect(invoke).toHaveBeenCalledWith("get_skill_counts_summary");
    expect(usePlatformStore.getState().skillsByAgent).toEqual(mockCountsSummary.cachedSkillCounts);
    expect(usePlatformStore.getState().lastScanAt).toBe(mockCountsSummary.lastScanAt);
    expect(usePlatformStore.getState().scanGeneration).toBe(2);
    expect(usePlatformStore.getState().isLoading).toBe(false);
    expect(usePlatformStore.getState().isRefreshing).toBe(false);
  });

  it("hydrateShell applies persisted category visibility", async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce(mockBootstrapSnapshot)
      .mockResolvedValueOnce(JSON.stringify(mockCategoryVisibility));

    await usePlatformStore.getState().hydrateShell();

    expect(usePlatformStore.getState().categoryVisibility).toEqual(
      mockCategoryVisibility
    );
  });

  it("setCategoryVisibility persists the setting", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(undefined);

    await usePlatformStore.getState().setCategoryVisibility("coding", false);

    expect(invoke).toHaveBeenCalledWith("set_setting", {
      key: "platform_category_visibility",
      value: JSON.stringify({ coding: false, lobster: false }),
    });
    expect(usePlatformStore.getState().categoryVisibility).toEqual({
      coding: false,
      lobster: false,
    });
  });

  it("setAgentEnabled updates the target agent", async () => {
    usePlatformStore.setState({
      agents: mockAgents,
      skillsByAgent: {},
      collectionCount: 0,
      discoveredCount: 0,
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

    vi.mocked(invoke).mockResolvedValueOnce({
      ...mockAgents[0],
      is_enabled: false,
    });

    await usePlatformStore.getState().setAgentEnabled("claude-code", false);

    expect(invoke).toHaveBeenCalledWith("set_agent_enabled", {
      agentId: "claude-code",
      isEnabled: false,
    });
    expect(
      usePlatformStore.getState().agents.find((agent) => agent.id === "claude-code")
        ?.is_enabled
    ).toBe(false);
  });

  it("hydrateShell derives category visibility from enabled agents when nothing is saved", async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce(mockBootstrapSnapshot)
      .mockResolvedValueOnce(null);

    await usePlatformStore.getState().hydrateShell();

    expect(usePlatformStore.getState().categoryVisibility).toEqual({
      coding: true,
      lobster: false,
    });
  });
});
