import { create } from "zustand";
import { invoke, isTauriRuntime } from "@/lib/tauri";
import {
  CreateSshTargetRequest,
  SshTargetTestResult,
  TargetSummary,
  TestSshTargetRequest,
} from "@/types";

const LOCAL_TARGET: TargetSummary = {
  id: "local",
  kind: "local",
  label: "Local",
  isActive: true,
};

interface TargetState {
  targets: TargetSummary[];
  activeTarget: TargetSummary;
  isLoading: boolean;
  isCreating: boolean;
  testingTargetId: string | null;
  updatingPasswordTargetId: string | null;
  switchingTargetId: string | null;
  deletingTargetId: string | null;
  error: string | null;

  loadTargets: () => Promise<void>;
  createSshTarget: (request: CreateSshTargetRequest) => Promise<TargetSummary>;
  testSshTarget: (request: TestSshTargetRequest) => Promise<SshTargetTestResult>;
  updateSshTargetPassword: (targetId: string, password: string) => Promise<SshTargetTestResult>;
  deleteTarget: (targetId: string) => Promise<void>;
  switchTarget: (targetId: string) => Promise<TargetSummary>;
  clearError: () => void;
}

function resolveActiveTarget(targets: TargetSummary[]): TargetSummary {
  return targets.find((target) => target.isActive) ?? targets[0] ?? LOCAL_TARGET;
}

function markActive(targets: TargetSummary[], targetId: string): TargetSummary[] {
  return targets.map((target) => ({
    ...target,
    isActive: target.id === targetId,
  }));
}

export const useTargetStore = create<TargetState>((set, get) => ({
  targets: [LOCAL_TARGET],
  activeTarget: LOCAL_TARGET,
  isLoading: false,
  isCreating: false,
  testingTargetId: null,
  updatingPasswordTargetId: null,
  switchingTargetId: null,
  deletingTargetId: null,
  error: null,

  loadTargets: async () => {
    if (!isTauriRuntime()) {
      set({ targets: [LOCAL_TARGET], activeTarget: LOCAL_TARGET, error: null });
      return;
    }

    set({ isLoading: true, error: null });
    try {
      const targets = await invoke<TargetSummary[]>("list_targets");
      set({
        targets: targets ?? [LOCAL_TARGET],
        activeTarget: resolveActiveTarget(targets ?? [LOCAL_TARGET]),
        isLoading: false,
      });
    } catch (err) {
      set({ error: String(err), isLoading: false });
      throw err;
    }
  },

  createSshTarget: async (request) => {
    if (!isTauriRuntime()) {
      throw new Error("Remote targets are available only in the Tauri app.");
    }

    set({ isCreating: true, error: null });
    try {
      const target = await invoke<TargetSummary>("create_ssh_target", { request });
      set((state) => ({
        targets: [...state.targets.filter((item) => item.id !== target.id), target],
        activeTarget: resolveActiveTarget([
          ...state.targets.filter((item) => item.id !== target.id),
          target,
        ]),
        isCreating: false,
      }));
      await get().loadTargets();
      return target;
    } catch (err) {
      set({ error: String(err), isCreating: false });
      throw err;
    }
  },

  testSshTarget: async (request) => {
    if (!isTauriRuntime()) {
      return {
        ok: true,
        remoteHome: "/home/fixture",
        remoteOs: "Linux",
        message: "Browser fixture target is reachable.",
      };
    }

    const testingTargetId = request.id ?? "new";
    set({ testingTargetId, error: null });
    try {
      return await invoke<SshTargetTestResult>("test_ssh_target", { request });
    } catch (err) {
      set({ error: String(err) });
      throw err;
    } finally {
      set({ testingTargetId: null });
    }
  },

  updateSshTargetPassword: async (targetId, password) => {
    if (!isTauriRuntime()) {
      return {
        ok: true,
        remoteHome: "/home/fixture",
        remoteOs: "Linux",
        message: "Browser fixture target password is saved.",
      };
    }

    set({ updatingPasswordTargetId: targetId, error: null });
    try {
      const result = await invoke<SshTargetTestResult>("update_ssh_target_password", {
        targetId,
        password,
      });
      if (result.ok) {
        await get().loadTargets();
      }
      return result;
    } catch (err) {
      set({ error: String(err) });
      throw err;
    } finally {
      set({ updatingPasswordTargetId: null });
    }
  },

  deleteTarget: async (targetId) => {
    if (!isTauriRuntime()) {
      return;
    }

    set({ deletingTargetId: targetId, error: null });
    try {
      await invoke<void>("delete_target", { targetId });
      await get().loadTargets();
    } catch (err) {
      set({ error: String(err) });
      throw err;
    } finally {
      set({ deletingTargetId: null });
    }
  },

  switchTarget: async (targetId) => {
    if (!isTauriRuntime()) {
      const targets = markActive(get().targets, targetId);
      const activeTarget = resolveActiveTarget(targets);
      set({ targets, activeTarget });
      return activeTarget;
    }

    set({ switchingTargetId: targetId, error: null });
    try {
      const activeTarget = await invoke<TargetSummary>("set_active_target", { targetId });
      set((state) => ({
        targets: markActive(state.targets, activeTarget.id),
        activeTarget,
      }));
      await get().loadTargets();
      return activeTarget;
    } catch (err) {
      set({ error: String(err) });
      throw err;
    } finally {
      set({ switchingTargetId: null });
    }
  },

  clearError: () => set({ error: null }),
}));
