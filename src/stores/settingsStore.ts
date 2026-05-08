import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import {
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
  isLoadingScanDirs: boolean;
  error: string | null;
  githubPat: string;
  isLoadingGitHubPat: boolean;
  isSavingGitHubPat: boolean;
  isTestingGitHubPat: boolean;
  githubPatTestResult: GitHubPatTestResult | null;

  loadScanDirectories: () => Promise<void>;
  addScanDirectory: (path: string, label?: string) => Promise<ScanDirectory>;
  removeScanDirectory: (path: string) => Promise<void>;
  toggleScanDirectory: (path: string, active: boolean) => Promise<void>;

  loadGitHubPat: () => Promise<void>;
  saveGitHubPat: (value: string) => Promise<void>;
  clearGitHubPat: () => Promise<void>;
  testGitHubPat: () => Promise<GitHubPatTestResult>;

  clearError: () => void;
}

export function createSettingsStoreInitialState(): Pick<
  SettingsState,
  | "scanDirectories"
  | "isLoadingScanDirs"
  | "error"
  | "githubPat"
  | "isLoadingGitHubPat"
  | "isSavingGitHubPat"
  | "isTestingGitHubPat"
  | "githubPatTestResult"
> &
  ReturnType<typeof createAiSettingsInitialState> {
  return {
    scanDirectories: [],
    isLoadingScanDirs: false,
    error: null,
    githubPat: "",
    isLoadingGitHubPat: false,
    isSavingGitHubPat: false,
    isTestingGitHubPat: false,
    githubPatTestResult: null,
    ...createAiSettingsInitialState(),
  };
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  ...createSettingsStoreInitialState(),
  ...createAiSettingsSlice(set, get),

  loadScanDirectories: async () => {
    set({ isLoadingScanDirs: true, error: null });
    try {
      const dirs = await invoke<ScanDirectory[]>("get_scan_directories");
      set({ scanDirectories: dirs, isLoadingScanDirs: false });
    } catch (err) {
      set({ error: String(err), isLoadingScanDirs: false });
    }
  },

  addScanDirectory: async (path: string, label?: string) => {
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
    await invoke<void>("remove_scan_directory", { path });
    set((state) => ({
      scanDirectories: state.scanDirectories.filter((dir) => dir.path !== path),
    }));
  },

  toggleScanDirectory: async (path: string, active: boolean) => {
    await invoke<void>("set_scan_directory_active", { path, isActive: active });
    set((state) => ({
      scanDirectories: state.scanDirectories.map((dir) =>
        dir.path === path ? { ...dir, is_active: active } : dir
      ),
    }));
  },

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

  clearError: () => set({ error: null }),
}));
