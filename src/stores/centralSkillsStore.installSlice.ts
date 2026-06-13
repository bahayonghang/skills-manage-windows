import { invoke, isTauriRuntime } from "@/lib/tauri";
import { invokeCommand } from "@/lib/ipc";
import {
  BatchDeleteCentralSkillPreviewResult,
  BatchDeleteCentralSkillResult,
  BatchInstallResult,
  CentralBatchInstallResult,
  CentralSkillUpdateState,
  DeleteSkillRepositoryPreview,
  DeleteSkillRepositoryResult,
  SkillAiTagReview,
  SkillRepositoryWithStats,
  SkillTag,
  SkillWithLinks,
} from "@/types";
import { indexUpdateStates } from "./centralSkillsStore.shared";
import type { CentralSkillsState, CentralStoreContext } from "./centralSkillsStore.types";

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
  | "loadRepositoryDeletePreview"
  | "deleteCentralSkill"
  | "deleteCentralSkills"
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
    try {
      const trimmedProjectPath = projectPath?.trim() ? projectPath.trim() : null;
      const result = trimmedProjectPath
        ? await invoke<CentralBatchInstallResult>("batch_install_central_skills", {
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
        : await invoke<BatchInstallResult>("batch_install_to_agents", {
            skillId,
            agentIds,
            method,
          }).then(normalizeBatchInstallResult);

      // Refresh central skills to get updated link status.
      const skills = await invoke<SkillWithLinks[]>("get_central_skills");
      const repositories = await invoke<SkillRepositoryWithStats[]>("get_skill_repositories");
      set({ skills, repositories: repositories ?? get().repositories, isInstalling: false });

      return normalizeBatchInstallResult(result);
    } catch (err) {
      set({ error: String(err), isInstalling: false });
      throw err;
    }
  },

  batchInstallSkills: async (skillIds, agentIds, method, projectPath) => {
    set({ isInstalling: true, error: null });
    try {
      const result = await invoke<CentralBatchInstallResult>("batch_install_central_skills", {
        skillIds,
        agentIds,
        method,
        projectPath: projectPath ?? null,
      }).then(normalizeCentralBatchInstallResult);

      const skills = await invoke<SkillWithLinks[]>("get_central_skills");
      const repositories = await invoke<SkillRepositoryWithStats[]>("get_skill_repositories");
      set({ skills, repositories: repositories ?? get().repositories, isInstalling: false });

      return result;
    } catch (err) {
      set({ error: String(err), isInstalling: false });
      throw err;
    }
  },

  loadDeletePreview: async (skillId) => {
    if (!isTauriRuntime()) {
      throw new Error("Desktop-only feature: Central skill deletion is available in the Tauri app.");
    }

    return invokeCommand("get_skill_detail", { skillId });
  },

  loadBatchDeletePreview: async (skillIds) => {
    if (!isTauriRuntime()) {
      throw new Error("Desktop-only feature: Central skill deletion is available in the Tauri app.");
    }

    return invoke<BatchDeleteCentralSkillPreviewResult>("preview_delete_central_skills", {
      skillIds,
    });
  },

  loadRepositoryDeletePreview: async (repositoryId) => {
    if (!isTauriRuntime()) {
      throw new Error("Desktop-only feature: repository deletion is available in the Tauri app.");
    }

    return invoke<DeleteSkillRepositoryPreview>("preview_delete_skill_repository", {
      repositoryId,
    });
  },

  deleteCentralSkill: async (skillId, removeAgentIds) => {
    if (!isTauriRuntime()) {
      throw new Error("Desktop-only feature: Central skill deletion is available in the Tauri app.");
    }

    set({ isDeleting: true, error: null });
    try {
      await invoke("delete_central_skill", { skillId, removeAgentIds });
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
      });
    } catch (err) {
      set({ error: String(err), isDeleting: false });
      throw err;
    }
  },

  deleteCentralSkills: async (requests) => {
    if (!isTauriRuntime()) {
      throw new Error("Desktop-only feature: Central skill deletion is available in the Tauri app.");
    }

    set({ isDeleting: true, error: null });
    try {
      const result = await invoke<BatchDeleteCentralSkillResult>("delete_central_skills", {
        requests,
      });
      const [skills, repositories, tags, reviews, updateStates] = await Promise.all([
        invoke<SkillWithLinks[]>("get_central_skills"),
        invoke<SkillRepositoryWithStats[]>("get_skill_repositories"),
        invoke<SkillTag[]>("get_skill_tags"),
        invoke<SkillAiTagReview[]>("get_pending_ai_tag_reviews"),
        invoke<CentralSkillUpdateState[]>("get_central_skill_update_states"),
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
      set({ error: String(err), isDeleting: false });
      throw err;
    }
  },

  deleteSkillRepository: async (repositoryId, requests) => {
    if (!isTauriRuntime()) {
      throw new Error("Desktop-only feature: repository deletion is available in the Tauri app.");
    }

    set({ isDeleting: true, error: null });
    try {
      const result = await invoke<DeleteSkillRepositoryResult>("delete_skill_repository", {
        repositoryId,
        requests,
      });
      const [skills, repositories, tags, reviews, updateStates] = await Promise.all([
        invoke<SkillWithLinks[]>("get_central_skills"),
        invoke<SkillRepositoryWithStats[]>("get_skill_repositories"),
        invoke<SkillTag[]>("get_skill_tags"),
        invoke<SkillAiTagReview[]>("get_pending_ai_tag_reviews"),
        invoke<CentralSkillUpdateState[]>("get_central_skill_update_states"),
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
      set({ error: String(err), isDeleting: false });
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

      const skills = await invoke<SkillWithLinks[]>("get_central_skills");
      const repositories = await invoke<SkillRepositoryWithStats[]>("get_skill_repositories");
      set({ skills, repositories: repositories ?? get().repositories, togglingAgentId: null });
    } catch (err) {
      set({ error: String(err), togglingAgentId: null });
      throw err;
    }
  },
  };
}
