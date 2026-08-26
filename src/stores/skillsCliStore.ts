import { create } from "zustand";
import { invoke, listen } from "@/lib/ipc";
import { backendErrorStateValue, parseBackendError } from "@/lib/backendError";
import {
  emptyPlacementOutcome,
  partitionLinkBatch,
  partitionUnlinkBatch,
  SKILLS_CLI_SKIP_NO_PLACEMENT,
  type PlacementMutationOutcome,
  type PlacementPartitionItem,
} from "@/pages/skillsCliBatchModel";
import {
  applySkillDocResponse,
  type SkillsCliDocState,
} from "@/pages/skillsCliDetailModel";
import { applySelectionsForNames } from "@/pages/skillsCliViewModel";
import {
  EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
  type SkillsCliAddResult,
  type SkillsCliApplyRecoveryResult,
  type SkillsCliApplyResult,
  type SkillsCliDoctorReport,
  type SkillsCliGlobalSkill,
  type SkillsCliInstallTarget,
  type SkillsCliPlacement,
  type SkillsCliRemovePlan,
  type SkillsCliSkillDoc,
  type SkillsCliSourcePreview,
  type SkillsCliUpdateInventory,
  type SkillsCliUpdateJobPhase,
  type SkillsCliUpdateProgress,
} from "@/types";

export type { SkillsCliDocState };

export type { PlacementMutationOutcome };

export interface SkillsCliExportInventoryInput {
  path: string;
  json: string;
}

export interface SkillsCliAddInput {
  source: string;
  skillNames: string[];
  skillportAgentIds: string[];
}

const BUSY_ENVELOPE =
  "skills_cli.busy:Another skill operation is using this target.";
const SELECTION_EMPTY_ENVELOPE =
  "skills_cli.selection_empty:Select at least one skill and one platform.";
const UPDATE_PROGRESS_EVENT = "skills-cli://update-progress";

export type SkillsCliUpdateJob = {
  jobId: string | null;
  phase: SkillsCliUpdateJobPhase;
};

const EMPTY_UPDATE_JOB: SkillsCliUpdateJob = { jobId: null, phase: null };

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
  docState: SkillsCliDocState;
  updateInventory: SkillsCliUpdateInventory;
  isLoadingUpdateCache: boolean;
  updateJob: SkillsCliUpdateJob;
  updateError: string | null;
  updateProgress: SkillsCliUpdateProgress | null;

  loadAll: () => Promise<void>;
  loadUpdateInventory: () => Promise<void>;
  checkUpdates: () => Promise<SkillsCliUpdateInventory>;
  verifyUpdateBaseline: (skillNames: string[]) => Promise<SkillsCliUpdateInventory>;
  applyUpdates: (input: {
    repositoryKey: string;
    skillNames: string[];
  }) => Promise<SkillsCliApplyResult>;
  retryUpdateRecovery: (
    operationId: string,
  ) => Promise<SkillsCliApplyRecoveryResult>;
  cancelUpdateJob: () => Promise<void>;
  previewSource: (source: string) => Promise<SkillsCliSourcePreview | null>;
  addGlobal: (input: SkillsCliAddInput) => Promise<SkillsCliAddResult>;
  removeGlobal: (skillName: string) => Promise<boolean>;
  previewRemoveGlobal: (skillName: string) => Promise<SkillsCliRemovePlan | null>;
  readSkillMd: (skillName: string) => Promise<SkillsCliSkillDoc | null>;
  readSkillDoc: (skillName: string) => Promise<void>;
  clearSkillDoc: (skillName?: string) => void;
  revealSkillFolder: (skillName: string) => Promise<void>;
  linkPlatform: (skillName: string, agentId: string) => Promise<void>;
  unlinkPlatform: (skillName: string, agentId: string) => Promise<void>;
  linkPlatformBatch: (
    skillNames: string[],
    agentId: string,
  ) => Promise<PlacementMutationOutcome>;
  unlinkManagedBatch: (skillNames: string[]) => Promise<PlacementMutationOutcome>;
  removeGlobalBatch: (skillNames: string[]) => Promise<PlacementMutationOutcome>;
  exportInventory: (input: SkillsCliExportInventoryInput) => Promise<void>;
  cancelJob: () => Promise<void>;
  resetForTargetChange: () => void;
}

function skillsCliOperationBusy(state: SkillsCliState): boolean {
  return (
    state.isMutating ||
    state.isCancelling ||
    state.updateJob.phase != null
  );
}

async function listenForUpdateProgress(
  get: () => SkillsCliState,
  set: (patch: Partial<SkillsCliState>) => void,
  jobId: string,
): Promise<() => void> {
  try {
    return await listen<SkillsCliUpdateProgress>(
      UPDATE_PROGRESS_EVENT,
      (event) => {
        if (event.payload.jobId !== jobId || get().updateJob.jobId !== jobId) {
          return;
        }
        set({ updateProgress: event.payload });
      },
    );
  } catch {
    return () => undefined;
  }
}

function errorCodeFrom(error: unknown): string {
  return parseBackendError(error).code ?? "internal.unexpected";
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
  set({ isMutating: true, actionError: null });
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
    }
  } finally {
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

  async loadUpdateInventory() {
    set({ isLoadingUpdateCache: true });
    try {
      const inventory = await invoke("skills_cli_update_inventory");
      set({
        updateInventory: inventory ?? EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
        isLoadingUpdateCache: false,
        updateError: null,
      });
    } catch (error) {
      set({
        isLoadingUpdateCache: false,
        updateError: backendErrorStateValue(error),
      });
    }
  },

  async checkUpdates() {
    if (skillsCliOperationBusy(get())) {
      throw new Error(BUSY_ENVELOPE);
    }
    const jobId = newJobId();
    set({
      updateJob: { jobId, phase: "checking" },
      updateError: null,
      updateProgress: null,
    });
    let unlisten: (() => void) | undefined;
    try {
      unlisten = await listenForUpdateProgress(get, set, jobId);
      const inventory = await invoke("skills_cli_check_updates", { jobId });
      if (get().updateJob.jobId !== jobId) {
        return inventory;
      }
      set({
        updateInventory: inventory,
        updateJob: EMPTY_UPDATE_JOB,
        updateProgress: null,
        updateError: null,
      });
      return inventory;
    } catch (error) {
      if (get().updateJob.jobId === jobId) {
        set({
          updateError: backendErrorStateValue(error),
          updateJob: EMPTY_UPDATE_JOB,
        });
      }
      throw error;
    } finally {
      try {
        unlisten?.();
      } catch {
        // Browser and test runtimes expose a no-op unlisten.
      }
    }
  },

  async verifyUpdateBaseline(skillNames) {
    if (skillsCliOperationBusy(get())) {
      throw new Error(BUSY_ENVELOPE);
    }
    const jobId = newJobId();
    set({
      updateJob: { jobId, phase: "verifying" },
      updateError: null,
      updateProgress: null,
    });
    let unlisten: (() => void) | undefined;
    try {
      unlisten = await listenForUpdateProgress(get, set, jobId);
      const inventory = await invoke("skills_cli_verify_update_baseline", {
        jobId,
        skillNames,
      });
      if (get().updateJob.jobId !== jobId) {
        return inventory;
      }
      set({
        updateInventory: inventory,
        updateJob: EMPTY_UPDATE_JOB,
        updateProgress: null,
        updateError: null,
      });
      return inventory;
    } catch (error) {
      if (get().updateJob.jobId === jobId) {
        set({
          updateError: backendErrorStateValue(error),
          updateJob: EMPTY_UPDATE_JOB,
        });
      }
      throw error;
    } finally {
      try {
        unlisten?.();
      } catch {
        // Browser and test runtimes expose a no-op unlisten.
      }
    }
  },

  async applyUpdates(input) {
    if (skillsCliOperationBusy(get())) {
      throw new Error(BUSY_ENVELOPE);
    }
    const selections = applySelectionsForNames(
      get().updateInventory,
      input.skillNames,
    );
    if (selections.length === 0) {
      set({ updateError: SELECTION_EMPTY_ENVELOPE });
      throw new Error(SELECTION_EMPTY_ENVELOPE);
    }
    const jobId = newJobId();
    set({
      updateJob: { jobId, phase: "applying" },
      updateError: null,
      updateProgress: null,
    });
    let unlisten: (() => void) | undefined;
    try {
      unlisten = await listenForUpdateProgress(get, set, jobId);
      const result = await invoke("skills_cli_apply_updates", {
        request: {
          jobId,
          repositoryKey: input.repositoryKey,
          selections,
        },
      });
      if (get().updateJob.jobId !== jobId) {
        return result;
      }
      try {
        await get().loadAll();
      } catch (refreshError) {
        if (get().updateJob.jobId === jobId) {
          set({
            updateError: backendErrorStateValue(refreshError),
            updateJob: EMPTY_UPDATE_JOB,
            updateProgress: null,
          });
        }
        return result;
      }
      if (get().updateJob.jobId === jobId) {
        set({ updateJob: EMPTY_UPDATE_JOB, updateProgress: null });
      }
      return result;
    } catch (error) {
      if (get().updateJob.jobId === jobId) {
        set({
          updateError: backendErrorStateValue(error),
          updateJob: EMPTY_UPDATE_JOB,
        });
      }
      throw error;
    } finally {
      try {
        unlisten?.();
      } catch {
        // Browser and test runtimes expose a no-op unlisten.
      }
    }
  },

  async retryUpdateRecovery(operationId) {
    if (skillsCliOperationBusy(get())) {
      throw new Error(BUSY_ENVELOPE);
    }
    const jobId = newJobId();
    set({
      updateJob: { jobId, phase: "recovering" },
      updateError: null,
      updateProgress: null,
    });
    let unlisten: (() => void) | undefined;
    try {
      unlisten = await listenForUpdateProgress(get, set, jobId);
      const result = await invoke("skills_cli_retry_update_recovery", {
        jobId,
        operationId,
      });
      if (get().updateJob.jobId !== jobId) {
        return result;
      }
      try {
        await get().loadAll();
      } catch (refreshError) {
        if (get().updateJob.jobId === jobId) {
          set({
            updateError: backendErrorStateValue(refreshError),
            updateJob: EMPTY_UPDATE_JOB,
            updateProgress: null,
          });
        }
        return result;
      }
      if (get().updateJob.jobId === jobId) {
        set({ updateJob: EMPTY_UPDATE_JOB, updateProgress: null });
      }
      return result;
    } catch (error) {
      if (get().updateJob.jobId === jobId) {
        set({
          updateError: backendErrorStateValue(error),
          updateJob: EMPTY_UPDATE_JOB,
        });
      }
      throw error;
    } finally {
      try {
        unlisten?.();
      } catch {
        // Browser and test runtimes expose a no-op unlisten.
      }
    }
  },

  async cancelUpdateJob() {
    const jobId = get().updateJob.jobId;
    if (!jobId) {
      return;
    }
    try {
      await invoke("cancel_skills_cli_job", { jobId });
    } finally {
      if (get().updateJob.jobId === jobId) {
        set({ isCancelling: false });
      }
    }
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
    if (skillsCliOperationBusy(get())) {
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
    if (skillsCliOperationBusy(get())) {
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
    if (skillsCliOperationBusy(get())) {
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

  async removeGlobalBatch(skillNames) {
    if (skillsCliOperationBusy(get())) {
      throw new Error(BUSY_ENVELOPE);
    }
    const outcome = emptyPlacementOutcome();
    if (skillNames.length === 0) {
      return outcome;
    }
    set({ isMutating: true, actionError: null });
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
      }
    } finally {
      if (get().isMutating) {
        set({ isMutating: false, jobId: null });
      }
      await refreshInventoryAfterMutation(get, set);
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
