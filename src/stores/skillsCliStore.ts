import { create } from "zustand";
import { invoke } from "@/lib/ipc";
import { backendErrorStateValue } from "@/lib/backendError";
import type {
  SkillsCliAddResult,
  SkillsCliDoctorReport,
  SkillsCliGlobalSkill,
  SkillsCliInstallTarget,
  SkillsCliSourcePreview,
} from "@/types";

const BUSY_ENVELOPE =
  "skills_cli.busy:Another skill operation is using this target.";

function newJobId(): string {
  return (
    globalThis.crypto?.randomUUID?.() ??
    `job-${Date.now()}-${Math.random().toString(16).slice(2)}`
  );
}

interface SkillsCliState {
  skills: SkillsCliGlobalSkill[];
  targets: SkillsCliInstallTarget[];
  preview: SkillsCliSourcePreview | null;
  doctor: SkillsCliDoctorReport | null;
  canonicalRoot: string | null;
  lockPath: string | null;
  isLoading: boolean;
  isRefreshing: boolean;
  isPreviewing: boolean;
  isMutating: boolean;
  isCancelling: boolean;
  jobId: string | null;
  /** Doctor rejection: write paths (install/uninstall) are degraded. */
  runtimeError: string | null;
  /** list_global / install_targets read failure: stale inventory is kept. */
  inventoryError: string | null;
  /** preview/add/remove failure: toast + inline in the install section. */
  actionError: string | null;

  loadAll: () => Promise<void>;
  previewSource: (source: string) => Promise<SkillsCliSourcePreview | null>;
  addGlobal: (input: {
    source: string;
    skillNames: string[];
    skillportAgentIds: string[];
  }) => Promise<SkillsCliAddResult | null>;
  removeGlobal: (skillName: string) => Promise<boolean>;
  cancelJob: () => Promise<void>;
  resetForTargetChange: () => void;
}

const emptyState = {
  skills: [] as SkillsCliGlobalSkill[],
  targets: [] as SkillsCliInstallTarget[],
  preview: null as SkillsCliSourcePreview | null,
  doctor: null as SkillsCliDoctorReport | null,
  canonicalRoot: null as string | null,
  lockPath: null as string | null,
  isLoading: false,
  isRefreshing: false,
  isPreviewing: false,
  isMutating: false,
  isCancelling: false,
  jobId: null as string | null,
  runtimeError: null as string | null,
  inventoryError: null as string | null,
  actionError: null as string | null,
};

export const useSkillsCliStore = create<SkillsCliState>((set, get) => ({
  ...emptyState,

  async loadAll() {
    // Inventory and runtime health settle on independent tracks: a doctor
    // failure must never discard a successfully read inventory, and a failed
    // inventory refresh must keep the stale list visible.
    const firstLoad = get().skills.length === 0;
    set({
      ...(firstLoad ? { isLoading: true } : { isRefreshing: true }),
      runtimeError: null,
      inventoryError: null,
    });
    const [inventory, runtime] = await Promise.allSettled([
      Promise.all([
        invoke("skills_cli_list_global"),
        invoke("skills_cli_install_targets"),
      ]),
      invoke("skills_cli_doctor"),
    ]);
    const patch: Partial<SkillsCliState> = {
      isLoading: false,
      isRefreshing: false,
    };
    if (inventory.status === "fulfilled") {
      const [snapshot, targets] = inventory.value;
      patch.skills = snapshot?.skills ?? [];
      patch.targets = targets ?? [];
      patch.canonicalRoot = snapshot?.canonicalRoot ?? null;
      patch.lockPath = snapshot?.lockPath ?? null;
    } else {
      patch.inventoryError = backendErrorStateValue(inventory.reason);
    }
    if (runtime.status === "fulfilled") {
      patch.doctor = runtime.value;
    } else {
      patch.doctor = null;
      patch.runtimeError = backendErrorStateValue(runtime.reason);
    }
    set(patch);
  },

  async previewSource(source) {
    set({ isPreviewing: true, actionError: null, preview: null });
    try {
      const preview = await invoke("skills_cli_preview_source", { source });
      set({ preview, isPreviewing: false });
      return preview;
    } catch (error) {
      set({
        actionError: backendErrorStateValue(error),
        isPreviewing: false,
        preview: null,
      });
      return null;
    }
  },

  async addGlobal(input) {
    if (input.skillNames.length === 0 || input.skillportAgentIds.length === 0) {
      set({ actionError: "skills_cli.selection_empty:Select at least one skill and one platform." });
      return null;
    }
    if (get().isMutating || get().isCancelling) {
      throw new Error(BUSY_ENVELOPE);
    }
    const jobId = newJobId();
    set({ isMutating: true, actionError: null, jobId });
    try {
      const result = await invoke("skills_cli_add_global", {
        jobId,
        source: input.source,
        skillNames: input.skillNames,
        skillportAgentIds: input.skillportAgentIds,
      });
      if (get().jobId !== jobId) {
        return result;
      }
      set({ isMutating: false, jobId: null, preview: null });
      await get().loadAll();
      return result;
    } catch (error) {
      if (get().jobId !== jobId) {
        return null;
      }
      set({
        actionError: backendErrorStateValue(error),
        isMutating: false,
        jobId: null,
      });
      return null;
    }
  },

  async removeGlobal(skillName) {
    if (get().isMutating || get().isCancelling) {
      throw new Error(BUSY_ENVELOPE);
    }
    const jobId = newJobId();
    set({ isMutating: true, actionError: null, jobId });
    try {
      await invoke("skills_cli_remove_global", { jobId, skillName });
      if (get().jobId !== jobId) {
        return true;
      }
      set({ isMutating: false, jobId: null });
      await get().loadAll();
      return true;
    } catch (error) {
      if (get().jobId !== jobId) {
        return false;
      }
      set({
        actionError: backendErrorStateValue(error),
        isMutating: false,
        jobId: null,
      });
      return false;
    }
  },

  async cancelJob() {
    const jobId = get().jobId;
    if (!jobId) {
      return;
    }
    set({ isCancelling: true });
    try {
      await invoke("cancel_skills_cli_job", { jobId });
    } finally {
      if (get().jobId === jobId) {
        set({ isCancelling: false });
      }
    }
  },

  resetForTargetChange() {
    set({ ...emptyState });
  },
}));
