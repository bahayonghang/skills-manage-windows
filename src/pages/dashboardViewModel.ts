import { useMemo } from "react";
import type { TFunction } from "i18next";

import { isTauriRuntime } from "@/lib/tauri";
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
  OperationLogEntry,
  SkillRepositoryWithStats,
  SkillTag,
  SkillWithLinks,
  TargetSummary,
} from "@/types";

interface DashboardCentralSummary {
  centralSkillCount: number;
  updatesAvailable: number;
  aiReviewCount: number;
  uncategorizedCount: number;
  unassignedSourceCount: number;
  sourceRepositories: SkillRepositoryWithStats[];
}

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

export interface DashboardViewModel {
  activeJob: DashboardActiveJobSummary | null;
  activeQueueItems: DashboardQueueItem[];
  activity: ReturnType<typeof buildActivitySummary>;
  aiReviewCount: number;
  centralPath: string;
  centralTotal: number;
  discoveredCount: number;
  enabledTargets: PlatformTarget[];
  hasLoadError: string | null;
  healthSummary: string;
  isLogsLoading: boolean;
  isPlatformLoading: boolean;
  isPlatformRefreshing: boolean;
  lastScanLabel: string;
  loadError: string | null;
  logTotal: number;
  recentLogs: OperationLogEntry[];
  registriesCount: number;
  resolvedCollectionCount: number;
  resolvedTarget: TargetSummary;
  scanState: string;
  scanStateLabel: string;
  sourceRepositoryCount: number;
  skillsByAgent: Record<string, number>;
  targetDescription: string;
  targetLabel: string;
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

export function useDashboardViewModel({
  t,
  platformAgents,
  skillsByAgent,
  collectionCount,
  discoveredCount,
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
}: {
  t: TFunction;
  platformAgents: AgentWithStatus[];
  skillsByAgent: Record<string, number>;
  collectionCount: number;
  discoveredCount: number;
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
}) {
  const resolvedSummary = dashboardCentralSummary ?? EMPTY_DASHBOARD_CENTRAL_SUMMARY;
  const resolvedCategoryVisibility =
    categoryVisibility ?? DEFAULT_PLATFORM_CATEGORY_VISIBILITY;
  const resolvedTarget = getResolvedTarget(activeTarget, t);

  const visiblePlatformTargets = useMemo(
    () => getPlatformTargetGroups(platformAgents, resolvedCategoryVisibility),
    [platformAgents, resolvedCategoryVisibility],
  );

  const centralPath =
    platformAgents.find((agent) => agent.id === "central")
      ?.global_skills_dir ?? "~/.skillsmanage/skills/";
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
  const targetDescription =
    resolvedTarget.kind === "ssh"
      ? [
          resolvedTarget.username && resolvedTarget.host
            ? `${resolvedTarget.username}@${resolvedTarget.host}`
            : resolvedTarget.host,
          resolvedTarget.remoteHome,
        ]
          .filter(Boolean)
          .join(" / ")
      : t("targets.localDescription");
  const targetLabel =
    resolvedTarget.kind === "ssh" ? resolvedTarget.label : t("targets.local");
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
      label: t("dashboard.queue.updates"),
      count: updatesAvailable,
      description: t("dashboard.queue.updatesDesc"),
    },
    {
      key: "ai",
      label: t("dashboard.queue.aiReviews"),
      count: aiReviewCount,
      description: t("dashboard.queue.aiReviewsDesc"),
    },
    {
      key: "uncategorized",
      label: t("dashboard.queue.uncategorized"),
      count: uncategorizedCount,
      description: t("dashboard.queue.uncategorizedDesc"),
    },
    {
      key: "unassigned",
      label: t("dashboard.queue.unassigned"),
      count: unassignedSourceCount,
      description: t("dashboard.queue.unassignedDesc"),
    },
  ];
  const activeQueueItems = queueItems.filter((item) => item.count > 0);
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
    discoveredCount,
    enabledTargets,
    hasLoadError: loadError,
    healthSummary,
    isLogsLoading,
    isPlatformLoading,
    isPlatformRefreshing,
    lastScanLabel,
    loadError,
    logTotal,
    recentLogs,
    registriesCount: registries.length,
    resolvedCollectionCount,
    resolvedTarget,
    scanState: scanState ?? "idle",
    scanStateLabel,
    sourceRepositoryCount,
    skillsByAgent,
    targetDescription,
    targetLabel,
    topTags,
    uncategorizedCount,
    unassignedSourceCount,
    updatesAvailable,
    visiblePlatformTargets,
  };
}
