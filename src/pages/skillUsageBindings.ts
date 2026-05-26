import { useEffect } from "react";

import { useUsageStore } from "@/stores/usageStore";
import { useTargetStore } from "@/stores/targetStore";

/** 进入页面时若 store 还没有 overview，或上次刷新超过 5 分钟，就自动 refresh。 */
const AUTO_REFRESH_TTL_MS = 5 * 60 * 1000;

export function useUsageBootstrap() {
  const overview = useUsageStore((s) => s.overview);
  const scope = useUsageStore((s) => s.scope);
  const lastRefreshMs = useUsageStore((s) => s.lastRefreshMs);
  const refresh = useUsageStore((s) => s.refresh);
  const subscribeTargetChanged = useUsageStore((s) => s.subscribeTargetChanged);
  const activeTargetId = useTargetStore((s) => s.activeTarget.id);

  useEffect(() => {
    const now = Date.now();
    const targetMismatch = Boolean(
      overview && (!scope || scope.targetId !== activeTargetId)
    );
    const stale =
      !overview || lastRefreshMs === null || now - lastRefreshMs > AUTO_REFRESH_TTL_MS;
    if (stale || targetMismatch) {
      void refresh(false);
    }
    // 订阅 active target 切换事件——切换后 store 自动 evict + reload
    let unlistenFn: (() => void) | null = null;
    void subscribeTargetChanged().then((u) => {
      unlistenFn = u;
    });
    return () => {
      if (unlistenFn) {
        try {
          unlistenFn();
        } catch {
          /* ignore */
        }
      }
    };
    // 仅在挂载时触发；后续依赖手动 Refresh 按钮
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
}

/** 把 store 里的 4 个面板原料 + 一些派生量打包给 UsageShell。 */
export function useUsageBindings() {
  const overview = useUsageStore((s) => s.overview);
  const recent = useUsageStore((s) => s.recent);
  const providers = useUsageStore((s) => s.providers);
  const detail = useUsageStore((s) => s.detail);
  const scope = useUsageStore((s) => s.scope);
  const refreshing = useUsageStore((s) => s.refreshing);
  const error = useUsageStore((s) => s.error);
  const lastRefreshMs = useUsageStore((s) => s.lastRefreshMs);
  const refresh = useUsageStore((s) => s.refresh);
  const loadDetail = useUsageStore((s) => s.loadDetail);
  const clearDetail = useUsageStore((s) => s.clearDetail);
  const resolveSkillId = useUsageStore((s) => s.resolveSkillId);

  return {
    overview,
    recent,
    providers,
    detail,
    scope,
    refreshing,
    error,
    lastRefreshMs,
    refresh,
    loadDetail,
    clearDetail,
    resolveSkillId,
  };
}
