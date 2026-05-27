import { create } from "zustand";
import { toast } from "sonner";
import { invoke, isTauriRuntime, listen } from "@/lib/tauri";
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
  loading: boolean;
  refreshing: boolean;
  error: string | null;
  /** 上次成功 refresh 的时间戳（毫秒）；用于 5min 自动刷新判定 */
  lastRefreshMs: number | null;

  refresh: (force?: boolean) => Promise<UsageRefreshResult | null>;
  loadOverview: (topSkillsLimit?: number) => Promise<void>;
  loadRecent: (limit?: number) => Promise<void>;
  loadProviders: () => Promise<void>;
  loadDetail: (skill: string) => Promise<void>;
  loadScope: () => Promise<UsageScopeInfo | null>;
  clearDetail: () => void;
  resolveSkillId: (skillName: string) => Promise<string | null>;
  /** 注册监听 active target 切换事件；返回 unlisten。Tauri-only。 */
  subscribeTargetChanged: () => Promise<() => void>;
}

const BROWSER_FIXTURE_OVERVIEW: UsageOverview = {
  kpis: { totalCalls: 0, uniqueSkills: 0, uniqueProjects: 0, uniqueSources: 0 },
  topSkills: [],
  heatmap: Array.from({ length: 16 * 7 }, (_, i) => ({
    date: new Date(Date.now() - (16 * 7 - 1 - i) * 86_400_000)
      .toISOString()
      .slice(0, 10),
    count: 0,
  })),
  lastScanMs: null,
};

const BROWSER_FIXTURE_PROVIDERS: ProviderHealth[] = [
  "claude-code",
  "codex",
  "droid",
  "opencode",
  "grok",
  "antigravity",
  "kiro",
  "zed",
].map((id) => ({
  providerId: id,
  displayName: id,
  available: false,
  callCount: 0,
  scannedAtMs: 0,
}));

let inFlightRefresh:
  | { targetId: string; promise: Promise<UsageRefreshResult | null> }
  | null = null;
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
        requestSequence === refreshSequence && targetId === activeUsageTargetId();

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
            result.refreshError && !result.usedCachedData ? result.refreshError : null,
          lastRefreshMs: result.summary.scannedAtMs > 0 ? result.summary.scannedAtMs : null,
        });
      };

      const applyRefreshError = (message: string) => {
        if (!stillLatestRequest()) {
          return;
        }
        set({ refreshing: false, error: message });
      };

      if (!isTauriRuntime()) {
        const result: UsageRefreshResult = {
          summary: {
            cached: false,
            callsWritten: 0,
            providersAvailable: 0,
            scannedAtMs: Date.now(),
          },
          overview: BROWSER_FIXTURE_OVERVIEW,
          recent: [],
          providers: BROWSER_FIXTURE_PROVIDERS,
          scope: {
            targetId: "local",
            label: "Local",
            isRemote: false,
            remoteReachable: false,
          },
          usedCachedData: false,
          refreshError: null,
        };
        applyRefreshResult(result);
        return result;
      }

      set({ refreshing: true, error: null });
      try {
        const result = await invoke<UsageRefreshResult>("usage_refresh", { force });
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

  async loadOverview(topSkillsLimit = 50) {
    if (!isTauriRuntime()) {
      set({ overview: BROWSER_FIXTURE_OVERVIEW });
      return;
    }
    set({ loading: true, error: null });
    try {
      const overview = await invoke<UsageOverview>("usage_get_overview", {
        topSkillsLimit,
      });
      set({ overview, loading: false });
    } catch (e) {
      set({ loading: false, error: errorMessage(e) });
    }
  },

  async loadRecent(limit = 20) {
    if (!isTauriRuntime()) {
      set({ recent: [] });
      return;
    }
    try {
      const recent = await invoke<SkillCall[]>("usage_get_recent", { limit });
      set({ recent });
    } catch (e) {
      set({ error: errorMessage(e) });
    }
  },

  async loadProviders() {
    if (!isTauriRuntime()) {
      set({ providers: BROWSER_FIXTURE_PROVIDERS });
      return;
    }
    try {
      const providers = await invoke<ProviderHealth[]>("usage_get_providers");
      set({ providers });
    } catch (e) {
      set({ error: errorMessage(e) });
    }
  },

  async loadDetail(skill) {
    if (!isTauriRuntime()) {
      set({ detail: null });
      return;
    }
    try {
      const detail = await invoke<SkillUsageDetail>("usage_get_skill_detail", {
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
    if (!isTauriRuntime()) {
      const scope = {
        targetId: "local",
        label: "Local",
        isRemote: false,
        remoteReachable: false,
      };
      set({
        scope,
      });
      return scope;
    }
    try {
      const scope = await invoke<UsageScopeInfo>("usage_get_scope_info");
      set({ scope });
      return scope;
    } catch {
      // 静默——scope 未读到不应阻塞页面
      return null;
    }
  },

  async subscribeTargetChanged() {
    if (!isTauriRuntime()) {
      return () => undefined;
    }
    try {
      const unlisten = await listen<string>("usage://target-changed", () => {
        // 切换 target 后保留旧数据直到新 refresh 完成，但立刻清空 target-bound
        // 详情与 freshness，避免旧 target 的「刚刷新过」状态阻止重扫。
        set({
          detail: null,
          lastRefreshMs: null,
          error: null,
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
    if (!isTauriRuntime()) {
      return null;
    }
    try {
      const id = await invoke<string | null>("usage_resolve_skill_id", {
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
