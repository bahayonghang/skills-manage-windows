import { useMemo } from "react";
import type { TFunction } from "i18next";

import { isTauriRuntime } from "@/lib/tauri";
import { isRemoteLikeTarget, isWslTarget } from "@/lib/targetKind";
import {
  DEFAULT_PLATFORM_CATEGORY_VISIBILITY,
  type PlatformCategoryVisibility,
} from "@/lib/platformVisibility";
import {
  getPlatformTargetGroups,
  type PlatformTarget,
} from "@/lib/platformTargetGroups";
import {
  buildActivitySummary,
  buildTopTags,
  EMPTY_DASHBOARD_CENTRAL_SUMMARY,
  formatDateTime,
  RECENT_LOG_LIMIT,
} from "@/pages/dashboardUtils";
import type {
  AgentWithStatus,
  DashboardCentralSummary,
  DashboardReadiness,
  OperationLogEntry,
  SkillRepositoryWithStats,
  SkillTag,
  SkillWithLinks,
  TargetSummary,
} from "@/types";

export interface DashboardQueueItem {
  key: string;
  /** 用于 dashboard work queue tab 过滤；mockup 的 All/Review/Metadata 三态。 */
  kind: "review" | "metadata";
  label: string;
  count: number;
  description: string;
}

export interface DashboardActiveJobSummary {
  completed: number;
  total: number;
}

export interface DashboardViewModel {
  activeJob: DashboardActiveJobSummary | null;
  activeQueueItems: DashboardQueueItem[];
  activity: ReturnType<typeof buildActivitySummary>;
  aiReviewCount: number;
  centralPath: string;
  centralTotal: number;
  enabledTargets: PlatformTarget[];
  hasLoadError: string | null;
  healthSummary: string;
  isLogsLoading: boolean;
  isPlatformLoading: boolean;
  isPlatformRefreshing: boolean;
  lastScanLabel: string;
  loadError: string | null;
  logTotal: number;
  queueItems: DashboardQueueItem[];
  readiness: DashboardReadiness;
  recentLogs: OperationLogEntry[];
  registriesCount: number;
  resolvedCollectionCount: number;
  resolvedTarget: TargetSummary;
  scanState: string;
  scanStateLabel: string;
  sourceRepositoryCount: number;
  skillsByAgent: Record<string, number>;
  sparkline: { points: number[]; max: number };
  targetDescription: string;
  targetLabel: string;
  quickMigratePath: string;
  quickMigrateDescription: string;
  topTags: ReturnType<typeof buildTopTags>;
  uncategorizedCount: number;
  unassignedSourceCount: number;
  updatesAvailable: number;
  visiblePlatformTargets: PlatformTarget[];
}

function getResolvedTarget(activeTarget: TargetSummary | undefined, t: TFunction) {
  return (
    activeTarget ?? {
      id: "local",
      kind: "local" as const,
      label: t("targets.local"),
      isActive: true,
    }
  );
}

function resolveCentralPath(agents: AgentWithStatus[]): string {
  return agents.find((agent) => agent.id === "central")?.global_skills_dir ?? "";
}

export function useDashboardViewModel({
  t,
  platformAgents,
  skillsByAgent,
  collectionCount,
  dashboardCentralSummary,
  categoryVisibility,
  lastScanAt,
  scanState,
  isPlatformLoading,
  isPlatformRefreshing,
  centralSkills,
  repositories,
  aiTagReviews,
  aiTagJob,
  updateStatuses,
  updateJob,
  centralError,
  collections,
  collectionsError,
  registries,
  marketplaceError,
  logEntries,
  logTotal,
  isLogsLoading,
  logsError,
  activeTarget,
  targets,
}: {
  t: TFunction;
  platformAgents: AgentWithStatus[];
  skillsByAgent: Record<string, number>;
  collectionCount: number;
  dashboardCentralSummary: DashboardCentralSummary | null | undefined;
  categoryVisibility: PlatformCategoryVisibility | null | undefined;
  lastScanAt: string | null | undefined;
  scanState: string | null | undefined;
  isPlatformLoading: boolean;
  isPlatformRefreshing: boolean;
  centralSkills: SkillWithLinks[];
  repositories: SkillRepositoryWithStats[];
  aiTagReviews: Array<{ skill_id: string }>;
  aiTagJob: { status: string; completed: number; total: number } | null | undefined;
  updateStatuses: Record<string, { status?: string | null }>;
  updateJob: { status: string; completed: number; total: number } | null | undefined;
  centralError: string | null | undefined;
  collections: Array<{ id: string }>;
  collectionsError: string | null | undefined;
  registries: Array<{ id: string }>;
  marketplaceError: string | null | undefined;
  logEntries: OperationLogEntry[];
  logTotal: number;
  isLogsLoading: boolean;
  logsError: string | null | undefined;
  activeTarget: TargetSummary | undefined;
  targets: TargetSummary[];
}) {
  const resolvedSummary = dashboardCentralSummary ?? EMPTY_DASHBOARD_CENTRAL_SUMMARY;
  const resolvedCategoryVisibility =
    categoryVisibility ?? DEFAULT_PLATFORM_CATEGORY_VISIBILITY;
  const resolvedTarget = getResolvedTarget(activeTarget, t);

  const visiblePlatformTargets = useMemo(
    () => getPlatformTargetGroups(platformAgents, resolvedCategoryVisibility),
    [platformAgents, resolvedCategoryVisibility],
  );

  const centralPath = resolveCentralPath(platformAgents);
  const centralTotal =
    centralSkills.length > 0
      ? centralSkills.length
      : resolvedSummary.centralSkillCount || skillsByAgent.central || 0;
  const resolvedCollectionCount =
    collections.length > 0 ? collections.length : collectionCount;
  const enabledTargets = visiblePlatformTargets.filter((agent) => agent.is_enabled);
  const hasCentralSkillData = centralSkills.length > 0;
  const updatesAvailable = hasCentralSkillData
    ? centralSkills.filter(
        (skill) => updateStatuses[skill.id]?.status === "update_available",
      ).length
    : resolvedSummary.updatesAvailable;
  const aiReviewCount =
    aiTagReviews.length > 0 ? aiTagReviews.length : resolvedSummary.aiReviewCount;
  const uncategorizedCount = hasCentralSkillData
    ? centralSkills.filter((skill) => {
        const skillTags = (skill.tags ?? []) as SkillTag[];
        return (
          skillTags.length === 0 ||
          skillTags.some((tag) => tag.id === "uncategorized")
        );
      }).length
    : resolvedSummary.uncategorizedCount;
  const unassignedSourceCount = hasCentralSkillData
    ? centralSkills.filter(
        (skill) => skill.is_source_unknown || skill.repository?.is_unknown,
      ).length
    : resolvedSummary.unassignedSourceCount;
  const sourceRepositoryCount =
    repositories.length > 0
      ? repositories.length
      : resolvedSummary.sourceRepositories.length;
  const targetDescription = isRemoteLikeTarget(resolvedTarget)
    ? [
        isWslTarget(resolvedTarget)
          ? resolvedTarget.distribution
          : resolvedTarget.username && resolvedTarget.host
            ? `${resolvedTarget.username}@${resolvedTarget.host}`
            : resolvedTarget.host,
        resolvedTarget.remoteHome,
      ]
        .filter(Boolean)
        .join(" / ")
    : t("targets.localDescription");
  const targetLabel = isRemoteLikeTarget(resolvedTarget)
    ? resolvedTarget.label
    : t("targets.local");
  const quickMigrateTarget = isRemoteLikeTarget(resolvedTarget)
    ? resolvedTarget
    : targets.find(isRemoteLikeTarget);
  const hasRemoteSyncTarget =
    Boolean(quickMigrateTarget);
  const quickMigratePath = hasRemoteSyncTarget
    ? "/settings?section=remote-targets&action=local-remote-sync"
    : "/settings?section=remote-targets";
  const quickMigrateDescription = hasRemoteSyncTarget
    ? t("dashboard.hero.ctaQuickMigrateRemoteDesc", {
        target: quickMigrateTarget?.label ?? targetLabel,
      })
    : t("dashboard.hero.ctaQuickMigrateSetupDesc");
  const scanStateLabel =
    isPlatformLoading || isPlatformRefreshing
      ? t("dashboard.scanState.loading")
      : t(`dashboard.scanState.${scanState ?? "idle"}`);
  const lastScanLabel = formatDateTime(lastScanAt, t("dashboard.neverScanned"));
  const activeJob =
    aiTagJob?.status === "running"
      ? { completed: aiTagJob.completed, total: aiTagJob.total }
      : updateJob?.status === "running"
        ? { completed: updateJob.completed, total: updateJob.total }
        : null;
  const activity = useMemo(() => buildActivitySummary(logEntries), [logEntries]);
  const topTags = useMemo(() => buildTopTags(centralSkills), [centralSkills]);
  const recentLogs = logEntries.slice(0, RECENT_LOG_LIMIT);
  const loadError =
    centralError ??
    collectionsError ??
    logsError ??
    (isTauriRuntime() ? marketplaceError ?? null : null);
  const queueItems: DashboardQueueItem[] = [
    {
      key: "updates",
      kind: "review",
      label: t("dashboard.queue.updates"),
      count: updatesAvailable,
      description: t("dashboard.queue.updatesDesc"),
    },
    {
      key: "ai",
      kind: "review",
      label: t("dashboard.queue.aiReviews"),
      count: aiReviewCount,
      description: t("dashboard.queue.aiReviewsDesc"),
    },
    {
      key: "uncategorized",
      kind: "metadata",
      label: t("dashboard.queue.uncategorized"),
      count: uncategorizedCount,
      description: t("dashboard.queue.uncategorizedDesc"),
    },
    {
      key: "unassigned",
      kind: "metadata",
      label: t("dashboard.queue.unassigned"),
      count: unassignedSourceCount,
      description: t("dashboard.queue.unassignedDesc"),
    },
  ];
  const activeQueueItems = queueItems.filter((item) => item.count > 0);
  const readiness = resolvedSummary.readiness ?? {
    score: 0,
    categorizedRatio: 0,
    describedRatio: 0,
    sourcedRatio: 0,
    installHealthRatio: 0,
  };
  const sparkline = {
    points: activity.buckets.map((bucket) => bucket.count),
    max: activity.max,
  };
  const healthSummary =
    centralTotal > 0
      ? t("dashboard.health.summary", {
          aiReviewCount,
          centralTotal,
          sourceRepositoryCount,
          uncategorizedCount,
        })
      : t("dashboard.health.emptySummary");

  return {
    activeJob,
    activeQueueItems,
    activity,
    aiReviewCount,
    centralPath,
    centralTotal,
    enabledTargets,
    hasLoadError: loadError,
    healthSummary,
    isLogsLoading,
    isPlatformLoading,
    isPlatformRefreshing,
    lastScanLabel,
    loadError,
    logTotal,
    queueItems,
    readiness,
    recentLogs,
    registriesCount: registries.length,
    resolvedCollectionCount,
    resolvedTarget,
    scanState: scanState ?? "idle",
    scanStateLabel,
    sourceRepositoryCount,
    skillsByAgent,
    sparkline,
    targetDescription,
    targetLabel,
    quickMigratePath,
    quickMigrateDescription,
    topTags,
    uncategorizedCount,
    unassignedSourceCount,
    updatesAvailable,
    visiblePlatformTargets,
  };
}
