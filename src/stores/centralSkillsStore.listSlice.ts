import { invoke, isTauriRuntime } from "@/lib/tauri";
import {
  AgentWithStatus,
  CentralStoreLocationChangeResult,
  CentralStoreLocationPreview,
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

type AiApiKeyState = { configured: boolean };

async function loadAiApiKeyState(): Promise<AiApiKeyState | null> {
  try {
    return (await invoke<AiApiKeyState>("get_ai_api_key_state")) ?? null;
  } catch {
    return null;
  }
}

export function createCentralListSlice({ set, getGeneration }: CentralStoreContext): Pick<
  CentralSkillsState,
  "loadCentralSkills" | "previewCentralStoreLocationChange" | "applyCentralStoreLocationChange"
> {
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
      const [skills, agents, repositories, tags, reviews, updateStates, aiApiKeyState] = await Promise.all([
        invoke<SkillWithLinks[]>("get_central_skills"),
        invoke<AgentWithStatus[]>("get_agents"),
        invoke<SkillRepositoryWithStats[]>("get_skill_repositories"),
        invoke<SkillTag[]>("get_skill_tags"),
        invoke<SkillAiTagReview[]>("get_pending_ai_tag_reviews"),
        invoke<CentralSkillUpdateState[]>("get_central_skill_update_states"),
        loadAiApiKeyState(),
      ]);
      if (generation === getGeneration()) {
        set({
          skills: skills ?? [],
          agents: agents ?? [],
          repositories: repositories ?? [],
          tags: tags ?? [],
          aiTagReviews: reviews ?? [],
          updateStatuses: indexUpdateStates(updateStates ?? []),
          aiTaggingAvailable: !!aiApiKeyState?.configured,
          isLoading: false,
        });
      }
    } catch (err) {
      if (generation === getGeneration()) {
        set({ error: String(err), isLoading: false });
      }
    }
  },
  previewCentralStoreLocationChange: async (targetPath: string) => {
    return invoke<CentralStoreLocationPreview>("preview_central_store_location_change", {
      request: { targetPath },
    });
  },
  applyCentralStoreLocationChange: async (targetPath: string) => {
    return invoke<CentralStoreLocationChangeResult>("apply_central_store_location_change", {
      request: { targetPath, overwriteExisting: true },
    });
  },
  };
}
