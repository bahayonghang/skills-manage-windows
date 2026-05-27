import { invoke, isTauriRuntime } from "@/lib/tauri";
import {
  SkillAiTagReview,
  SkillRepository,
  SkillRepositoryWithStats,
  SkillTag,
  SkillTagSuggestionResult,
  SkillWithLinks,
} from "@/types";
import {
  createRunningAiTagJob,
  summarizeAiTagResults,
} from "./centralSkillsStore.shared";
import type { CentralSkillsState, CentralStoreContext } from "./centralSkillsStore.types";

export function createCentralMetadataSlice({ set }: CentralStoreContext): Pick<CentralSkillsState,
  | "createRepository"
  | "assignSkillsToRepository"
  | "setRepositoryPinned"
  | "createTag"
  | "assignSkillTags"
  | "loadAiTagReviews"
  | "acceptAiTagReview"
  | "skipAiTagReview"
  | "bulkSuggestSkillTags"
> {
  return {
  createRepository: async (name) => {
    if (!isTauriRuntime()) {
      throw new Error("Desktop-only feature: repository metadata is available in the Tauri app.");
    }
    set({ isMetadataUpdating: true, error: null });
    try {
      const repository = await invoke<SkillRepository>("create_or_update_skill_repository", {
        name,
        sourceType: "manual",
      });
      const repositories = await invoke<SkillRepositoryWithStats[]>("get_skill_repositories");
      set({ repositories, isMetadataUpdating: false });
      return repository;
    } catch (err) {
      set({ error: String(err), isMetadataUpdating: false });
      throw err;
    }
  },

  assignSkillsToRepository: async (skillIds, repositoryId) => {
    set({ isMetadataUpdating: true, error: null });
    try {
      await invoke("assign_skills_to_repository", { skillIds, repositoryId });
      const [skills, repositories] = await Promise.all([
        invoke<SkillWithLinks[]>("get_central_skills"),
        invoke<SkillRepositoryWithStats[]>("get_skill_repositories"),
      ]);
      set({ skills, repositories, isMetadataUpdating: false });
    } catch (err) {
      set({ error: String(err), isMetadataUpdating: false });
      throw err;
    }
  },

  setRepositoryPinned: async (repositoryId, pinned) => {
    set({ isMetadataUpdating: true, error: null });
    try {
      await invoke<SkillRepository>("set_skill_repository_pinned", { repositoryId, pinned });
      const repositories = await invoke<SkillRepositoryWithStats[]>("get_skill_repositories");
      set({ repositories, isMetadataUpdating: false });
    } catch (err) {
      set({ error: String(err), isMetadataUpdating: false });
      throw err;
    }
  },

  createTag: async (name) => {
    if (!isTauriRuntime()) {
      throw new Error("Desktop-only feature: tag metadata is available in the Tauri app.");
    }
    set({ isMetadataUpdating: true, error: null });
    try {
      const tag = await invoke<SkillTag>("create_skill_tag", { name });
      const tags = await invoke<SkillTag[]>("get_skill_tags");
      set({ tags, isMetadataUpdating: false });
      return tag;
    } catch (err) {
      set({ error: String(err), isMetadataUpdating: false });
      throw err;
    }
  },

  assignSkillTags: async (skillIds, tagIds) => {
    set({ isMetadataUpdating: true, error: null });
    try {
      await invoke("assign_skill_tags", { skillIds, tagIds });
      const skills = await invoke<SkillWithLinks[]>("get_central_skills");
      set({ skills, isMetadataUpdating: false });
    } catch (err) {
      set({ error: String(err), isMetadataUpdating: false });
      throw err;
    }
  },

  loadAiTagReviews: async () => {
    if (!isTauriRuntime()) {
      set({ aiTagReviews: [] });
      return;
    }
    const reviews = await invoke<SkillAiTagReview[]>("get_pending_ai_tag_reviews");
    set({ aiTagReviews: reviews ?? [] });
  },

  acceptAiTagReview: async (skillId, tagIds) => {
    set({ isMetadataUpdating: true, error: null });
    try {
      await invoke("accept_ai_tag_review", { skillId, tagIds });
      const [skills, reviews] = await Promise.all([
        invoke<SkillWithLinks[]>("get_central_skills"),
        invoke<SkillAiTagReview[]>("get_pending_ai_tag_reviews"),
      ]);
      set({ skills, aiTagReviews: reviews ?? [], isMetadataUpdating: false });
    } catch (err) {
      set({ error: String(err), isMetadataUpdating: false });
      throw err;
    }
  },

  skipAiTagReview: async (skillId) => {
    set({ isMetadataUpdating: true, error: null });
    try {
      await invoke("skip_ai_tag_review", { skillId });
      const reviews = await invoke<SkillAiTagReview[]>("get_pending_ai_tag_reviews");
      set({ aiTagReviews: reviews ?? [], isMetadataUpdating: false });
    } catch (err) {
      set({ error: String(err), isMetadataUpdating: false });
      throw err;
    }
  },

  bulkSuggestSkillTags: async (skillIds) => {
    if (skillIds.length === 0) {
      return [];
    }

    set({
      isSuggestingTags: true,
      error: null,
      aiTagJob: createRunningAiTagJob(skillIds),
    });
    try {
      const result = await invoke<SkillTagSuggestionResult[]>("bulk_suggest_skill_tags", {
        skillIds,
      });
      const [skills, reviews] = await Promise.all([
        invoke<SkillWithLinks[]>("get_central_skills"),
        invoke<SkillAiTagReview[]>("get_pending_ai_tag_reviews"),
      ]);
      set((state) => ({
        skills,
        aiTagReviews: reviews ?? [],
        isSuggestingTags: false,
        aiTagJob:
          state.aiTagJob.status === "completed" || state.aiTagJob.status === "cancelled"
            ? state.aiTagJob
            : summarizeAiTagResults(skillIds, result),
      }));
      return result;
    } catch (err) {
      set((state) => ({
        error: String(err),
        isSuggestingTags: false,
        aiTagJob: {
          ...state.aiTagJob,
          status: "failed",
          error: String(err),
        },
      }));
      throw err;
    }
  },

  };
}
