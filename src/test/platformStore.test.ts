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
    id: "central",
    display_name: "Central Skills",
    category: "central",
    global_skills_dir: "~/.agents/skills/",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
];

const mockBootstrapSnapshot: BootstrapSnapshot = {
  agents: mockAgents,
  cachedSkillCounts: {
    "claude-code": 5,
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
    central: 4,
  },
  lastScanAt: "2026-04-23T01:05:00Z",
};

const mockCountsSummary: SkillCountsSummary = {
  cachedSkillCounts: {
    "claude-code": 9,
    central: 4,
  },
  lastScanAt: "2026-04-23T01:06:00Z",
  scanState: "idle",
};

describe("platformStore", () => {
  beforeEach(() => {
    usePlatformStore.setState({
      agents: [],
      skillsByAgent: {},
      collectionCount: 0,
      discoveredCount: 0,
      lastScanAt: null,
      scanState: "idle",
      isLoading: false,
      isRefreshing: false,
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
    expect(state.lastScanAt).toBeNull();
    expect(state.scanState).toBe("idle");
    expect(state.isLoading).toBe(false);
    expect(state.isRefreshing).toBe(false);
    expect(state.error).toBeNull();
  });

  it("initialize hydrates the shell first and then refreshes in background", async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce(mockBootstrapSnapshot)
      .mockResolvedValueOnce({ total_skills: 12, agents_scanned: 2, skills_by_agent: {} })
      .mockResolvedValueOnce(refreshedSnapshot);

    await usePlatformStore.getState().initialize();

    const state = usePlatformStore.getState();
    expect(invoke).toHaveBeenNthCalledWith(1, "get_bootstrap_snapshot");
    expect(invoke).toHaveBeenNthCalledWith(2, "scan_all_skills");
    expect(invoke).toHaveBeenNthCalledWith(3, "get_bootstrap_snapshot");
    expect(state.skillsByAgent).toEqual(refreshedSnapshot.cachedSkillCounts);
    expect(state.collectionCount).toBe(2);
    expect(state.discoveredCount).toBe(7);
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
      .mockResolvedValueOnce({ total_skills: 12, agents_scanned: 2, skills_by_agent: {} })
      .mockResolvedValueOnce(refreshedSnapshot);

    const firstCall = usePlatformStore.getState().initialize();
    const secondCall = usePlatformStore.getState().initialize();

    resolveSnapshot(mockBootstrapSnapshot);
    await Promise.all([firstCall, secondCall]);

    expect(invoke).toHaveBeenCalledTimes(3);
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
      lastScanAt: mockBootstrapSnapshot.lastScanAt,
      scanState: "idle",
      isLoading: false,
      isRefreshing: false,
      error: null,
    });

    vi.mocked(invoke).mockResolvedValueOnce(mockCountsSummary);

    await usePlatformStore.getState().refreshCounts();

    expect(invoke).toHaveBeenCalledWith("get_skill_counts_summary");
    expect(usePlatformStore.getState().skillsByAgent).toEqual(mockCountsSummary.cachedSkillCounts);
    expect(usePlatformStore.getState().lastScanAt).toBe(mockCountsSummary.lastScanAt);
    expect(usePlatformStore.getState().isLoading).toBe(false);
    expect(usePlatformStore.getState().isRefreshing).toBe(false);
  });
});
