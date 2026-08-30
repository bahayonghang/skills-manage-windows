import { create } from "zustand";
import { invoke } from "@/lib/ipc";
import { backendErrorStateValue } from "@/lib/backendError";
import {
  applySkillDocResponse,
  type SkillsCliDocState,
} from "@/pages/skillsCliDetailModel";
import {
  EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
  type SkillsCliGlobalSkill,
  type SkillsCliInstallTarget,
  type SkillsCliSourcePreview,
  type SkillsCliDoctorReport,
  type SkillsCliUpdateProgress,
} from "@/types";
import {
  createSkillsCliUpdateSlice,
  EMPTY_UPDATE_JOB,
  newJobId,
  skillsCliOperationBusy,
  BUSY_ENVELOPE,
  SELECTION_EMPTY_ENVELOPE,
} from "./skillsCliStore.updateSlice";
import {
  createSkillsCliPlacementSlice,
  errorCodeFrom,
} from "./skillsCliStore.placementSlice";
import type {
  SkillsCliState,
} from "./skillsCliStore.types";

export type { SkillsCliDocState };
export type { PlacementMutationOutcome } from "@/pages/skillsCliBatchModel";
export type {
  SkillsCliAddInput,
  SkillsCliBatchOperation,
  SkillsCliBatchProgress,
  SkillsCliExportInventoryInput,
  SkillsCliUpdateJob,
} from "./skillsCliStore.types";

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
  docState: { status: "idle" } as SkillsCliDocState,
  updateInventory: EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
  isLoadingUpdateCache: false,
  updateJob: EMPTY_UPDATE_JOB,
  updateError: null as string | null,
  updateProgress: null as SkillsCliUpdateProgress | null,
  batchProgress: null as SkillsCliState["batchProgress"],
};

export const useSkillsCliStore = create<SkillsCliState>((set, get) => ({
  ...emptyState,
  ...createSkillsCliUpdateSlice(set, get),
  ...createSkillsCliPlacementSlice(set, get),

  async loadAll() {
    // Inventory and runtime health settle on independent tracks: a doctor
    // failure must never discard a successfully read inventory, and a failed
    // inventory refresh must keep the stale list visible.
    const firstLoad = get().skills.length === 0;
    set({
      ...(firstLoad ? { isLoading: true } : { isRefreshing: true }),
      runtimeError: null,
      inventoryError: null,
      isLoadingUpdateCache: true,
    });
    const [inventory, runtime, updateCache] = await Promise.allSettled([
      Promise.all([
        invoke("skills_cli_list_global"),
        invoke("skills_cli_install_targets"),
      ]),
      invoke("skills_cli_doctor"),
      invoke("skills_cli_update_inventory"),
    ]);
    const patch: Partial<SkillsCliState> = {
      isLoading: false,
      isRefreshing: false,
      isLoadingUpdateCache: false,
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
    if (updateCache.status === "fulfilled") {
      patch.updateInventory =
        updateCache.value ?? EMPTY_SKILLS_CLI_UPDATE_INVENTORY;
      patch.updateError = null;
    } else if (errorCodeFrom(updateCache.reason) === "skills_cli.local_target_only") {
      patch.updateInventory = EMPTY_SKILLS_CLI_UPDATE_INVENTORY;
      patch.updateError = null;
    } else {
      patch.updateError = backendErrorStateValue(updateCache.reason);
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
      set({ actionError: SELECTION_EMPTY_ENVELOPE });
      throw new Error(SELECTION_EMPTY_ENVELOPE);
    }
    if (skillsCliOperationBusy(get())) {
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
      return result;
    } catch (error) {
      if (get().jobId === jobId) {
        set({
          actionError: backendErrorStateValue(error),
          isMutating: false,
          jobId: null,
        });
      }
      throw error;
    }
  },

  async removeGlobal(skillName) {
    const outcome = await get().removeGlobalBatch([skillName]);
    return (
      outcome.failed.length === 0 &&
      outcome.succeeded.some((item) => item.skillName === skillName)
    );
  },

  async previewRemoveGlobal(skillName) {
    set({ actionError: null });
    try {
      return await invoke("skills_cli_preview_remove_global", { skillName });
    } catch (error) {
      set({ actionError: backendErrorStateValue(error) });
      return null;
    }
  },

  async readSkillMd(skillName) {
    set({ actionError: null });
    try {
      return await invoke("skills_cli_read_skill_md", { skillName });
    } catch (error) {
      set({ actionError: backendErrorStateValue(error) });
      return null;
    }
  },

  async readSkillDoc(skillName) {
    const requestId = newJobId();
    set({
      docState: { status: "loading", skillName, requestId },
    });
    try {
      const doc = await invoke("skills_cli_read_skill_md", { skillName });
      const next = applySkillDocResponse(get().docState, requestId, skillName, {
        ok: true,
        content: doc?.content ?? "",
        byteSize: doc?.byteSize ?? 0,
      });
      set({ docState: next });
    } catch (error) {
      const next = applySkillDocResponse(get().docState, requestId, skillName, {
        ok: false,
        errorCode: errorCodeFrom(error),
      });
      set({ docState: next });
    }
  },

  clearSkillDoc(skillName) {
    const current = get().docState;
    if (
      skillName &&
      current.status !== "idle" &&
      current.skillName !== skillName
    ) {
      return;
    }
    set({ docState: { status: "idle" } });
  },

  async revealSkillFolder(skillName) {
    await invoke("skills_cli_reveal_skill_folder", { skillName });
  },

  async exportInventory({ path, json }) {
    set({ actionError: null });
    try {
      await invoke("skills_cli_export_inventory", { path, json });
    } catch (error) {
      set({ actionError: backendErrorStateValue(error) });
      throw error;
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
