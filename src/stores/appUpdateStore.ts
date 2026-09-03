import { create } from "zustand";

import { invoke, isTauriRuntime } from "@/lib/ipc";

export type AppUpdateStatus =
  | "idle"
  | "checking"
  | "available"
  | "downloading"
  | "installing"
  | "upToDate"
  | "unsupported"
  | "error";

export interface AppUpdateProgress {
  downloaded: number;
  total: number | null;
}

interface DownloadEventLike {
  event: "Started" | "Progress" | "Finished";
  data?: {
    contentLength?: number;
    chunkLength?: number;
  };
}

interface UpdateLike {
  currentVersion: string;
  version: string;
  body?: string;
  downloadAndInstall: (onEvent?: (event: DownloadEventLike) => void) => Promise<void>;
}

interface AppUpdateState {
  status: AppUpdateStatus;
  currentVersion: string;
  latestVersion: string | null;
  releaseNotes: string | null;
  progress: AppUpdateProgress;
  error: string | null;
  hasChecked: boolean;
  checkForUpdate: () => Promise<void>;
  installUpdate: () => Promise<void>;
  reset: () => void;
}

let cachedUpdate: UpdateLike | null = null;

const EMPTY_PROGRESS: AppUpdateProgress = { downloaded: 0, total: null };
const CURRENT_VERSION = __APP_VERSION__;

function toErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function resetCachedUpdate() {
  cachedUpdate = null;
}

async function isWindowsUpdaterSupported(): Promise<boolean> {
  if (!isTauriRuntime()) {
    return false;
  }

  const runtimeInfo = await invoke("get_app_runtime_info");
  return runtimeInfo.windowsUpdaterSupported;
}

export const useAppUpdateStore = create<AppUpdateState>((set, get) => ({
  status: "idle",
  currentVersion: CURRENT_VERSION,
  latestVersion: null,
  releaseNotes: null,
  progress: EMPTY_PROGRESS,
  error: null,
  hasChecked: false,

  async checkForUpdate() {
    set({
      status: "checking",
      currentVersion: CURRENT_VERSION,
      latestVersion: null,
      releaseNotes: null,
      error: null,
      progress: EMPTY_PROGRESS,
    });
    resetCachedUpdate();

    if (!isTauriRuntime()) {
      set({
        status: "unsupported",
        error: null,
        hasChecked: true,
      });
      return;
    }

    try {
      if (!(await isWindowsUpdaterSupported())) {
        set({
          status: "unsupported",
          error: null,
          hasChecked: true,
        });
        return;
      }

      const { check } = await import("@tauri-apps/plugin-updater");
      const update = (await check()) as UpdateLike | null;

      if (!update) {
        set({
          status: "upToDate",
          currentVersion: CURRENT_VERSION,
          latestVersion: null,
          releaseNotes: null,
          hasChecked: true,
        });
        return;
      }

      cachedUpdate = update;
      set({
        status: "available",
        currentVersion: update.currentVersion || CURRENT_VERSION,
        latestVersion: update.version,
        releaseNotes: update.body ?? null,
        hasChecked: true,
      });
    } catch (error) {
      set({
        status: "error",
        error: toErrorMessage(error),
        hasChecked: true,
      });
    }
  },

  async installUpdate() {
    const update = cachedUpdate;
    if (!update) {
      await get().checkForUpdate();
    }

    const nextUpdate = cachedUpdate;
    if (!nextUpdate) {
      return;
    }

    set({ status: "downloading", error: null, progress: EMPTY_PROGRESS });

    try {
      await nextUpdate.downloadAndInstall((event) => {
        if (event.event === "Started") {
          set({
            status: "downloading",
            progress: {
              downloaded: 0,
              total: event.data?.contentLength ?? null,
            },
          });
          return;
        }

        if (event.event === "Progress") {
          const chunkLength = event.data?.chunkLength ?? 0;
          set((state) => ({
            status: "downloading",
            progress: {
              downloaded: state.progress.downloaded + chunkLength,
              total: state.progress.total,
            },
          }));
          return;
        }

        set((state) => ({
          status: "installing",
          progress: {
            downloaded: state.progress.total ?? state.progress.downloaded,
            total: state.progress.total,
          },
        }));
      });

      set({ status: "installing" });
      const { relaunch } = await import("@tauri-apps/plugin-process");
      await relaunch();
    } catch (error) {
      set({
        status: "error",
        error: toErrorMessage(error),
      });
    }
  },

  reset() {
    resetCachedUpdate();
    set({
      status: "idle",
      currentVersion: CURRENT_VERSION,
      latestVersion: null,
      releaseNotes: null,
      progress: EMPTY_PROGRESS,
      error: null,
      hasChecked: false,
    });
  },
}));

export function getAppUpdateProgressPercent(progress: AppUpdateProgress): number {
  if (!progress.total || progress.total <= 0) {
    return 0;
  }

  return Math.min(100, Math.round((progress.downloaded / progress.total) * 100));
}

export function formatAppUpdateBytes(bytes: number | null): string {
  if (bytes === null) return "—";
  if (bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB"];
  let value = bytes;
  let unitIndex = 0;

  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }

  const fractionDigits = value >= 10 || unitIndex === 0 ? 0 : 1;
  return `${value.toFixed(fractionDigits)} ${units[unitIndex]}`;
}
