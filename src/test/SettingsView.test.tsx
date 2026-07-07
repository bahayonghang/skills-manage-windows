import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor, within } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { SettingsView } from "../pages/SettingsView";
import type { AiSettings } from "../stores/settingsStore";
import {
  AiApiKeyState,
  ScanDirectory,
  AgentWithStatus,
  TargetSummary,
  GitHubPatState,
  WslDistributionSummary,
} from "../types";
import { invoke } from "@/lib/ipc";

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
  THEME_FLAVORS: ["mocha", "macchiato", "frappe", "latte", "claude-light", "claude-dark"],
}));

vi.mock("../stores/targetStore", () => ({
  useTargetStore: vi.fn(),
}));

vi.mock("../stores/centralSkillsStore", () => ({
  useCentralSkillsStore: vi.fn(),
}));

vi.mock("../stores/marketplaceStore", () => ({
  useMarketplaceStore: vi.fn(),
}));

vi.mock("../stores/appUpdateStore", async () => {
  const actual = await vi.importActual<typeof import("../stores/appUpdateStore")>(
    "../stores/appUpdateStore"
  );
  return {
    ...actual,
    useAppUpdateStore: vi.fn(),
  };
});

vi.mock("@/lib/ipc", () => ({
  invoke: vi.fn(),
  isTauriRuntime: vi.fn(() => true),
}));

import { useSettingsStore } from "../stores/settingsStore";
import { usePlatformStore } from "../stores/platformStore";
import { useThemeStore } from "../stores/themeStore";
import { useTargetStore } from "../stores/targetStore";
import { useCentralSkillsStore } from "../stores/centralSkillsStore";
import { useMarketplaceStore } from "../stores/marketplaceStore";
import { useAppUpdateStore } from "../stores/appUpdateStore";

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

const defaultAiSettings: AiSettings = {
  provider: "claude",
  region: "intl",
  apiKey: "",
  model: "",
  customUrl: "",
  protocol: "",
  tagConcurrency: "1",
  tagIntervalMs: "4000",
  tagStopOnRateLimit: true,
};

function setupMocks({
  scanDirs = [] as ScanDirectory[],
  centralUpdateCheckMode = "regular" as const,
  centralUpdateCheckModeLoaded = true,
  isLoadingCentralUpdateCheckMode = false,
  isLoadingScanDirs = false,
  agents = [] as AgentWithStatus[],
  loadScanDirectories = vi.fn(),
  loadCentralUpdateCheckMode = vi.fn(),
  setCentralUpdateCheckMode = vi.fn(),
  addScanDirectory = vi.fn(),
  removeScanDirectory = vi.fn(),
  toggleScanDirectory = vi.fn(),
  addCustomAgent = vi.fn(),
  updateCustomAgent = vi.fn(),
  removeCustomAgent = vi.fn(),
  githubPatState: providedGitHubPatState = undefined as GitHubPatState | undefined,
  isLoadingGitHubPat = false,
  isSavingGitHubPat = false,
  isTestingGitHubPat = false,
  loadGitHubPat = vi.fn(),
  revealGitHubPat = vi.fn(),
  saveGitHubPat = vi.fn(),
  clearGitHubPat = vi.fn(),
  testGitHubPat = vi.fn(),
  aiSettings = defaultAiSettings,
  aiRawSettings = {} as Record<string, string | null | undefined>,
  aiApiKeyState: providedAiApiKeyState = undefined as AiApiKeyState | undefined,
  aiSettingsLoaded = true,
  isLoadingAiSettings = false,
  aiSaveStatus = "saved" as const,
  aiSaveError = null as string | null,
  aiTesting = false,
  aiTestResult = null as { ok: boolean; msg: string; code?: string; details?: string } | null,
  loadAiSettings = vi.fn(),
  updateAiSettings = vi.fn(),
  switchAiProvider = vi.fn(),
  revealAiApiKey = vi.fn(),
  clearAiApiKey = vi.fn(),
  flushAiSettings = vi.fn(),
  testAiConnection = vi.fn(),
  rescan = vi.fn(),
  refreshCounts = vi.fn(),
  categoryVisibility = { coding: true, lobster: false },
  setCategoryVisibility = vi.fn(),
  setAgentEnabled = vi.fn(),
  targets = [{ id: "local", kind: "local" as const, label: "Local", isActive: true }] as TargetSummary[],
  activeTarget = { id: "local", kind: "local" as const, label: "Local", isActive: true } as TargetSummary,
  wslDistributions = [] as WslDistributionSummary[],
  loadTargets = vi.fn(),
  loadWslDistributions = vi.fn().mockResolvedValue(undefined),
  isLoadingWslDistributions = false,
  wslDistributionError = null as string | null,
  createSshTarget = vi.fn(),
  updateSshTarget = vi.fn(),
  testSshTarget = vi.fn(),
  createWslTarget = vi.fn(),
  updateWslTarget = vi.fn(),
  testWslTarget = vi.fn(),
  updateSshTargetPassword = vi.fn(),
  deleteTarget = vi.fn(),
  switchTarget = vi.fn(),
  loadCentralSkills = vi.fn(),
  loadMarketplaceRegistries = vi.fn(),
  loadMarketplaceSkills = vi.fn(),
  flavor = "mocha" as const,
  setFlavor = vi.fn(),
  accent = "lavender" as const,
  setAccent = vi.fn(),
  appUpdateState = {} as Partial<ReturnType<typeof useAppUpdateStore.getState>>,
} = {}) {
  const githubPatState =
    providedGitHubPatState ??
    ({
      configured: false,
      storageState: "missing",
      error: null,
    } satisfies GitHubPatState);
  const aiApiKeyState =
    providedAiApiKeyState ??
    ({
      configured: false,
      storageState: "missing",
      fingerprint: null,
      error: null,
    } satisfies AiApiKeyState);

  vi.mocked(useSettingsStore).mockImplementation((selector) =>
    selector({
      scanDirectories: scanDirs,
      centralUpdateCheckMode,
      centralUpdateCheckModeLoaded,
      isLoadingCentralUpdateCheckMode,
      isLoadingScanDirs,
      error: null,
      loadScanDirectories,
      loadCentralUpdateCheckMode,
      setCentralUpdateCheckMode,
      addScanDirectory,
      removeScanDirectory,
      toggleScanDirectory,
      githubPatState,
      isLoadingGitHubPat,
      isSavingGitHubPat,
      isTestingGitHubPat,
      githubPatTestResult: null,
      aiSettings,
      aiRawSettings,
      aiApiKeyState,
      aiSettingsLoaded,
      isLoadingAiSettings,
      aiSaveStatus,
      aiSaveError,
      aiTesting,
      aiTestResult,
      loadGitHubPat,
      revealGitHubPat,
      saveGitHubPat,
      clearGitHubPat,
      testGitHubPat,
      loadAiSettings,
      updateAiSettings,
      switchAiProvider,
      revealAiApiKey,
      clearAiApiKey,
      flushAiSettings,
      testAiConnection,
      clearError: vi.fn(),
    })
  );

  vi.mocked(usePlatformStore).mockImplementation((selector) =>
    selector({
      agents,
      platformPaths: {},
    skillsByAgent: {},
      collectionCount: 0,
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
      addCustomAgent,
      updateCustomAgent,
      removeCustomAgent,
    })
  );

  vi.mocked(useTargetStore).mockImplementation((selector) =>
    selector({
      targets,
      activeTarget,
      wslDistributions,
      isLoading: false,
      isLoadingWslDistributions,
      isCreating: false,
      updatingTargetId: null,
      testingTargetId: null,
      updatingPasswordTargetId: null,
      switchingTargetId: null,
      deletingTargetId: null,
      error: null,
      wslDistributionError,
      loadTargets,
      loadWslDistributions,
      createSshTarget,
      updateSshTarget,
      testSshTarget,
      createWslTarget,
      updateWslTarget,
      testWslTarget,
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

  const appUpdateDefaults = {
    status: "idle" as const,
    currentVersion: "0.10.7",
    latestVersion: null,
    releaseNotes: null,
    progress: { downloaded: 0, total: null },
    error: null,
    hasChecked: false,
    checkForUpdate: vi.fn(),
    installUpdate: vi.fn(),
    reset: vi.fn(),
    ...appUpdateState,
  };

  vi.mocked(useAppUpdateStore).mockImplementation((selector) =>
    selector(appUpdateDefaults)
  );
}

function renderSettingsView(initialEntry = "/settings") {
  return render(
    <MemoryRouter initialEntries={[initialEntry]}>
      <Routes>
        <Route path="/settings/*" element={<SettingsView />} />
      </Routes>
    </MemoryRouter>
  );
}

// ─── Tests ────────────────────────────────────────────────────────────────────

describe("SettingsView", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    window.localStorage.removeItem("settings.sectionCollapsed.v1");
    vi.mocked(invoke).mockResolvedValue(null);
  });

  // ── Rendering ─────────────────────────────────────────────────────────────

  it("renders the settings header", () => {
    setupMocks();
    renderSettingsView();
    const heading = screen.getByRole("heading", { name: "设置" });
    expect(heading).toBeTruthy();
    expect(heading).toHaveClass("font-heading");
  });

  it("renders the github token section", () => {
    setupMocks();
    renderSettingsView("/settings/integrations");
    expect(screen.getByText("GitHub 导入访问令牌")).toBeTruthy();
  });

  it("renders AI tag rate controls", () => {
    setupMocks();
    renderSettingsView("/settings/integrations");
    expect(screen.getByText(/AI Tag (速率限制|rate limit)/)).toBeTruthy();
    expect(screen.getByLabelText(/AI Tag .*429/)).toBeTruthy();
  });

  it("links SSH target form labels to inputs", () => {
    setupMocks();
    renderSettingsView("/settings/connections");
    expect(screen.getByLabelText("别名")).toBeTruthy();
    expect(screen.getByLabelText("主机")).toBeTruthy();
    expect(screen.getByLabelText("用户")).toBeTruthy();
    expect(screen.getByLabelText("端口")).toBeTruthy();
    expect(screen.getByLabelText("SSH 密钥路径")).toBeTruthy();
  });

  it("renders SSH and WSL add flows as separate cards", () => {
    setupMocks({
      wslDistributions: [
        {
          name: "Ubuntu-24.04",
          isDefault: true,
          state: "Stopped",
          version: "2",
        },
      ],
    });
    renderSettingsView("/settings/connections");

    expect(screen.getByText("新增 SSH 目标")).toBeTruthy();
    expect(screen.getByText("新增 WSL 发行版")).toBeTruthy();
    expect(
      screen
        .getAllByText(/新增 (SSH 目标|WSL 发行版)/)
        .map((node) => node.textContent)
    ).toEqual(["新增 SSH 目标", "新增 WSL 发行版"]);
    expect(screen.queryByRole("button", { name: "WSL" })).toBeNull();
    expect(screen.getByLabelText("主机")).toBeTruthy();
    expect(screen.getByLabelText("WSL 别名")).toBeTruthy();
    expect(screen.getByLabelText("WSL 发行版")).toBeTruthy();
    expect(screen.getByText(/不需要 SSH 主机或凭据/)).toBeTruthy();
  });

  it("opens the local remote sync dialog from the settings URL action", async () => {
    setupMocks({
      targets: [
        { id: "local", kind: "local" as const, label: "Local", isActive: false },
        {
          id: "wsl-ubuntu",
          kind: "wsl" as const,
          label: "Ubuntu",
          distribution: "Ubuntu-24.04",
          remoteHome: "/home/lyh",
          isActive: true,
        },
      ],
      activeTarget: {
        id: "wsl-ubuntu",
        kind: "wsl" as const,
        label: "Ubuntu",
        distribution: "Ubuntu-24.04",
        isActive: true,
      },
    });

    renderSettingsView("/settings?section=remote-targets&action=local-remote-sync");

    expect(await screen.findByRole("dialog")).toHaveTextContent(
      "同步本机 repo 与 skills"
    );
    expect(screen.getByRole("dialog")).toHaveTextContent("Ubuntu");
  });

  it("keeps the remote target section visible when URL action has no remote target", async () => {
    setupMocks();

    renderSettingsView("/settings?section=remote-targets&action=local-remote-sync");

    expect(await screen.findByText("开始远程同步前，请先添加 SSH 或 WSL 目标。")).toBeTruthy();
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(screen.getByText("新增 SSH 目标")).toBeTruthy();
  });

  it("marks the selected SSH auth method", () => {
    setupMocks();
    renderSettingsView("/settings/connections");
    expect(screen.getByRole("button", { name: "SSH key" })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("button", { name: "账号密码" })).toHaveAttribute("aria-pressed", "false");
  });

  it("links AI settings labels to inputs", () => {
    setupMocks();
    renderSettingsView("/settings/integrations");
    expect(screen.getByLabelText("API Key")).toBeTruthy();
    expect(screen.getByLabelText("模型")).toBeTruthy();
    expect(screen.getByLabelText("并发数")).toBeTruthy();
    expect(screen.getByLabelText("请求间隔 ms")).toBeTruthy();
  });

  it("shows the selected provider API key acquisition link", () => {
    setupMocks({
      aiSettings: {
        ...defaultAiSettings,
        provider: "openrouter",
        model: "anthropic/claude-sonnet-4.6",
      },
    });
    renderSettingsView("/settings/integrations");

    const link = screen.getByRole("link", {
      name: "打开 OpenRouter API Key 获取页面",
    });
    expect(link).toHaveAttribute("href", "https://openrouter.ai/keys");
    expect(link).toHaveAttribute("target", "_blank");
  });

  it("uses the selected region for provider API key acquisition links", () => {
    setupMocks({
      aiSettings: {
        ...defaultAiSettings,
        provider: "glm",
        region: "cn",
        model: "glm-5",
      },
    });
    const view = renderSettingsView("/settings/integrations");

    expect(
      screen.getByRole("link", {
        name: "打开 智谱 GLM API Key 获取页面",
      })
    ).toHaveAttribute("href", "https://bigmodel.cn/usercenter/proj-mgmt/apikeys");

    setupMocks({
      aiSettings: {
        ...defaultAiSettings,
        provider: "glm",
        region: "intl",
        model: "glm-5",
      },
    });
    view.rerender(
      <MemoryRouter initialEntries={["/settings/integrations"]}>
        <Routes>
          <Route path="/settings/*" element={<SettingsView />} />
        </Routes>
      </MemoryRouter>
    );

    expect(
      screen.getByRole("link", {
        name: "打开 智谱 GLM API Key 获取页面",
      })
    ).toHaveAttribute("href", "https://z.ai/manage-apikey/apikey-list");
  });

  it("hides the API key acquisition link for custom providers", () => {
    setupMocks({
      aiSettings: {
        provider: "custom",
        region: "intl",
        apiKey: "",
        model: "custom-model",
        customUrl: "https://proxy.example.com/v1",
        protocol: "openai",
        tagConcurrency: "1",
        tagIntervalMs: "4000",
        tagStopOnRateLimit: true,
      },
    });
    renderSettingsView("/settings/integrations");

    expect(screen.queryByRole("link", { name: /API Key 获取页面/ })).toBeNull();
  });

  it("hides a saved AI API key until the reveal eye is clicked", async () => {
    const revealAiApiKey = vi.fn().mockResolvedValue("sk-openrouter-secret");
    setupMocks({
      revealAiApiKey,
      aiApiKeyState: {
        provider: "openrouter",
        configured: true,
        storageState: "stored",
        fingerprint: "sha256:abcd1234",
        error: null,
      },
      aiSettings: {
        ...defaultAiSettings,
        provider: "openrouter",
        model: "openrouter/auto",
      },
    });
    renderSettingsView("/settings/integrations");
    expect(screen.getByText("当前 OpenRouter Key：已配置")).toBeTruthy();
    expect(screen.getByText("Key 指纹：sha256:abcd1234")).toBeTruthy();
    expect(screen.getByText("已保存到安全存储")).toBeTruthy();
    expect(screen.getByLabelText("API Key")).toHaveValue("");
    expect(screen.getByPlaceholderText("••••••••••••")).toBeTruthy();
    const revealButton = screen.getByRole("button", { name: "显示已保存 API Key" });
    fireEvent.click(revealButton);

    await waitFor(() => {
      expect(revealAiApiKey).toHaveBeenCalledWith("openrouter");
    });
    expect(screen.getByLabelText("API Key")).toHaveValue("sk-openrouter-secret");
    expect(screen.getByRole("button", { name: "隐藏已保存 API Key" })).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "隐藏已保存 API Key" }));

    expect(screen.getByLabelText("API Key")).toHaveValue("");
    expect(screen.getByRole("button", { name: "测试当前 Key" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "清除 Key" })).toBeEnabled();
  });

  it("enables the AI API key eye button only for the current input", () => {
    const updateAiSettings = vi.fn();
    setupMocks({
      aiSettings: {
        ...defaultAiSettings,
        apiKey: "sk-new-input",
        provider: "openrouter",
      },
      aiApiKeyState: {
        provider: "openrouter",
        configured: true,
        storageState: "stored",
        fingerprint: "sha256:abcd1234",
        error: null,
      },
      updateAiSettings,
    });
    renderSettingsView("/settings/integrations");

    expect(screen.getByText("保存后将替换当前 OpenRouter Key。")).toBeTruthy();
    const input = screen.getByLabelText("API Key");
    expect(input).toHaveAttribute("type", "password");
    const revealButton = screen.getByRole("button", { name: "显示本次输入的 Key" });
    expect(revealButton).toBeEnabled();

    fireEvent.click(revealButton);

    expect(input).toHaveAttribute("type", "text");
    expect(screen.getByRole("button", { name: "隐藏本次输入的 Key" })).toBeTruthy();
  });

  it("clears revealed saved AI API key text when switching provider", async () => {
    const revealAiApiKey = vi.fn().mockResolvedValue("sk-openrouter-secret");
    const switchAiProvider = vi.fn().mockResolvedValue(undefined);
    setupMocks({
      revealAiApiKey,
      switchAiProvider,
      aiApiKeyState: {
        provider: "openrouter",
        configured: true,
        storageState: "stored",
        fingerprint: "sha256:abcd1234",
        error: null,
      },
      aiSettings: {
        ...defaultAiSettings,
        provider: "openrouter",
      },
    });
    const view = renderSettingsView("/settings/integrations");

    fireEvent.click(screen.getByRole("button", { name: "显示已保存 API Key" }));

    await waitFor(() => {
      expect(screen.getByLabelText("API Key")).toHaveValue("sk-openrouter-secret");
    });

    fireEvent.click(screen.getByRole("button", { name: "DeepSeek" }));

    await waitFor(() => {
      expect(switchAiProvider).toHaveBeenCalledWith("deepseek");
    });
    setupMocks({
      revealAiApiKey,
      switchAiProvider,
      aiApiKeyState: {
        provider: "deepseek",
        configured: false,
        storageState: "missing",
        fingerprint: null,
        error: null,
      },
      aiSettings: {
        ...defaultAiSettings,
        provider: "deepseek",
      },
    });
    view.rerender(
      <MemoryRouter initialEntries={["/settings/integrations"]}>
        <Routes>
          <Route path="/settings/*" element={<SettingsView />} />
        </Routes>
      </MemoryRouter>
    );
    expect(screen.getByLabelText("API Key")).toHaveValue("");
  });

  it("switches AI provider through the provider-scoped store action", async () => {
    const switchAiProvider = vi.fn().mockResolvedValue(undefined);
    setupMocks({ switchAiProvider });
    renderSettingsView("/settings/integrations");

    fireEvent.click(screen.getByRole("button", { name: "DeepSeek" }));

    await waitFor(() => {
      expect(switchAiProvider).toHaveBeenCalledWith("deepseek");
    });
  });

  it("shows custom protocol selector and resolved OpenAI URL", () => {
    setupMocks({
      aiSettings: {
        provider: "custom",
        region: "intl",
        apiKey: "",
        model: "custom-model",
        customUrl: "https://proxy.example.com/v1",
        protocol: "openai",
        tagConcurrency: "1",
        tagIntervalMs: "4000",
        tagStopOnRateLimit: true,
      },
    });
    renderSettingsView("/settings/integrations");

    expect(screen.getByText("自定义协议")).toBeTruthy();
    expect(screen.getByRole("radio", { name: /OpenAI/ })).toHaveAttribute("aria-checked", "true");
    expect(screen.getByText("https://proxy.example.com/v1/chat/completions")).toBeTruthy();
  });

  it("resolves OpenRouter as OpenAI-compatible chat completions", () => {
    setupMocks({
      aiSettings: {
        ...defaultAiSettings,
        provider: "openrouter",
        model: "anthropic/claude-sonnet-4.6",
        protocol: "openai",
      },
    });
    renderSettingsView("/settings/integrations");

    expect(screen.getByText("https://openrouter.ai/api/v1/chat/completions")).toBeTruthy();
    expect(screen.queryByText("https://openrouter.ai/api/v1/messages")).toBeNull();
  });

  it("clears the AI API key through the secure-storage action", async () => {
    const clearAiApiKey = vi.fn().mockResolvedValue(undefined);
    setupMocks({
      clearAiApiKey,
      aiApiKeyState: {
        configured: true,
        storageState: "stored",
        fingerprint: "sha256:oldkey12",
        error: null,
      },
    });
    renderSettingsView("/settings/integrations");

    fireEvent.click(screen.getByRole("button", { name: "清除 Key" }));

    await waitFor(() => {
      expect(clearAiApiKey).toHaveBeenCalled();
    });
  });

  it("renders only the default appearance page content on /settings", () => {
    setupMocks();
    renderSettingsView("/settings");

    expect(screen.getAllByText("外观").length).toBeGreaterThan(0);
    expect(screen.getAllByText("主题风格").length).toBeGreaterThan(0);
    expect(screen.queryByText("新增 SSH 目标")).toBeNull();
    expect(screen.queryByText("GitHub 导入访问令牌")).toBeNull();
    expect(screen.queryByText("扫描目录")).toBeNull();
    expect(screen.queryByText("自定义平台")).toBeNull();
    expect(screen.queryByText("应用版本")).toBeNull();
  });

  it("routes each settings page to its own content", () => {
    setupMocks({ agents: [mockBuiltinAgent], scanDirs: [mockBuiltinDir] });
    const { unmount } = renderSettingsView("/settings/connections");
    expect(screen.getByRole("heading", { name: "连接与同步" })).toBeTruthy();
    expect(screen.getByText("新增 SSH 目标")).toBeTruthy();
    unmount();

    setupMocks({ agents: [mockBuiltinAgent], scanDirs: [mockBuiltinDir] });
    const platforms = renderSettingsView("/settings/platforms");
    expect(screen.getByRole("heading", { name: "平台管理" })).toBeTruthy();
    expect(screen.getByText("自定义平台")).toBeTruthy();
    expect(screen.getByText("平台可见性")).toBeTruthy();
    platforms.unmount();

    setupMocks({ agents: [mockBuiltinAgent], scanDirs: [mockBuiltinDir] });
    const integrations = renderSettingsView("/settings/integrations");
    expect(screen.getByRole("heading", { name: "集成与密钥" })).toBeTruthy();
    expect(screen.getByText("GitHub 导入访问令牌")).toBeTruthy();
    expect(screen.getByText("AI 提供商")).toBeTruthy();
    integrations.unmount();

    setupMocks({ agents: [mockBuiltinAgent], scanDirs: [mockBuiltinDir] });
    const sources = renderSettingsView("/settings/skill-sources");
    expect(screen.getByRole("heading", { name: "技能来源" })).toBeTruthy();
    expect(screen.getByText("扫描目录")).toBeTruthy();
    sources.unmount();

    setupMocks({ agents: [mockBuiltinAgent], scanDirs: [mockBuiltinDir] });
    renderSettingsView("/settings/about");
    expect(screen.getByRole("heading", { name: "帮助与关于" })).toBeTruthy();
    expect(screen.getByText("应用版本")).toBeTruthy();
  });

  it("applies stable section tones to settings cards and page navigation", () => {
    setupMocks();
    const { container } = renderSettingsView("/settings/integrations");

    expect(
      container.querySelector(
        '[data-settings-section="github-pat"][data-settings-section-tone="github-pat"]'
      )
    ).toBeTruthy();
    expect(
      container.querySelector(
        '[data-settings-section="ai-provider"][data-settings-section-tone="ai-provider"]'
      )
    ).toBeTruthy();

    expect(
      container.querySelector(
        '[data-settings-page-nav="integrations"][aria-current="page"]'
      )
    ).toBeTruthy();
    expect(
      container.querySelector('[data-settings-page-nav="appearance"]')
    ).toBeTruthy();
  });

  it("stacks the integrations sections vertically", () => {
    setupMocks();
    const { container } = renderSettingsView("/settings/integrations");

    const integrationsLayout = container.querySelector(
      'div.space-y-4 > section#github-pat-section + section#ai-section'
    );

    expect(integrationsLayout).toBeTruthy();
    expect(
      container.querySelector(
        'div.grid.xl\\:grid-cols-\\[minmax\\(0\\,0\\.85fr\\)_minmax\\(0\\,1\\.15fr\\)\\]'
      )
    ).toBeNull();
  });

  it("wires settings titles through heading font and descriptive text through body defaults", () => {
    setupMocks();
    const { container } = renderSettingsView("/settings/connections");

    expect(screen.getByRole("heading", { name: "设置" })).toHaveClass(
      "font-heading"
    );
    expect(screen.getByRole("heading", { name: "连接与同步" })).toHaveClass(
      "font-heading"
    );
    expect(
      container.querySelector('[data-slot="card-title"]')
    ).toHaveClass("font-heading");
    expect(
      screen
        .getAllByText("管理本机、SSH 与 WSL 目标，并启动本机到远程同步。")
        .every((node) => !node.className.includes("font-heading"))
    ).toBe(true);
  });

  it("keeps the settings side navigation aria-current state when navigating pages", async () => {
    setupMocks();
    const { container } = renderSettingsView();

    const appearanceNav = container.querySelector('[data-settings-page-nav="appearance"]');
    const sourcesNav = container.querySelector('[data-settings-page-nav="skill-sources"]');

    expect(appearanceNav).toHaveAttribute("aria-current", "page");
    expect(sourcesNav).not.toHaveAttribute("aria-current");

    fireEvent.click(sourcesNav as HTMLElement);

    expect(await screen.findByRole("heading", { name: "技能来源" })).toBeTruthy();
    expect(sourcesNav).toHaveAttribute("aria-current", "page");
    expect(appearanceNav).not.toHaveAttribute("aria-current");
  });

  it("maps legacy settings links to the matching settings page", async () => {
    setupMocks();
    renderSettingsView("/settings#remote-targets-section");

    expect(await screen.findByRole("heading", { name: "连接与同步" })).toBeTruthy();
    expect(screen.getByText("新增 SSH 目标")).toBeTruthy();
  });

  it("maps Central update mode hashes to Skill Sources", async () => {
    setupMocks();
    renderSettingsView("/settings#central-update-check-mode");

    expect(await screen.findByRole("heading", { name: "技能来源" })).toBeTruthy();
    expect(screen.getByText("更新检查模式")).toBeTruthy();
  });

  it("persists collapsed settings sections in localStorage", () => {
    setupMocks();
    const { unmount } = renderSettingsView("/settings/skill-sources");
    const scanToggle = screen.getByRole("button", { name: "折叠 扫描目录 栏目" });

    expect(scanToggle).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByText("暂无扫描目录")).toBeTruthy();

    fireEvent.click(scanToggle);

    expect(screen.queryByText("暂无扫描目录")).toBeNull();
    expect(window.localStorage.getItem("settings.sectionCollapsed.v1")).toBe(
      JSON.stringify({ "scan-directories": true })
    );

    unmount();
    setupMocks();
    renderSettingsView("/settings/skill-sources");

    const restoredToggle = screen.getByRole("button", { name: "展开 扫描目录 栏目" });
    expect(restoredToggle).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByText("暂无扫描目录")).toBeNull();

    fireEvent.click(restoredToggle);

    expect(screen.getByText("暂无扫描目录")).toBeTruthy();
    expect(window.localStorage.getItem("settings.sectionCollapsed.v1")).toBe(
      JSON.stringify({ "scan-directories": false })
    );
  });

  it("renders the release cockpit resources and repository url in about", () => {
    setupMocks();
    renderSettingsView("/settings/about");

    expect(screen.getByText("Release Cockpit")).toBeTruthy();
    expect(screen.getByText("仓库地址")).toBeTruthy();
    expect(screen.getAllByRole("link", { name: "https://github.com/bahayonghang/skills-manage-windows" })[0]).toHaveAttribute(
      "href",
      "https://github.com/bahayonghang/skills-manage-windows"
    );
    expect(screen.getByRole("link", { name: /^README/ })).toHaveAttribute(
      "href",
      "https://github.com/bahayonghang/skills-manage-windows#readme"
    );
    expect(screen.getByRole("link", { name: /中文 README/ })).toBeTruthy();
    expect(screen.getByRole("link", { name: /Releases/ })).toBeTruthy();
    expect(screen.getByRole("link", { name: /Issues/ })).toBeTruthy();
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
    renderSettingsView("/settings/integrations");
    expect(loadGitHubPat).toHaveBeenCalled();
  });

  it("does not load targets or WSL distributions outside the connections page", () => {
    const loadTargets = vi.fn().mockResolvedValue(undefined);
    const loadWslDistributions = vi.fn().mockResolvedValue(undefined);
    setupMocks({ loadTargets, loadWslDistributions });
    renderSettingsView("/settings");
    expect(loadTargets).not.toHaveBeenCalled();
    expect(loadWslDistributions).not.toHaveBeenCalled();
  });

  it("loads targets on connections page without auto-discovering WSL distributions", () => {
    const loadTargets = vi.fn().mockResolvedValue(undefined);
    const loadWslDistributions = vi.fn().mockResolvedValue(undefined);
    setupMocks({ loadTargets, loadWslDistributions });
    renderSettingsView("/settings/connections");
    expect(loadTargets).toHaveBeenCalledTimes(1);
    expect(loadWslDistributions).not.toHaveBeenCalled();
    expect(
      screen.getByText("不会自动探测 WSL。点击“刷新 WSL 列表”后再选择发行版。")
    ).toBeTruthy();
  });

  it("discovers WSL distributions only after manual refresh", () => {
    const loadWslDistributions = vi.fn().mockResolvedValue(undefined);
    setupMocks({ loadWslDistributions });
    renderSettingsView("/settings/connections");

    expect(loadWslDistributions).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "刷新 WSL 列表" }));

    expect(loadWslDistributions).toHaveBeenCalledTimes(1);
  });

  it("keeps the saved github pat hidden until reveal", async () => {
    const revealGitHubPat = vi.fn().mockResolvedValue("github_pat_saved_value");
    setupMocks({
      revealGitHubPat,
      githubPatState: { configured: true, storageState: "stored", error: null },
    });
    renderSettingsView("/settings/integrations");

    expect(screen.getByLabelText("GitHub Personal Access Token")).toHaveValue("");
    expect(screen.getByText("已保存到安全存储")).toBeTruthy();
    expect(screen.getByText(/它绝不会被发送到公共镜像或代理回退链路/)).toBeTruthy();
    expect(screen.getByText(/当 GitHub 预览\/导入遇到限流/)).toBeTruthy();
    expect(screen.getByPlaceholderText("••••••••••••")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "显示已保存令牌" }));

    await waitFor(() => {
      expect(revealGitHubPat).toHaveBeenCalled();
    });
    expect(screen.getByLabelText("GitHub Personal Access Token")).toHaveValue(
      "github_pat_saved_value"
    );

    fireEvent.click(screen.getByRole("button", { name: "隐藏已保存令牌" }));
    expect(screen.getByLabelText("GitHub Personal Access Token")).toHaveValue("");
  });

  it("saves the github pat from settings", async () => {
    const saveGitHubPat = vi.fn().mockResolvedValue(undefined);
    setupMocks({ saveGitHubPat });
    renderSettingsView("/settings/integrations");

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
    setupMocks({
      githubPatState: { configured: true, storageState: "stored", error: null },
      clearGitHubPat,
    });
    renderSettingsView("/settings/integrations");

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
    renderSettingsView("/settings/connections");

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
      loadMarketplaceRegistries: vi.fn().mockResolvedValue(undefined),
    });
    renderSettingsView("/settings/connections");

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

  it("tests and creates a WSL target from settings", async () => {
    const createWslTarget = vi.fn().mockResolvedValue({
      id: "wsl-ubuntu",
      kind: "wsl" as const,
      label: "Ubuntu",
      distribution: "Ubuntu-24.04",
      remoteHome: "/home/lyh",
      remoteOs: "Linux",
      isActive: false,
    });
    const testWslTarget = vi.fn().mockResolvedValue({
      ok: true,
      remoteHome: "/home/lyh",
      remoteOs: "Linux",
      message: "ok",
    });
    setupMocks({
      createWslTarget,
      testWslTarget,
      wslDistributions: [
        {
          name: "Ubuntu-24.04",
          isDefault: true,
          state: "Stopped",
          version: "2",
        },
      ],
    });
    renderSettingsView("/settings/connections");

    fireEvent.change(screen.getByLabelText("WSL 别名"), {
      target: { value: "Ubuntu" },
    });
    fireEvent.change(screen.getByLabelText("WSL 发行版"), {
      target: { value: "Ubuntu-24.04" },
    });
    fireEvent.click(screen.getAllByRole("button", { name: "测试" })[1]);

    await waitFor(() => {
      expect(testWslTarget).toHaveBeenCalledWith({
        label: "Ubuntu",
        distribution: "Ubuntu-24.04",
      });
    });
    expect(await screen.findByText("连接成功：Linux /home/lyh")).toBeTruthy();
    expect(screen.getByText("状态: Stopped")).toBeTruthy();
    expect(screen.getByText("版本: 2")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "新增 WSL" }));

    await waitFor(() => {
      expect(createWslTarget).toHaveBeenCalledWith({
        label: "Ubuntu",
        distribution: "Ubuntu-24.04",
      });
    });
    expect(await screen.findByText("已创建目标 Ubuntu")).toBeTruthy();
  });

  it("marks already-added WSL distributions in the add card", () => {
    setupMocks({
      targets: [
        { id: "local", kind: "local" as const, label: "Local", isActive: true },
        {
          id: "wsl-ubuntu",
          kind: "wsl" as const,
          label: "Ubuntu",
          distribution: "Ubuntu-24.04",
          isActive: false,
        },
      ],
      wslDistributions: [
        {
          name: "Ubuntu-24.04",
          isDefault: true,
          state: "Stopped",
          version: "2",
        },
      ],
    });
    renderSettingsView("/settings/connections");

    const option = screen.getByRole("option", {
      name: /Ubuntu-24\.04.*已添加/,
    });
    expect(option).toBeDisabled();
    expect(screen.getByRole("button", { name: "新增 WSL" })).toBeDisabled();
  });

  it("updates an existing WSL target without SSH credentials", async () => {
    const updateWslTarget = vi.fn().mockResolvedValue({
      id: "wsl-ubuntu",
      kind: "wsl" as const,
      label: "Ubuntu Work",
      distribution: "Ubuntu",
      remoteHome: "/home/lyh",
      remoteOs: "Linux",
      isActive: true,
    });
    setupMocks({
      targets: [
        { id: "local", kind: "local" as const, label: "Local", isActive: false },
        {
          id: "wsl-ubuntu",
          kind: "wsl" as const,
          label: "Ubuntu",
          distribution: "Ubuntu-24.04",
          remoteHome: "/home/lyh",
          remoteOs: "Linux",
          cacheDbPath: "targets/wsl-ubuntu/db.sqlite",
          isActive: true,
        },
      ],
      activeTarget: {
        id: "wsl-ubuntu",
        kind: "wsl" as const,
        label: "Ubuntu",
        distribution: "Ubuntu-24.04",
        isActive: true,
      },
      updateWslTarget,
      rescan: vi.fn().mockResolvedValue(undefined),
      loadCentralSkills: vi.fn().mockResolvedValue(undefined),
      loadMarketplaceRegistries: vi.fn().mockResolvedValue(undefined),
    });
    renderSettingsView("/settings/connections");

    expect(screen.queryByText(/需要密码|Password needed/i)).toBeNull();
    fireEvent.click(screen.getByLabelText("编辑目标 Ubuntu"));
    fireEvent.change(screen.getByDisplayValue("Ubuntu"), {
      target: { value: "Ubuntu Work" },
    });
    fireEvent.change(screen.getByDisplayValue("Ubuntu-24.04"), {
      target: { value: "Ubuntu" },
    });
    fireEvent.click(screen.getByRole("button", { name: "保存修改" }));

    await waitFor(() => {
      expect(updateWslTarget).toHaveBeenCalledWith({
        id: "wsl-ubuntu",
        label: "Ubuntu Work",
        distribution: "Ubuntu",
      });
    });
    expect(await screen.findByText("已更新目标 Ubuntu Work")).toBeTruthy();
  });

  it("shows loading state for scan directories", () => {
    setupMocks({ isLoadingScanDirs: true });
    renderSettingsView("/settings/skill-sources");
    expect(screen.getByText("加载中...")).toBeTruthy();
  });

  it("renders and persists the Central update check mode preference", async () => {
    const setCentralUpdateCheckMode = vi.fn().mockResolvedValue(undefined);
    setupMocks({
      centralUpdateCheckMode: "regular",
      setCentralUpdateCheckMode,
    });
    renderSettingsView("/settings/skill-sources");

    expect(screen.getByText("更新检查模式")).toBeTruthy();
    expect(screen.getByRole("button", { name: /常规检查/ })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    fireEvent.click(screen.getByRole("button", { name: /增量和删减模式/ }));

    await waitFor(() => {
      expect(setCentralUpdateCheckMode).toHaveBeenCalledWith("sync");
    });
  });

  it("shows empty state when no scan directories", () => {
    setupMocks({ scanDirs: [] });
    renderSettingsView("/settings/skill-sources");
    expect(screen.getByText("暂无扫描目录")).toBeTruthy();
  });

  it("renders builtin scan directory with 内置目录 label", () => {
    setupMocks({ scanDirs: [mockBuiltinDir] });
    renderSettingsView("/settings/skill-sources");
    expect(screen.getAllByText(/内置目录/).length).toBeGreaterThan(0);
  });

  it("does not show remove button for builtin directories", () => {
    setupMocks({ scanDirs: [mockBuiltinDir] });
    renderSettingsView("/settings/skill-sources");
    // No delete button should be present for builtin dir
    expect(
      screen.queryByRole("button", { name: /删除目录 ~\/.agents\/skills\// })
    ).toBeNull();
  });

  it("shows remove button for custom directories", () => {
    setupMocks({ scanDirs: [mockCustomDir] });
    renderSettingsView("/settings/skill-sources");
    expect(
      screen.getByRole("button", { name: `删除目录 ${mockCustomDir.path}` })
    ).toBeTruthy();
  });

  it("shows toggle for custom directories", () => {
    setupMocks({ scanDirs: [mockCustomDir] });
    renderSettingsView("/settings/skill-sources");
    expect(
      screen.getByLabelText(`启用 ${mockCustomDir.path}`)
    ).toBeTruthy();
  });

  it("does not show toggle for builtin directories", () => {
    setupMocks({ scanDirs: [mockBuiltinDir] });
    renderSettingsView("/settings/skill-sources");
    expect(
      screen.queryByLabelText(`启用 ${mockBuiltinDir.path}`)
    ).toBeNull();
  });

  it("shows 启用 label when directory is active", () => {
    setupMocks({ scanDirs: [{ ...mockCustomDir, is_active: true }] });
    renderSettingsView("/settings/skill-sources");
    expect(screen.getByText("启用")).toBeTruthy();
  });

  it("shows 禁用 label when directory is inactive", () => {
    setupMocks({ scanDirs: [{ ...mockCustomDir, is_active: false }] });
    renderSettingsView("/settings/skill-sources");
    expect(screen.getByText("禁用")).toBeTruthy();
  });

  it("shows add directory button", () => {
    setupMocks();
    renderSettingsView("/settings/skill-sources");
    expect(screen.getByRole("button", { name: "添加项目目录" })).toBeTruthy();
  });

  it("opens add directory dialog when button is clicked", async () => {
    setupMocks();
    renderSettingsView("/settings/skill-sources");
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
    renderSettingsView("/settings/skill-sources");

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
    renderSettingsView("/settings/skill-sources");

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
    renderSettingsView("/settings/platforms");
    expect(screen.getByText("暂无自定义平台。点击「添加平台」注册新平台。")).toBeTruthy();
  });

  it("renders custom platform with name and path", () => {
    setupMocks({ agents: [mockBuiltinAgent, mockCustomAgent] });
    renderSettingsView("/settings/platforms");
    expect(screen.getAllByText("QClaw").length).toBeGreaterThan(0);
    expect(screen.getByText("/Users/test/.qclaw/skills/")).toBeTruthy();
  });

  it("shows edit button for custom platforms", () => {
    setupMocks({ agents: [mockBuiltinAgent, mockCustomAgent] });
    renderSettingsView("/settings/platforms");
    expect(
      screen.getByRole("button", { name: `编辑平台 ${mockCustomAgent.display_name}` })
    ).toBeTruthy();
  });

  it("shows remove button for custom platforms", () => {
    setupMocks({ agents: [mockBuiltinAgent, mockCustomAgent] });
    renderSettingsView("/settings/platforms");
    expect(
      screen.getByRole("button", { name: `删除平台 ${mockCustomAgent.display_name}` })
    ).toBeTruthy();
  });

  it("does not show builtin agents in custom platforms list", () => {
    setupMocks({ agents: [mockBuiltinAgent] });
    renderSettingsView("/settings/platforms");
    // builtin agent should not appear in custom platforms section
    expect(
      screen.queryByRole("button", { name: `编辑平台 ${mockBuiltinAgent.display_name}` })
    ).toBeNull();
  });

  it("shows add platform button", () => {
    setupMocks();
    renderSettingsView("/settings/platforms");
    expect(screen.getByRole("button", { name: "添加自定义平台" })).toBeTruthy();
  });

  it("opens add platform dialog when button is clicked", async () => {
    setupMocks();
    renderSettingsView("/settings/platforms");
    fireEvent.click(screen.getByRole("button", { name: "添加自定义平台" }));
    await waitFor(() => {
      expect(screen.getByText("添加自定义平台")).toBeTruthy();
    });
  });

  it("opens edit platform dialog when edit button is clicked", async () => {
    setupMocks({ agents: [mockBuiltinAgent, mockCustomAgent] });
    renderSettingsView("/settings/platforms");
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
    renderSettingsView("/settings/platforms");

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
    renderSettingsView("/settings/platforms");

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
    renderSettingsView("/settings/about");
    expect(screen.getByRole("heading", { name: "SkillPort" })).toBeTruthy();
    expect(screen.getAllByText(/v\d+\.\d+\.\d+/).length).toBeGreaterThan(0);
  });

  it("shows the database path in the about section", () => {
    setupMocks({ scanDirs: [mockBuiltinDir], agents: [mockBuiltinAgent] });
    renderSettingsView("/settings/about");
    expect(screen.getByText("/Users/test/.skillsmanage/db.sqlite")).toBeTruthy();
  });

  it("shows version label, update controls, diagnostics, and stack chips", () => {
    setupMocks();
    renderSettingsView("/settings/about");

    expect(screen.getByText("应用版本")).toBeTruthy();
    expect(screen.getByText("数据库路径")).toBeTruthy();
    expect(screen.getByText("应用更新")).toBeTruthy();
    expect(screen.getByRole("button", { name: /检查更新/ })).toBeEnabled();
    expect(screen.getByRole("button", { name: /下载并安装/ })).toBeDisabled();
    expect(screen.getByText("Windows x64 NSIS")).toBeTruthy();
    expect(screen.getByText(/自动更新需要发布流水线注入 Tauri updater 公钥/)).toBeTruthy();
    expect(screen.getByText("Rust")).toBeTruthy();
    expect(screen.getByText("Tauri 2")).toBeTruthy();
    expect(screen.getByText("React 19")).toBeTruthy();
    expect(screen.getByText("TypeScript 6")).toBeTruthy();
    expect(screen.getByText("Tailwind CSS 4")).toBeTruthy();
    expect(screen.getByText("SQLite/sqlx")).toBeTruthy();
    expect(screen.getByText("Zustand")).toBeTruthy();
    expect(screen.getByText("i18next")).toBeTruthy();
  });

  it("calls the update store when checking and installing updates", () => {
    const checkForUpdate = vi.fn();
    const installUpdate = vi.fn();
    setupMocks({
      appUpdateState: {
        status: "available",
        latestVersion: "0.10.8",
        releaseNotes: "Bug fixes",
        checkForUpdate,
        installUpdate,
      },
    });
    renderSettingsView("/settings/about");

    fireEvent.click(screen.getByRole("button", { name: /检查更新/ }));
    fireEvent.click(screen.getByRole("button", { name: /下载并安装/ }));

    expect(checkForUpdate).toHaveBeenCalledOnce();
    expect(installUpdate).toHaveBeenCalledOnce();
    expect(screen.getByText("v0.10.8 可用")).toBeTruthy();
    expect(screen.getByText("Bug fixes")).toBeTruthy();
  });

  it.each([
    ["upToDate", "已是最新版本。"],
    ["unsupported", "当前内置更新仅支持 Windows x64 桌面版。"],
    ["error", "metadata missing"],
  ] as const)("renders update state %s", (status, message) => {
    setupMocks({
      appUpdateState: {
        status,
        error: status === "error" ? "metadata missing" : null,
      },
    });
    renderSettingsView("/settings/about");

    expect(screen.getByText(message)).toBeTruthy();
  });

  it("renders downloading progress", () => {
    setupMocks({
      appUpdateState: {
        status: "downloading",
        progress: { downloaded: 50, total: 100 },
      },
    });
    renderSettingsView("/settings/about");

    expect(screen.getByRole("progressbar", { name: "应用更新下载进度" })).toHaveAttribute(
      "aria-valuenow",
      "50"
    );
    expect(screen.getByText("50 B / 100 B · 50%")).toBeTruthy();
  });

  // ── Flavor Switcher ──────────────────────────────────────────────────────

  it("shows flavor label in about section", () => {
    setupMocks();
    renderSettingsView("/settings");
    expect(screen.getAllByText("主题风格").length).toBeGreaterThan(0);
  });

  it("renders all flavor buttons", () => {
    setupMocks();
    renderSettingsView("/settings");
    expect(screen.getByRole("button", { name: /Mocha/ })).toBeTruthy();
    expect(screen.getByRole("button", { name: /Macchiato/ })).toBeTruthy();
    expect(screen.getByRole("button", { name: /Frappé/ })).toBeTruthy();
    expect(screen.getByRole("button", { name: /Latte/ })).toBeTruthy();
    expect(screen.getByRole("button", { name: /Claude 浅色/ })).toBeTruthy();
    expect(screen.getByRole("button", { name: /Claude 深色/ })).toBeTruthy();
  });

  it("active flavor button has aria-pressed=true", () => {
    setupMocks({ flavor: "mocha" });
    renderSettingsView("/settings");
    const mochaBtn = screen.getByRole("button", { name: /Mocha/ });
    expect(mochaBtn).toHaveAttribute("aria-pressed", "true");
  });

  it("inactive flavor button has aria-pressed=false", () => {
    setupMocks({ flavor: "mocha" });
    renderSettingsView("/settings");
    const latteBtn = screen.getByRole("button", { name: /Latte/ });
    expect(latteBtn).toHaveAttribute("aria-pressed", "false");
  });

  it("clicking a flavor button calls setFlavor", () => {
    const setFlavor = vi.fn();
    setupMocks({ flavor: "mocha", setFlavor });
    renderSettingsView("/settings");
    fireEvent.click(screen.getByRole("button", { name: /Latte/ }));
    expect(setFlavor).toHaveBeenCalledWith("latte");
  });

  it("each flavor button shows a color dot", () => {
    setupMocks();
    renderSettingsView("/settings");
    // Each flavor button should have a colored dot (inline span with rounded-full)
    const buttons = [
      screen.getByRole("button", { name: /Mocha/ }),
      screen.getByRole("button", { name: /Macchiato/ }),
      screen.getByRole("button", { name: /Frappé/ }),
      screen.getByRole("button", { name: /Latte/ }),
      screen.getByRole("button", { name: /Claude 浅色/ }),
      screen.getByRole("button", { name: /Claude 深色/ }),
    ];
    for (const btn of buttons) {
      const dot = btn.querySelector(".rounded-full");
      expect(dot).toBeTruthy();
    }
  });

  // ── Accent Color Picker ──────────────────────────────────────────────────

  it("shows accent color label in about section", () => {
    setupMocks();
    renderSettingsView("/settings");
    expect(screen.getAllByText("强调色").length).toBeGreaterThan(0);
  });

  it("renders 14 accent color swatches", () => {
    setupMocks();
    renderSettingsView("/settings");
    const swatches = screen.getAllByRole("radio");
    expect(swatches).toHaveLength(14);
  });

  it("active accent swatch has aria-checked=true", () => {
    setupMocks({ accent: "lavender" });
    renderSettingsView("/settings");
    const lavenderSwatch = screen.getByRole("radio", { name: "薰衣草" });
    expect(lavenderSwatch).toHaveAttribute("aria-checked", "true");
  });

  it("inactive accent swatch has aria-checked=false", () => {
    setupMocks({ accent: "lavender" });
    renderSettingsView("/settings");
    const greenSwatch = screen.getByRole("radio", { name: "绿色" });
    expect(greenSwatch).toHaveAttribute("aria-checked", "false");
  });

  it("clicking an accent swatch calls setAccent", () => {
    const setAccent = vi.fn();
    setupMocks({ accent: "lavender", setAccent });
    renderSettingsView("/settings");
    fireEvent.click(screen.getByRole("radio", { name: "绿色" }));
    expect(setAccent).toHaveBeenCalledWith("green");
  });

  it("accent swatches use CSS custom properties for background color", () => {
    setupMocks();
    renderSettingsView("/settings");
    const rosewaterSwatch = screen.getByRole("radio", { name: "玫瑰水" });
    expect(rosewaterSwatch.style.backgroundColor).toBe("var(--ctp-rosewater)");
  });

  it("keeps language and font scale controls operable with aria-pressed", () => {
    setupMocks();
    renderSettingsView("/settings");

    expect(screen.getByRole("button", { name: /中文/ })).toHaveAttribute(
      "aria-pressed",
      "true"
    );
    expect(screen.getByRole("button", { name: /English/ })).toHaveAttribute(
      "aria-pressed",
      "false"
    );

    expect(screen.getByRole("button", { name: /English/ })).toBeEnabled();

    const defaultScale = screen.getByRole("button", { name: /Default|默认/ });
    const spaciousScale = screen.getByRole("button", { name: /Spacious|宽松/ });

    expect(defaultScale).toHaveAttribute("aria-pressed", "true");
    expect(spaciousScale).toHaveAttribute("aria-pressed", "false");

    fireEvent.click(spaciousScale);

    expect(spaciousScale).toHaveAttribute("aria-pressed", "true");
    expect(defaultScale).toHaveAttribute("aria-pressed", "false");
  });

  it("shows custom font inputs only inside the selected font group", () => {
    setupMocks();
    renderSettingsView("/settings");

    expect(screen.queryByLabelText("自定义标题字体族")).toBeNull();
    expect(screen.queryByLabelText("自定义正文字体族")).toBeNull();

    const displayGroup = screen.getByRole("group", { name: "标题字体选项" });
    fireEvent.click(within(displayGroup).getByRole("button", { name: /自定义/ }));

    expect(within(displayGroup).getByLabelText("自定义标题字体族")).toBeTruthy();
    expect(screen.queryByLabelText("自定义正文字体族")).toBeNull();

    const bodyGroup = screen.getByRole("group", { name: "正文字体选项" });
    fireEvent.click(within(bodyGroup).getByRole("button", { name: /自定义/ }));

    expect(within(displayGroup).getByLabelText("自定义标题字体族")).toBeTruthy();
    expect(within(bodyGroup).getByLabelText("自定义正文字体族")).toBeTruthy();
  });

  it("updates the preview specimen chips from selected font and scale state", () => {
    setupMocks();
    renderSettingsView("/settings");

    expect(screen.getByText("Midnight Type Lab")).toBeTruthy();
    expect(screen.getByLabelText("标题字体 Geist Sans")).toBeTruthy();
    expect(screen.getByLabelText("正文字体 JetBrains Mono")).toBeTruthy();
    expect(screen.getByLabelText("字号缩放 默认")).toBeTruthy();

    const displayGroup = screen.getByRole("group", { name: "标题字体选项" });
    const bodyGroup = screen.getByRole("group", { name: "正文字体选项" });

    fireEvent.click(
      within(displayGroup).getByRole("button", { name: /Instrument Serif/ })
    );
    fireEvent.click(within(bodyGroup).getByRole("button", { name: /Inter/ }));
    fireEvent.click(screen.getByRole("button", { name: /宽松/ }));

    expect(screen.getByLabelText("标题字体 Instrument Serif")).toBeTruthy();
    expect(screen.getByLabelText("正文字体 Inter")).toBeTruthy();
    expect(screen.getByLabelText("字号缩放 宽松")).toBeTruthy();
  });

  it("toggles coding group visibility from settings", async () => {
    const setCategoryVisibility = vi.fn().mockResolvedValue(undefined);
    setupMocks({ setCategoryVisibility });
    renderSettingsView("/settings/platforms");

    fireEvent.click(screen.getByLabelText("切换 编程类 分组显示"));

    await waitFor(() => {
      expect(setCategoryVisibility).toHaveBeenCalledWith("coding", false);
    });
  });

  it("toggles an individual platform visibility from settings", async () => {
    const setAgentEnabled = vi.fn().mockResolvedValue(undefined);
    setupMocks({ agents: [mockBuiltinAgent, mockCustomAgent], setAgentEnabled });
    renderSettingsView("/settings/platforms");

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
    renderSettingsView("/settings/platforms");

    expect(screen.getByText("已隐藏 · 0 / 1 已启用")).toBeTruthy();
    expect(screen.queryByText("OpenClaw")).toBeNull();
  });

  it("collapses coding platform details without hiding the group", () => {
    setupMocks({
      agents: [mockBuiltinAgent, mockOpenCodeAgent, mockCursorAgent],
      categoryVisibility: { coding: true, lobster: false },
    });
    renderSettingsView("/settings/platforms");

    fireEvent.click(screen.getByRole("button", { name: "收起 编程类 分组的平台详情" }));

    expect(screen.queryByText("Claude Code")).toBeNull();
    expect(screen.getByText("已折叠 · 2 / 3 已启用 · 分组显示中")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "展开 编程类 分组的平台详情" }));

    expect(screen.getByLabelText("切换平台 Claude Code 的显示")).toBeTruthy();
  });

  it("keeps search results visible even when a platform group was collapsed", () => {
    setupMocks({
      agents: [mockBuiltinAgent, mockOpenCodeAgent, mockCursorAgent],
      categoryVisibility: { coding: true, lobster: false },
    });
    renderSettingsView("/settings/platforms");

    fireEvent.click(screen.getByRole("button", { name: "收起 编程类 分组的平台详情" }));
    fireEvent.change(screen.getByLabelText("搜索平台"), {
      target: { value: "cursor" },
    });

    expect(screen.getByText("Cursor")).toBeTruthy();
    expect(screen.queryByText(/已折叠/)).toBeNull();
  });

  it("filters platform groups by the local search box", () => {
    setupMocks({
      agents: [mockBuiltinAgent, mockOpenCodeAgent, mockCursorAgent, mockLobsterAgent],
    });
    renderSettingsView("/settings/platforms");

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
    renderSettingsView("/settings/platforms");

    expect(screen.queryByText("显示工具")).toBeNull();
  });
});
