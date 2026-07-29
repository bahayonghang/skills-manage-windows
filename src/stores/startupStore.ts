import { create } from "zustand";

import { invoke } from "@/lib/ipc";
import type { StartupStatus } from "@/types";

export type StartupAction = "retry" | "rebuild" | "exit";

interface StartupState {
  status: StartupStatus | null;
  isInitialLoading: boolean;
  activeAction: StartupAction | null;
  actionError: StartupAction | "load" | null;
  loadStatus: () => Promise<void>;
  retry: () => Promise<void>;
  rebuild: () => Promise<void>;
  exit: () => Promise<void>;
}

const STATUS_UNAVAILABLE: StartupStatus = {
  phase: "fatal",
  issue: "startup_state_unavailable",
};

let initialLoadPromise: Promise<void> | null = null;

export const useStartupStore = create<StartupState>((set, get) => ({
  status: null,
  isInitialLoading: false,
  activeAction: null,
  actionError: null,

  loadStatus() {
    if (get().status !== null) return Promise.resolve();
    if (initialLoadPromise) return initialLoadPromise;

    set({ isInitialLoading: true, actionError: null });
    initialLoadPromise = invoke("get_startup_status")
      .then((status) => set({ status, actionError: null }))
      .catch(() => set({ status: STATUS_UNAVAILABLE, actionError: "load" }))
      .finally(() => {
        set({ isInitialLoading: false });
        initialLoadPromise = null;
      });
    return initialLoadPromise;
  },

  async retry() {
    if (get().activeAction !== null) return;
    set({ activeAction: "retry", actionError: null });
    try {
      const status = await invoke("retry_startup");
      set({ status });
    } catch {
      set({ actionError: "retry" });
    } finally {
      set({ activeAction: null });
    }
  },

  async rebuild() {
    if (get().activeAction !== null) return;
    set({ activeAction: "rebuild", actionError: null });
    try {
      const status = await invoke("rebuild_startup_database");
      set({ status });
    } catch {
      set({ actionError: "rebuild" });
    } finally {
      set({ activeAction: null });
    }
  },

  async exit() {
    if (get().activeAction !== null) return;
    set({ activeAction: "exit", actionError: null });
    try {
      await invoke("exit_startup");
    } catch {
      set({ activeAction: null, actionError: "exit" });
    }
  },
}));

export function resetStartupStoreForTests(): void {
  initialLoadPromise = null;
  useStartupStore.setState({
    status: null,
    isInitialLoading: false,
    activeAction: null,
    actionError: null,
  });
}
