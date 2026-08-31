import { invoke, isTauriRuntime } from "@/lib/ipc";
import { IpcInvokeError } from "@/lib/ipc/errors";
import { backendErrorStateValue } from "@/lib/backendError";
import {
  BatchDeleteCentralSkillPreviewResult,
  BatchInstallResult,
  CentralBatchInstallResult,
  DeleteSkillRepositoryPreview,
  SkillAiTagReview,
  SkillRepositoryWithStats,
  SkillTag,
  SkillWithLinks,
} from "@/types";
import { indexUpdateStates } from "./centralSkillsStore.shared";
import type { CentralSkillsState, CentralStoreContext } from "./centralSkillsStore.types";
import { useUpdateCenterStore } from "./updateCenterStore";

function normalizeBatchInstallResult(result: BatchInstallResult): Required<BatchInstallResult> {
  return {
    succeeded: result.succeeded ?? [],
    skipped: result.skipped ?? [],
    failed: result.failed ?? [],
  };
}

function normalizeCentralBatchInstallResult(
  result: CentralBatchInstallResult
): Required<CentralBatchInstallResult> {
  return {
    succeeded: result.succeeded ?? [],
    skipped: result.skipped ?? [],
    failed: result.failed ?? [],
  };
}

export function createCentralInstallSlice({ set, get }: CentralStoreContext): Pick<CentralSkillsState,
  | "installSkill"
  | "batchInstallSkills"
  | "loadDeletePreview"
  | "loadBatchDeletePreview"
  | "loadUnknownSourceResetPreview"
  | "loadRepositoryDeletePreview"
  | "deleteCentralSkill"
  | "deleteCentralSkills"
  | "resetUnknownSourceSkills"
  | "deleteSkillRepository"
  | "togglePlatformLink"
> {
  return {
  /**
   * Install a skill to one or more agents. Refreshes the skill list after
   * a successful (or partial) install so link status icons update.
   */
  installSkill: async (skillId, agentIds, method, projectPath) => {
    set({ isInstalling: true, error: null });
    const trimmedProjectPath = projectPath?.trim() ? projectPath.trim() : null;
    let result: BatchInstallResult;
    try {
      result = trimmedProjectPath
        ? await invoke("batch_install_central_skills", {
            skillIds: [skillId],
            agentIds,
            method,
            projectPath: trimmedProjectPath,
          }).then((batchResult) => {
            const normalized = normalizeCentralBatchInstallResult(batchResult);
            return {
              succeeded: normalized.succeeded.map((success) => success.agent_id),
              skipped: normalized.skipped.map((skipped) => ({
                agent_id: skipped.agent_id,
                target_path: skipped.target_path,
                reason: skipped.reason,
              })),
              failed: normalized.failed.map((failure) => ({
                agent_id: failure.agent_id,
                error: failure.error,
              })),
            };
          })
        : await invoke("batch_install_to_agents", {
            skillId,
            agentIds,
            method,
          }).then(normalizeBatchInstallResult);
    } catch (err) {
      set({ error: String(err), isInstalling: false });
      throw err;
    }

    try {
      const skills = await invoke<SkillWithLinks[]>("get_central_skills");
      const repositories = await invoke<SkillRepositoryWithStats[]>("get_skill_repositories");
      set({
        skills,
        repositories: repositories ?? get().repositories,
        isInstalling: false,
        requiresCentralReload: false,
      });
    } catch (refreshErr) {
      set({
        error: String(refreshErr),
        isInstalling: false,
        requiresCentralReload: true,
      });
      throw refreshErr;
    }

    return normalizeBatchInstallResult(result);
  },

  batchInstallSkills: async (skillIds, agentIds, method, projectPath) => {
    set({ isInstalling: true, error: null });
    let result: Required<CentralBatchInstallResult>;
    try {
      result = await invoke("batch_install_central_skills", {
        skillIds,
        agentIds,
        method,
        projectPath: projectPath ?? null,
      }).then(normalizeCentralBatchInstallResult);
    } catch (err) {
      set({ error: String(err), isInstalling: false });
      throw err;
    }

    try {
      const skills = await invoke<SkillWithLinks[]>("get_central_skills");
      const repositories = await invoke<SkillRepositoryWithStats[]>("get_skill_repositories");
      set({
        skills,
        repositories: repositories ?? get().repositories,
        isInstalling: false,
        requiresCentralReload: false,
      });
    } catch (refreshErr) {
      set({
        error: String(refreshErr),
        isInstalling: false,
        requiresCentralReload: true,
      });
      throw refreshErr;
    }

    return result;
  },

  loadDeletePreview: async (skillId) => {
    if (!isTauriRuntime()) {
      throw new Error("Desktop-only feature: Central skill deletion is available in the Tauri app.");
    }

    const result = await invoke<BatchDeleteCentralSkillPreviewResult>("preview_delete_central_skills", {
      skillIds: [skillId],
    });
    const preview = result.previews[0];
    if (preview) {
      return preview;
    }
    const failure = result.failed[0];
    throw new IpcInvokeError({
      code: failure?.error_code || "central_skills.delete_preview_failed",
      message: failure?.error || "This Central skill could not be deleted.",
      retryable: false,
    });
  },

  loadBatchDeletePreview: async (skillIds) => {
    if (!isTauriRuntime()) {
      throw new Error("Desktop-only feature: Central skill deletion is available in the Tauri app.");
    }

    return invoke<BatchDeleteCentralSkillPreviewResult>("preview_delete_central_skills", {
      skillIds,
    });
  },

  loadUnknownSourceResetPreview: async () => {
    return invoke("preview_reset_unknown_source_skills");
  },

  loadRepositoryDeletePreview: async (repositoryId) => {
    if (!isTauriRuntime()) {
      throw new Error("Desktop-only feature: repository deletion is available in the Tauri app.");
    }

    return invoke<DeleteSkillRepositoryPreview>("preview_delete_skill_repository", {
      repositoryId,
    });
  },

  deleteCentralSkill: async (skillId, removeAgentIds, force = false) => {
    if (!isTauriRuntime()) {
      throw new Error("Desktop-only feature: Central skill deletion is available in the Tauri app.");
    }

    set({ isDeleting: true, error: null });
    try {
      await invoke("delete_central_skill", { skillId, removeAgentIds, force });
    } catch (err) {
      set({ error: backendErrorStateValue(err), isDeleting: false });
      throw err;
    }

    try {
      const [skills, repositories, tags, reviews] = await Promise.all([
        invoke<SkillWithLinks[]>("get_central_skills"),
        invoke<SkillRepositoryWithStats[]>("get_skill_repositories"),
        invoke<SkillTag[]>("get_skill_tags"),
        invoke<SkillAiTagReview[]>("get_pending_ai_tag_reviews"),
      ]);
      set({
        skills: skills ?? [],
        repositories: repositories ?? [],
        tags: tags ?? [],
        aiTagReviews: reviews ?? [],
        updateStatuses: Object.fromEntries(
          Object.entries(get().updateStatuses).filter(([id]) => id !== skillId)
        ),
        isDeleting: false,
        requiresCentralReload: false,
      });
    } catch (refreshErr) {
      set({
        error: backendErrorStateValue(refreshErr),
        isDeleting: false,
        requiresCentralReload: true,
      });
      throw refreshErr;
    }
  },

  deleteCentralSkills: async (requests) => {
    if (!isTauriRuntime()) {
      throw new Error("Desktop-only feature: Central skill deletion is available in the Tauri app.");
    }

    set({ isDeleting: true, error: null });
    try {
      const result = await invoke("delete_central_skills", {
        requests,
      });
      const [skills, repositories, tags, reviews, updateStates] = await Promise.all([
        invoke<SkillWithLinks[]>("get_central_skills"),
        invoke<SkillRepositoryWithStats[]>("get_skill_repositories"),
        invoke<SkillTag[]>("get_skill_tags"),
        invoke<SkillAiTagReview[]>("get_pending_ai_tag_reviews"),
        invoke("get_central_skill_update_states"),
      ]);
      set({
        skills: skills ?? [],
        repositories: repositories ?? [],
        tags: tags ?? [],
        aiTagReviews: reviews ?? [],
        updateStatuses: indexUpdateStates(updateStates ?? []),
        isDeleting: false,
      });
      return result;
    } catch (err) {
      set({ error: backendErrorStateValue(err), isDeleting: false });
      throw err;
    }
  },

  resetUnknownSourceSkills: async (skillIds, removeCopyAgentIds) => {
    set({ isDeleting: true, error: null });
    try {
      const result = await invoke("reset_unknown_source_skills", {
        skillIds,
        removeCopyAgentIds,
      });
      const [skills, repositories, tags, reviews, updateStates] = await Promise.all([
        invoke<SkillWithLinks[]>("get_central_skills"),
        invoke<SkillRepositoryWithStats[]>("get_skill_repositories"),
        invoke<SkillTag[]>("get_skill_tags"),
        invoke<SkillAiTagReview[]>("get_pending_ai_tag_reviews"),
        invoke("get_central_skill_update_states"),
      ]);
      set({
        skills: skills ?? [],
        repositories: repositories ?? [],
        tags: tags ?? [],
        aiTagReviews: reviews ?? [],
        updateStatuses: indexUpdateStates(updateStates ?? []),
        isDeleting: false,
      });
      try {
        await useUpdateCenterStore.getState().loadInventory();
      } catch (inventoryError) {
        set({ error: backendErrorStateValue(inventoryError) });
      }
      return result;
    } catch (err) {
      set({ error: backendErrorStateValue(err), isDeleting: false });
      throw err;
    }
  },

  deleteSkillRepository: async (repositoryId, requests) => {
    if (!isTauriRuntime()) {
      throw new Error("Desktop-only feature: repository deletion is available in the Tauri app.");
    }

    set({ isDeleting: true, error: null });
    try {
      const result = await invoke("delete_skill_repository", {
        repositoryId,
        requests,
      });
      const [skills, repositories, tags, reviews, updateStates] = await Promise.all([
        invoke<SkillWithLinks[]>("get_central_skills"),
        invoke<SkillRepositoryWithStats[]>("get_skill_repositories"),
        invoke<SkillTag[]>("get_skill_tags"),
        invoke<SkillAiTagReview[]>("get_pending_ai_tag_reviews"),
        invoke("get_central_skill_update_states"),
      ]);
      set({
        skills: skills ?? [],
        repositories: repositories ?? [],
        tags: tags ?? [],
        aiTagReviews: reviews ?? [],
        updateStatuses: indexUpdateStates(updateStates ?? []),
        isDeleting: false,
      });
      return result;
    } catch (err) {
      set({ error: backendErrorStateValue(err), isDeleting: false });
      throw err;
    }
  },

  /**
   * Toggle a single platform link for a skill.
   * If linked, uninstalls; if not linked, installs via the backend default method.
   * Refreshes the skill list afterward so linked_agents updates.
   */
  togglePlatformLink: async (skillId, agentId) => {
    set({ togglingAgentId: agentId, error: null });
    try {
      const skill = get().skills.find((s) => s.id === skillId);
      const isLinked = skill?.linked_agents.includes(agentId) ?? false;

      if (isLinked) {
        await invoke("uninstall_skill_from_agent", { skillId, agentId });
      } else {
        await invoke("install_skill_to_agent", { skillId, agentId, method: "auto" });
      }
    } catch (err) {
      set({ error: String(err), togglingAgentId: null });
      throw err;
    }

    try {
      const skills = await invoke<SkillWithLinks[]>("get_central_skills");
      const repositories = await invoke<SkillRepositoryWithStats[]>("get_skill_repositories");
      set({
        skills,
        repositories: repositories ?? get().repositories,
        togglingAgentId: null,
        requiresCentralReload: false,
      });
    } catch (refreshErr) {
      set({
        error: String(refreshErr),
        togglingAgentId: null,
        requiresCentralReload: true,
      });
      throw refreshErr;
    }
  },
  };
}
