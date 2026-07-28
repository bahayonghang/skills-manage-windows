import { create } from "zustand";
import { toast } from "sonner";

import i18n from "@/i18n";
import { invoke, listen } from "@/lib/ipc";
import { useTargetStore } from "@/stores/targetStore";
import type {
  ProviderHealth,
  RecentSkillCall,
  SkillUsageDetail,
  UsageOverview,
  UsageRefreshResult,
  UsageScopeInfo,
} from "@/types/usage";

interface UsageState {
  overview: UsageOverview | null;
  recent: RecentSkillCall[];
  providers: ProviderHealth[];
  detail: SkillUsageDetail | null;
  scope: UsageScopeInfo | null;
  selectedSource: string | null;
  selectedSkill: string | null;
  loading: boolean;
  refreshing: boolean;
  detailLoading: boolean;
  error: string | null;
  refreshError: string | null;
  usedCachedData: boolean;
  lastRefreshMs: number | null;

  refresh: (force?: boolean) => Promise<UsageRefreshResult | null>;
  selectSource: (source: string | null) => Promise<void>;
  loadDetail: (skill: string) => Promise<void>;
  clearDetail: () => void;
  loadScope: () => Promise<UsageScopeInfo | null>;
  subscribeTargetChanged: () => Promise<() => void>;
}

let inFlightRefresh: {
  targetId: string;
  promise: Promise<UsageRefreshResult | null>;
} | null = null;
let refreshSequence = 0;
let pageSequence = 0;
let detailSequence = 0;

function activeUsageTargetId(): string {
  return useTargetStore.getState().activeTarget.id ?? "local";
}

function pageRequestMatches(request: number, targetId: string): boolean {
  return request === pageSequence && targetId === activeUsageTargetId();
}

export const useUsageStore = create<UsageState>((set, get) => ({
  overview: null,
  recent: [],
  providers: [],
  detail: null,
  scope: null,
  selectedSource: null,
  selectedSkill: null,
  loading: false,
  refreshing: false,
  detailLoading: false,
  error: null,
  refreshError: null,
  usedCachedData: false,
  lastRefreshMs: null,

  async refresh(force = false) {
    const targetId = activeUsageTargetId();
    if (inFlightRefresh && inFlightRefresh.targetId === targetId) {
      return inFlightRefresh.promise;
    }

    const requestSequence = ++refreshSequence;
    ++pageSequence;
    const refreshPromise = (async () => {
      const stillLatestRequest = () =>
        requestSequence === refreshSequence &&
        targetId === activeUsageTargetId();

      set({ refreshing: true, error: null, refreshError: null });
      try {
        const result = await invoke("usage_refresh", { force });
        if (!stillLatestRequest()) return result;

        const commonState = {
          providers: result.providers,
          scope: result.scope,
          refreshing: false,
          refreshError: result.refreshError,
          usedCachedData: result.usedCachedData,
          error:
            result.refreshError && !result.usedCachedData
              ? result.refreshError
              : null,
          lastRefreshMs:
            result.summary.scannedAtMs > 0 ? result.summary.scannedAtMs : null,
        };
        const selected = get().selectedSource;
        const selectedStillExists = selected
          ? result.providers.some(
              (provider) =>
                provider.displayName === selected && provider.callCount > 0,
            )
          : false;

        if (selected && selectedStillExists) {
          set(commonState);
          const filteredRequest = ++pageSequence;
          try {
            const [overview, recent] = await Promise.all([
              invoke("usage_get_overview", {
                topSkillsLimit: 0,
                source: selected,
              }),
              invoke("usage_get_recent", { limit: 20, source: selected }),
            ]);
            if (
              stillLatestRequest() &&
              pageRequestMatches(filteredRequest, targetId) &&
              get().selectedSource === selected
            ) {
              set({ overview, recent });
            }
          } catch (error) {
            if (stillLatestRequest()) {
              set({ error: errorMessage(error) });
            }
          }
        } else {
          set({
            ...commonState,
            overview: result.overview,
            recent: result.recent,
            selectedSource: selectedStillExists ? selected : null,
          });
        }

        if (result.usedCachedData && result.refreshError) {
          toast.info(i18n.t("skillUsage.showingCachedAfterError"));
        }
        return result;
      } catch (error) {
        if (stillLatestRequest()) {
          const message = errorMessage(error);
          const hasCachedPage = get().overview !== null;
          set({
            refreshing: false,
            refreshError: message,
            usedCachedData: hasCachedPage,
            error: hasCachedPage ? null : message,
          });
        }
        return null;
      }
    })();

    inFlightRefresh = { targetId, promise: refreshPromise };
    return refreshPromise.finally(() => {
      if (inFlightRefresh?.promise === refreshPromise) {
        inFlightRefresh = null;
      }
    });
  },

  async selectSource(source) {
    if (source === get().selectedSource) return;

    const targetId = activeUsageTargetId();
    const requestSequence = ++pageSequence;
    ++detailSequence;
    set({
      loading: true,
      error: null,
      selectedSkill: null,
      detail: null,
      detailLoading: false,
    });

    try {
      const [overview, recent] = await Promise.all([
        invoke("usage_get_overview", { topSkillsLimit: 0, source }),
        invoke("usage_get_recent", { limit: 20, source }),
      ]);
      if (pageRequestMatches(requestSequence, targetId)) {
        set({
          overview,
          recent,
          selectedSource: source,
          loading: false,
        });
      }
    } catch (error) {
      if (pageRequestMatches(requestSequence, targetId)) {
        set({ loading: false, error: errorMessage(error) });
      }
    }
  },

  async loadDetail(skill) {
    const targetId = activeUsageTargetId();
    const source = get().selectedSource;
    const requestSequence = ++detailSequence;
    set({
      selectedSkill: skill,
      detail: null,
      detailLoading: true,
      error: null,
    });
    try {
      const detail = await invoke("usage_get_skill_detail", { skill, source });
      if (
        requestSequence === detailSequence &&
        targetId === activeUsageTargetId() &&
        source === get().selectedSource &&
        skill === get().selectedSkill
      ) {
        set({ detail, detailLoading: false });
      }
    } catch (error) {
      if (requestSequence === detailSequence) {
        set({ detailLoading: false, error: errorMessage(error) });
      }
    }
  },

  clearDetail() {
    ++detailSequence;
    set({ selectedSkill: null, detail: null, detailLoading: false });
  },

  async loadScope() {
    try {
      const scope = await invoke("usage_get_scope_info");
      set({ scope });
      return scope;
    } catch {
      return null;
    }
  },

  async subscribeTargetChanged() {
    try {
      const unlisten = await listen<string>("usage://target-changed", () => {
        ++refreshSequence;
        ++pageSequence;
        ++detailSequence;
        // 连同页面数据一起清空：重扫期间不得继续展示上一个 target 的面板
        set({
          overview: null,
          recent: [],
          providers: [],
          loading: false,
          detail: null,
          selectedSkill: null,
          detailLoading: false,
          scope: null,
          lastRefreshMs: null,
          error: null,
          refreshError: null,
          usedCachedData: false,
          selectedSource: null,
        });
        void get().refresh(true);
      });
      return () => {
        try {
          const result: unknown = unlisten();
          if (result && typeof result === "object" && "catch" in result) {
            (result as Promise<unknown>).catch(() => undefined);
          }
        } catch {
          // Browser fixtures expose a no-op listener.
        }
      };
    } catch {
      return () => undefined;
    }
  },
}));

function errorMessage(error: unknown): string {
  if (typeof error === "string") return error;
  if (error instanceof Error) return error.message;
  return String(error);
}
