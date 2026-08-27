import { create } from "zustand";
import { invoke } from "@/lib/ipc";
import { backendErrorStateValue, parseBackendError } from "@/lib/backendError";
import {
  emptyPlacementOutcome,
  partitionLinkBatch,
  partitionUnlinkBatch,
  partitionUnlinkBatchForAgent,
  selectedSkillsInStoreOrder,
  SKILLS_CLI_SKIP_NO_PLACEMENT,
  type PlacementMutationOutcome,
  type PlacementPartitionItem,
} from "@/pages/skillsCliBatchModel";
import {
  applySelectionsForNames,
  groupSkillNamesByRepositoryKey,
} from "@/pages/skillsCliViewModel";
import {
  applySkillDocResponse,
  type SkillsCliDocState,
} from "@/pages/skillsCliDetailModel";
import {
  EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
  type SkillsCliGlobalSkill,
  type SkillsCliInstallTarget,
  type SkillsCliPlacement,
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
import type {
  SkillsCliBatchOperation,
  SkillsCliState,
} from "./skillsCliStore.types";

export type { SkillsCliDocState };
export type { PlacementMutationOutcome };
export type {
  SkillsCliAddInput,
  SkillsCliBatchOperation,
  SkillsCliBatchProgress,
  SkillsCliExportInventoryInput,
  SkillsCliUpdateJob,
} from "./skillsCliStore.types";

function errorCodeFrom(error: unknown): string {
  return parseBackendError(error).code ?? "internal.unexpected";
}

function beginBatchProgress(
  set: (patch: Partial<SkillsCliState>) => void,
  operation: SkillsCliBatchOperation,
  total: number,
): void {
  set({
    batchProgress: { operation, completed: 0, total },
  });
}

function incrementBatchProgress(
  get: () => SkillsCliState,
  set: (patch: Partial<SkillsCliState>) => void,
): void {
  const current = get().batchProgress;
  if (!current) {
    return;
  }
  set({
    batchProgress: {
      ...current,
      completed: current.completed + 1,
    },
  });
}

function clearBatchProgress(
  set: (patch: Partial<SkillsCliState>) => void,
): void {
  set({ batchProgress: null });
}

function batchAlreadyRunning(state: SkillsCliState): boolean {
  return state.batchProgress !== null;
}

function replacePlacement(
  skills: SkillsCliGlobalSkill[],
  skillName: string,
  agentId: string,
  next: SkillsCliPlacement,
): SkillsCliGlobalSkill[] {
  return skills.map((skill) => {
    if (skill.name !== skillName) {
      return skill;
    }
    return {
      ...skill,
      placements: skill.placements.map((placement) =>
        placement.agentId === agentId ? next : placement,
      ),
    };
  });
}

function omitSkill(
  skills: SkillsCliGlobalSkill[],
  skillName: string,
): SkillsCliGlobalSkill[] {
  return skills.filter((skill) => skill.name !== skillName);
}

function restoreSkill(
  skills: SkillsCliGlobalSkill[],
  snapshot: SkillsCliGlobalSkill,
): SkillsCliGlobalSkill[] {
  if (skills.some((skill) => skill.name === snapshot.name)) {
    return skills.map((skill) =>
      skill.name === snapshot.name ? snapshot : skill,
    );
  }
  return [...skills, snapshot];
}

function throwIfSingleOutcomeFailed(outcome: PlacementMutationOutcome): void {
  const failed = outcome.failed[0];
  if (failed) {
    throw new Error(`${failed.errorCode}:`);
  }
  const skipped = outcome.skipped[0];
  if (skipped && outcome.succeeded.length === 0) {
    throw new Error(`${skipped.reasonCode}:`);
  }
}

function optimisticPlacement(
  previous: SkillsCliPlacement,
  kind: "link" | "unlink",
): SkillsCliPlacement {
  if (kind === "link") {
    return {
      ...previous,
      state: "managed_link",
      managedLinkKind: previous.managedLinkKind ?? "windows_junction",
      reasonCode: null,
    };
  }
  return {
    ...previous,
    state: "missing",
    managedLinkKind: null,
    reasonCode: null,
  };
}

async function runPlacementBatch(
  get: () => SkillsCliState,
  set: (patch: Partial<SkillsCliState>) => void,
  items: PlacementPartitionItem[],
  kind: "link" | "unlink",
): Promise<PlacementMutationOutcome> {
  const outcome = emptyPlacementOutcome();
  if (items.length === 0) {
    return outcome;
  }
  const operation: SkillsCliBatchOperation = kind === "link" ? "link" : "unlink";
  set({ isMutating: true, actionError: null });
  beginBatchProgress(set, operation, items.length);
  try {
    for (const item of items) {
      const jobId = newJobId();
      set({ jobId });
      const previous =
        get().skills
          .find((skill) => skill.name === item.skillName)
          ?.placements.find((placement) => placement.agentId === item.agentId) ??
        item.placement;
      set({
        skills: replacePlacement(
          get().skills,
          item.skillName,
          item.agentId,
          optimisticPlacement(previous, kind),
        ),
      });
      try {
        const result =
          kind === "link"
            ? await invoke("skills_cli_link_platform", {
                jobId,
                skillName: item.skillName,
                skillportAgentId: item.agentId,
              })
            : await invoke("skills_cli_unlink_platform", {
                jobId,
                skillName: item.skillName,
                skillportAgentId: item.agentId,
              });
        if (get().jobId === jobId && result) {
          set({
            skills: replacePlacement(
              get().skills,
              item.skillName,
              item.agentId,
              result,
            ),
          });
        }
        outcome.succeeded.push({
          skillName: item.skillName,
          agentId: item.agentId,
        });
      } catch (error) {
        if (get().jobId === jobId) {
          set({
            skills: replacePlacement(
              get().skills,
              item.skillName,
              item.agentId,
              previous,
            ),
            actionError: backendErrorStateValue(error),
          });
        }
        outcome.failed.push({
          skillName: item.skillName,
          agentId: item.agentId,
          errorCode: errorCodeFrom(error),
        });
      }
      incrementBatchProgress(get, set);
    }
  } finally {
    clearBatchProgress(set);
    if (get().isMutating) {
      set({ isMutating: false, jobId: null });
    }
    await refreshInventoryAfterMutation(get, set);
  }
  return outcome;
}

async function refreshInventoryAfterMutation(
  get: () => SkillsCliState,
  set: (patch: Partial<SkillsCliState>) => void,
): Promise<void> {
  try {
    await get().loadAll();
  } catch (error) {
    set({
      isLoading: false,
      isRefreshing: false,
      inventoryError: get().inventoryError ?? backendErrorStateValue(error),
    });
  }
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

  async linkPlatform(skillName, agentId) {
    const outcome = await get().linkPlatformBatch([skillName], agentId);
    throwIfSingleOutcomeFailed(outcome);
  },

  async unlinkPlatform(skillName, agentId) {
    if (batchAlreadyRunning(get()) || skillsCliOperationBusy(get())) {
      throw new Error(BUSY_ENVELOPE);
    }
    const partition = partitionUnlinkBatch(get().skills, [skillName]);
    const match = partition.allowed.filter((item) => item.agentId === agentId);
    const skipped = partition.skipped.filter((item) => item.agentId === agentId);
    const outcome = emptyPlacementOutcome();
    outcome.skipped.push(...skipped);
    if (match.length === 0) {
      if (skipped.length === 0) {
        outcome.skipped.push({
          skillName,
          agentId,
          reasonCode: SKILLS_CLI_SKIP_NO_PLACEMENT,
        });
      }
      throwIfSingleOutcomeFailed(outcome);
      return;
    }
    const batchOutcome = await runPlacementBatch(
      get,
      set,
      match,
      "unlink",
    );
    outcome.succeeded.push(...batchOutcome.succeeded);
    outcome.failed.push(...batchOutcome.failed);
    throwIfSingleOutcomeFailed(outcome);
  },

  async linkPlatformBatch(skillNames, agentId) {
    if (batchAlreadyRunning(get()) || skillsCliOperationBusy(get())) {
      throw new Error(BUSY_ENVELOPE);
    }
    const partition = partitionLinkBatch(get().skills, skillNames, agentId);
    const outcome = emptyPlacementOutcome();
    outcome.skipped.push(...partition.skipped);
    if (partition.allowed.length === 0) {
      return outcome;
    }
    const ran = await runPlacementBatch(get, set, partition.allowed, "link");
    outcome.succeeded.push(...ran.succeeded);
    outcome.failed.push(...ran.failed);
    return outcome;
  },

  async unlinkManagedBatch(skillNames) {
    if (batchAlreadyRunning(get()) || skillsCliOperationBusy(get())) {
      throw new Error(BUSY_ENVELOPE);
    }
    const partition = partitionUnlinkBatch(get().skills, skillNames);
    const outcome = emptyPlacementOutcome();
    outcome.skipped.push(...partition.skipped);
    if (partition.allowed.length === 0) {
      return outcome;
    }
    const ran = await runPlacementBatch(get, set, partition.allowed, "unlink");
    outcome.succeeded.push(...ran.succeeded);
    outcome.failed.push(...ran.failed);
    return outcome;
  },

  async unlinkPlatformBatch(skillNames, agentId) {
    if (batchAlreadyRunning(get()) || skillsCliOperationBusy(get())) {
      throw new Error(BUSY_ENVELOPE);
    }
    const partition = partitionUnlinkBatchForAgent(
      get().skills,
      skillNames,
      agentId,
    );
    const outcome = emptyPlacementOutcome();
    outcome.skipped.push(...partition.skipped);
    if (partition.allowed.length === 0) {
      return outcome;
    }
    const ran = await runPlacementBatch(get, set, partition.allowed, "unlink");
    outcome.succeeded.push(...ran.succeeded);
    outcome.failed.push(...ran.failed);
    return outcome;
  },

  async removeGlobalBatch(skillNames) {
    if (batchAlreadyRunning(get()) || skillsCliOperationBusy(get())) {
      throw new Error(BUSY_ENVELOPE);
    }
    const outcome = emptyPlacementOutcome();
    if (skillNames.length === 0) {
      return outcome;
    }
    set({ isMutating: true, actionError: null });
    beginBatchProgress(set, "cleanup", skillNames.length);
    try {
      for (const skillName of skillNames) {
        const jobId = newJobId();
        set({ jobId });
        const snapshot = get().skills.find((skill) => skill.name === skillName);
        if (snapshot) {
          set({ skills: omitSkill(get().skills, skillName) });
        }
        try {
          await invoke("skills_cli_remove_global", { jobId, skillName });
          outcome.succeeded.push({ skillName });
        } catch (error) {
          if (get().jobId === jobId && snapshot) {
            set({ skills: restoreSkill(get().skills, snapshot) });
          }
          if (get().jobId === jobId) {
            set({ actionError: backendErrorStateValue(error) });
          }
          outcome.failed.push({
            skillName,
            errorCode: errorCodeFrom(error),
          });
        }
        incrementBatchProgress(get, set);
      }
    } finally {
      clearBatchProgress(set);
      if (get().isMutating) {
        set({ isMutating: false, jobId: null });
      }
      await refreshInventoryAfterMutation(get, set);
    }
    return outcome;
  },

  async applyUpdatesBatch(skillNames) {
    if (batchAlreadyRunning(get())) {
      return emptyPlacementOutcome();
    }
    if (skillsCliOperationBusy(get())) {
      throw new Error(BUSY_ENVELOPE);
    }
    const outcome = emptyPlacementOutcome();
    const ordered = selectedSkillsInStoreOrder(
      get().skills,
      new Set(skillNames),
    ).map((skill) => skill.name);
    const inventory = get().updateInventory;
    const groups = groupSkillNamesByRepositoryKey(ordered, inventory)
      .map((group) => {
        const skillNames = group.skillNames.filter(
          (name) => applySelectionsForNames(inventory, [name]).length > 0,
        );
        return {
          repositoryKey: group.repositoryKey,
          skillNames,
          selections: applySelectionsForNames(inventory, skillNames),
        };
      })
      .filter((group) => group.selections.length > 0);
    if (groups.length === 0) {
      return outcome;
    }
    beginBatchProgress(set, "update", groups.length);
    try {
      for (const group of groups) {
        try {
          const result = await get().applyUpdates({
            repositoryKey: group.repositoryKey,
            skillNames: group.skillNames,
            selections: group.selections,
          });
          for (const skillName of result.appliedSkillNames) {
            outcome.succeeded.push({ skillName });
          }
        } catch (error) {
          const errorCode = errorCodeFrom(error);
          for (const skillName of group.skillNames) {
            outcome.failed.push({ skillName, errorCode });
          }
        }
        incrementBatchProgress(get, set);
      }
    } finally {
      clearBatchProgress(set);
    }
    return outcome;
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
