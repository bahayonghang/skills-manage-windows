import { invoke, isTauriRuntime, listen } from "@/lib/tauri";
import {
  AiTagProgressPayload,
  CentralSkillUpdateProgressPayload,
  CentralSkillUpdateResult,
  CentralSkillUpdateState,
  SkillportStatePortabilityProgressPayload,
  SkillportStateImportPreview,
  SkillportStateImportResolution,
  SkillportStateImportResult,
  SkillRepositoryWithStats,
  SkillTag,
  SkillWithLinks,
} from "@/types";
import type {
  CentralRepositorySyncApplyResult,
  CentralRepositorySyncPreview,
} from "@/types/centralRepositorySync";
import {
  AI_TAG_PROGRESS_EVENT,
  CENTRAL_UPDATE_PROGRESS_EVENT,
  PORTABILITY_PROGRESS_EVENT,
  createCentralSkillsInitialState,
  createIdlePortabilityJob,
  createRunningUpdateJob,
  indexUpdateStates,
  mergeAiTagProgress,
  mergePortabilityProgress,
  mergeUpdateProgress,
  mergeUpdateStates,
} from "./centralSkillsStore.shared";
import type { CentralSkillsState, CentralStoreContext } from "./centralSkillsStore.types";

export function createCentralUpdateSlice({ set, get, bumpGeneration }: CentralStoreContext): Pick<CentralSkillsState,
  | "checkSkillUpdates"
  | "checkRepositorySync"
  | "applyRepositorySync"
  | "updateSkills"
  | "cancelCentralUpdates"
  | "keepRemoteMissingSkills"
  | "cancelAiTagJob"
  | "subscribeAiTagProgress"
  | "subscribeUpdateProgress"
  | "subscribePortabilityProgress"
  | "cancelSkillportStatePortability"
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

  checkRepositorySync: async (repositoryIds, skillIds) => {
    if (repositoryIds.length === 0) {
      return {
        states: [],
        remoteAdded: [],
        remoteMissing: [],
        repositories: [],
        failedRepositories: [],
      };
    }
    if (!isTauriRuntime()) {
      throw new Error("Desktop-only feature: repository sync is available in the Tauri app.");
    }

    const targetIds = skillIds ?? get().skills.map((skill) => skill.id);
    set({
      isCheckingUpdates: true,
      error: null,
      updateJob: createRunningUpdateJob("checking", targetIds),
    });
    try {
      const preview = await invoke<CentralRepositorySyncPreview>(
        "check_central_repository_sync",
        {
          repositoryIds,
          skillIds: skillIds ?? null,
        }
      );
      set((state) => ({
        updateStatuses: mergeUpdateStates(state.updateStatuses, preview.states ?? []),
        isCheckingUpdates: false,
        updateJob:
          state.updateJob.status === "running"
            ? {
                ...state.updateJob,
                status:
                  preview.failedRepositories.length > 0 ? "failed" : "completed",
                completed: preview.states.length,
                failed: preview.failedRepositories.length,
              }
            : state.updateJob,
      }));
      return preview;
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

  applyRepositorySync: async (decisions) => {
    if (!isTauriRuntime()) {
      throw new Error("Desktop-only feature: repository sync is available in the Tauri app.");
    }

    const targetIds = [
      ...decisions.keepSkillIds,
      ...decisions.deleteRequests.map((request) => request.skill_id),
      ...decisions.additions.flatMap((item) =>
        item.selections.map((selection) => selection.sourcePath)
      ),
    ];
    set({
      updatingSkillIds: targetIds,
      error: null,
      updateJob: createRunningUpdateJob("updating", targetIds),
    });
    try {
      const result = await invoke<CentralRepositorySyncApplyResult>(
        "apply_central_repository_sync",
        { decisions }
      );
      const [skills, repositories, tags, updateStates] = await Promise.all([
        invoke<SkillWithLinks[]>("get_central_skills"),
        invoke<SkillRepositoryWithStats[]>("get_skill_repositories"),
        invoke<SkillTag[]>("get_skill_tags"),
        invoke<CentralSkillUpdateState[]>("get_central_skill_update_states"),
      ]);
      const failed =
        result.deleteResult.failed.length + result.failedRepositories.length;
      const imported = result.importResults.reduce(
        (count, item) => count + item.importedSkills.length,
        0
      );
      set((state) => ({
        skills: skills ?? [],
        repositories: repositories ?? state.repositories,
        tags: tags ?? state.tags,
        updateStatuses: indexUpdateStates(updateStates ?? result.states ?? []),
        updatingSkillIds: [],
        updateJob:
          state.updateJob.status === "running"
            ? {
                ...state.updateJob,
                status: failed > 0 ? "failed" : "completed",
                completed:
                  result.keptSkillIds.length +
                  result.deleteResult.succeeded.length +
                  imported +
                  failed,
                succeeded:
                  result.keptSkillIds.length +
                  result.deleteResult.succeeded.length +
                  imported,
                failed,
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
      const skills = await invoke<SkillWithLinks[]>("get_central_skills");
      set((state) => ({
        skills: skills ?? [],
        updateStatuses: mergeUpdateStates(state.updateStatuses, result.states ?? []),
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

  cancelCentralUpdates: async () => {
    if (!isTauriRuntime()) {
      return;
    }
    set((state) =>
      state.updateJob.status === "running"
        ? { updateJob: { ...state.updateJob, status: "cancelling" } }
        : {}
    );
    try {
      await invoke("cancel_central_skill_updates");
    } catch (err) {
      set({ error: String(err) });
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

  subscribePortabilityProgress: async () => {
    if (!isTauriRuntime()) {
      return () => {};
    }

    return listen<SkillportStatePortabilityProgressPayload>(PORTABILITY_PROGRESS_EVENT, (event) => {
      set((state) => ({
        portabilityJob: mergePortabilityProgress(state.portabilityJob, event.payload),
      }));
    });
  },

  cancelSkillportStatePortability: async () => {
    if (!isTauriRuntime()) {
      return;
    }
    set((state) =>
      state.portabilityJob.status === "running"
        ? { portabilityJob: { ...state.portabilityJob, status: "cancelling" } }
        : {}
    );
    try {
      await invoke("cancel_skillport_state_portability");
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },

  exportSkillportState: async () => {
    set({
      portabilityJob: {
        ...createIdlePortabilityJob(),
        phase: "exporting",
        status: "running",
        total: 1,
      },
    });
    try {
      const json = await invoke<string>("export_skillport_state", { options: {} });
      set((state) => ({
        portabilityJob:
          state.portabilityJob.status === "running"
            ? { ...state.portabilityJob, status: "completed", completed: state.portabilityJob.total }
            : state.portabilityJob,
      }));
      return json;
    } catch (err) {
      set((state) => ({
        portabilityJob: {
          ...state.portabilityJob,
          status: String(err).includes("cancelled") ? "cancelled" : "failed",
          error: String(err),
        },
      }));
      throw err;
    }
  },

  previewSkillportStateImport: async (json: string) => {
    set({
      portabilityJob: {
        ...createIdlePortabilityJob(),
        phase: "previewing",
        status: "running",
        total: 3,
      },
    });
    try {
      const preview = await invoke<SkillportStateImportPreview>("preview_skillport_state_import", { json });
      set((state) => ({
        portabilityJob:
          state.portabilityJob.status === "running"
            ? { ...state.portabilityJob, status: "completed", completed: state.portabilityJob.total }
            : state.portabilityJob,
      }));
      return preview;
    } catch (err) {
      set((state) => ({
        portabilityJob: {
          ...state.portabilityJob,
          status: String(err).includes("cancelled") ? "cancelled" : "failed",
          error: String(err),
        },
      }));
      throw err;
    }
  },

  importSkillportState: async (json: string, resolutions: SkillportStateImportResolution[]) => {
    set({
      portabilityJob: {
        ...createIdlePortabilityJob(),
        phase: "importing",
        status: "running",
        total: Math.max(1, resolutions.length),
      },
    });
    let result: SkillportStateImportResult;
    try {
      result = await invoke<SkillportStateImportResult>("import_skillport_state", {
        json,
        resolutions,
      });
    } catch (err) {
      set((state) => ({
        portabilityJob: {
          ...state.portabilityJob,
          status: String(err).includes("cancelled") ? "cancelled" : "failed",
          error: String(err),
        },
      }));
      throw err;
    }
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
      portabilityJob: {
        ...get().portabilityJob,
        status: result.cancelled
          ? "cancelled"
          : result.failedSkills.length > 0
            ? "failed"
            : "completed",
      },
    });
    return result;
  },

  resetForTargetChange: () => {
    bumpGeneration();
    set(createCentralSkillsInitialState());
  },
  };
}
