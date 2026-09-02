import { invoke, isTauriRuntime } from "@/lib/ipc";
import {
  AgentWithStatus,
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
    return (await invoke("get_ai_api_key_state")) ?? null;
  } catch {
    return null;
  }
}

let loadRequestId = 0;

export function createCentralListSlice({ set, get, getGeneration }: CentralStoreContext): Pick<
  CentralSkillsState,
  "loadCentralSkills" | "previewCentralStoreLocationChange" | "applyCentralStoreLocationChange"
> {
  return {
  /**
   * Load all Central Skills with per-platform link status, along with the
   * list of all registered agents. Called when navigating to /central.
   *
   * 默认吞错只写 store error（约 15 个 fire-and-forget 调用点依赖此语义）；
   * 显式传 `{ throwOnError: true }` 时 rethrow，由可见 UI 调用方负责 toast。
   * latest-wins：只有最新一次请求的 set 生效，被覆盖请求的写入全部丢弃
   * （rethrow 不受门控，调用方拿自己这次请求的真实结果）。
   */
  loadCentralSkills: async (options?: { throwOnError?: boolean }) => {
    const requestId = ++loadRequestId;
    const generation = getGeneration();
    // 已有列表数据时走后台刷新态，保留旧内容；空数据维持整页加载空态。
    if (get().skills.length > 0) {
      set({ isRefreshingList: true, error: null });
    } else {
      set({ isLoading: true, error: null });
    }
    if (!isTauriRuntime()) {
      if (requestId === loadRequestId && generation === getGeneration()) {
        set(createCentralBrowserFixtureState());
      }
      return;
    }
    try {
      const [skills, agents, repositories, tags, reviews, updateStates, aiApiKeyState] = await Promise.all([
        invoke<SkillWithLinks[]>("get_central_skills"),
        invoke<AgentWithStatus[]>("get_agents"),
        invoke<SkillRepositoryWithStats[]>("get_skill_repositories"),
        invoke<SkillTag[]>("get_skill_tags"),
        invoke<SkillAiTagReview[]>("get_pending_ai_tag_reviews"),
        invoke("get_central_skill_update_states"),
        loadAiApiKeyState(),
      ]);
      if (requestId === loadRequestId && generation === getGeneration()) {
        set({
          skills: skills ?? [],
          agents: agents ?? [],
          repositories: repositories ?? [],
          tags: tags ?? [],
          aiTagReviews: reviews ?? [],
          updateStatuses: indexUpdateStates(updateStates ?? []),
          aiTaggingAvailable: !!aiApiKeyState?.configured,
          isLoading: false,
          isRefreshingList: false,
          hasLoaded: true,
          requiresCentralReload: false,
        });
      }
    } catch (err) {
      if (requestId === loadRequestId && generation === getGeneration()) {
        set({ error: String(err), isLoading: false, isRefreshingList: false });
      }
      if (options?.throwOnError) {
        throw err;
      }
    }
  },
  previewCentralStoreLocationChange: async (targetPath: string) => {
    return invoke("preview_central_store_location_change", {
      request: { targetPath },
    });
  },
  applyCentralStoreLocationChange: async (targetPath: string) => {
    return invoke("apply_central_store_location_change", {
      request: { targetPath, overwriteExisting: true },
    });
  },
  };
}
