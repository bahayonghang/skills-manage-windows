import { invoke, listen } from "@/lib/ipc";
import { backendErrorStateValue } from "@/lib/backendError";
import { applySelectionsForNames } from "@/pages/skillsCliViewModel";
import {
  EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
  type SkillsCliUpdateProgress,
} from "@/types";
import type { SkillsCliState, SkillsCliUpdateJob } from "./skillsCliStore.types";

export const BUSY_ENVELOPE =
  "skills_cli.busy:Another skill operation is using this target.";
export const SELECTION_EMPTY_ENVELOPE =
  "skills_cli.selection_empty:Select at least one skill and one platform.";
const UPDATE_PROGRESS_EVENT = "skills-cli://update-progress";

export const EMPTY_UPDATE_JOB: SkillsCliUpdateJob = { jobId: null, phase: null };

export function newJobId(): string {
  return (
    globalThis.crypto?.randomUUID?.() ??
    `job-${Date.now()}-${Math.random().toString(16).slice(2)}`
  );
}

export function skillsCliOperationBusy(state: SkillsCliState): boolean {
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

export function createSkillsCliUpdateSlice(
  set: (patch: Partial<SkillsCliState>) => void,
  get: () => SkillsCliState,
): Pick<
  SkillsCliState,
  | "loadUpdateInventory"
  | "checkUpdates"
  | "verifyUpdateBaseline"
  | "applyUpdates"
  | "retryUpdateRecovery"
  | "cancelUpdateJob"
> {
  return {
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
  };
}
