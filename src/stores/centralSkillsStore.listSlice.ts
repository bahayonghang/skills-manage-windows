import { invoke, isTauriRuntime } from "@/lib/tauri";
import {
  AgentWithStatus,
  CentralSkillUpdateState,
  SkillAiTagReview,
  SkillRepositoryWithStats,
  SkillTag,
  SkillWithLinks,
} from "@/types";
import {
  createCentralBrowserFixtureState,
  indexUpdateStates,
} from "./centralSkillsStore.shared";
import type { CentralSkillsState, CentralStoreContext } from "./centralSkillsStore.types";

export function createCentralListSlice({ set, getGeneration }: CentralStoreContext): Pick<CentralSkillsState, "loadCentralSkills"> {
  return {
  /**
   * Load all Central Skills with per-platform link status, along with the
   * list of all registered agents. Called when navigating to /central.
   */
  loadCentralSkills: async () => {
    const generation = getGeneration();
    set({ isLoading: true, error: null });
    if (!isTauriRuntime()) {
      set(createCentralBrowserFixtureState());
      return;
    }
    try {
      const [skills, agents, repositories, tags, reviews, updateStates, aiApiKey] = await Promise.all([
        invoke<SkillWithLinks[]>("get_central_skills"),
        invoke<AgentWithStatus[]>("get_agents"),
        invoke<SkillRepositoryWithStats[]>("get_skill_repositories"),
        invoke<SkillTag[]>("get_skill_tags"),
        invoke<SkillAiTagReview[]>("get_pending_ai_tag_reviews"),
        invoke<CentralSkillUpdateState[]>("get_central_skill_update_states"),
        Promise.resolve(invoke<string | null>("get_setting", { key: "ai_api_key" })).catch(() => null),
      ]);
      if (generation === getGeneration()) {
        set({
          skills: skills ?? [],
          agents: agents ?? [],
          repositories: repositories ?? [],
          tags: tags ?? [],
          aiTagReviews: reviews ?? [],
          updateStatuses: indexUpdateStates(updateStates ?? []),
          aiTaggingAvailable: !!aiApiKey,
          isLoading: false,
        });
      }
    } catch (err) {
      if (generation === getGeneration()) {
        set({ error: String(err), isLoading: false });
      }
    }
  },
  };
}
