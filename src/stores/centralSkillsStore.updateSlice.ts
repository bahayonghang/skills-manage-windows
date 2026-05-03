import { invoke, isTauriRuntime, listen } from "@/lib/tauri";
import {
  AiTagProgressPayload,
  CentralSkillUpdateProgressPayload,
  CentralSkillUpdateResult,
  CentralSkillUpdateState,
  SkillportStateImportPreview,
  SkillportStateImportResolution,
  SkillportStateImportResult,
  SkillRepositoryWithStats,
  SkillTag,
  SkillWithLinks,
} from "@/types";
import {
  AI_TAG_PROGRESS_EVENT,
  CENTRAL_UPDATE_PROGRESS_EVENT,
  createCentralSkillsInitialState,
  createRunningUpdateJob,
  indexUpdateStates,
  mergeAiTagProgress,
  mergeUpdateProgress,
  mergeUpdateStates,
} from "./centralSkillsStore.shared";
import type { CentralSkillsState, CentralStoreContext } from "./centralSkillsStore.types";

export function createCentralUpdateSlice({ set, get, bumpGeneration }: CentralStoreContext): Pick<CentralSkillsState,
  | "checkSkillUpdates"
  | "updateSkills"
  | "keepRemoteMissingSkills"
  | "cancelAiTagJob"
  | "subscribeAiTagProgress"
  | "subscribeUpdateProgress"
  | "exportSkillportState"
  | "previewSkillportStateImport"
  | "importSkillportState"
  | "resetForTargetChange"
> {
  return {
  checkSkillUpdates: async (skillIds) => {
    if (!isTauriRuntime()) {
      return [];
    }

    const targetIds = skillIds ?? get().skills.map((skill) => skill.id);
    set({
      isCheckingUpdates: true,
      error: null,
      updateJob: createRunningUpdateJob("checking", targetIds),
    });
    try {
      const states = await invoke<CentralSkillUpdateState[]>("check_central_skill_updates", {
        skillIds: skillIds ?? null,
      });
      set((state) => ({
        updateStatuses: mergeUpdateStates(state.updateStatuses, states ?? []),
        isCheckingUpdates: false,
        updateJob:
          state.updateJob.status === "running"
            ? {
                ...state.updateJob,
                status: "completed",
                completed: states?.length ?? state.updateJob.completed,
              }
            : state.updateJob,
      }));
      return states ?? [];
    } catch (err) {
      set((state) => ({
        error: String(err),
        isCheckingUpdates: false,
        updateJob: {
          ...state.updateJob,
          status: "failed",
          error: String(err),
        },
      }));
      throw err;
    }
  },

  updateSkills: async (skillIds) => {
    if (skillIds.length === 0) {
      return { succeeded: [], failed: [], skipped: [], states: [] };
    }
    if (!isTauriRuntime()) {
      throw new Error("Desktop-only feature: Central skill updates are available in the Tauri app.");
    }

    set({
      updatingSkillIds: skillIds,
      error: null,
      updateJob: createRunningUpdateJob("updating", skillIds),
    });
    try {
      const result = await invoke<CentralSkillUpdateResult>("update_central_skills", {
        skillIds,
      });
      const [skills, repositories, updateStates] = await Promise.all([
        invoke<SkillWithLinks[]>("get_central_skills"),
        invoke<SkillRepositoryWithStats[]>("get_skill_repositories"),
        invoke<CentralSkillUpdateState[]>("get_central_skill_update_states"),
      ]);
      set((state) => ({
        skills: skills ?? [],
        repositories: repositories ?? state.repositories,
        updateStatuses: indexUpdateStates(updateStates ?? result.states ?? []),
        updatingSkillIds: [],
        updateJob:
          state.updateJob.status === "running"
            ? {
                ...state.updateJob,
                status: result.failed.length > 0 ? "failed" : "completed",
                completed: result.succeeded.length + result.failed.length + result.skipped.length,
                succeeded: result.succeeded.length,
                failed: result.failed.length,
                skipped: result.skipped.length,
              }
            : state.updateJob,
      }));
      return result;
    } catch (err) {
      set((state) => ({
        error: String(err),
        updatingSkillIds: [],
        updateJob: {
          ...state.updateJob,
          status: "failed",
          error: String(err),
        },
      }));
      throw err;
    }
  },

  keepRemoteMissingSkills: async (skillIds) => {
    if (skillIds.length === 0) {
      return [];
    }
    if (!isTauriRuntime()) {
      throw new Error("Desktop-only feature: remote-missing update decisions are available in the Tauri app.");
    }

    set({ error: null });
    try {
      const kept = await invoke<string[]>("keep_remote_missing_central_skills", {
        skillIds,
      });
      const [skills, repositories, updateStates] = await Promise.all([
        invoke<SkillWithLinks[]>("get_central_skills"),
        invoke<SkillRepositoryWithStats[]>("get_skill_repositories"),
        invoke<CentralSkillUpdateState[]>("get_central_skill_update_states"),
      ]);
      set((state) => ({
        skills: skills ?? [],
        repositories: repositories ?? state.repositories,
        updateStatuses: indexUpdateStates(updateStates ?? []),
      }));
      return kept ?? [];
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },

  cancelAiTagJob: async () => {
    const jobId = get().aiTagJob.jobId;
    if (!jobId) {
      return;
    }

    await invoke("cancel_ai_tag_job", { jobId });
    set((state) => ({
      aiTagJob: {
        ...state.aiTagJob,
        status: "cancelled",
        error: state.aiTagJob.error ?? "AI tagging cancellation requested",
      },
    }));
  },

  subscribeAiTagProgress: async () => {
    if (!isTauriRuntime()) {
      return () => {};
    }

    return listen<AiTagProgressPayload>(AI_TAG_PROGRESS_EVENT, (event) => {
      set((state) => ({
        aiTagJob: mergeAiTagProgress(state.aiTagJob, event.payload),
      }));
    });
  },

  subscribeUpdateProgress: async () => {
    if (!isTauriRuntime()) {
      return () => {};
    }

    return listen<CentralSkillUpdateProgressPayload>(CENTRAL_UPDATE_PROGRESS_EVENT, (event) => {
      set((state) => ({
        updateJob: mergeUpdateProgress(state.updateJob, event.payload),
      }));
    });
  },

  exportSkillportState: async () => {
    return invoke<string>("export_skillport_state", { options: {} });
  },

  previewSkillportStateImport: async (json: string) => {
    return invoke<SkillportStateImportPreview>("preview_skillport_state_import", { json });
  },

  importSkillportState: async (json: string, resolutions: SkillportStateImportResolution[]) => {
    const result = await invoke<SkillportStateImportResult>("import_skillport_state", {
      json,
      resolutions,
    });
    const [skills, repositories, tags, updateStates] = await Promise.all([
      invoke<SkillWithLinks[]>("get_central_skills"),
      invoke<SkillRepositoryWithStats[]>("get_skill_repositories"),
      invoke<SkillTag[]>("get_skill_tags"),
      invoke<CentralSkillUpdateState[]>("get_central_skill_update_states"),
    ]);
    set({
      skills: skills ?? [],
      repositories: repositories ?? [],
      tags: tags ?? [],
      updateStatuses: indexUpdateStates(updateStates ?? []),
    });
    return result;
  },

  resetForTargetChange: () => {
    bumpGeneration();
    set(createCentralSkillsInitialState());
  },
  };
}
