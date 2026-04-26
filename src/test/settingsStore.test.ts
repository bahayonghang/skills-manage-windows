import { describe, it, expect, vi, beforeEach } from "vitest";
import { ScanDirectory, AgentWithStatus } from "../types";

// Mock Tauri core before importing the store
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import { useSettingsStore } from "../stores/settingsStore";

// ─── Fixtures ─────────────────────────────────────────────────────────────────

const mockBuiltinDir: ScanDirectory = {
  id: 1,
  path: "~/.agents/skills/",
  label: "Central Skills",
  is_active: true,
  is_builtin: true,
  added_at: "2026-01-01T00:00:00Z",
};

const mockCustomDir: ScanDirectory = {
  id: 2,
  path: "~/projects/my-project",
  label: "My Project",
  is_active: true,
  is_builtin: false,
  added_at: "2026-01-02T00:00:00Z",
};

const mockScanDirectories: ScanDirectory[] = [mockBuiltinDir, mockCustomDir];

const mockAgent: AgentWithStatus = {
  id: "custom-qclaw",
  display_name: "QClaw",
  category: "other",
  global_skills_dir: "~/.qclaw/skills/",
  is_detected: false,
  is_builtin: false,
  is_enabled: true,
};

// ─── Tests ────────────────────────────────────────────────────────────────────

describe("settingsStore", () => {
  beforeEach(() => {
    // Reset store to initial state before each test
    useSettingsStore.setState({
      scanDirectories: [],
      isLoadingScanDirs: false,
      error: null,
      githubPat: "",
      isLoadingGitHubPat: false,
      isSavingGitHubPat: false,
      isTestingGitHubPat: false,
      githubPatTestResult: null,
    });
    vi.clearAllMocks();
  });

  // ── Initial State ─────────────────────────────────────────────────────────

  it("has correct initial state", () => {
    const state = useSettingsStore.getState();
    expect(state.scanDirectories).toEqual([]);
    expect(state.isLoadingScanDirs).toBe(false);
    expect(state.error).toBeNull();
  });

  // ── loadScanDirectories ───────────────────────────────────────────────────

  it("loadScanDirectories sets isLoadingScanDirs while loading", async () => {
    let resolve!: (value: ScanDirectory[]) => void;
    vi.mocked(invoke).mockReturnValueOnce(
      new Promise<ScanDirectory[]>((r) => (resolve = r))
    );

    const loadPromise = useSettingsStore.getState().loadScanDirectories();
    expect(useSettingsStore.getState().isLoadingScanDirs).toBe(true);

    resolve(mockScanDirectories);
    await loadPromise;
    expect(useSettingsStore.getState().isLoadingScanDirs).toBe(false);
  });

  it("loadScanDirectories populates scanDirectories on success", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(mockScanDirectories);

    await useSettingsStore.getState().loadScanDirectories();

    const state = useSettingsStore.getState();
    expect(state.scanDirectories).toEqual(mockScanDirectories);
    expect(state.isLoadingScanDirs).toBe(false);
    expect(state.error).toBeNull();
  });

  it("loadScanDirectories calls get_scan_directories command", async () => {
    vi.mocked(invoke).mockResolvedValueOnce([]);

    await useSettingsStore.getState().loadScanDirectories();

    expect(invoke).toHaveBeenCalledWith("get_scan_directories");
  });

  it("loadScanDirectories sets error on failure", async () => {
    vi.mocked(invoke).mockRejectedValueOnce(new Error("DB error"));

    await useSettingsStore.getState().loadScanDirectories();

    const state = useSettingsStore.getState();
    expect(state.error).toContain("DB error");
    expect(state.isLoadingScanDirs).toBe(false);
    expect(state.scanDirectories).toEqual([]);
  });

  // ── addScanDirectory ──────────────────────────────────────────────────────

  it("addScanDirectory appends new directory to the list", async () => {
    // Start with one builtin dir
    useSettingsStore.setState({ scanDirectories: [mockBuiltinDir] });

    vi.mocked(invoke).mockResolvedValueOnce(mockCustomDir);

    const result = await useSettingsStore.getState().addScanDirectory(
      "~/projects/my-project",
      "My Project"
    );

    expect(result).toEqual(mockCustomDir);
    const state = useSettingsStore.getState();
    expect(state.scanDirectories).toHaveLength(2);
    expect(state.scanDirectories[1]).toEqual(mockCustomDir);
  });

  it("addScanDirectory calls add_scan_directory with correct args", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(mockCustomDir);

    await useSettingsStore.getState().addScanDirectory("~/my-dir", "Label");

    expect(invoke).toHaveBeenCalledWith("add_scan_directory", {
      path: "~/my-dir",
      label: "Label",
    });
  });

  it("addScanDirectory passes null for label when not provided", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(mockCustomDir);

    await useSettingsStore.getState().addScanDirectory("~/my-dir");

    expect(invoke).toHaveBeenCalledWith("add_scan_directory", {
      path: "~/my-dir",
      label: null,
    });
  });

  it("addScanDirectory throws on failure", async () => {
    vi.mocked(invoke).mockRejectedValueOnce(new Error("UNIQUE constraint"));

    await expect(
      useSettingsStore.getState().addScanDirectory("/duplicate")
    ).rejects.toThrow("UNIQUE constraint");
  });

  // ── removeScanDirectory ───────────────────────────────────────────────────

  it("removeScanDirectory removes directory from list", async () => {
    useSettingsStore.setState({ scanDirectories: [mockBuiltinDir, mockCustomDir] });

    vi.mocked(invoke).mockResolvedValueOnce(undefined);

    await useSettingsStore.getState().removeScanDirectory("~/projects/my-project");

    const state = useSettingsStore.getState();
    expect(state.scanDirectories).toHaveLength(1);
    expect(state.scanDirectories[0].path).toBe("~/.agents/skills/");
  });

  it("removeScanDirectory calls remove_scan_directory command", async () => {
    useSettingsStore.setState({ scanDirectories: [mockCustomDir] });
    vi.mocked(invoke).mockResolvedValueOnce(undefined);

    await useSettingsStore.getState().removeScanDirectory("~/projects/my-project");

    expect(invoke).toHaveBeenCalledWith("remove_scan_directory", {
      path: "~/projects/my-project",
    });
  });

  it("removeScanDirectory throws on failure", async () => {
    vi.mocked(invoke).mockRejectedValueOnce(new Error("Cannot remove builtin"));

    await expect(
      useSettingsStore.getState().removeScanDirectory("~/.agents/skills/")
    ).rejects.toThrow("Cannot remove builtin");
  });

  // ── toggleScanDirectory ───────────────────────────────────────────────────

  it("toggleScanDirectory updates is_active in local state", async () => {
    useSettingsStore.setState({
      scanDirectories: [
        { ...mockCustomDir, is_active: true },
      ],
    });

    vi.mocked(invoke).mockResolvedValueOnce(undefined);
    await useSettingsStore.getState().toggleScanDirectory("~/projects/my-project", false);

    const state = useSettingsStore.getState();
    expect(state.scanDirectories[0].is_active).toBe(false);
  });

  it("toggleScanDirectory calls set_scan_directory_active command", async () => {
    useSettingsStore.setState({
      scanDirectories: [{ ...mockCustomDir, is_active: true }],
    });

    vi.mocked(invoke).mockResolvedValueOnce(undefined);
    await useSettingsStore.getState().toggleScanDirectory("~/projects/my-project", false);

    expect(invoke).toHaveBeenCalledWith("set_scan_directory_active", {
      path: "~/projects/my-project",
      isActive: false,
    });
  });

  it("toggleScanDirectory re-enables a disabled directory", async () => {
    useSettingsStore.setState({
      scanDirectories: [
        { ...mockCustomDir, is_active: false },
      ],
    });

    vi.mocked(invoke).mockResolvedValueOnce(undefined);
    await useSettingsStore.getState().toggleScanDirectory("~/projects/my-project", true);

    expect(useSettingsStore.getState().scanDirectories[0].is_active).toBe(true);
  });

  it("toggleScanDirectory only affects the targeted directory", async () => {
    useSettingsStore.setState({
      scanDirectories: [
        { ...mockBuiltinDir, is_active: true },
        { ...mockCustomDir, is_active: true },
      ],
    });

    vi.mocked(invoke).mockResolvedValueOnce(undefined);
    await useSettingsStore.getState().toggleScanDirectory("~/projects/my-project", false);

    const state = useSettingsStore.getState();
    // builtin dir should be unchanged
    expect(state.scanDirectories[0].is_active).toBe(true);
    // custom dir should be toggled
    expect(state.scanDirectories[1].is_active).toBe(false);
  });

  it("toggleScanDirectory throws on backend failure", async () => {
    useSettingsStore.setState({
      scanDirectories: [{ ...mockCustomDir, is_active: true }],
    });

    vi.mocked(invoke).mockRejectedValueOnce(new Error("DB error"));
    await expect(
      useSettingsStore.getState().toggleScanDirectory("~/projects/my-project", false)
    ).rejects.toThrow("DB error");
  });

  // ── addCustomAgent ────────────────────────────────────────────────────────

  it("addCustomAgent calls add_custom_agent and returns the agent", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(mockAgent);

    const config = {
      display_name: "QClaw",
      global_skills_dir: "~/.qclaw/skills/",
    };

    const result = await useSettingsStore.getState().addCustomAgent(config);

    expect(result).toEqual(mockAgent);
    expect(invoke).toHaveBeenCalledWith("add_custom_agent", { config });
  });

  it("addCustomAgent throws on failure", async () => {
    vi.mocked(invoke).mockRejectedValueOnce(new Error("UNIQUE constraint"));

    await expect(
      useSettingsStore.getState().addCustomAgent({
        display_name: "Dup",
        global_skills_dir: "/dup",
      })
    ).rejects.toThrow("UNIQUE constraint");
  });

  // ── updateCustomAgent ─────────────────────────────────────────────────────

  it("updateCustomAgent calls update_custom_agent and returns updated agent", async () => {
    const updatedAgent = { ...mockAgent, display_name: "QClaw v2" };
    vi.mocked(invoke).mockResolvedValueOnce(updatedAgent);

    const config = {
      display_name: "QClaw v2",
      global_skills_dir: "~/.qclaw/skills/",
    };

    const result = await useSettingsStore.getState().updateCustomAgent("custom-qclaw", config);

    expect(result).toEqual(updatedAgent);
    expect(invoke).toHaveBeenCalledWith("update_custom_agent", {
      agentId: "custom-qclaw",
      config,
    });
  });

  // ── removeCustomAgent ─────────────────────────────────────────────────────

  it("removeCustomAgent calls remove_custom_agent command", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(undefined);

    await useSettingsStore.getState().removeCustomAgent("custom-qclaw");

    expect(invoke).toHaveBeenCalledWith("remove_custom_agent", {
      agentId: "custom-qclaw",
    });
  });

  it("removeCustomAgent throws on failure", async () => {
    vi.mocked(invoke).mockRejectedValueOnce(new Error("Not found"));

    await expect(
      useSettingsStore.getState().removeCustomAgent("nonexistent")
    ).rejects.toThrow("Not found");
  });

  // ── clearError ────────────────────────────────────────────────────────────

  it("clearError resets error to null", () => {
    useSettingsStore.setState({ error: "Some error" });
    useSettingsStore.getState().clearError();
    expect(useSettingsStore.getState().error).toBeNull();
  });

  it("loadGitHubPat reads the app-wide github_pat setting", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(" github_pat_123 ");

    await useSettingsStore.getState().loadGitHubPat();

    expect(invoke).toHaveBeenCalledWith("get_github_pat");
    expect(useSettingsStore.getState().githubPat).toBe(" github_pat_123 ");
    expect(useSettingsStore.getState().isLoadingGitHubPat).toBe(false);
  });

  it("saveGitHubPat persists a trimmed github_pat setting", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(undefined);

    await useSettingsStore.getState().saveGitHubPat("  github_pat_abc  ");

    expect(invoke).toHaveBeenCalledWith("set_github_pat", {
      value: "  github_pat_abc  ",
    });
    expect(useSettingsStore.getState().githubPat).toBe("github_pat_abc");
    expect(useSettingsStore.getState().isSavingGitHubPat).toBe(false);
  });

  it("clearGitHubPat clears the saved github_pat setting", async () => {
    useSettingsStore.setState({ githubPat: "github_pat_abc" });
    vi.mocked(invoke).mockResolvedValueOnce(undefined);

    await useSettingsStore.getState().clearGitHubPat();

    expect(invoke).toHaveBeenCalledWith("clear_github_pat");
    expect(useSettingsStore.getState().githubPat).toBe("");
    expect(useSettingsStore.getState().isSavingGitHubPat).toBe(false);
  });

  it("testGitHubPat stores the app-wide token test result", async () => {
    vi.mocked(invoke).mockResolvedValueOnce({
      configured: true,
      ok: true,
      status: 200,
      message: "GitHub token is usable",
    });

    const result = await useSettingsStore.getState().testGitHubPat();

    expect(invoke).toHaveBeenCalledWith("test_github_pat");
    expect(result.ok).toBe(true);
    expect(useSettingsStore.getState().githubPatTestResult?.status).toBe(200);
    expect(useSettingsStore.getState().isTestingGitHubPat).toBe(false);
  });
});
