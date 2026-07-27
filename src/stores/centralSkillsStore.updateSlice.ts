import { invoke, isTauriRuntime, listen } from "@/lib/ipc";
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
  createLocalJobId,
  createRunningUpdateJob,
  indexUpdateStates,
  mergeAiTagProgress,
  mergePortabilityProgress,
  mergeUpdateProgress,
  mergeUpdateStates,
} from "./centralSkillsStore.shared";
import type { CentralSkillsState, CentralStoreContext } from "./centralSkillsStore.types";
import { usePlatformStore } from "./platformStore";

/** 更新类操作改变中央库状态后，让 Dashboard summary 立即失效重取（AC6）。
 *  fire-and-forget：refreshDashboardSummary 内部已吞错，不阻塞本流程。 */
function invalidateDashboardSummary() {
  void usePlatformStore.getState().refreshDashboardSummary();
}

function isActiveJob(status: string): boolean {
  return status === "running" || status === "cancelling";
}

function assertJobCanStart(status: string, code: string, summary: string) {
  if (isActiveJob(status)) {
    throw new Error(`${code}:${summary}`);
  }
}

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
  | "saveSkillportStateExport"
  | "previewSkillportStateImport"
  | "previewSkillportStateImportFile"
  | "importSkillportState"
  | "resetForTargetChange"
> {
  return {
  checkSkillUpdates: async (skillIds) => {
    if (!isTauriRuntime()) {
      return [];
    }

    assertJobCanStart(
      get().updateJob.status,
      "job.central_update_busy",
      "A Central update job is already running.",
    );
    const jobId = createLocalJobId();
    const targetIds = skillIds ?? get().skills.map((skill) => skill.id);
    set({
      isCheckingUpdates: true,
      error: null,
      updateJob: createRunningUpdateJob("checking", targetIds, jobId),
    });
    try {
      const states = await invoke<CentralSkillUpdateState[]>("check_central_skill_updates", {
        jobId,
        skillIds: skillIds ?? null,
      });
      set((state) => state.updateJob.jobId === jobId ? ({
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
      }) : {});
      invalidateDashboardSummary();
      return states ?? [];
    } catch (err) {
      set((state) => state.updateJob.jobId === jobId ? ({
        error: String(err),
        isCheckingUpdates: false,
        updateJob: {
          ...state.updateJob,
          status: "failed",
          error: String(err),
        },
      }) : {});
      throw err;
    }
  },

  checkRepositorySync: async (repositoryIds, skillIds) => {
    if (repositoryIds.length === 0) {
      return {
        states: [],
        remoteAdded: [],
        skippedRemoteAdded: [],
        remoteMissing: [],
        repositories: [],
        failedRepositories: [],
      };
    }
    if (!isTauriRuntime()) {
      throw new Error("Desktop-only feature: repository sync is available in the Tauri app.");
    }

    assertJobCanStart(
      get().updateJob.status,
      "job.central_update_busy",
      "A Central update job is already running.",
    );
    const jobId = createLocalJobId();
    const targetIds = skillIds ?? get().skills.map((skill) => skill.id);
    set({
      isCheckingUpdates: true,
      error: null,
      updateJob: createRunningUpdateJob("checking", targetIds, jobId),
    });
    try {
      const preview = await invoke<CentralRepositorySyncPreview>(
        "check_central_repository_sync",
        {
          jobId,
          repositoryIds,
          skillIds: skillIds ?? null,
        }
      );
      set((state) => state.updateJob.jobId === jobId ? ({
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
      }) : {});
      return preview;
    } catch (err) {
      set((state) => state.updateJob.jobId === jobId ? ({
        error: String(err),
        isCheckingUpdates: false,
        updateJob: {
          ...state.updateJob,
          status: "failed",
          error: String(err),
        },
      }) : {});
      throw err;
    }
  },

  applyRepositorySync: async (decisions) => {
    if (!isTauriRuntime()) {
      throw new Error("Desktop-only feature: repository sync is available in the Tauri app.");
    }

    assertJobCanStart(
      get().updateJob.status,
      "job.central_update_busy",
      "A Central update job is already running.",
    );
    const jobId = createLocalJobId();
    const targetIds = [
        ...decisions.keepSkillIds,
        ...decisions.deleteRequests.map((request) => request.skill_id),
        ...decisions.additions.flatMap((item) =>
          item.selections.map((selection) => selection.sourcePath)
        ),
        ...(decisions.skipAdditions ?? []).map((item) => item.sourcePath),
        ...(decisions.unskipAdditions ?? []).map((item) => item.sourcePath),
      ];
    set({
      updatingSkillIds: targetIds,
      error: null,
      updateJob: createRunningUpdateJob("updating", targetIds, jobId),
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
      set((state) => state.updateJob.jobId === jobId ? ({
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
                  (result.skippedAdditions?.length ?? 0) +
                  (result.unskippedAdditions?.length ?? 0) +
                  failed,
                succeeded:
                  result.keptSkillIds.length +
                  result.deleteResult.succeeded.length +
                  imported +
                  (result.skippedAdditions?.length ?? 0) +
                  (result.unskippedAdditions?.length ?? 0),
                failed,
              }
            : state.updateJob,
      }) : {});
      return result;
    } catch (err) {
      set((state) => state.updateJob.jobId === jobId ? ({
        error: String(err),
        updatingSkillIds: [],
        updateJob: {
          ...state.updateJob,
          status: "failed",
          error: String(err),
        },
      }) : {});
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

    assertJobCanStart(
      get().updateJob.status,
      "job.central_update_busy",
      "A Central update job is already running.",
    );
    const jobId = createLocalJobId();
    set({
      updatingSkillIds: skillIds,
      error: null,
      updateJob: createRunningUpdateJob("updating", skillIds, jobId),
    });
    try {
      const result = await invoke<CentralSkillUpdateResult>("update_central_skills", {
        jobId,
        skillIds,
      });
      const skills = await invoke<SkillWithLinks[]>("get_central_skills");
      set((state) => state.updateJob.jobId === jobId ? ({
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
      }) : {});
      invalidateDashboardSummary();
      return result;
    } catch (err) {
      set((state) => state.updateJob.jobId === jobId ? ({
        error: String(err),
        updatingSkillIds: [],
        updateJob: {
          ...state.updateJob,
          status: "failed",
          error: String(err),
        },
      }) : {});
      throw err;
    }
  },

  cancelCentralUpdates: async () => {
    if (!isTauriRuntime()) {
      return;
    }
    const jobId = get().updateJob.jobId;
    if (!jobId) {
      return;
    }
    set((state) =>
      state.updateJob.status === "running"
        ? { updateJob: { ...state.updateJob, status: "cancelling" } }
        : {}
    );
    try {
      await invoke("cancel_central_skill_updates", { jobId });
    } catch (err) {
      set((state) => state.updateJob.jobId === jobId ? { error: String(err) } : {});
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
    const jobId = get().portabilityJob.jobId;
    if (!jobId) {
      return;
    }
    set((state) =>
      state.portabilityJob.status === "running"
        ? { portabilityJob: { ...state.portabilityJob, status: "cancelling" } }
        : {}
    );
    try {
      await invoke("cancel_skillport_state_portability", { jobId });
    } catch (err) {
      set((state) => state.portabilityJob.jobId === jobId ? { error: String(err) } : {});
      throw err;
    }
  },

  exportSkillportState: async () => {
    assertJobCanStart(
      get().portabilityJob.status,
      "job.portability_busy",
      "A portability job is already running.",
    );
    const jobId = createLocalJobId();
    set({
      portabilityJob: {
        ...createIdlePortabilityJob(),
        jobId,
        phase: "exporting",
        status: "running",
        total: 1,
      },
    });
    try {
      const json = await invoke<string>("export_skillport_state", { jobId, options: {} });
      set((state) => state.portabilityJob.jobId === jobId ? ({
        portabilityJob:
          state.portabilityJob.jobId === jobId && state.portabilityJob.status === "running"
            ? { ...state.portabilityJob, status: "completed", completed: state.portabilityJob.total }
            : state.portabilityJob,
      }) : {});
      return json;
    } catch (err) {
      set((state) => state.portabilityJob.jobId === jobId ? ({
        portabilityJob: {
          ...state.portabilityJob,
          status: String(err).includes("cancelled") ? "cancelled" : "failed",
          error: String(err),
        },
      }) : {});
      throw err;
    }
  },

  saveSkillportStateExport: async (path: string, json: string) => {
    await invoke("save_skillport_state_export", { path, json });
  },

  previewSkillportStateImport: async (json: string) => {
    assertJobCanStart(
      get().portabilityJob.status,
      "job.portability_busy",
      "A portability job is already running.",
    );
    const jobId = createLocalJobId();
    set({
      portabilityJob: {
        ...createIdlePortabilityJob(),
        jobId,
        phase: "previewing",
        status: "running",
        total: 3,
      },
    });
    try {
      const preview = await invoke<SkillportStateImportPreview>("preview_skillport_state_import", {
        jobId,
        json,
      });
      set((state) => state.portabilityJob.jobId === jobId ? ({
        portabilityJob:
          state.portabilityJob.jobId === jobId && state.portabilityJob.status === "running"
            ? { ...state.portabilityJob, status: "completed", completed: state.portabilityJob.total }
            : state.portabilityJob,
      }) : {});
      return preview;
    } catch (err) {
      set((state) => state.portabilityJob.jobId === jobId ? ({
        portabilityJob: {
          ...state.portabilityJob,
          status: String(err).includes("cancelled") ? "cancelled" : "failed",
          error: String(err),
        },
      }) : {});
      throw err;
    }
  },

  previewSkillportStateImportFile: async (path: string) => {
    assertJobCanStart(
      get().portabilityJob.status,
      "job.portability_busy",
      "A portability job is already running.",
    );
    const jobId = createLocalJobId();
    set({
      portabilityJob: {
        ...createIdlePortabilityJob(),
        jobId,
        phase: "previewing",
        status: "running",
        total: 3,
      },
    });
    try {
      const result = await invoke<{ json: string; preview: SkillportStateImportPreview }>(
        "preview_skillport_state_import_file",
        { jobId, path },
      );
      set((state) => state.portabilityJob.jobId === jobId ? ({
        portabilityJob:
          state.portabilityJob.jobId === jobId && state.portabilityJob.status === "running"
            ? { ...state.portabilityJob, status: "completed", completed: state.portabilityJob.total }
            : state.portabilityJob,
      }) : {});
      return result;
    } catch (err) {
      set((state) => state.portabilityJob.jobId === jobId ? ({
        portabilityJob: {
          ...state.portabilityJob,
          status: String(err).includes("cancelled") ? "cancelled" : "failed",
          error: String(err),
        },
      }) : {});
      throw err;
    }
  },

  importSkillportState: async (json: string, resolutions: SkillportStateImportResolution[]) => {
    assertJobCanStart(
      get().portabilityJob.status,
      "job.portability_busy",
      "A portability job is already running.",
    );
    const jobId = createLocalJobId();
    set({
      portabilityJob: {
        ...createIdlePortabilityJob(),
        jobId,
        phase: "importing",
        status: "running",
        total: Math.max(1, resolutions.length),
      },
    });
    let result: SkillportStateImportResult;
    try {
      result = await invoke<SkillportStateImportResult>("import_skillport_state", {
        jobId,
        json,
        resolutions,
      });
    } catch (err) {
      set((state) => state.portabilityJob.jobId === jobId ? ({
        portabilityJob: {
          ...state.portabilityJob,
          status: String(err).includes("cancelled") ? "cancelled" : "failed",
          error: String(err),
        },
      }) : {});
      throw err;
    }
    const [skills, repositories, tags, updateStates] = await Promise.all([
      invoke<SkillWithLinks[]>("get_central_skills"),
      invoke<SkillRepositoryWithStats[]>("get_skill_repositories"),
      invoke<SkillTag[]>("get_skill_tags"),
      invoke<CentralSkillUpdateState[]>("get_central_skill_update_states"),
    ]);
    set((state) => state.portabilityJob.jobId === jobId ? ({
        skills: skills ?? [],
        repositories: repositories ?? [],
        tags: tags ?? [],
        updateStatuses: indexUpdateStates(updateStates ?? []),
        portabilityJob: {
          ...state.portabilityJob,
          status: result.cancelled
            ? "cancelled"
            : result.failedSkills.length > 0
              ? "failed"
              : "completed",
        },
      }) : {});
    return result;
  },

  resetForTargetChange: () => {
    bumpGeneration();
    set(createCentralSkillsInitialState());
  },
  };
}
