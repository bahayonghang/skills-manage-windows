import { create } from "zustand";
import { invoke, isTauriRuntime } from "@/lib/ipc";
import {
  CENTRAL_UPDATE_CHECK_MODE_SETTING_KEY,
  DEFAULT_UPDATE_CHECK_MODE,
  normalizeUpdateCheckMode,
  type UpdateCheckMode,
} from "@/pages/centralUpdateCheckMode";
import {
  GitHubPatState,
  GitHubPatTestResult,
  ScanDirectory,
} from "@/types";
import {
  type AiConnectionTestResult,
  type AiSaveStatus,
  type AiSettings,
  type AiSettingsSlice,
  createAiSettingsInitialState,
  createAiSettingsSlice,
} from "./settingsStore.aiSlice";

export type { AiConnectionTestResult, AiSaveStatus, AiSettings };

interface SettingsState extends AiSettingsSlice {
  scanDirectories: ScanDirectory[];
  centralUpdateCheckMode: UpdateCheckMode;
  centralUpdateCheckModeLoaded: boolean;
  isLoadingCentralUpdateCheckMode: boolean;
  isLoadingScanDirs: boolean;
  error: string | null;
  isLoadingGitHubPat: boolean;
  isSavingGitHubPat: boolean;
  isTestingGitHubPat: boolean;
  githubPatState: GitHubPatState;
  githubPatTestResult: GitHubPatTestResult | null;

  loadScanDirectories: () => Promise<void>;
  addScanDirectory: (path: string, label?: string) => Promise<ScanDirectory>;
  removeScanDirectory: (path: string) => Promise<void>;
  toggleScanDirectory: (path: string, active: boolean) => Promise<void>;

  loadCentralUpdateCheckMode: () => Promise<void>;
  setCentralUpdateCheckMode: (mode: UpdateCheckMode) => Promise<void>;

  loadGitHubPat: () => Promise<void>;
  revealGitHubPat: () => Promise<string | null>;
  saveGitHubPat: (value: string) => Promise<void>;
  clearGitHubPat: () => Promise<void>;
  testGitHubPat: () => Promise<GitHubPatTestResult>;

  clearError: () => void;
}


const BROWSER_FIXTURE_SCAN_DIRECTORIES: ScanDirectory[] = [
  {
    id: 1,
    path: "~/.agents/skills/",
    label: "Central Skills",
    is_active: true,
    is_builtin: true,
    added_at: "2026-01-01T00:00:00Z",
  },
];

const BROWSER_FIXTURE_GITHUB_PAT_STATE: GitHubPatState = {
  configured: false,
  storageState: "missing",
  error: null,
};

const BROWSER_FIXTURE_GITHUB_PAT_TEST_RESULT: GitHubPatTestResult = {
  configured: false,
  ok: false,
  status: null,
  messageKey: "settings.githubPatMissing",
  message: "GitHub PAT is not configured in the browser fixture.",
};

export function createSettingsStoreInitialState(): Pick<
  SettingsState,
  | "scanDirectories"
  | "centralUpdateCheckMode"
  | "centralUpdateCheckModeLoaded"
  | "isLoadingCentralUpdateCheckMode"
  | "isLoadingScanDirs"
  | "error"
  | "isLoadingGitHubPat"
  | "isSavingGitHubPat"
  | "isTestingGitHubPat"
  | "githubPatState"
  | "githubPatTestResult"
> &
  ReturnType<typeof createAiSettingsInitialState> {
  return {
    scanDirectories: [],
    centralUpdateCheckMode: DEFAULT_UPDATE_CHECK_MODE,
    centralUpdateCheckModeLoaded: false,
    isLoadingCentralUpdateCheckMode: false,
    isLoadingScanDirs: false,
    error: null,
    isLoadingGitHubPat: false,
    isSavingGitHubPat: false,
    isTestingGitHubPat: false,
    githubPatState: {
      configured: false,
      storageState: "missing",
      error: null,
    },
    githubPatTestResult: null,
    ...createAiSettingsInitialState(),
  };
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  ...createSettingsStoreInitialState(),
  ...createAiSettingsSlice(set, get),

  loadScanDirectories: async () => {
    set({ isLoadingScanDirs: true, error: null });
    if (!isTauriRuntime()) {
      set({
        scanDirectories: BROWSER_FIXTURE_SCAN_DIRECTORIES,
        isLoadingScanDirs: false,
      });
      return;
    }
    try {
      const dirs = await invoke<ScanDirectory[]>("get_scan_directories");
      set({ scanDirectories: dirs, isLoadingScanDirs: false });
    } catch (err) {
      set({ error: String(err), isLoadingScanDirs: false });
    }
  },

  addScanDirectory: async (path: string, label?: string) => {
    if (!isTauriRuntime()) {
      const dir: ScanDirectory = {
        id: Date.now(),
        path,
        label: label || undefined,
        is_active: true,
        is_builtin: false,
        added_at: new Date(0).toISOString(),
      };
      set((state) => ({
        scanDirectories: [...state.scanDirectories, dir],
      }));
      return dir;
    }
    const dir = await invoke<ScanDirectory>("add_scan_directory", {
      path,
      label: label || null,
    });
    set((state) => ({
      scanDirectories: [...state.scanDirectories, dir],
    }));
    return dir;
  },

  removeScanDirectory: async (path: string) => {
    if (!isTauriRuntime()) {
      set((state) => ({
        scanDirectories: state.scanDirectories.filter((dir) => dir.path !== path),
      }));
      return;
    }
    await invoke<void>("remove_scan_directory", { path });
    set((state) => ({
      scanDirectories: state.scanDirectories.filter((dir) => dir.path !== path),
    }));
  },

  toggleScanDirectory: async (path: string, active: boolean) => {
    if (!isTauriRuntime()) {
      set((state) => ({
        scanDirectories: state.scanDirectories.map((dir) =>
          dir.path === path ? { ...dir, is_active: active } : dir
        ),
      }));
      return;
    }
    await invoke<void>("set_scan_directory_active", { path, isActive: active });
    set((state) => ({
      scanDirectories: state.scanDirectories.map((dir) =>
        dir.path === path ? { ...dir, is_active: active } : dir
      ),
    }));
  },

  loadCentralUpdateCheckMode: async () => {
    set({ isLoadingCentralUpdateCheckMode: true, error: null });
    if (!isTauriRuntime()) {
      set({
        centralUpdateCheckMode: DEFAULT_UPDATE_CHECK_MODE,
        centralUpdateCheckModeLoaded: true,
        isLoadingCentralUpdateCheckMode: false,
      });
      return;
    }
    try {
      const value = await invoke<string | null>("get_setting", {
        key: CENTRAL_UPDATE_CHECK_MODE_SETTING_KEY,
      });
      set({
        centralUpdateCheckMode: normalizeUpdateCheckMode(value),
        centralUpdateCheckModeLoaded: true,
        isLoadingCentralUpdateCheckMode: false,
      });
    } catch (err) {
      set({
        error: String(err),
        centralUpdateCheckModeLoaded: true,
        isLoadingCentralUpdateCheckMode: false,
      });
    }
  },

  setCentralUpdateCheckMode: async (mode) => {
    const normalizedMode = normalizeUpdateCheckMode(mode);
    set({
      centralUpdateCheckMode: normalizedMode,
      centralUpdateCheckModeLoaded: true,
      isLoadingCentralUpdateCheckMode: true,
      error: null,
    });
    if (!isTauriRuntime()) {
      set({ isLoadingCentralUpdateCheckMode: false });
      return;
    }
    try {
      await invoke("set_setting", {
        key: CENTRAL_UPDATE_CHECK_MODE_SETTING_KEY,
        value: normalizedMode,
      });
      set({ isLoadingCentralUpdateCheckMode: false });
    } catch (err) {
      set({ error: String(err), isLoadingCentralUpdateCheckMode: false });
      throw err;
    }
  },

  loadGitHubPat: async () => {
    set({ isLoadingGitHubPat: true, error: null });
    if (!isTauriRuntime()) {
      set({
        githubPatState: BROWSER_FIXTURE_GITHUB_PAT_STATE,
        isLoadingGitHubPat: false,
      });
      return;
    }
    try {
      const state = await invoke<GitHubPatState>("get_github_pat");
      set({
        githubPatState: state,
        isLoadingGitHubPat: false,
      });
    } catch (err) {
      set({
        error: String(err),
        isLoadingGitHubPat: false,
      });
    }
  },

  revealGitHubPat: async () => {
    if (!isTauriRuntime()) {
      return null;
    }
    return invoke<string | null>("reveal_github_pat");
  },

  saveGitHubPat: async (value: string) => {
    set({ isSavingGitHubPat: true, error: null });
    if (!isTauriRuntime()) {
      set({
        githubPatState: value.trim()
          ? { configured: true, storageState: "session", error: null }
          : BROWSER_FIXTURE_GITHUB_PAT_STATE,
        isSavingGitHubPat: false,
        githubPatTestResult: null,
      });
      return;
    }
    try {
      const state = await invoke<GitHubPatState>("set_github_pat", { value });
      set({
        githubPatState: state,
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
    if (!isTauriRuntime()) {
      set({
        githubPatState: BROWSER_FIXTURE_GITHUB_PAT_STATE,
        isSavingGitHubPat: false,
        githubPatTestResult: null,
      });
      return;
    }
    try {
      const state = await invoke<GitHubPatState>("clear_github_pat");
      set({
        githubPatState: state,
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
    if (!isTauriRuntime()) {
      set({
        githubPatTestResult: BROWSER_FIXTURE_GITHUB_PAT_TEST_RESULT,
        isTestingGitHubPat: false,
      });
      return BROWSER_FIXTURE_GITHUB_PAT_TEST_RESULT;
    }
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

  clearError: () => set({ error: null }),
}));
