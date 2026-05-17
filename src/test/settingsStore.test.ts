import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { ScanDirectory } from "../types";

// Mock Tauri core before importing the store
vi.mock("@/lib/tauri", () => ({
  invoke: vi.fn(),
  isTauriRuntime: vi.fn(() => true),
}));

import { invoke } from "@/lib/tauri";
import {
  createSettingsStoreInitialState,
  useSettingsStore,
} from "../stores/settingsStore";
import { resetAiSettingsSliceForTests } from "../stores/settingsStore.aiSlice";

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

function resetSettingsStoreState() {
  useSettingsStore.setState(createSettingsStoreInitialState());
}

// ─── Tests ────────────────────────────────────────────────────────────────────

describe("settingsStore", () => {
  beforeEach(() => {
    resetAiSettingsSliceForTests();
    resetSettingsStoreState();
    vi.useRealTimers();
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.useRealTimers();
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

  // ── clearError ────────────────────────────────────────────────────────────

  it("clearError resets error to null", () => {
    useSettingsStore.setState({ error: "Some error" });
    useSettingsStore.getState().clearError();
    expect(useSettingsStore.getState().error).toBeNull();
  });

  it("loadGitHubPat reads the app-wide token status without plaintext", async () => {
    vi.mocked(invoke).mockResolvedValueOnce({
      configured: true,
      storageState: "stored",
      error: null,
    });

    await useSettingsStore.getState().loadGitHubPat();

    expect(invoke).toHaveBeenCalledWith("get_github_pat");
    expect(useSettingsStore.getState().githubPatState).toEqual({
      configured: true,
      storageState: "stored",
      error: null,
    });
    expect(useSettingsStore.getState().isLoadingGitHubPat).toBe(false);
  });

  it("saveGitHubPat persists a token through secure storage", async () => {
    vi.mocked(invoke).mockResolvedValueOnce({
      configured: true,
      storageState: "stored",
      error: null,
    });

    await useSettingsStore.getState().saveGitHubPat("  github_pat_abc  ");

    expect(invoke).toHaveBeenCalledWith("set_github_pat", {
      value: "  github_pat_abc  ",
    });
    expect(useSettingsStore.getState().githubPatState.configured).toBe(true);
    expect(useSettingsStore.getState().isSavingGitHubPat).toBe(false);
  });

  it("clearGitHubPat clears the saved token status", async () => {
    useSettingsStore.setState({
      githubPatState: { configured: true, storageState: "stored", error: null },
    });
    vi.mocked(invoke).mockResolvedValueOnce({
      configured: false,
      storageState: "missing",
      error: null,
    });

    await useSettingsStore.getState().clearGitHubPat();

    expect(invoke).toHaveBeenCalledWith("clear_github_pat");
    expect(useSettingsStore.getState().githubPatState.configured).toBe(false);
    expect(useSettingsStore.getState().isSavingGitHubPat).toBe(false);
  });

  it("testGitHubPat stores the app-wide token test result", async () => {
    vi.mocked(invoke).mockResolvedValueOnce({
      configured: true,
      ok: true,
      status: 200,
      messageKey: "settings.githubPatTestSuccess",
      message: "GitHub token is usable",
    });

    const result = await useSettingsStore.getState().testGitHubPat();

    expect(invoke).toHaveBeenCalledWith("test_github_pat");
    expect(result.ok).toBe(true);
    expect(useSettingsStore.getState().githubPatTestResult?.status).toBe(200);
    expect(useSettingsStore.getState().isTestingGitHubPat).toBe(false);
  });

  it("loadAiSettings reads AI settings with one batch command", async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce({
        ai_provider: "glm",
        ai_region: "cn",
        ai_model: "legacy-model",
        ai_api_url: "https://legacy.example.com/v1/messages",
        ai_model__glm: "glm-5",
        ai_api_url__glm: "https://open.bigmodel.cn/api/anthropic/v1/messages",
        ai_protocol__glm: "anthropic",
        ai_tag_concurrency: "2",
        ai_tag_interval_ms: "5000",
        ai_tag_stop_on_rate_limit: "false",
      })
      .mockResolvedValueOnce({
        configured: true,
        storageState: "stored",
        fingerprint: "sha256:1234abcd",
        provider: "glm",
        error: null,
      });

    await useSettingsStore.getState().loadAiSettings();

    expect(invoke).toHaveBeenCalledWith("get_settings", {
      keys: expect.arrayContaining([
        "ai_provider",
        "ai_region",
        "ai_model",
        "ai_api_url",
        "ai_model__glm",
        "ai_api_url__custom",
        "ai_protocol__custom",
        "ai_tag_concurrency",
        "ai_tag_interval_ms",
        "ai_tag_stop_on_rate_limit",
      ]),
    });
    expect(useSettingsStore.getState().aiSettings).toMatchObject({
      provider: "glm",
      region: "cn",
      apiKey: "",
      model: "glm-5",
      tagConcurrency: "2",
      tagIntervalMs: "5000",
      tagStopOnRateLimit: false,
    });
    expect(useSettingsStore.getState().aiApiKeyState.configured).toBe(true);
    expect(useSettingsStore.getState().aiApiKeyState).toMatchObject({
      provider: "glm",
      fingerprint: "sha256:1234abcd",
    });
    expect(invoke).toHaveBeenCalledWith("get_ai_api_key_state", { provider: "glm" });
  });

  it("debounces rapid AI setting edits into one batch save", async () => {
    vi.useFakeTimers();
    vi.mocked(invoke)
      .mockResolvedValueOnce({
        configured: true,
        storageState: "stored",
        fingerprint: "sha256:savedabc",
        provider: "claude",
        error: null,
      })
      .mockResolvedValueOnce(undefined);
    useSettingsStore.setState({
      aiSettingsLoaded: true,
      aiSettings: {
        provider: "claude",
        region: "intl",
        apiKey: "",
        model: "",
        customUrl: "",
        protocol: "",
        tagConcurrency: "1",
        tagIntervalMs: "4000",
        tagStopOnRateLimit: true,
      },
    });

    useSettingsStore.getState().updateAiSettings({ apiKey: "a" });
    useSettingsStore.getState().updateAiSettings({ apiKey: "ab" });
    useSettingsStore.getState().updateAiSettings({ apiKey: "abc" });

    expect(invoke).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(800);

    expect(invoke).toHaveBeenCalledTimes(2);
    expect(invoke).toHaveBeenNthCalledWith(1, "set_ai_api_key", {
      provider: "claude",
      value: "abc",
    });
    expect(invoke).toHaveBeenNthCalledWith(2, "set_settings", {
      values: expect.objectContaining({
        ai_region__claude: "intl",
        ai_model__claude: "",
        ai_tag_concurrency: "1",
        ai_tag_interval_ms: "4000",
        ai_tag_stop_on_rate_limit: "true",
      }),
    });
    expect(useSettingsStore.getState().aiSettings.apiKey).toBe("");
    expect(useSettingsStore.getState().aiApiKeyState).toMatchObject({
      configured: true,
      fingerprint: "sha256:savedabc",
    });
  });

  it("flushes AI settings before testing the connection", async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce({
        configured: true,
        storageState: "stored",
        fingerprint: "sha256:testabcd",
        provider: "claude",
        error: null,
      })
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce({ ok: true, msg: "OK" });
    useSettingsStore.setState({
      aiSettingsLoaded: true,
      aiSettings: {
        provider: "claude",
        region: "intl",
        apiKey: "key",
        model: "claude-sonnet-4-20250514",
        customUrl: "",
        protocol: "",
        tagConcurrency: "1",
        tagIntervalMs: "4000",
        tagStopOnRateLimit: true,
      },
    });

    const result = await useSettingsStore.getState().testAiConnection();

    expect(result.ok).toBe(true);
    expect(invoke).toHaveBeenNthCalledWith(1, "set_ai_api_key", {
      provider: "claude",
      value: "key",
    });
    expect(invoke).toHaveBeenNthCalledWith(2, "set_settings", {
      values: expect.not.objectContaining({ ai_api_key: expect.any(String) }),
    });
    expect(invoke).toHaveBeenNthCalledWith(3, "test_ai_connection");
    expect(useSettingsStore.getState().aiSettings.apiKey).toBe("");
    expect(useSettingsStore.getState().aiApiKeyState.fingerprint).toBe("sha256:testabcd");
  });

  it("switchAiProvider restores scoped settings and reads scoped key state", async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce({
        configured: true,
        storageState: "stored",
        fingerprint: "sha256:deepabcd",
        provider: "deepseek",
        error: null,
      })
      .mockResolvedValueOnce(undefined);
    useSettingsStore.setState({
      aiSettingsLoaded: true,
      aiRawSettings: {
        ai_model__deepseek: "deepseek-v4-flash",
        ai_region__deepseek: "cn",
        ai_api_url__custom: "https://proxy.example.com/v1/chat/completions",
        ai_custom_base_url__custom: "https://proxy.example.com/v1",
        ai_protocol__custom: "openai",
      },
      aiSettings: {
        provider: "claude",
        region: "intl",
        apiKey: "",
        model: "claude-sonnet-4-20250514",
        customUrl: "",
        protocol: "",
        tagConcurrency: "1",
        tagIntervalMs: "4000",
        tagStopOnRateLimit: true,
      },
    });

    await useSettingsStore.getState().switchAiProvider("deepseek");

    expect(useSettingsStore.getState().aiSettings).toMatchObject({
      provider: "deepseek",
      region: "cn",
      model: "deepseek-v4-flash",
      protocol: "",
    });
    expect(useSettingsStore.getState().aiApiKeyState).toMatchObject({
      provider: "deepseek",
      configured: true,
      fingerprint: "sha256:deepabcd",
    });
    expect(invoke).toHaveBeenNthCalledWith(1, "get_ai_api_key_state", { provider: "deepseek" });
    expect(invoke).toHaveBeenNthCalledWith(2, "set_settings", {
      values: expect.objectContaining({
        ai_provider: "deepseek",
        ai_model__deepseek: "deepseek-v4-flash",
      }),
    });
  });

  it("switchAiProvider uses OpenRouter OpenAI-compatible defaults", async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce({
        configured: false,
        storageState: "missing",
        fingerprint: null,
        provider: "openrouter",
        error: null,
      })
      .mockResolvedValueOnce(undefined);
    useSettingsStore.setState({
      aiSettingsLoaded: true,
      aiRawSettings: {},
      aiSettings: {
        provider: "claude",
        region: "intl",
        apiKey: "",
        model: "claude-sonnet-4-20250514",
        customUrl: "",
        protocol: "",
        tagConcurrency: "1",
        tagIntervalMs: "4000",
        tagStopOnRateLimit: true,
      },
    });

    await useSettingsStore.getState().switchAiProvider("openrouter");

    expect(useSettingsStore.getState().aiSettings).toMatchObject({
      provider: "openrouter",
      region: "intl",
      model: "anthropic/claude-sonnet-4.6",
      protocol: "openai",
    });
    expect(invoke).toHaveBeenNthCalledWith(2, "set_settings", {
      values: expect.objectContaining({
        ai_provider: "openrouter",
        ai_api_url__openrouter: "https://openrouter.ai/api/v1/chat/completions",
        ai_protocol__openrouter: "openai",
      }),
    });
  });

  it("normalizes the legacy OpenRouter messages endpoint to chat completions", async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce({
        ai_provider: "openrouter",
        ai_api_url__openrouter: "https://openrouter.ai/api/v1/messages",
      })
      .mockResolvedValueOnce({
        configured: false,
        storageState: "missing",
        fingerprint: null,
        provider: "openrouter",
        error: null,
      });

    await useSettingsStore.getState().loadAiSettings();

    expect(useSettingsStore.getState().aiSettings).toMatchObject({
      provider: "openrouter",
      customUrl: "https://openrouter.ai/api/v1/chat/completions",
      protocol: "openai",
    });
  });

  it("forces OpenRouter to OpenAI protocol even with legacy scoped protocol", async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce({
        ai_provider: "openrouter",
        ai_protocol__openrouter: "anthropic",
      })
      .mockResolvedValueOnce({
        configured: false,
        storageState: "missing",
        fingerprint: null,
        provider: "openrouter",
        error: null,
      });

    await useSettingsStore.getState().loadAiSettings();

    expect(useSettingsStore.getState().aiSettings.protocol).toBe("openai");
  });

  it("serializes custom base URL separately from resolved OpenAI URL", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(undefined);
    useSettingsStore.setState({
      aiSettingsLoaded: true,
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

    await useSettingsStore.getState().flushAiSettings();

    expect(invoke).toHaveBeenCalledWith("set_settings", {
      values: expect.objectContaining({
        ai_api_url__custom: "https://proxy.example.com/v1/chat/completions",
        ai_custom_base_url__custom: "https://proxy.example.com/v1",
        ai_protocol__custom: "openai",
      }),
    });
  });

  it("clearAiApiKey clears the secure AI key state without touching other settings", async () => {
    useSettingsStore.setState({
      aiApiKeyState: {
        configured: true,
        storageState: "stored",
        fingerprint: "sha256:oldkey12",
        error: null,
      },
    });
    vi.mocked(invoke).mockResolvedValueOnce({
      configured: false,
      storageState: "missing",
      fingerprint: null,
      error: null,
    });

    await useSettingsStore.getState().clearAiApiKey();

    expect(invoke).toHaveBeenCalledWith("clear_ai_api_key", { provider: "claude" });
    expect(useSettingsStore.getState().aiApiKeyState.configured).toBe(false);
    expect(useSettingsStore.getState().aiApiKeyState.fingerprint).toBeNull();
    expect(useSettingsStore.getState().aiSettings.apiKey).toBe("");
  });
});
