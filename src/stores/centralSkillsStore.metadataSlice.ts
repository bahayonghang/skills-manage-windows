import { invoke, isTauriRuntime } from "@/lib/ipc";
import { SkillTagSuggestionResult } from "@/types";
import {
  createRunningAiTagJob,
  summarizeAiTagResults,
} from "./centralSkillsStore.shared";
import type {
  CentralSkillsState,
  CentralStoreContext,
  CentralStoreSet,
} from "./centralSkillsStore.types";

async function mutateThenRefresh<T>(
  set: CentralStoreSet,
  mutate: () => Promise<T>,
  refresh: () => Promise<Partial<CentralSkillsState>>,
): Promise<T> {
  set({ isMetadataUpdating: true, error: null });
  let result: T;
  try {
    result = await mutate();
  } catch (err) {
    set({ error: String(err), isMetadataUpdating: false });
    throw err;
  }
  try {
    const next = await refresh();
    set({ ...next, isMetadataUpdating: false, requiresCentralReload: false });
    return result;
  } catch (refreshErr) {
    set({
      error: String(refreshErr),
      isMetadataUpdating: false,
      requiresCentralReload: true,
    });
    throw refreshErr;
  }
}

export function createCentralMetadataSlice({
  set,
}: CentralStoreContext): Pick<
  CentralSkillsState,
  | "createRepository"
  | "assignSkillsToRepository"
  | "setRepositoryPinned"
  | "createTag"
  | "assignSkillTags"
  | "unassignSkillTags"
  | "loadAiTagReviews"
  | "acceptAiTagReview"
  | "skipAiTagReview"
  | "bulkSuggestSkillTags"
> {
  return {
    createRepository: async (name) => {
      if (!isTauriRuntime()) {
        throw new Error(
          "Desktop-only feature: repository metadata is available in the Tauri app.",
        );
      }
      return mutateThenRefresh(
        set,
        () =>
          invoke("create_or_update_skill_repository", {
            id: null,
            name,
            sourceType: "manual",
            owner: null,
            repo: null,
            branch: null,
            url: null,
            isUnknown: null,
          }),
        async () => {
          const repositories = await invoke("get_skill_repositories");
          return { repositories };
        },
      );
    },

    assignSkillsToRepository: async (skillIds, repositoryId) => {
      await mutateThenRefresh(
        set,
        () => invoke("assign_skills_to_repository", { skillIds, repositoryId }),
        async () => {
          const [skills, repositories] = await Promise.all([
            invoke("get_central_skills"),
            invoke("get_skill_repositories"),
          ]);
          return { skills, repositories };
        },
      );
    },

    setRepositoryPinned: async (repositoryId, pinned) => {
      await mutateThenRefresh(
        set,
        () =>
          invoke("set_skill_repository_pinned", {
            repositoryId,
            pinned,
          }),
        async () => {
          const repositories = await invoke("get_skill_repositories");
          return { repositories };
        },
      );
    },

    createTag: async (name) => {
      if (!isTauriRuntime()) {
        throw new Error(
          "Desktop-only feature: tag metadata is available in the Tauri app.",
        );
      }
      return mutateThenRefresh(
        set,
        () =>
          invoke("create_skill_tag", {
            name,
            description: null,
            color: null,
          }),
        async () => {
          const tags = await invoke("get_skill_tags");
          return { tags };
        },
      );
    },

    assignSkillTags: async (skillIds, tagIds) => {
      await mutateThenRefresh(
        set,
        () => invoke("assign_skill_tags", { skillIds, tagIds }),
        async () => {
          const skills = await invoke("get_central_skills");
          return { skills };
        },
      );
    },

    unassignSkillTags: async (skillId, tagIds) => {
      if (tagIds.length === 0) return;
      await mutateThenRefresh(
        set,
        () => invoke("unassign_skill_tags", { skillId, tagIds }),
        async () => {
          const skills = await invoke("get_central_skills");
          return { skills };
        },
      );
    },

    loadAiTagReviews: async () => {
      if (!isTauriRuntime()) {
        set({ aiTagReviews: [] });
        return;
      }
      try {
        const reviews = await invoke("get_pending_ai_tag_reviews");
        set({ aiTagReviews: reviews ?? [], error: null });
      } catch (err) {
        set({ error: String(err) });
      }
    },

    acceptAiTagReview: async (skillId, tagIds) => {
      await mutateThenRefresh(
        set,
        () => invoke("accept_ai_tag_review", { skillId, tagIds }),
        async () => {
          const [skills, reviews] = await Promise.all([
            invoke("get_central_skills"),
            invoke("get_pending_ai_tag_reviews"),
          ]);
          return { skills, aiTagReviews: reviews ?? [] };
        },
      );
    },

    skipAiTagReview: async (skillId) => {
      await mutateThenRefresh(
        set,
        () => invoke("skip_ai_tag_review", { skillId }),
        async () => {
          const reviews = await invoke("get_pending_ai_tag_reviews");
          return { aiTagReviews: reviews ?? [] };
        },
      );
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
      let result: SkillTagSuggestionResult[];
      try {
        result = await invoke("bulk_suggest_skill_tags", {
          skillIds,
        });
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
      try {
        const [skills, reviews] = await Promise.all([
          invoke("get_central_skills"),
          invoke("get_pending_ai_tag_reviews"),
        ]);
        set((state) => ({
          skills,
          aiTagReviews: reviews ?? [],
          isSuggestingTags: false,
          requiresCentralReload: false,
          aiTagJob:
            state.aiTagJob.status === "completed" ||
            state.aiTagJob.status === "cancelled"
              ? state.aiTagJob
              : summarizeAiTagResults(skillIds, result),
        }));
        return result;
      } catch (refreshErr) {
        set((state) => ({
          error: String(refreshErr),
          isSuggestingTags: false,
          requiresCentralReload: true,
          aiTagJob:
            state.aiTagJob.status === "completed" ||
            state.aiTagJob.status === "cancelled"
              ? state.aiTagJob
              : summarizeAiTagResults(skillIds, result),
        }));
        throw refreshErr;
      }
    },
  };
}
