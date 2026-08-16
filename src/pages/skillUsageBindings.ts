import { useEffect, useRef } from "react";

import { useUsageStore } from "@/stores/usageStore";
import { useTargetStore } from "@/stores/targetStore";

/** 进入页面时若 store 还没有 overview，或上次刷新超过 5 分钟，就自动 refresh。 */
const AUTO_REFRESH_TTL_MS = 5 * 60 * 1000;

export function useUsageBootstrap() {
  const overview = useUsageStore((s) => s.overview);
  const unused = useUsageStore((s) => s.unused);
  const scope = useUsageStore((s) => s.scope);
  const lastRefreshMs = useUsageStore((s) => s.lastRefreshMs);
  const refresh = useUsageStore((s) => s.refresh);
  const refreshUnused = useUsageStore((s) => s.refreshUnused);
  const subscribeTargetChanged = useUsageStore((s) => s.subscribeTargetChanged);
  const subscribeScanCompleted = useUsageStore(
    (s) => s.subscribeScanCompleted,
  );
  const activeTargetId = useTargetStore((s) => s.activeTarget.id);
  const initialBootstrapStateRef = useRef({
    activeTargetId,
    lastRefreshMs,
    overview,
    refresh,
    refreshUnused,
    scope,
    subscribeScanCompleted,
    subscribeTargetChanged,
    unused,
  });

  useEffect(() => {
    const {
      activeTargetId: initialActiveTargetId,
      lastRefreshMs: initialLastRefreshMs,
      overview: initialOverview,
      refresh: initialRefresh,
      refreshUnused: initialRefreshUnused,
      scope: initialScope,
      subscribeScanCompleted: initialSubscribeScanCompleted,
      subscribeTargetChanged: initialSubscribeTargetChanged,
      unused: initialUnused,
    } = initialBootstrapStateRef.current;
    const now = Date.now();
    const targetMismatch = Boolean(
      initialOverview &&
      (!initialScope || initialScope.targetId !== initialActiveTargetId),
    );
    const stale =
      !initialOverview ||
      initialLastRefreshMs === null ||
      now - initialLastRefreshMs > AUTO_REFRESH_TTL_MS;
    if (stale || targetMismatch) {
      // refresh 成功后会自动触发 refreshUnused
      void initialRefresh(false);
    } else if (initialUnused === null) {
      void initialRefreshUnused();
    }
    // 订阅 active target 切换事件——切换后 store 自动 evict + reload；
    // 以及后台重扫完成事件——到达后 store 内静默重取页面数据
    let disposed = false;
    const unlistenFns: (() => void)[] = [];
    const trackUnlisten = (registration: Promise<() => void>) => {
      void registration.then((u) => {
        if (disposed) {
          try {
            u();
          } catch {
            /* ignore */
          }
          return;
        }
        unlistenFns.push(u);
      });
    };
    trackUnlisten(initialSubscribeTargetChanged());
    trackUnlisten(initialSubscribeScanCompleted());
    return () => {
      disposed = true;
      for (const unlistenFn of unlistenFns) {
        try {
          unlistenFn();
        } catch {
          /* ignore */
        }
      }
    };
  }, []);
}

/** 把 store 里的面板原料 + 一些派生量打包给 SkillUsageView。 */
export function useUsageBindings() {
  const overview = useUsageStore((s) => s.overview);
  const recent = useUsageStore((s) => s.recent);
  const providers = useUsageStore((s) => s.providers);
  const detail = useUsageStore((s) => s.detail);
  const unused = useUsageStore((s) => s.unused);
  const unusedLoading = useUsageStore((s) => s.unusedLoading);
  const unusedError = useUsageStore((s) => s.unusedError);
  const pendingUnlinkKeys = useUsageStore((s) => s.pendingUnlinkKeys);
  const scope = useUsageStore((s) => s.scope);
  const selectedSource = useUsageStore((s) => s.selectedSource);
  const selectedSkill = useUsageStore((s) => s.selectedSkill);
  const refreshing = useUsageStore((s) => s.refreshing);
  const loading = useUsageStore((s) => s.loading);
  const detailLoading = useUsageStore((s) => s.detailLoading);
  const error = useUsageStore((s) => s.error);
  const refreshError = useUsageStore((s) => s.refreshError);
  const usedCachedData = useUsageStore((s) => s.usedCachedData);
  const lastRefreshMs = useUsageStore((s) => s.lastRefreshMs);
  const backgroundScanning = useUsageStore((s) => s.backgroundScanning);
  const refresh = useUsageStore((s) => s.refresh);
  const selectSource = useUsageStore((s) => s.selectSource);
  const loadDetail = useUsageStore((s) => s.loadDetail);
  const clearDetail = useUsageStore((s) => s.clearDetail);
  const refreshUnused = useUsageStore((s) => s.refreshUnused);
  const unlinkUnusedSkill = useUsageStore((s) => s.unlinkUnusedSkill);

  return {
    overview,
    recent,
    providers,
    detail,
    unused,
    unusedLoading,
    unusedError,
    pendingUnlinkKeys,
    scope,
    selectedSource,
    selectedSkill,
    refreshing,
    loading,
    detailLoading,
    error,
    refreshError,
    usedCachedData,
    lastRefreshMs,
    backgroundScanning,
    refresh,
    selectSource,
    loadDetail,
    clearDetail,
    refreshUnused,
    unlinkUnusedSkill,
  };
}
