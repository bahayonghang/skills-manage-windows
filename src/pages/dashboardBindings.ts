import { useEffect, useRef } from "react";

import { useCentralSkillsStore } from "@/stores/centralSkillsStore";
import { useOperationLogStore } from "@/stores/operationLogStore";
import { usePlatformStore } from "@/stores/platformStore";
import { useTargetStore } from "@/stores/targetStore";
import { ACTIVITY_DAY_COUNT, TOP_TAG_LIMIT } from "@/pages/dashboardUtils";

/** Activity 图表窗口天数：与后端恰好返回的本地日桶数一致。 */
const ACTIVITY_CHART_DAYS = ACTIVITY_DAY_COUNT;
/** TopTags 面板展示条数。 */
const TOP_TAGS_LIMIT = TOP_TAG_LIMIT;

type DashboardLogQuery = {
  limit: number;
  offset: number;
};

async function noopAsync() {
  return undefined;
}

async function noopUnsubscribeFactory() {
  return () => undefined;
}

export function useDashboardBindings() {
  const platformAgents = usePlatformStore((state) => state.agents) ?? [];
  const skillsByAgent = usePlatformStore((state) => state.skillsByAgent) ?? {};
  const dashboardCentralSummary = usePlatformStore(
    (state) => state.dashboardCentralSummary,
  );
  const categoryVisibility = usePlatformStore(
    (state) => state.categoryVisibility,
  );
  const lastScanAt = usePlatformStore((state) => state.lastScanAt);
  const scanState = usePlatformStore((state) => state.scanState);
  const scanGeneration =
    usePlatformStore((state) => state.scanGeneration) ?? 0;
  const isPlatformLoading = usePlatformStore((state) => state.isLoading) ?? false;
  const isPlatformRefreshing =
    usePlatformStore((state) => state.isRefreshing) ?? false;
  const topTags = usePlatformStore((state) => state.topTags) ?? [];
  const isTopTagsLoading =
    usePlatformStore((state) => state.isTopTagsLoading) ?? false;
  const topTagsError = usePlatformStore((state) => state.topTagsError);
  const refreshDashboardSummary =
    usePlatformStore((state) => state.refreshDashboardSummary) ?? noopAsync;
  const loadTopTags =
    usePlatformStore((state) => state.loadTopTags) ?? noopAsync;

  const aiTagJob = useCentralSkillsStore((state) => state.aiTagJob);
  const updateJob = useCentralSkillsStore((state) => state.updateJob);
  const centralError = useCentralSkillsStore((state) => state.error);
  const subscribeAiTagProgress =
    useCentralSkillsStore((state) => state.subscribeAiTagProgress) ??
    noopUnsubscribeFactory;
  const subscribeUpdateProgress =
    useCentralSkillsStore((state) => state.subscribeUpdateProgress) ??
    noopUnsubscribeFactory;

  const logEntries = useOperationLogStore((state) => state.entries) ?? [];
  const logTotal = useOperationLogStore((state) => state.total) ?? 0;
  const isLogsLoading = useOperationLogStore((state) => state.isLoading) ?? false;
  const logsError = useOperationLogStore((state) => state.error);
  const loadLogs =
    useOperationLogStore((state) => state.loadLogs) ??
    (async (_query: DashboardLogQuery) => undefined);
  const dailyCounts = useOperationLogStore((state) => state.dailyCounts) ?? [];
  const isDailyCountsLoading =
    useOperationLogStore((state) => state.isDailyCountsLoading) ?? false;
  const dailyCountsError = useOperationLogStore(
    (state) => state.dailyCountsError,
  );
  const loadDailyCounts =
    useOperationLogStore((state) => state.loadDailyCounts) ?? noopAsync;

  const activeTarget = useTargetStore((state) => state.activeTarget);
  const targets = useTargetStore((state) => state.targets) ?? [];

  return {
    platformAgents,
    skillsByAgent,
    dashboardCentralSummary,
    categoryVisibility,
    lastScanAt,
    scanState,
    scanGeneration,
    isPlatformLoading,
    isPlatformRefreshing,
    topTags,
    isTopTagsLoading,
    topTagsError,
    refreshDashboardSummary,
    loadTopTags,
    aiTagJob,
    updateJob,
    centralError,
    subscribeAiTagProgress,
    subscribeUpdateProgress,
    logEntries,
    logTotal,
    isLogsLoading,
    logsError,
    loadLogs,
    dailyCounts,
    isDailyCountsLoading,
    dailyCountsError,
    loadDailyCounts,
    activeTarget,
    targets,
  };
}

export function useDashboardBootstrap({
  refreshDashboardSummary,
  loadTopTags,
  loadDailyCounts,
  loadLogs,
  subscribeAiTagProgress,
  subscribeUpdateProgress,
  scanGeneration,
  recentLogLimit,
}: {
  refreshDashboardSummary: () => Promise<void>;
  loadTopTags: (limit?: number) => Promise<void>;
  loadDailyCounts: (days: number) => Promise<void>;
  loadLogs: (query: DashboardLogQuery) => Promise<unknown>;
  subscribeAiTagProgress: () => Promise<() => void>;
  subscribeUpdateProgress: () => Promise<() => void>;
  scanGeneration: number;
  recentLogLimit: number;
}) {
  const requestedChartsRef = useRef(false);
  const requestedLogsRef = useRef(false);
  const lastScanGenerationRef = useRef<number | null>(null);

  // 挂载：summary + 两个图表 + 最近日志各拉一次（ref 防重入）。
  useEffect(() => {
    if (requestedChartsRef.current) return;

    requestedChartsRef.current = true;
    void refreshDashboardSummary();
    void loadTopTags(TOP_TAGS_LIMIT);
    void loadDailyCounts(ACTIVITY_CHART_DAYS);
  }, [refreshDashboardSummary, loadTopTags, loadDailyCounts]);

  useEffect(() => {
    if (requestedLogsRef.current) return;

    requestedLogsRef.current = true;
    void loadLogs({ limit: recentLogLimit, offset: 0 });
  }, [loadLogs, recentLogLimit]);

  // scanGeneration 变化（重扫完成）后 summary / topTags / dailyCounts 过期，
  // 全部重载；首次渲染只记录基线，避免与挂载加载重复。
  useEffect(() => {
    if (lastScanGenerationRef.current === null) {
      lastScanGenerationRef.current = scanGeneration;
      return;
    }
    if (lastScanGenerationRef.current === scanGeneration) return;

    lastScanGenerationRef.current = scanGeneration;
    void refreshDashboardSummary();
    void loadTopTags(TOP_TAGS_LIMIT);
    void loadDailyCounts(ACTIVITY_CHART_DAYS);
  }, [scanGeneration, refreshDashboardSummary, loadTopTags, loadDailyCounts]);

  useEffect(() => {
    let aiUnsubscribe: (() => void) | undefined;
    let updateUnsubscribe: (() => void) | undefined;
    let disposed = false;

    void subscribeAiTagProgress().then((unsubscribe) => {
      if (disposed) {
        unsubscribe();
        return;
      }
      aiUnsubscribe = unsubscribe;
    });

    void subscribeUpdateProgress().then((unsubscribe) => {
      if (disposed) {
        unsubscribe();
        return;
      }
      updateUnsubscribe = unsubscribe;
    });

    return () => {
      disposed = true;
      aiUnsubscribe?.();
      updateUnsubscribe?.();
    };
  }, [subscribeAiTagProgress, subscribeUpdateProgress]);
}
