import { create } from "zustand";

import { invoke, isTauriRuntime } from "@/lib/ipc";
import type {
  LocalRemoteSyncApplyRequest,
  LocalRemoteSyncApplyResult,
  LocalRemoteSyncPreview,
  LocalRemoteSyncPreviewRequest,
} from "@/types";

interface LocalRemoteSyncState {
  preview: LocalRemoteSyncPreview | null;
  result: LocalRemoteSyncApplyResult | null;
  isPreviewing: boolean;
  isApplying: boolean;
  error: string | null;
  previewSync: (request: LocalRemoteSyncPreviewRequest) => Promise<LocalRemoteSyncPreview>;
  applySync: (request: LocalRemoteSyncApplyRequest) => Promise<LocalRemoteSyncApplyResult>;
  reset: () => void;
}

function ensureTauriRuntime() {
  if (!isTauriRuntime()) {
    throw new Error("Remote sync is available only in the Tauri app.");
  }
}

const INITIAL_STATE = {
  preview: null,
  result: null,
  isPreviewing: false,
  isApplying: false,
  error: null,
};

export const useLocalRemoteSyncStore = create<LocalRemoteSyncState>((set) => ({
  ...INITIAL_STATE,

  previewSync: async (request) => {
    ensureTauriRuntime();
    set({ isPreviewing: true, error: null, result: null });
    try {
      const preview = await invoke<LocalRemoteSyncPreview>("preview_local_remote_sync", {
        request,
      });
      set({ preview, isPreviewing: false });
      return preview;
    } catch (error) {
      set({ error: String(error), isPreviewing: false });
      throw error;
    }
  },

  applySync: async (request) => {
    ensureTauriRuntime();
    set({ isApplying: true, error: null });
    try {
      const result = await invoke<LocalRemoteSyncApplyResult>("apply_local_remote_sync", {
        request,
      });
      set({ result, isApplying: false });
      return result;
    } catch (error) {
      set({ error: String(error), isApplying: false });
      throw error;
    }
  },

  reset: () => set(INITIAL_STATE),
}));
