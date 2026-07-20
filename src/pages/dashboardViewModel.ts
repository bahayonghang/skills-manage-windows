import { useMemo } from "react";
import type { TFunction } from "i18next";

import { isRemoteLikeTarget } from "@/lib/targetKind";
import {
  DEFAULT_PLATFORM_CATEGORY_VISIBILITY,
  type PlatformCategoryVisibility,
} from "@/lib/platformVisibility";
import {
  getPlatformTargetGroups,
  type PlatformTarget,
} from "@/lib/platformTargetGroups";
import {
  EMPTY_DASHBOARD_CENTRAL_SUMMARY,
  formatDateTime,
  RECENT_LOG_LIMIT,
  ACTIVITY_DAY_COUNT,
  TOP_TAG_LIMIT,
} from "@/pages/dashboardUtils";
import type {
  AgentWithStatus,
  CentralTopTag,
  DailyOperationCount,
  DashboardCentralSummary,
  DashboardReadiness,
  OperationLogEntry,
  TargetSummary,
} from "@/types";

export interface DashboardQueueItem {
  key: string;
  label: string;
  count: number;
  description: string;
}

export interface DashboardActiveJobSummary {
  completed: number;
  total: number;
}

const EMPTY_READINESS: DashboardReadiness = {
  score: 0,
  categorizedRatio: 0,
  describedRatio: 0,
  sourcedRatio: 0,
  installHealthRatio: 0,
};

export interface DashboardViewModel {
  activeJob: DashboardActiveJobSummary | null;
  centralTotal: number;
  dailyCounts: DailyOperationCount[];
  dailyCountsError: string | null;
  enabledTargetsCount: number;
  isDailyCountsLoading: boolean;
  isLogsLoading: boolean;
  isTopTagsLoading: boolean;
  lastScanLabel: string;
  loadError: string | null;
  logTotal: number;
  queueItems: DashboardQueueItem[];
  readiness: DashboardReadiness;
  recentLogs: OperationLogEntry[];
  retryDailyCounts: () => void;
  retryTopTags: () => void;
  scanState: string;
  scanStateLabel: string;
  skillsByAgent: Record<string, number>;
  sourceRepositoryCount: number;
  topTags: CentralTopTag[];
  topTagsError: string | null;
  quickMigratePath: string;
  quickMigrateDescription: string;
  visiblePlatformTargets: PlatformTarget[];
}

export function useDashboardViewModel({
  t,
  platformAgents,
  skillsByAgent,
  dashboardCentralSummary,
  categoryVisibility,
  lastScanAt,
  scanState,
  isPlatformLoading,
  isPlatformRefreshing,
  topTags,
  isTopTagsLoading,
  topTagsError,
  loadTopTags,
  aiTagJob,
  updateJob,
  centralError,
  logEntries,
  logTotal,
  isLogsLoading,
  logsError,
  dailyCounts,
  isDailyCountsLoading,
  dailyCountsError,
  loadDailyCounts,
  activeTarget,
  targets,
}: {
  t: TFunction;
  platformAgents: AgentWithStatus[];
  skillsByAgent: Record<string, number>;
  dashboardCentralSummary: DashboardCentralSummary | null | undefined;
  categoryVisibility: PlatformCategoryVisibility | null | undefined;
  lastScanAt: string | null | undefined;
  scanState: string | null | undefined;
  isPlatformLoading: boolean;
  isPlatformRefreshing: boolean;
  topTags: CentralTopTag[];
  isTopTagsLoading: boolean;
  topTagsError: string | null;
  loadTopTags: (limit?: number) => Promise<void>;
  aiTagJob: { status: string; completed: number; total: number } | null | undefined;
  updateJob: { status: string; completed: number; total: number } | null | undefined;
  centralError: string | null | undefined;
  logEntries: OperationLogEntry[];
  logTotal: number;
  isLogsLoading: boolean;
  logsError: string | null | undefined;
  dailyCounts: DailyOperationCount[];
  isDailyCountsLoading: boolean;
  dailyCountsError: string | null;
  loadDailyCounts: (days: number) => Promise<void>;
  activeTarget: TargetSummary | undefined;
  targets: TargetSummary[];
}): DashboardViewModel {
  // 计数口径唯一来源：后端聚合的 dashboardCentralSummary（R4d）。
  const summary = dashboardCentralSummary ?? EMPTY_DASHBOARD_CENTRAL_SUMMARY;
  const resolvedCategoryVisibility =
    categoryVisibility ?? DEFAULT_PLATFORM_CATEGORY_VISIBILITY;

  const visiblePlatformTargets = useMemo(
    () => getPlatformTargetGroups(platformAgents, resolvedCategoryVisibility),
    [platformAgents, resolvedCategoryVisibility],
  );
  const enabledTargetsCount = useMemo(
    () => visiblePlatformTargets.filter((agent) => agent.is_enabled).length,
    [visiblePlatformTargets],
  );

  const centralTotal =
    summary.centralSkillCount > 0
      ? summary.centralSkillCount
      : skillsByAgent.central || 0;
  const sourceRepositoryCount = summary.sourceRepositories.length;

  const quickMigrateTarget =
    activeTarget && isRemoteLikeTarget(activeTarget)
      ? activeTarget
      : targets.find(isRemoteLikeTarget);
  const hasRemoteSyncTarget = Boolean(quickMigrateTarget);
  const quickMigratePath = hasRemoteSyncTarget
    ? "/settings/connections?action=local-remote-sync&section=remote-targets"
    : "/settings/connections";
  const quickMigrateDescription = hasRemoteSyncTarget
    ? t("dashboard.hero.ctaQuickMigrateRemoteDesc", {
        target: quickMigrateTarget?.label ?? t("targets.local"),
      })
    : t("dashboard.hero.ctaQuickMigrateSetupDesc");

  const scanStateLabel =
    isPlatformLoading || isPlatformRefreshing
      ? t("dashboard.scanState.loading")
      : t(`dashboard.scanState.${scanState ?? "idle"}`);
  const lastScanLabel = useMemo(
    () => formatDateTime(lastScanAt, t("dashboard.neverScanned")),
    [lastScanAt, t],
  );

  const activeJob =
    aiTagJob?.status === "running"
      ? { completed: aiTagJob.completed, total: aiTagJob.total }
      : updateJob?.status === "running"
        ? { completed: updateJob.completed, total: updateJob.total }
        : null;

  const recentLogs = useMemo(
    () => logEntries.slice(0, RECENT_LOG_LIMIT),
    [logEntries],
  );

  const loadError = centralError ?? logsError ?? null;

  const queueItems: DashboardQueueItem[] = useMemo(
    () => [
      {
        key: "updates",
        label: t("dashboard.queue.updates"),
        count: summary.updatesAvailable,
        description: t("dashboard.queue.updatesDesc"),
      },
      {
        key: "ai",
        label: t("dashboard.queue.aiReviews"),
        count: summary.aiReviewCount,
        description: t("dashboard.queue.aiReviewsDesc"),
      },
      {
        key: "uncategorized",
        label: t("dashboard.queue.uncategorized"),
        count: summary.uncategorizedCount,
        description: t("dashboard.queue.uncategorizedDesc"),
      },
      {
        key: "unassigned",
        label: t("dashboard.queue.unassigned"),
        count: summary.unassignedSourceCount,
        description: t("dashboard.queue.unassignedDesc"),
      },
    ],
    [t, summary],
  );

  return {
    activeJob,
    centralTotal,
    dailyCounts,
    dailyCountsError,
    enabledTargetsCount,
    isDailyCountsLoading,
    isLogsLoading,
    isTopTagsLoading,
    lastScanLabel,
    loadError,
    logTotal,
    queueItems,
    readiness: summary.readiness ?? EMPTY_READINESS,
    recentLogs,
    retryDailyCounts: () => {
      void loadDailyCounts(ACTIVITY_DAY_COUNT);
    },
    retryTopTags: () => {
      void loadTopTags(TOP_TAG_LIMIT);
    },
    scanState: scanState ?? "idle",
    scanStateLabel,
    skillsByAgent,
    sourceRepositoryCount,
    topTags,
    topTagsError,
    quickMigratePath,
    quickMigrateDescription,
    visiblePlatformTargets,
  };
}
