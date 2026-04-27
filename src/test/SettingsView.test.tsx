import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { SettingsView } from "../pages/SettingsView";
import { ScanDirectory, AgentWithStatus, TargetSummary } from "../types";
import { invoke } from "@tauri-apps/api/core";

// Mock stores
vi.mock("../stores/settingsStore", () => ({
  useSettingsStore: vi.fn(),
}));

vi.mock("../stores/platformStore", () => ({
  usePlatformStore: vi.fn(),
}));

vi.mock("../stores/themeStore", () => ({
  useThemeStore: vi.fn(),
  ACCENT_NAMES: [
    "rosewater", "flamingo", "pink", "mauve", "red", "maroon",
    "peach", "yellow", "green", "teal", "sky", "sapphire",
    "blue", "lavender",
  ],
}));

vi.mock("../stores/targetStore", () => ({
  useTargetStore: vi.fn(),
}));

vi.mock("../stores/centralSkillsStore", () => ({
  useCentralSkillsStore: vi.fn(),
}));

vi.mock("../stores/discoverStore", () => ({
  useDiscoverStore: vi.fn(),
}));

vi.mock("../stores/marketplaceStore", () => ({
  useMarketplaceStore: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { useSettingsStore } from "../stores/settingsStore";
import { usePlatformStore } from "../stores/platformStore";
import { useThemeStore } from "../stores/themeStore";
import { useTargetStore } from "../stores/targetStore";
import { useCentralSkillsStore } from "../stores/centralSkillsStore";
import { useDiscoverStore } from "../stores/discoverStore";
import { useMarketplaceStore } from "../stores/marketplaceStore";

// ─── Fixtures ─────────────────────────────────────────────────────────────────

const mockBuiltinDir: ScanDirectory = {
  id: 1,
  path: "/Users/test/.agents/skills/",
  label: "Central Skills",
  is_active: true,
  is_builtin: true,
  added_at: "2026-01-01T00:00:00Z",
};

const mockCustomDir: ScanDirectory = {
  id: 2,
  path: "/Users/test/projects/my-project",
  label: "My Project",
  is_active: true,
  is_builtin: false,
  added_at: "2026-01-02T00:00:00Z",
};

const mockCustomAgent: AgentWithStatus = {
  id: "custom-qclaw",
  display_name: "QClaw",
  category: "other",
  global_skills_dir: "/Users/test/.qclaw/skills/",
  is_detected: false,
  is_builtin: false,
  is_enabled: true,
};

const mockBuiltinAgent: AgentWithStatus = {
  id: "claude-code",
  display_name: "Claude Code",
  category: "coding",
  global_skills_dir: "/Users/test/.claude/skills/",
  is_detected: true,
  is_builtin: true,
  is_enabled: true,
};

const mockOpenCodeAgent: AgentWithStatus = {
  id: "opencode",
  display_name: "OpenCode",
  category: "coding",
  global_skills_dir: "~/.opencode/skills/",
  is_detected: true,
  is_builtin: true,
  is_enabled: true,
};

const mockCursorAgent: AgentWithStatus = {
  id: "cursor",
  display_name: "Cursor",
  category: "coding",
  global_skills_dir: "~/.cursor/skills/",
  is_detected: true,
  is_builtin: true,
  is_enabled: false,
};

const mockLobsterAgent: AgentWithStatus = {
  id: "openclaw",
  display_name: "OpenClaw",
  category: "lobster",
  global_skills_dir: "~/.openclaw/skills/",
  is_detected: true,
  is_builtin: true,
  is_enabled: false,
};

// ─── Helpers ──────────────────────────────────────────────────────────────────

function setupMocks({
  scanDirs = [] as ScanDirectory[],
  isLoadingScanDirs = false,
  agents = [] as AgentWithStatus[],
  loadScanDirectories = vi.fn(),
  addScanDirectory = vi.fn(),
  removeScanDirectory = vi.fn(),
  toggleScanDirectory = vi.fn(),
  addCustomAgent = vi.fn(),
  updateCustomAgent = vi.fn(),
  removeCustomAgent = vi.fn(),
  githubPat = "",
  isLoadingGitHubPat = false,
  isSavingGitHubPat = false,
  isTestingGitHubPat = false,
  loadGitHubPat = vi.fn(),
  saveGitHubPat = vi.fn(),
  clearGitHubPat = vi.fn(),
  testGitHubPat = vi.fn(),
  rescan = vi.fn(),
  refreshCounts = vi.fn(),
  categoryVisibility = { coding: true, lobster: false },
  setCategoryVisibility = vi.fn(),
  setAgentEnabled = vi.fn(),
  targets = [{ id: "local", kind: "local" as const, label: "Local", isActive: true }] as TargetSummary[],
  activeTarget = { id: "local", kind: "local" as const, label: "Local", isActive: true } as TargetSummary,
  loadTargets = vi.fn(),
  createSshTarget = vi.fn(),
  updateSshTarget = vi.fn(),
  testSshTarget = vi.fn(),
  updateSshTargetPassword = vi.fn(),
  deleteTarget = vi.fn(),
  switchTarget = vi.fn(),
  loadCentralSkills = vi.fn(),
  refreshDiscoverCounts = vi.fn(),
  loadMarketplaceRegistries = vi.fn(),
  loadMarketplaceSkills = vi.fn(),
  flavor = "mocha" as const,
  setFlavor = vi.fn(),
  accent = "lavender" as const,
  setAccent = vi.fn(),
} = {}) {
  vi.mocked(useSettingsStore).mockImplementation((selector) =>
    selector({
      scanDirectories: scanDirs,
      isLoadingScanDirs,
      error: null,
      loadScanDirectories,
      addScanDirectory,
      removeScanDirectory,
      toggleScanDirectory,
      addCustomAgent,
      updateCustomAgent,
      removeCustomAgent,
      githubPat,
      isLoadingGitHubPat,
      isSavingGitHubPat,
      isTestingGitHubPat,
      githubPatTestResult: null,
      loadGitHubPat,
      saveGitHubPat,
      clearGitHubPat,
      testGitHubPat,
      clearError: vi.fn(),
    })
  );

  vi.mocked(usePlatformStore).mockImplementation((selector) =>
    selector({
      agents,
      skillsByAgent: {},
      collectionCount: 0,
      discoveredCount: 0,
      categoryVisibility,
      lastScanAt: "2026-04-23T01:00:00Z",
      scanState: "idle",
      isLoading: false,
      isRefreshing: false,
      error: null,
      initialize: vi.fn(),
      hydrateShell: vi.fn(),
      refreshScanInBackground: vi.fn(),
      rescan,
      refreshCounts,
      resetForTargetChange: vi.fn(),
      setCategoryVisibility,
      setAgentEnabled,
      applyScanSummary: vi.fn(),
      setCollectionCount: vi.fn(),
      setDiscoveredCount: vi.fn(),
    })
  );

  vi.mocked(useTargetStore).mockImplementation((selector) =>
    selector({
      targets,
      activeTarget,
      isLoading: false,
      isCreating: false,
      updatingTargetId: null,
      testingTargetId: null,
      updatingPasswordTargetId: null,
      switchingTargetId: null,
      deletingTargetId: null,
      error: null,
      loadTargets,
      createSshTarget,
      updateSshTarget,
      testSshTarget,
      updateSshTargetPassword,
      deleteTarget,
      switchTarget,
      clearError: vi.fn(),
    })
  );

  vi.mocked(useCentralSkillsStore).mockImplementation((selector) =>
    selector({
      loadCentralSkills,
    } as never)
  );

  vi.mocked(useDiscoverStore).mockImplementation((selector) =>
    selector({
      refreshCounts: refreshDiscoverCounts,
    } as never)
  );

  vi.mocked(useMarketplaceStore).mockImplementation((selector) =>
    selector({
      selectedRegistryId: null,
      loadRegistries: loadMarketplaceRegistries,
      loadSkills: loadMarketplaceSkills,
    } as never)
  );

  vi.mocked(useThemeStore).mockImplementation((selector) =>
    selector({
      flavor,
      setFlavor,
      accent,
      setAccent,
      init: vi.fn(),
    })
  );
}

function renderSettingsView() {
  return render(
    <MemoryRouter initialEntries={["/settings"]}>
      <Routes>
        <Route path="/settings" element={<SettingsView />} />
      </Routes>
    </MemoryRouter>
  );
}

// ─── Tests ────────────────────────────────────────────────────────────────────

describe("SettingsView", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(invoke).mockResolvedValue(null);
  });

  // ── Rendering ─────────────────────────────────────────────────────────────

  it("renders the settings header", () => {
    setupMocks();
    renderSettingsView();
    expect(screen.getByRole("heading", { name: "设置" })).toBeTruthy();
  });

  it("renders the github token section", () => {
    setupMocks();
    renderSettingsView();
    expect(screen.getByText("GitHub 导入访问令牌")).toBeTruthy();
  });

  it("renders AI tag rate controls", () => {
    setupMocks();
    renderSettingsView();
    expect(screen.getByText(/AI Tag (速率限制|rate limit)/)).toBeTruthy();
    expect(screen.getByLabelText(/AI Tag .*429/)).toBeTruthy();
  });

  it("links SSH target form labels to inputs", () => {
    setupMocks();
    renderSettingsView();
    expect(screen.getByLabelText("别名")).toBeTruthy();
    expect(screen.getByLabelText("主机")).toBeTruthy();
    expect(screen.getByLabelText("用户")).toBeTruthy();
    expect(screen.getByLabelText("端口")).toBeTruthy();
    expect(screen.getByLabelText("SSH 密钥路径")).toBeTruthy();
  });

  it("marks the selected SSH auth method", () => {
    setupMocks();
    renderSettingsView();
    expect(screen.getByRole("button", { name: "SSH key" })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("button", { name: "账号密码" })).toHaveAttribute("aria-pressed", "false");
  });

  it("links AI settings labels to inputs", () => {
    setupMocks();
    renderSettingsView();
    expect(screen.getByLabelText("API Key")).toBeTruthy();
    expect(screen.getByLabelText("模型")).toBeTruthy();
    expect(screen.getByLabelText("并发数")).toBeTruthy();
    expect(screen.getByLabelText("请求间隔 ms")).toBeTruthy();
  });

  it("renders the existing settings sections", () => {
    setupMocks();
    renderSettingsView();
    expect(screen.getByText("扫描目录")).toBeTruthy();
    expect(screen.getByText("自定义平台")).toBeTruthy();
    expect(screen.getByText("平台可见性")).toBeTruthy();
    expect(screen.getByText("关于")).toBeTruthy();
  });

  it("renders the repository url in about", () => {
    setupMocks();
    renderSettingsView();
    expect(screen.getByText("仓库地址")).toBeTruthy();
    expect(screen.getByRole("link", { name: "https://github.com/bahayonghang/skills-manage-windows" })).toHaveAttribute(
      "href",
      "https://github.com/bahayonghang/skills-manage-windows"
    );
  });

  it("calls loadScanDirectories on mount", () => {
    const loadScanDirectories = vi.fn();
    setupMocks({ loadScanDirectories });
    renderSettingsView();
    expect(loadScanDirectories).toHaveBeenCalled();
  });

  it("calls loadGitHubPat on mount", () => {
    const loadGitHubPat = vi.fn();
    setupMocks({ loadGitHubPat });
    renderSettingsView();
    expect(loadGitHubPat).toHaveBeenCalled();
  });

  it("renders the saved github pat value and explanation copy", () => {
    setupMocks({ githubPat: "github_pat_saved" });
    renderSettingsView();

    expect(screen.getByLabelText("GitHub Personal Access Token")).toHaveValue("github_pat_saved");
    expect(screen.getByText(/它绝不会被发送到公共镜像或代理回退链路/)).toBeTruthy();
    expect(screen.getByText(/当 GitHub 预览\/导入遇到限流/)).toBeTruthy();
  });

  it("saves the github pat from settings", async () => {
    const saveGitHubPat = vi.fn().mockResolvedValue(undefined);
    setupMocks({ githubPat: "", saveGitHubPat });
    renderSettingsView();

    fireEvent.change(screen.getByLabelText("GitHub Personal Access Token"), {
      target: { value: "  github_pat_new  " },
    });
    fireEvent.click(screen.getByRole("button", { name: "保存" }));

    await waitFor(() => {
      expect(saveGitHubPat).toHaveBeenCalledWith("  github_pat_new  ");
    });
    expect(await screen.findByText("GitHub 令牌已保存")).toBeTruthy();
  });

  it("clears the github pat from settings", async () => {
    const clearGitHubPat = vi.fn().mockResolvedValue(undefined);
    setupMocks({ githubPat: "github_pat_saved", clearGitHubPat });
    renderSettingsView();

    fireEvent.click(screen.getByRole("button", { name: "清除令牌" }));

    await waitFor(() => {
      expect(clearGitHubPat).toHaveBeenCalled();
    });
    expect(await screen.findByText("GitHub 令牌已清除")).toBeTruthy();
  });

  // ── Scan Directories section ──────────────────────────────────────────────

  it("saves the password for an existing password ssh target", async () => {
    const updateSshTargetPassword = vi.fn().mockResolvedValue({
      ok: true,
      remoteHome: "/home/alice",
      remoteOs: "Linux",
      credentialStatus: "session",
      message: "saved",
    });
    setupMocks({
      targets: [
        { id: "local", kind: "local" as const, label: "Local", isActive: true },
        {
          id: "ssh-demo",
          kind: "ssh" as const,
          label: "Lab",
          host: "lab.local",
          username: "alice",
          port: 22,
          authMethod: "password" as const,
          remoteHome: "/home/alice",
          remoteOs: "Linux",
          hasStoredPassword: false,
          credentialStatus: "missing",
          isActive: false,
        },
      ],
      updateSshTargetPassword,
    });
    renderSettingsView();

    expect(screen.getByText(/需要密码|Password needed/i)).toBeTruthy();
    const passwordInput = screen.getByLabelText("Lab 的 SSH 密码");
    fireEvent.change(passwordInput, {
      target: { value: "secret" },
    });
    fireEvent.click(screen.getByRole("button", { name: "保存密码" }));

    await waitFor(() => {
      expect(updateSshTargetPassword).toHaveBeenCalledWith("ssh-demo", "secret");
    });
    expect(await screen.findByText(/本次会话|session/i)).toBeTruthy();
  });

  it("updates an existing ssh target without recreating it", async () => {
    const updateSshTarget = vi.fn().mockResolvedValue({
      id: "ssh-demo",
      kind: "ssh" as const,
      label: "Lab",
      host: "new.lab.local",
      username: "alice",
      port: 22,
      authMethod: "password" as const,
      remoteHome: "/home/alice",
      remoteOs: "Linux",
      credentialStatus: "stored",
      isActive: true,
    });
    setupMocks({
      targets: [
        { id: "local", kind: "local" as const, label: "Local", isActive: false },
        {
          id: "ssh-demo",
          kind: "ssh" as const,
          label: "Lab",
          host: "lab.local",
          username: "alice",
          port: 22,
          authMethod: "password" as const,
          remoteHome: "/home/alice",
          remoteOs: "Linux",
          hasStoredPassword: true,
          credentialStatus: "stored",
          isActive: true,
        },
      ],
      activeTarget: {
        id: "ssh-demo",
        kind: "ssh" as const,
        label: "Lab",
        isActive: true,
      },
      updateSshTarget,
      rescan: vi.fn().mockResolvedValue(undefined),
      loadCentralSkills: vi.fn().mockResolvedValue(undefined),
      refreshDiscoverCounts: vi.fn().mockResolvedValue(undefined),
      loadMarketplaceRegistries: vi.fn().mockResolvedValue(undefined),
    });
    renderSettingsView();

    fireEvent.click(screen.getByLabelText("编辑目标 Lab"));
    fireEvent.change(screen.getByDisplayValue("lab.local"), {
      target: { value: "new.lab.local" },
    });
    fireEvent.change(screen.getByPlaceholderText("SSH 密码（留空则沿用已保存密码）"), {
      target: { value: "new-secret" },
    });
    fireEvent.click(screen.getByRole("button", { name: "保存修改" }));

    await waitFor(() => {
      expect(updateSshTarget).toHaveBeenCalledWith({
        id: "ssh-demo",
        label: "Lab",
        host: "new.lab.local",
        username: "alice",
        port: 22,
        authMethod: "password",
        keyPath: null,
        password: "new-secret",
      });
    });
    expect(await screen.findByText("已更新目标 Lab")).toBeTruthy();
  });

  it("shows loading state for scan directories", () => {
    setupMocks({ isLoadingScanDirs: true });
    renderSettingsView();
    expect(screen.getByText("加载中...")).toBeTruthy();
  });

  it("shows empty state when no scan directories", () => {
    setupMocks({ scanDirs: [] });
    renderSettingsView();
    expect(screen.getByText("暂无扫描目录")).toBeTruthy();
  });

  it("renders builtin scan directory with 内置目录 label", () => {
    setupMocks({ scanDirs: [mockBuiltinDir] });
    renderSettingsView();
    expect(screen.getAllByText(/内置目录/).length).toBeGreaterThan(0);
  });

  it("does not show remove button for builtin directories", () => {
    setupMocks({ scanDirs: [mockBuiltinDir] });
    renderSettingsView();
    // No delete button should be present for builtin dir
    expect(
      screen.queryByRole("button", { name: /删除目录 ~\/.agents\/skills\// })
    ).toBeNull();
  });

  it("shows remove button for custom directories", () => {
    setupMocks({ scanDirs: [mockCustomDir] });
    renderSettingsView();
    expect(
      screen.getByRole("button", { name: `删除目录 ${mockCustomDir.path}` })
    ).toBeTruthy();
  });

  it("shows toggle for custom directories", () => {
    setupMocks({ scanDirs: [mockCustomDir] });
    renderSettingsView();
    expect(
      screen.getByLabelText(`启用 ${mockCustomDir.path}`)
    ).toBeTruthy();
  });

  it("does not show toggle for builtin directories", () => {
    setupMocks({ scanDirs: [mockBuiltinDir] });
    renderSettingsView();
    expect(
      screen.queryByLabelText(`启用 ${mockBuiltinDir.path}`)
    ).toBeNull();
  });

  it("shows 启用 label when directory is active", () => {
    setupMocks({ scanDirs: [{ ...mockCustomDir, is_active: true }] });
    renderSettingsView();
    expect(screen.getByText("启用")).toBeTruthy();
  });

  it("shows 禁用 label when directory is inactive", () => {
    setupMocks({ scanDirs: [{ ...mockCustomDir, is_active: false }] });
    renderSettingsView();
    expect(screen.getByText("禁用")).toBeTruthy();
  });

  it("shows add directory button", () => {
    setupMocks();
    renderSettingsView();
    expect(screen.getByRole("button", { name: "添加项目目录" })).toBeTruthy();
  });

  it("opens add directory dialog when button is clicked", async () => {
    setupMocks();
    renderSettingsView();
    fireEvent.click(screen.getByRole("button", { name: "添加项目目录" }));
    await waitFor(() => {
      expect(screen.getByText("添加项目目录")).toBeTruthy();
    });
  });

  it("removes a custom directory after inline confirmation", async () => {
    const removeScanDirectory = vi.fn().mockResolvedValue(undefined);
    const rescan = vi.fn().mockResolvedValue(undefined);
    setupMocks({
      scanDirs: [mockCustomDir],
      removeScanDirectory,
      rescan,
    });
    renderSettingsView();

    fireEvent.click(
      screen.getByRole("button", { name: `删除目录 ${mockCustomDir.path}` })
    );
    expect(removeScanDirectory).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "确认删除" }));

    await waitFor(() => {
      expect(removeScanDirectory).toHaveBeenCalledWith(mockCustomDir.path);
    });
  });

  it("refreshes counts after removing a directory", async () => {
    const removeScanDirectory = vi.fn().mockResolvedValue(undefined);
    const refreshCounts = vi.fn().mockResolvedValue(undefined);
    setupMocks({ scanDirs: [mockCustomDir], removeScanDirectory, refreshCounts });
    renderSettingsView();

    fireEvent.click(
      screen.getByRole("button", { name: `删除目录 ${mockCustomDir.path}` })
    );
    fireEvent.click(screen.getByRole("button", { name: "确认删除" }));

    await waitFor(() => {
      expect(refreshCounts).toHaveBeenCalled();
    });
  });

  // ── Custom Platforms section ──────────────────────────────────────────────

  it("shows empty state when no custom platforms", () => {
    setupMocks({ agents: [mockBuiltinAgent] }); // only builtin agents
    renderSettingsView();
    expect(screen.getByText("暂无自定义平台。点击「添加平台」注册新平台。")).toBeTruthy();
  });

  it("renders custom platform with name and path", () => {
    setupMocks({ agents: [mockBuiltinAgent, mockCustomAgent] });
    renderSettingsView();
    expect(screen.getAllByText("QClaw").length).toBeGreaterThan(0);
    expect(screen.getByText("/Users/test/.qclaw/skills/")).toBeTruthy();
  });

  it("shows edit button for custom platforms", () => {
    setupMocks({ agents: [mockBuiltinAgent, mockCustomAgent] });
    renderSettingsView();
    expect(
      screen.getByRole("button", { name: `编辑平台 ${mockCustomAgent.display_name}` })
    ).toBeTruthy();
  });

  it("shows remove button for custom platforms", () => {
    setupMocks({ agents: [mockBuiltinAgent, mockCustomAgent] });
    renderSettingsView();
    expect(
      screen.getByRole("button", { name: `删除平台 ${mockCustomAgent.display_name}` })
    ).toBeTruthy();
  });

  it("does not show builtin agents in custom platforms list", () => {
    setupMocks({ agents: [mockBuiltinAgent] });
    renderSettingsView();
    // builtin agent should not appear in custom platforms section
    expect(
      screen.queryByRole("button", { name: `编辑平台 ${mockBuiltinAgent.display_name}` })
    ).toBeNull();
  });

  it("shows add platform button", () => {
    setupMocks();
    renderSettingsView();
    expect(screen.getByRole("button", { name: "添加自定义平台" })).toBeTruthy();
  });

  it("opens add platform dialog when button is clicked", async () => {
    setupMocks();
    renderSettingsView();
    fireEvent.click(screen.getByRole("button", { name: "添加自定义平台" }));
    await waitFor(() => {
      expect(screen.getByText("添加自定义平台")).toBeTruthy();
    });
  });

  it("opens edit platform dialog when edit button is clicked", async () => {
    setupMocks({ agents: [mockBuiltinAgent, mockCustomAgent] });
    renderSettingsView();
    fireEvent.click(
      screen.getByRole("button", { name: `编辑平台 ${mockCustomAgent.display_name}` })
    );
    await waitFor(() => {
      expect(screen.getByText("编辑自定义平台")).toBeTruthy();
    });
  });

  it("removes a custom platform after inline confirmation", async () => {
    const removeCustomAgent = vi.fn().mockResolvedValue(undefined);
    const rescan = vi.fn().mockResolvedValue(undefined);
    setupMocks({
      agents: [mockBuiltinAgent, mockCustomAgent],
      removeCustomAgent,
      rescan,
    });
    renderSettingsView();

    fireEvent.click(
      screen.getByRole("button", { name: `删除平台 ${mockCustomAgent.display_name}` })
    );
    expect(removeCustomAgent).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "确认删除" }));

    await waitFor(() => {
      expect(removeCustomAgent).toHaveBeenCalledWith(mockCustomAgent.id);
    });
  });

  it("triggers rescan after removing a platform", async () => {
    const removeCustomAgent = vi.fn().mockResolvedValue(undefined);
    const rescan = vi.fn().mockResolvedValue(undefined);
    setupMocks({
      agents: [mockBuiltinAgent, mockCustomAgent],
      removeCustomAgent,
      rescan,
    });
    renderSettingsView();

    fireEvent.click(
      screen.getByRole("button", { name: `删除平台 ${mockCustomAgent.display_name}` })
    );
    fireEvent.click(screen.getByRole("button", { name: "确认删除" }));

    await waitFor(() => {
      expect(rescan).toHaveBeenCalled();
    });
  });

  // ── About section ─────────────────────────────────────────────────────────

  it("shows the app version in the about section", () => {
    setupMocks();
    renderSettingsView();
    expect(screen.getByText(/SkillPort v\d+\.\d+\.\d+/)).toBeTruthy();
  });

  it("shows the database path in the about section", () => {
    setupMocks({ scanDirs: [mockBuiltinDir], agents: [mockBuiltinAgent] });
    renderSettingsView();
    expect(screen.getByText("/Users/test/.skillsmanage/db.sqlite")).toBeTruthy();
  });

  it("shows version label", () => {
    setupMocks();
    renderSettingsView();
    expect(screen.getByText("应用版本")).toBeTruthy();
  });

  it("shows database path label", () => {
    setupMocks();
    renderSettingsView();
    expect(screen.getByText("数据库路径")).toBeTruthy();
  });

  // ── Flavor Switcher ──────────────────────────────────────────────────────

  it("shows flavor label in about section", () => {
    setupMocks();
    renderSettingsView();
    expect(screen.getByText("主题风格")).toBeTruthy();
  });

  it("renders all 4 flavor buttons", () => {
    setupMocks();
    renderSettingsView();
    expect(screen.getByRole("button", { name: /Mocha/ })).toBeTruthy();
    expect(screen.getByRole("button", { name: /Macchiato/ })).toBeTruthy();
    expect(screen.getByRole("button", { name: /Frappé/ })).toBeTruthy();
    expect(screen.getByRole("button", { name: /Latte/ })).toBeTruthy();
  });

  it("active flavor button has aria-pressed=true", () => {
    setupMocks({ flavor: "mocha" });
    renderSettingsView();
    const mochaBtn = screen.getByRole("button", { name: /Mocha/ });
    expect(mochaBtn).toHaveAttribute("aria-pressed", "true");
  });

  it("inactive flavor button has aria-pressed=false", () => {
    setupMocks({ flavor: "mocha" });
    renderSettingsView();
    const latteBtn = screen.getByRole("button", { name: /Latte/ });
    expect(latteBtn).toHaveAttribute("aria-pressed", "false");
  });

  it("clicking a flavor button calls setFlavor", () => {
    const setFlavor = vi.fn();
    setupMocks({ flavor: "mocha", setFlavor });
    renderSettingsView();
    fireEvent.click(screen.getByRole("button", { name: /Latte/ }));
    expect(setFlavor).toHaveBeenCalledWith("latte");
  });

  it("each flavor button shows a color dot", () => {
    setupMocks();
    renderSettingsView();
    // Each flavor button should have a colored dot (inline span with rounded-full)
    const buttons = [
      screen.getByRole("button", { name: /Mocha/ }),
      screen.getByRole("button", { name: /Macchiato/ }),
      screen.getByRole("button", { name: /Frappé/ }),
      screen.getByRole("button", { name: /Latte/ }),
    ];
    for (const btn of buttons) {
      const dot = btn.querySelector(".rounded-full");
      expect(dot).toBeTruthy();
    }
  });

  // ── Accent Color Picker ──────────────────────────────────────────────────

  it("shows accent color label in about section", () => {
    setupMocks();
    renderSettingsView();
    expect(screen.getByText("强调色")).toBeTruthy();
  });

  it("renders 14 accent color swatches", () => {
    setupMocks();
    renderSettingsView();
    const swatches = screen.getAllByRole("radio");
    expect(swatches).toHaveLength(14);
  });

  it("active accent swatch has aria-checked=true", () => {
    setupMocks({ accent: "lavender" });
    renderSettingsView();
    const lavenderSwatch = screen.getByRole("radio", { name: "薰衣草" });
    expect(lavenderSwatch).toHaveAttribute("aria-checked", "true");
  });

  it("inactive accent swatch has aria-checked=false", () => {
    setupMocks({ accent: "lavender" });
    renderSettingsView();
    const greenSwatch = screen.getByRole("radio", { name: "绿色" });
    expect(greenSwatch).toHaveAttribute("aria-checked", "false");
  });

  it("clicking an accent swatch calls setAccent", () => {
    const setAccent = vi.fn();
    setupMocks({ accent: "lavender", setAccent });
    renderSettingsView();
    fireEvent.click(screen.getByRole("radio", { name: "绿色" }));
    expect(setAccent).toHaveBeenCalledWith("green");
  });

  it("accent swatches use CSS custom properties for background color", () => {
    setupMocks();
    renderSettingsView();
    const rosewaterSwatch = screen.getByRole("radio", { name: "玫瑰水" });
    expect(rosewaterSwatch.style.backgroundColor).toBe("var(--ctp-rosewater)");
  });

  it("toggles coding group visibility from settings", async () => {
    const setCategoryVisibility = vi.fn().mockResolvedValue(undefined);
    setupMocks({ setCategoryVisibility });
    renderSettingsView();

    fireEvent.click(screen.getByLabelText("切换 编程类 分组显示"));

    await waitFor(() => {
      expect(setCategoryVisibility).toHaveBeenCalledWith("coding", false);
    });
  });

  it("toggles an individual platform visibility from settings", async () => {
    const setAgentEnabled = vi.fn().mockResolvedValue(undefined);
    setupMocks({ agents: [mockBuiltinAgent, mockCustomAgent], setAgentEnabled });
    renderSettingsView();

    fireEvent.click(screen.getByLabelText("切换平台 Claude Code 的显示"));

    await waitFor(() => {
      expect(setAgentEnabled).toHaveBeenCalledWith("claude-code", false);
    });
  });

  it("keeps lobster collapsed when the group is hidden", () => {
    setupMocks({
      agents: [mockBuiltinAgent, mockLobsterAgent],
      categoryVisibility: { coding: true, lobster: false },
    });
    renderSettingsView();

    expect(screen.getByText("已隐藏 · 0 / 1 已启用")).toBeTruthy();
    expect(screen.queryByText("OpenClaw")).toBeNull();
  });

  it("filters platform groups by the local search box", () => {
    setupMocks({
      agents: [mockBuiltinAgent, mockOpenCodeAgent, mockCursorAgent, mockLobsterAgent],
    });
    renderSettingsView();

    fireEvent.change(screen.getByLabelText("搜索平台"), {
      target: { value: "open" },
    });

    expect(screen.getByText("OpenCode")).toBeTruthy();
    expect(screen.getByText("OpenClaw")).toBeTruthy();
    expect(screen.queryByText("Claude Code")).toBeNull();
    expect(screen.queryByText("Cursor")).toBeNull();
  });

  it("does not repeat the 显示工具 text on every platform row", () => {
    setupMocks({
      agents: [mockBuiltinAgent, mockOpenCodeAgent, mockLobsterAgent],
    });
    renderSettingsView();

    expect(screen.queryByText("显示工具")).toBeNull();
  });
});
