import { create } from "zustand";
import { toast } from "sonner";
import { invoke, listen } from "@/lib/ipc";
import i18n from "@/i18n";
import { useTargetStore } from "@/stores/targetStore";
import type {
  ProviderHealth,
  SkillCall,
  SkillUsageDetail,
  UsageRefreshResult,
  UsageOverview,
  UsageScopeInfo,
} from "@/types/usage";

/**
 * Skill Usage 页面的 Zustand store。
 *
 * 与后端 commands/usage.rs 一一对应：
 * - refresh()       → invoke('usage_refresh', { force })
 * - loadOverview()  → invoke('usage_get_overview')（补充读接口）
 * - loadRecent()    → invoke('usage_get_recent', { limit })（补充读接口）
 * - loadProviders() → invoke('usage_get_providers')（补充读接口）
 * - loadDetail()    → invoke('usage_get_skill_detail', { skill })
 * - resolveSkillId()→ invoke('usage_resolve_skill_id', { skillName })
 *
 * 浏览器调试模式（非 Tauri runtime）下走 fixture 数据，让 vitest 可单独测组件。
 */

interface UsageState {
  overview: UsageOverview | null;
  recent: SkillCall[];
  providers: ProviderHealth[];
  detail: SkillUsageDetail | null;
  scope: UsageScopeInfo | null;
  /** 当前选中的平台（provider displayName）；null = 全部平台 */
  selectedSource: string | null;
  loading: boolean;
  refreshing: boolean;
  error: string | null;
  /** 上次成功 refresh 的时间戳（毫秒）；用于 5min 自动刷新判定 */
  lastRefreshMs: number | null;

  refresh: (force?: boolean) => Promise<UsageRefreshResult | null>;
  /** 切换平台筛选：写状态后按所选 source 重拉 overview + recent。 */
  selectSource: (source: string | null) => Promise<void>;
  loadOverview: (
    topSkillsLimit?: number,
    source?: string | null,
  ) => Promise<void>;
  loadRecent: (limit?: number, source?: string | null) => Promise<void>;
  loadProviders: () => Promise<void>;
  loadDetail: (skill: string) => Promise<void>;
  loadScope: () => Promise<UsageScopeInfo | null>;
  clearDetail: () => void;
  resolveSkillId: (skillName: string) => Promise<string | null>;
  /** 注册监听 active target 切换事件；返回 unlisten。Tauri-only。 */
  subscribeTargetChanged: () => Promise<() => void>;
}

let inFlightRefresh: {
  targetId: string;
  promise: Promise<UsageRefreshResult | null>;
} | null = null;
let refreshSequence = 0;

function activeUsageTargetId(): string {
  return useTargetStore.getState().activeTarget.id ?? "local";
}

export const useUsageStore = create<UsageState>((set, get) => ({
  overview: null,
  recent: [],
  providers: [],
  detail: null,
  scope: null,
  selectedSource: null,
  loading: false,
  refreshing: false,
  error: null,
  lastRefreshMs: null,

  async refresh(force = false) {
    const targetId = activeUsageTargetId();
    if (inFlightRefresh && inFlightRefresh.targetId === targetId) {
      return inFlightRefresh.promise;
    }

    const requestSequence = ++refreshSequence;
    const refreshPromise = (async () => {
      const stillLatestRequest = () =>
        requestSequence === refreshSequence &&
        targetId === activeUsageTargetId();

      const applyRefreshResult = (result: UsageRefreshResult) => {
        if (!stillLatestRequest()) {
          return;
        }
        set({
          overview: result.overview,
          recent: result.recent,
          providers: result.providers,
          scope: result.scope,
          refreshing: false,
          error:
            result.refreshError && !result.usedCachedData
              ? result.refreshError
              : null,
          lastRefreshMs:
            result.summary.scannedAtMs > 0 ? result.summary.scannedAtMs : null,
        });

        // refresh 永远返回未过滤 base + 完整 providers；若此前选中了某平台
        // 且刷新后它仍有数据，则按 source 重拉 overview/recent；否则回落「全部」。
        const selected = get().selectedSource;
        if (selected) {
          const stillHasData = result.providers.some(
            (p) => p.displayName === selected && p.callCount > 0,
          );
          if (stillHasData) {
            void get().loadOverview(50, selected);
            void get().loadRecent(20, selected);
          } else {
            set({ selectedSource: null });
          }
        }
      };

      const applyRefreshError = (message: string) => {
        if (!stillLatestRequest()) {
          return;
        }
        set({ refreshing: false, error: message });
      };

      set({ refreshing: true, error: null });
      try {
        const result = await invoke("usage_refresh", {
          force,
        });
        applyRefreshResult(result);
        if (result.usedCachedData && result.refreshError) {
          toast.info(i18n.t("skillUsage.showingCachedAfterError"));
        }
        return result;
      } catch (e) {
        applyRefreshError(errorMessage(e));
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
    set({ selectedSource: source });
    await Promise.all([
      get().loadOverview(50, source),
      get().loadRecent(20, source),
    ]);
  },

  async loadOverview(topSkillsLimit = 50, source = null) {
    set({ loading: true, error: null });
    try {
      const overview = await invoke("usage_get_overview", {
        topSkillsLimit,
        source,
      });
      set({ overview, loading: false });
    } catch (e) {
      set({ loading: false, error: errorMessage(e) });
    }
  },

  async loadRecent(limit = 20, source = null) {
    try {
      const recent = await invoke("usage_get_recent", {
        limit,
        source,
      });
      set({ recent });
    } catch (e) {
      set({ error: errorMessage(e) });
    }
  },

  async loadProviders() {
    try {
      const providers = await invoke("usage_get_providers");
      set({ providers });
    } catch (e) {
      set({ error: errorMessage(e) });
    }
  },

  async loadDetail(skill) {
    try {
      const detail = await invoke("usage_get_skill_detail", {
        skill,
      });
      set({ detail });
    } catch (e) {
      set({ error: errorMessage(e) });
    }
  },

  clearDetail() {
    set({ detail: null });
  },

  async loadScope() {
    try {
      const scope = await invoke("usage_get_scope_info");
      set({ scope });
      return scope;
    } catch {
      // 静默——scope 未读到不应阻塞页面
      return null;
    }
  },

  async subscribeTargetChanged() {
    try {
      const unlisten = await listen<string>("usage://target-changed", () => {
        // 切换 target 后保留旧数据直到新 refresh 完成，但立刻清空 target-bound
        // 详情与 freshness，避免旧 target 的「刚刷新过」状态阻止重扫。
        // 平台全集随机器变化，故所选平台也一并重置回「全部」。
        set({
          detail: null,
          lastRefreshMs: null,
          error: null,
          selectedSource: null,
        });
        void get().refresh(true);
      });
      // 包一层 try/catch：unlisten() 在 jsdom/纯前端测试环境下可能因
      // Tauri 内部句柄缺失而抛 Promise rejection；吞掉避免污染测试输出。
      return () => {
        try {
          const result: unknown = unlisten();
          if (result && typeof result === "object" && "catch" in result) {
            (result as Promise<unknown>).catch(() => undefined);
          }
        } catch {
          /* ignore */
        }
      };
    } catch {
      return () => undefined;
    }
  },

  async resolveSkillId(skillName) {
    try {
      const id = await invoke("usage_resolve_skill_id", {
        skillName,
      });
      return id ?? null;
    } catch {
      return null;
    }
  },
}));

function errorMessage(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  return String(e);
}
