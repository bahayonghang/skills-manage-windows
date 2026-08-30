import { create } from "zustand";
import { invoke } from "@/lib/ipc";
import {
  CreateSshTargetRequest,
  CreateWslTargetRequest,
  SshTargetTestResult,
  TargetConfigQuarantineStatus,
  TargetSummary,
  TestSshTargetRequest,
  TestWslTargetRequest,
  UpdateSshTargetRequest,
  UpdateWslTargetRequest,
  WslDistributionSummary,
  WslTargetTestResult,
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
  quarantineStatus: TargetConfigQuarantineStatus | null;
  quarantineStatusError: string | null;
  wslDistributions: WslDistributionSummary[];
  isLoading: boolean;
  isLoadingWslDistributions: boolean;
  isCreating: boolean;
  updatingTargetId: string | null;
  testingTargetId: string | null;
  updatingPasswordTargetId: string | null;
  switchingTargetId: string | null;
  deletingTargetId: string | null;
  requiresTargetReload: boolean;
  error: string | null;
  wslDistributionError: string | null;

  loadTargets: () => Promise<void>;
  loadWslDistributions: () => Promise<void>;
  createSshTarget: (request: CreateSshTargetRequest) => Promise<TargetSummary>;
  updateSshTarget: (request: UpdateSshTargetRequest) => Promise<TargetSummary>;
  testSshTarget: (
    request: TestSshTargetRequest,
  ) => Promise<SshTargetTestResult>;
  createWslTarget: (request: CreateWslTargetRequest) => Promise<TargetSummary>;
  updateWslTarget: (request: UpdateWslTargetRequest) => Promise<TargetSummary>;
  testWslTarget: (
    request: TestWslTargetRequest,
  ) => Promise<WslTargetTestResult>;
  updateSshTargetPassword: (
    targetId: string,
    password: string,
  ) => Promise<SshTargetTestResult>;
  deleteTarget: (targetId: string) => Promise<void>;
  switchTarget: (targetId: string) => Promise<TargetSummary>;
  clearError: () => void;
}

function resolveActiveTarget(targets: TargetSummary[]): TargetSummary {
  return (
    targets.find((target) => target.isActive) ?? targets[0] ?? LOCAL_TARGET
  );
}

function markActive(
  targets: TargetSummary[],
  targetId: string,
): TargetSummary[] {
  return targets.map((target) => ({
    ...target,
    isActive: target.id === targetId,
  }));
}

async function refreshTargetsAfterMutation(
  loadTargets: () => Promise<void>,
  markReloadRequired: () => void,
): Promise<void> {
  try {
    await loadTargets();
  } catch {
    markReloadRequired();
  }
}

export const useTargetStore = create<TargetState>((set, get) => ({
  targets: [LOCAL_TARGET],
  activeTarget: LOCAL_TARGET,
  quarantineStatus: null,
  quarantineStatusError: null,
  wslDistributions: [],
  isLoading: false,
  isLoadingWslDistributions: false,
  isCreating: false,
  updatingTargetId: null,
  testingTargetId: null,
  updatingPasswordTargetId: null,
  switchingTargetId: null,
  deletingTargetId: null,
  requiresTargetReload: false,
  error: null,
  wslDistributionError: null,

  loadTargets: async () => {
    set({ isLoading: true, error: null, quarantineStatusError: null });
    const [targetsResult, quarantineResult] = await Promise.allSettled([
      invoke("list_targets"),
      invoke("get_target_config_quarantine_status"),
    ]);

    if (targetsResult.status === "rejected") {
      set({
        error: String(targetsResult.reason),
        quarantineStatus:
          quarantineResult.status === "fulfilled"
            ? quarantineResult.value
            : get().quarantineStatus,
        quarantineStatusError:
          quarantineResult.status === "rejected"
            ? String(quarantineResult.reason)
            : null,
        isLoading: false,
      });
      throw targetsResult.reason;
    }

    const targets = targetsResult.value ?? [LOCAL_TARGET];
    set({
      targets,
      activeTarget: resolveActiveTarget(targets),
      quarantineStatus:
        quarantineResult.status === "fulfilled"
          ? quarantineResult.value
          : get().quarantineStatus,
      quarantineStatusError:
        quarantineResult.status === "rejected"
          ? String(quarantineResult.reason)
          : null,
      isLoading: false,
      requiresTargetReload: false,
    });
  },

  loadWslDistributions: async () => {
    set({ isLoadingWslDistributions: true, wslDistributionError: null });
    try {
      const distributions = await invoke("list_wsl_distributions");
      set({
        wslDistributions: distributions ?? [],
        isLoadingWslDistributions: false,
      });
    } catch (err) {
      set({
        wslDistributionError: String(err),
        isLoadingWslDistributions: false,
      });
      throw err;
    }
  },

  createSshTarget: async (request) => {
    set({ isCreating: true, error: null });
    try {
      const target = await invoke("create_ssh_target", {
        request,
      });
      set((state) => ({
        targets: [
          ...state.targets.filter((item) => item.id !== target.id),
          target,
        ],
        activeTarget: resolveActiveTarget([
          ...state.targets.filter((item) => item.id !== target.id),
          target,
        ]),
        isCreating: false,
      }));
      await refreshTargetsAfterMutation(get().loadTargets, () =>
        set({ requiresTargetReload: true }),
      );
      return target;
    } catch (err) {
      set({ error: String(err), isCreating: false });
      throw err;
    }
  },

  updateSshTarget: async (request) => {
    set({ updatingTargetId: request.id, error: null });
    try {
      const target = await invoke("update_ssh_target", {
        request,
      });
      await refreshTargetsAfterMutation(get().loadTargets, () =>
        set({ requiresTargetReload: true }),
      );
      return target;
    } catch (err) {
      set({ error: String(err) });
      throw err;
    } finally {
      set({ updatingTargetId: null });
    }
  },

  testSshTarget: async (request) => {
    const testingTargetId = request.id ?? "new";
    set({ testingTargetId, error: null });
    try {
      return await invoke("test_ssh_target", { request });
    } catch (err) {
      set({ error: String(err) });
      throw err;
    } finally {
      set({ testingTargetId: null });
    }
  },

  createWslTarget: async (request) => {
    set({ isCreating: true, error: null });
    try {
      const target = await invoke("create_wsl_target", {
        request,
      });
      set((state) => ({
        targets: [
          ...state.targets.filter((item) => item.id !== target.id),
          target,
        ],
        activeTarget: resolveActiveTarget([
          ...state.targets.filter((item) => item.id !== target.id),
          target,
        ]),
        isCreating: false,
      }));
      await refreshTargetsAfterMutation(get().loadTargets, () =>
        set({ requiresTargetReload: true }),
      );
      return target;
    } catch (err) {
      set({ error: String(err), isCreating: false });
      throw err;
    }
  },

  updateWslTarget: async (request) => {
    set({ updatingTargetId: request.id, error: null });
    try {
      const target = await invoke("update_wsl_target", {
        request,
      });
      await refreshTargetsAfterMutation(get().loadTargets, () =>
        set({ requiresTargetReload: true }),
      );
      return target;
    } catch (err) {
      set({ error: String(err) });
      throw err;
    } finally {
      set({ updatingTargetId: null });
    }
  },

  testWslTarget: async (request) => {
    const testingTargetId = request.id ?? "new-wsl";
    set({ testingTargetId, error: null });
    try {
      return await invoke("test_wsl_target", { request });
    } catch (err) {
      set({ error: String(err) });
      throw err;
    } finally {
      set({ testingTargetId: null });
    }
  },

  updateSshTargetPassword: async (targetId, password) => {
    set({ updatingPasswordTargetId: targetId, error: null });
    try {
      const result = await invoke("update_ssh_target_password", {
        targetId,
        password,
      });
      if (result.ok) {
        await refreshTargetsAfterMutation(get().loadTargets, () =>
          set({ requiresTargetReload: true }),
        );
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
    set({ deletingTargetId: targetId, error: null });
    try {
      await invoke("delete_target", { targetId });
      await refreshTargetsAfterMutation(get().loadTargets, () =>
        set({ requiresTargetReload: true }),
      );
    } catch (err) {
      set({ error: String(err) });
      throw err;
    } finally {
      set({ deletingTargetId: null });
    }
  },

  switchTarget: async (targetId) => {
    set({ switchingTargetId: targetId, error: null });
    try {
      const activeTarget = await invoke("set_active_target", {
        targetId,
      });
      set((state) => ({
        targets: markActive(state.targets, activeTarget.id),
        activeTarget,
      }));
      await refreshTargetsAfterMutation(get().loadTargets, () =>
        set({ requiresTargetReload: true }),
      );
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
