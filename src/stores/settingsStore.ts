import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import {
  AgentWithStatus,
  CustomAgentConfig,
  GitHubPatTestResult,
  ScanDirectory,
  UpdateCustomAgentConfig,
} from "@/types";
import { AI_PROVIDERS, type RegionId } from "@/data/aiProviders";


export type AiSaveStatus = "idle" | "saving" | "saved" | "error";

export interface AiSettings {
  provider: string;
  region: RegionId;
  apiKey: string;
  model: string;
  customUrl: string;
  tagConcurrency: string;
  tagIntervalMs: string;
  tagStopOnRateLimit: boolean;
}

export interface AiConnectionTestResult {
  ok: boolean;
  msg: string;
  details?: string;
}

const DEFAULT_AI_SETTINGS: AiSettings = {
  provider: "claude",
  region: "intl",
  apiKey: "",
  model: "",
  customUrl: "",
  tagConcurrency: "1",
  tagIntervalMs: "4000",
  tagStopOnRateLimit: true,
};

const AI_SETTING_KEYS = [
  "ai_provider",
  "ai_region",
  "ai_api_key",
  "ai_model",
  "ai_api_url",
  "ai_tag_concurrency",
  "ai_tag_interval_ms",
  "ai_tag_stop_on_rate_limit",
];

const AI_SAVE_DEBOUNCE_MS = 800;
let aiSaveTimer: ReturnType<typeof setTimeout> | null = null;
let aiSaveSequence = 0;

function resolveAiApiUrl(settings: AiSettings): string {
  if (settings.provider === "custom") return settings.customUrl;
  const provider = AI_PROVIDERS.find((item) => item.id === settings.provider);
  return provider?.endpoints[settings.region] ?? "";
}

function normalizeAiSettings(values: Record<string, string | null | undefined>): AiSettings {
  const provider = values.ai_provider || DEFAULT_AI_SETTINGS.provider;
  const providerMeta = AI_PROVIDERS.find((item) => item.id === provider);
  const fallbackRegion = providerMeta?.regions[0] ?? DEFAULT_AI_SETTINGS.region;
  const rawRegion = values.ai_region || fallbackRegion;
  const region = (providerMeta?.regions.includes(rawRegion as RegionId)
    ? rawRegion
    : fallbackRegion) as RegionId;

  return {
    provider,
    region,
    apiKey: values.ai_api_key ?? DEFAULT_AI_SETTINGS.apiKey,
    model: values.ai_model ?? providerMeta?.defaultModel ?? DEFAULT_AI_SETTINGS.model,
    customUrl: values.ai_api_url ?? DEFAULT_AI_SETTINGS.customUrl,
    tagConcurrency: values.ai_tag_concurrency ?? DEFAULT_AI_SETTINGS.tagConcurrency,
    tagIntervalMs: values.ai_tag_interval_ms ?? DEFAULT_AI_SETTINGS.tagIntervalMs,
    tagStopOnRateLimit:
      values.ai_tag_stop_on_rate_limit == null
        ? DEFAULT_AI_SETTINGS.tagStopOnRateLimit
        : values.ai_tag_stop_on_rate_limit !== "false" && values.ai_tag_stop_on_rate_limit !== "0",
  };
}

function serializeAiSettings(settings: AiSettings): Record<string, string> {
  return {
    ai_provider: settings.provider,
    ai_region: settings.region,
    ai_api_key: settings.apiKey,
    ai_model: settings.model,
    ai_api_url: resolveAiApiUrl(settings),
    ai_tag_concurrency: settings.tagConcurrency,
    ai_tag_interval_ms: settings.tagIntervalMs,
    ai_tag_stop_on_rate_limit: settings.tagStopOnRateLimit ? "true" : "false",
  };
}

function parseAiConnectionError(err: unknown): AiConnectionTestResult {
  const raw = String(err);
  let msg = raw;
  let details: string | undefined;
  const prefix = "API 请求失败: ";
  if (raw.startsWith(prefix)) {
    const after = raw.slice(prefix.length);
    const nlIdx = after.indexOf("\n");
    if (nlIdx > 0) {
      msg = after.slice(nlIdx + 1);
      details = after.slice(0, nlIdx);
    } else {
      msg = after;
    }
  }
  return { ok: false, msg, details };
}

// ─── State ────────────────────────────────────────────────────────────────────

interface SettingsState {
  scanDirectories: ScanDirectory[];
  isLoadingScanDirs: boolean;
  error: string | null;
  githubPat: string;
  isLoadingGitHubPat: boolean;
  isSavingGitHubPat: boolean;
  isTestingGitHubPat: boolean;
  githubPatTestResult: GitHubPatTestResult | null;
  aiSettings: AiSettings;
  aiSettingsLoaded: boolean;
  isLoadingAiSettings: boolean;
  aiSaveStatus: AiSaveStatus;
  aiSaveError: string | null;
  aiTesting: boolean;
  aiTestResult: AiConnectionTestResult | null;

  // Actions — scan directories
  loadScanDirectories: () => Promise<void>;
  addScanDirectory: (path: string, label?: string) => Promise<ScanDirectory>;
  removeScanDirectory: (path: string) => Promise<void>;
  toggleScanDirectory: (path: string, active: boolean) => Promise<void>;

  // Actions — GitHub PAT
  loadGitHubPat: () => Promise<void>;
  saveGitHubPat: (value: string) => Promise<void>;
  clearGitHubPat: () => Promise<void>;
  testGitHubPat: () => Promise<GitHubPatTestResult>;

  // Actions — AI settings
  loadAiSettings: () => Promise<void>;
  updateAiSettings: (patch: Partial<AiSettings>) => void;
  flushAiSettings: () => Promise<void>;
  testAiConnection: () => Promise<AiConnectionTestResult>;

  // Actions — custom agents
  addCustomAgent: (config: CustomAgentConfig) => Promise<AgentWithStatus>;
  updateCustomAgent: (agentId: string, config: UpdateCustomAgentConfig) => Promise<AgentWithStatus>;
  removeCustomAgent: (agentId: string) => Promise<void>;

  clearError: () => void;
}

// ─── Store ────────────────────────────────────────────────────────────────────

export const useSettingsStore = create<SettingsState>((set, get) => ({
  scanDirectories: [],
  isLoadingScanDirs: false,
  error: null,
  githubPat: "",
  isLoadingGitHubPat: false,
  isSavingGitHubPat: false,
  isTestingGitHubPat: false,
  githubPatTestResult: null,
  aiSettings: DEFAULT_AI_SETTINGS,
  aiSettingsLoaded: false,
  isLoadingAiSettings: false,
  aiSaveStatus: "idle",
  aiSaveError: null,
  aiTesting: false,
  aiTestResult: null,

  // ── Scan Directories ───────────────────────────────────────────────────────

  /**
   * Load all scan directories from the backend.
   */
  loadScanDirectories: async () => {
    set({ isLoadingScanDirs: true, error: null });
    try {
      const dirs = await invoke<ScanDirectory[]>("get_scan_directories");
      set({ scanDirectories: dirs, isLoadingScanDirs: false });
    } catch (err) {
      set({ error: String(err), isLoadingScanDirs: false });
    }
  },

  /**
   * Add a new custom scan directory.
   * Returns the created ScanDirectory or throws on error.
   */
  addScanDirectory: async (path: string, label?: string) => {
    const dir = await invoke<ScanDirectory>("add_scan_directory", {
      path,
      label: label || null,
    });
    // Refresh the list
    set((state) => ({
      scanDirectories: [...state.scanDirectories, dir],
    }));
    return dir;
  },

  /**
   * Remove a custom scan directory by path.
   */
  removeScanDirectory: async (path: string) => {
    await invoke<void>("remove_scan_directory", { path });
    set((state) => ({
      scanDirectories: state.scanDirectories.filter((d) => d.path !== path),
    }));
  },

  /**
   * Toggle the active state of a custom scan directory.
   * Persists the change to the backend database.
   */
  toggleScanDirectory: async (path: string, active: boolean) => {
    await invoke<void>("set_scan_directory_active", { path, isActive: active });
    set((state) => ({
      scanDirectories: state.scanDirectories.map((d) =>
        d.path === path ? { ...d, is_active: active } : d
      ),
    }));
  },

  // ── GitHub PAT ────────────────────────────────────────────────────────────

  loadGitHubPat: async () => {
    set({ isLoadingGitHubPat: true, error: null });
    try {
      const value = await invoke<string | null>("get_github_pat");
      set({
        githubPat: value ?? "",
        isLoadingGitHubPat: false,
      });
    } catch (err) {
      set({
        error: String(err),
        isLoadingGitHubPat: false,
      });
    }
  },

  saveGitHubPat: async (value: string) => {
    set({ isSavingGitHubPat: true, error: null });
    try {
      await invoke("set_github_pat", { value });
      set({
        githubPat: value.trim(),
        isSavingGitHubPat: false,
        githubPatTestResult: null,
      });
    } catch (err) {
      set({
        error: String(err),
        isSavingGitHubPat: false,
      });
      throw err;
    }
  },

  clearGitHubPat: async () => {
    set({ isSavingGitHubPat: true, error: null });
    try {
      await invoke("clear_github_pat");
      set({
        githubPat: "",
        isSavingGitHubPat: false,
        githubPatTestResult: null,
      });
    } catch (err) {
      set({
        error: String(err),
        isSavingGitHubPat: false,
      });
      throw err;
    }
  },

  testGitHubPat: async () => {
    set({ isTestingGitHubPat: true, error: null, githubPatTestResult: null });
    try {
      const result = await invoke<GitHubPatTestResult>("test_github_pat");
      set({
        githubPatTestResult: result,
        isTestingGitHubPat: false,
      });
      return result;
    } catch (err) {
      set({
        error: String(err),
        isTestingGitHubPat: false,
      });
      throw err;
    }
  },

  // ── AI Settings ────────────────────────────────────────────────────────────

  loadAiSettings: async () => {
    set({ isLoadingAiSettings: true, aiSaveError: null });
    try {
      const values = await invoke<Record<string, string | null>>("get_settings", {
        keys: AI_SETTING_KEYS,
      });
      set({
        aiSettings: normalizeAiSettings(values),
        aiSettingsLoaded: true,
        isLoadingAiSettings: false,
        aiSaveStatus: "saved",
      });
    } catch (err) {
      set({
        error: String(err),
        aiSaveError: String(err),
        aiSettingsLoaded: true,
        isLoadingAiSettings: false,
        aiSaveStatus: "error",
      });
    }
  },

  updateAiSettings: (patch) => {
    const next = { ...get().aiSettings, ...patch };
    set({
      aiSettings: next,
      aiSaveStatus: get().aiSettingsLoaded ? "saving" : get().aiSaveStatus,
      aiSaveError: null,
      aiTestResult: null,
    });

    if (!get().aiSettingsLoaded) return;
    if (aiSaveTimer) clearTimeout(aiSaveTimer);
    aiSaveTimer = setTimeout(() => {
      aiSaveTimer = null;
      void get().flushAiSettings();
    }, AI_SAVE_DEBOUNCE_MS);
  },

  flushAiSettings: async () => {
    if (!get().aiSettingsLoaded) return;
    if (aiSaveTimer) {
      clearTimeout(aiSaveTimer);
      aiSaveTimer = null;
    }

    const sequence = ++aiSaveSequence;
    const settings = get().aiSettings;
    set({ aiSaveStatus: "saving", aiSaveError: null });
    try {
      await invoke("set_settings", { values: serializeAiSettings(settings) });
      if (sequence === aiSaveSequence) {
        set({ aiSaveStatus: "saved", aiSaveError: null });
      }
    } catch (err) {
      if (sequence === aiSaveSequence) {
        set({ aiSaveStatus: "error", aiSaveError: String(err), error: String(err) });
      }
      throw err;
    }
  },

  testAiConnection: async () => {
    await get().flushAiSettings();
    set({ aiTesting: true, aiTestResult: null, error: null });
    try {
      const result = await invoke<string>("explain_skill", {
        content: "Test connection. Reply with: OK",
      });
      const testResult = { ok: true, msg: result.slice(0, 60) };
      set({ aiTesting: false, aiTestResult: testResult });
      return testResult;
    } catch (err) {
      const testResult = parseAiConnectionError(err);
      set({ aiTesting: false, aiTestResult: testResult });
      return testResult;
    }
  },

  // ── Custom Agents ──────────────────────────────────────────────────────────

  /**
   * Register a new user-defined agent.
   * Returns the created AgentWithStatus or throws on error.
   */
  addCustomAgent: async (config: CustomAgentConfig) => {
    const agent = await invoke<AgentWithStatus>("add_custom_agent", { config });
    return agent;
  },

  /**
   * Update an existing user-defined agent.
   * Returns the updated AgentWithStatus or throws on error.
   */
  updateCustomAgent: async (agentId: string, config: UpdateCustomAgentConfig) => {
    const agent = await invoke<AgentWithStatus>("update_custom_agent", {
      agentId,
      config,
    });
    return agent;
  },

  /**
   * Remove a user-defined agent by ID.
   */
  removeCustomAgent: async (agentId: string) => {
    await invoke<void>("remove_custom_agent", { agentId });
  },

  // ── Misc ───────────────────────────────────────────────────────────────────

  clearError: () => set({ error: null }),
}));
