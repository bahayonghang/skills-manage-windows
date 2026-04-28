import { useEffect, useMemo, useRef, type ReactNode } from "react";
import { useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";
import {
  AlertCircle,
  Blocks,
  CheckCircle2,
  CircleDot,
  Clock3,
  Database,
  FolderGit2,
  Layers,
  LayoutDashboard,
  ListChecks,
  Loader2,
  PackageSearch,
  Radar,
  ScrollText,
  Server,
  Store,
  Tags,
} from "lucide-react";

import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { PlatformIcon } from "@/components/platform/PlatformIcon";
import { useCentralSkillsStore } from "@/stores/centralSkillsStore";
import { useCollectionStore } from "@/stores/collectionStore";
import { useMarketplaceStore } from "@/stores/marketplaceStore";
import { useOperationLogStore } from "@/stores/operationLogStore";
import { usePlatformStore } from "@/stores/platformStore";
import { useTargetStore } from "@/stores/targetStore";
import {
  DEFAULT_PLATFORM_CATEGORY_VISIBILITY,
} from "@/lib/platformVisibility";
import {
  getPlatformTargetGroups,
  getPlatformTargetPathHint,
  isUniversalPlatformTarget,
} from "@/lib/platformTargetGroups";
import { cn } from "@/lib/utils";
import type {
  OperationLogEntry,
  SkillRepositoryWithStats,
} from "@/types";

const RECENT_LOG_LIMIT = 5;

const EMPTY_DASHBOARD_CENTRAL_SUMMARY = {
  centralSkillCount: 0,
  updatesAvailable: 0,
  aiReviewCount: 0,
  uncategorizedCount: 0,
  unassignedSourceCount: 0,
  sourceRepositories: [],
};

function formatDateTime(value: string | null | undefined, fallback: string) {
  if (!value) return fallback;
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat(undefined, {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

function getRepositorySkillCount(repository: SkillRepositoryWithStats): number {
  return repository.is_unknown
    ? repository.unknown_skill_count
    : repository.skill_count;
}

function logStatusClass(status: string) {
  switch (status) {
    case "failed":
      return "text-destructive";
    case "partial":
      return "text-primary";
    default:
      return "text-muted-foreground";
  }
}

function DashboardMetric({
  icon,
  label,
  value,
  description,
  onClick,
  testId,
}: {
  icon: ReactNode;
  label: string;
  value: string | number;
  description: string;
  onClick?: () => void;
  testId?: string;
}) {
  const content = (
    <Card
      data-testid={testId}
      className={cn(
        "min-h-[7.5rem] border-border/90 bg-card/95",
        onClick && "cursor-pointer hover:border-primary/45"
      )}
    >
      <CardHeader className="pb-1">
        <div className="flex items-center justify-between gap-3">
          <CardDescription className="text-xs font-medium uppercase tracking-wide">
            {label}
          </CardDescription>
          <span className="grid size-8 place-items-center rounded-lg bg-primary/10 text-primary ring-1 ring-primary/20">
            {icon}
          </span>
        </div>
      </CardHeader>
      <CardContent>
        <div className="text-2xl font-semibold tabular-nums tracking-tight">
          {value}
        </div>
        <p className="mt-2 text-xs leading-5 text-muted-foreground">
          {description}
        </p>
      </CardContent>
    </Card>
  );

  if (!onClick) return content;

  return (
    <button
      type="button"
      onClick={onClick}
      className="block w-full text-left"
    >
      {content}
    </button>
  );
}

function SectionCard({
  title,
  description,
  action,
  children,
}: {
  title: string;
  description?: string;
  action?: ReactNode;
  children: ReactNode;
}) {
  return (
    <Card className="border-border/90 bg-card/95">
      <CardHeader className="border-b border-border/70 pb-3">
        <div className="flex items-start justify-between gap-3">
          <div>
            <CardTitle>{title}</CardTitle>
            {description && (
              <CardDescription className="mt-1 text-xs">
                {description}
              </CardDescription>
            )}
          </div>
          {action}
        </div>
      </CardHeader>
      <CardContent className="pt-4">{children}</CardContent>
    </Card>
  );
}

function QueueRow({
  label,
  count,
  description,
  onClick,
}: {
  label: string;
  count: number;
  description: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="flex w-full items-center justify-between gap-3 rounded-lg border border-border/80 bg-background px-3 py-2 text-left transition-colors hover:border-primary/45 hover:bg-muted/30"
    >
      <span className="min-w-0">
        <span className="block truncate text-sm font-medium">{label}</span>
        <span className="mt-1 block truncate text-xs text-muted-foreground">
          {description}
        </span>
      </span>
      <span className="rounded-full bg-primary/10 px-2 py-0.5 text-xs font-semibold tabular-nums text-primary ring-1 ring-primary/20">
        {count}
      </span>
    </button>
  );
}

function LogRow({ entry }: { entry: OperationLogEntry }) {
  return (
    <div className="rounded-lg border border-border/80 bg-background px-3 py-2">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="truncate text-sm font-medium">{entry.summary}</div>
          <div className="mt-1 flex flex-wrap items-center gap-x-2 gap-y-1 text-xs text-muted-foreground">
            <span>{entry.action}</span>
            <span className="h-3 w-px bg-border" />
            <span>{entry.targetLabel ?? entry.targetKind}</span>
          </div>
        </div>
        <span
          className={cn(
            "shrink-0 text-xs font-medium",
            logStatusClass(entry.status)
          )}
        >
          {entry.status}
        </span>
      </div>
    </div>
  );
}

export function DashboardView() {
  const { t } = useTranslation();
  const navigate = useNavigate();

  const platformAgents = usePlatformStore((s) => s.agents);
  const skillsByAgent = usePlatformStore((s) => s.skillsByAgent);
  const collectionCount = usePlatformStore((s) => s.collectionCount);
  const discoveredCount = usePlatformStore((s) => s.discoveredCount);
  const dashboardCentralSummary =
    usePlatformStore((s) => s.dashboardCentralSummary) ??
    EMPTY_DASHBOARD_CENTRAL_SUMMARY;
  const categoryVisibility =
    usePlatformStore((s) => s.categoryVisibility) ??
    DEFAULT_PLATFORM_CATEGORY_VISIBILITY;
  const lastScanAt = usePlatformStore((s) => s.lastScanAt);
  const scanState = usePlatformStore((s) => s.scanState);
  const isPlatformLoading = usePlatformStore((s) => s.isLoading);
  const isPlatformRefreshing = usePlatformStore((s) => s.isRefreshing);

  const centralSkills = useCentralSkillsStore((s) => s.skills);
  const repositories = useCentralSkillsStore((s) => s.repositories);
  const aiTagReviews = useCentralSkillsStore((s) => s.aiTagReviews);
  const aiTagJob = useCentralSkillsStore((s) => s.aiTagJob);
  const updateStatuses = useCentralSkillsStore((s) => s.updateStatuses);
  const updateJob = useCentralSkillsStore((s) => s.updateJob);
  const isCentralLoading = useCentralSkillsStore((s) => s.isLoading);
  const centralError = useCentralSkillsStore((s) => s.error);
  const subscribeAiTagProgress = useCentralSkillsStore(
    (s) => s.subscribeAiTagProgress
  );
  const subscribeUpdateProgress = useCentralSkillsStore(
    (s) => s.subscribeUpdateProgress
  );

  const collections = useCollectionStore((s) => s.collections);
  const isCollectionsLoading = useCollectionStore((s) => s.isLoading);
  const collectionsError = useCollectionStore((s) => s.error);
  const loadCollections = useCollectionStore((s) => s.loadCollections);

  const registries = useMarketplaceStore((s) => s.registries);
  const isMarketplaceLoading = useMarketplaceStore((s) => s.isLoading);
  const marketplaceError = useMarketplaceStore((s) => s.error);
  const loadRegistries = useMarketplaceStore((s) => s.loadRegistries);

  const logEntries = useOperationLogStore((s) => s.entries);
  const logTotal = useOperationLogStore((s) => s.total);
  const isLogsLoading = useOperationLogStore((s) => s.isLoading);
  const logsError = useOperationLogStore((s) => s.error);
  const loadLogs = useOperationLogStore((s) => s.loadLogs);

  const activeTarget = useTargetStore((s) => s.activeTarget);

  const requestedCollectionsRef = useRef(false);
  const requestedRegistriesRef = useRef(false);
  const requestedLogsRef = useRef(false);

  useEffect(() => {
    if (
      requestedCollectionsRef.current ||
      isCollectionsLoading ||
      collections.length > 0
    ) {
      return;
    }

    requestedCollectionsRef.current = true;
    void loadCollections();
  }, [collections.length, isCollectionsLoading, loadCollections]);

  useEffect(() => {
    if (
      requestedRegistriesRef.current ||
      isMarketplaceLoading ||
      registries.length > 0
    ) {
      return;
    }

    requestedRegistriesRef.current = true;
    void loadRegistries();
  }, [isMarketplaceLoading, loadRegistries, registries.length]);

  useEffect(() => {
    if (requestedLogsRef.current) return;

    requestedLogsRef.current = true;
    void loadLogs({ limit: RECENT_LOG_LIMIT, offset: 0 });
  }, [loadLogs]);

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

  const visiblePlatformTargets = useMemo(
    () => getPlatformTargetGroups(platformAgents, categoryVisibility),
    [categoryVisibility, platformAgents]
  );

  const centralPath =
    platformAgents.find((agent) => agent.id === "central")?.global_skills_dir ??
    "~/.skillsmanage/skills/";
  const centralTotal =
    centralSkills.length > 0
      ? centralSkills.length
      : dashboardCentralSummary.centralSkillCount || skillsByAgent.central || 0;
  const resolvedCollectionCount =
    collections.length > 0 ? collections.length : collectionCount;
  const enabledTargets = visiblePlatformTargets.filter(
    (agent) => agent.is_enabled
  );
  const hasCentralSkillData = centralSkills.length > 0;
  const updatesAvailable = hasCentralSkillData
    ? centralSkills.filter(
        (skill) => updateStatuses[skill.id]?.status === "update_available"
      ).length
    : dashboardCentralSummary.updatesAvailable;
  const aiReviewCount =
    aiTagReviews.length > 0
      ? aiTagReviews.length
      : dashboardCentralSummary.aiReviewCount;
  const uncategorizedCount = hasCentralSkillData
    ? centralSkills.filter((skill) => {
        const skillTags = skill.tags ?? [];
        return (
          skillTags.length === 0 ||
          skillTags.some((tag) => tag.id === "uncategorized")
        );
      }).length
    : dashboardCentralSummary.uncategorizedCount;
  const unassignedSourceCount = hasCentralSkillData
    ? centralSkills.filter(
        (skill) => skill.is_source_unknown || skill.repository?.is_unknown
      ).length
    : dashboardCentralSummary.unassignedSourceCount;

  const sortedRepositories = useMemo(
    () =>
      [
        ...(repositories.length > 0
          ? repositories
          : dashboardCentralSummary.sourceRepositories),
      ]
        .sort(
          (left, right) =>
            getRepositorySkillCount(right) - getRepositorySkillCount(left)
        )
        .slice(0, 5),
    [dashboardCentralSummary.sourceRepositories, repositories]
  );

  const targetDescription =
    activeTarget.kind === "ssh"
      ? [
          activeTarget.username && activeTarget.host
            ? `${activeTarget.username}@${activeTarget.host}`
            : activeTarget.host,
          activeTarget.remoteHome,
        ]
          .filter(Boolean)
          .join(" / ")
      : t("targets.localDescription");
  const targetLabel =
    activeTarget.kind === "ssh" ? activeTarget.label : t("targets.local");
  const scanStateLabel =
    isPlatformLoading || isPlatformRefreshing
      ? t("dashboard.scanState.loading")
      : t(`dashboard.scanState.${scanState}`);
  const lastScanLabel = formatDateTime(
    lastScanAt,
    t("dashboard.neverScanned")
  );

  const queueItems = [
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
  const recentLogs = logEntries.slice(0, RECENT_LOG_LIMIT);
  const loadError =
    centralError ?? collectionsError ?? marketplaceError ?? logsError;

  return (
    <div className="flex h-full min-h-0 min-w-0 max-w-full flex-col overflow-y-auto overflow-x-hidden">
      <div className="min-w-0 shrink-0 border-b border-border px-3 py-4 sm:px-5">
        <div className="flex flex-col gap-3 lg:flex-row lg:items-end lg:justify-between">
          <div className="min-w-0">
            <div className="mb-2 inline-flex items-center gap-2 rounded-full border border-border bg-muted/20 px-2.5 py-1 text-xs font-medium text-muted-foreground">
              <LayoutDashboard className="size-3.5 text-primary" />
              {t("dashboard.eyebrow")}
            </div>
            <h1 className="text-2xl font-semibold tracking-tight">
              {t("dashboard.title")}
            </h1>
            <p className="mt-1 max-w-3xl text-sm leading-6 text-muted-foreground">
              {t("dashboard.description")}
            </p>
          </div>
          <div className="flex min-w-0 flex-wrap items-center gap-2">
            <Button
              variant="outline"
              size="sm"
              onClick={() => navigate("/central")}
            >
              <Blocks className="size-4" />
              {t("sidebar.centralSkills")}
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => navigate("/logs")}
            >
              <ScrollText className="size-4" />
              {t("logs.title")}
            </Button>
          </div>
        </div>

        <div className="mt-4 grid gap-3 lg:grid-cols-4">
          <div className="rounded-xl border border-border/90 bg-card px-3 py-2">
            <div className="flex items-center gap-2 text-xs font-medium text-muted-foreground">
              <Server className="size-3.5 text-primary" />
              {t("dashboard.target")}
            </div>
            <div className="mt-1 truncate text-sm font-semibold">{targetLabel}</div>
            <div className="mt-1 truncate text-xs text-muted-foreground">
              {targetDescription}
            </div>
          </div>
          <div className="rounded-xl border border-border/90 bg-card px-3 py-2">
            <div className="flex items-center gap-2 text-xs font-medium text-muted-foreground">
              <Database className="size-3.5 text-primary" />
              {t("dashboard.centralPath")}
            </div>
            <div className="mt-1 truncate text-sm font-semibold">{centralPath}</div>
          </div>
          <div className="rounded-xl border border-border/90 bg-card px-3 py-2">
            <div className="flex items-center gap-2 text-xs font-medium text-muted-foreground">
              <Clock3 className="size-3.5 text-primary" />
              {t("dashboard.lastScan")}
            </div>
            <div className="mt-1 truncate text-sm font-semibold">{lastScanLabel}</div>
          </div>
          <div className="rounded-xl border border-border/90 bg-card px-3 py-2">
            <div className="flex items-center gap-2 text-xs font-medium text-muted-foreground">
              {isPlatformLoading || isPlatformRefreshing ? (
                <Loader2 className="size-3.5 animate-spin text-primary" />
              ) : scanState === "error" ? (
                <AlertCircle className="size-3.5 text-destructive" />
              ) : (
                <CheckCircle2 className="size-3.5 text-primary" />
              )}
              {t("dashboard.scanStateLabel")}
            </div>
            <div className="mt-1 truncate text-sm font-semibold">
              {scanStateLabel}
            </div>
          </div>
        </div>
      </div>

      <div className="min-h-0 min-w-0 flex-1 space-y-5 p-3 sm:p-5">
        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
          <DashboardMetric
            testId="dashboard-metric-central"
            icon={<Blocks className="size-4" />}
            label={t("dashboard.metrics.central")}
            value={centralTotal}
            description={t("dashboard.metrics.centralDesc")}
            onClick={() => navigate("/central")}
          />
          <DashboardMetric
            testId="dashboard-metric-discovered"
            icon={<Radar className="size-4" />}
            label={t("dashboard.metrics.discovered")}
            value={discoveredCount}
            description={t("dashboard.metrics.discoveredDesc")}
            onClick={() => navigate("/discover")}
          />
          <DashboardMetric
            testId="dashboard-metric-collections"
            icon={<Layers className="size-4" />}
            label={t("dashboard.metrics.collections")}
            value={resolvedCollectionCount}
            description={t("dashboard.metrics.collectionsDesc")}
            onClick={() => navigate("/collections")}
          />
          <DashboardMetric
            testId="dashboard-metric-targets"
            icon={<Server className="size-4" />}
            label={t("dashboard.metrics.enabledTargets")}
            value={enabledTargets.length}
            description={t("dashboard.metrics.enabledTargetsDesc")}
            onClick={() => navigate("/settings")}
          />
        </div>

        <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
          <DashboardMetric
            testId="dashboard-metric-updates"
            icon={<PackageSearch className="size-4" />}
            label={t("dashboard.metrics.updates")}
            value={updatesAvailable}
            description={t("dashboard.metrics.updatesDesc")}
            onClick={() => navigate("/central")}
          />
          <DashboardMetric
            testId="dashboard-metric-ai"
            icon={<ListChecks className="size-4" />}
            label={t("dashboard.metrics.aiReviews")}
            value={aiReviewCount}
            description={t("dashboard.metrics.aiReviewsDesc")}
            onClick={() => navigate("/central")}
          />
          <DashboardMetric
            testId="dashboard-metric-uncategorized"
            icon={<Tags className="size-4" />}
            label={t("dashboard.metrics.uncategorized")}
            value={uncategorizedCount}
            description={t("dashboard.metrics.uncategorizedDesc")}
            onClick={() => navigate("/central")}
          />
          <DashboardMetric
            testId="dashboard-metric-unassigned"
            icon={<FolderGit2 className="size-4" />}
            label={t("dashboard.metrics.unassigned")}
            value={unassignedSourceCount}
            description={t("dashboard.metrics.unassignedDesc")}
            onClick={() => navigate("/central")}
          />
        </div>

        {loadError && (
          <div
            className="rounded-xl border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive"
            title={loadError}
          >
            {t("dashboard.loadWarning")}
          </div>
        )}

        <div className="grid gap-5 xl:grid-cols-[minmax(0,1fr)_minmax(22rem,0.85fr)]">
          <SectionCard
            title={t("dashboard.workQueues")}
            description={t("dashboard.workQueuesDesc")}
          >
            {activeQueueItems.length > 0 ? (
              <div className="space-y-2">
                {activeQueueItems.map((item) => (
                  <QueueRow
                    key={item.key}
                    label={item.label}
                    count={item.count}
                    description={item.description}
                    onClick={() => navigate("/central")}
                  />
                ))}
              </div>
            ) : (
              <div className="rounded-lg border border-border/80 bg-background px-3 py-4 text-sm text-muted-foreground">
                {t("dashboard.queue.empty")}
              </div>
            )}

            {(aiTagJob.status === "running" || updateJob.status === "running") && (
              <div className="mt-3 rounded-lg border border-border/80 bg-muted/20 px-3 py-2 text-xs text-muted-foreground">
                <span className="font-medium text-foreground">
                  {t("dashboard.activeJobs")}
                </span>
                <span className="ml-2">
                  {t("dashboard.jobProgress", {
                    completed:
                      aiTagJob.status === "running"
                        ? aiTagJob.completed
                        : updateJob.completed,
                    total:
                      aiTagJob.status === "running"
                        ? aiTagJob.total
                        : updateJob.total,
                  })}
                </span>
              </div>
            )}
          </SectionCard>

          <SectionCard
            title={t("dashboard.recentLogs")}
            description={t("dashboard.recentLogsDesc", { count: logTotal })}
            action={
              <Button
                variant="ghost"
                size="sm"
                onClick={() => navigate("/logs")}
              >
                <ScrollText className="size-3.5" />
                {t("dashboard.viewAllLogs")}
              </Button>
            }
          >
            {isLogsLoading ? (
              <div className="flex items-center gap-2 rounded-lg border border-border/80 bg-background px-3 py-4 text-sm text-muted-foreground">
                <Loader2 className="size-4 animate-spin" />
                {t("dashboard.loadingLogs")}
              </div>
            ) : recentLogs.length > 0 ? (
              <div className="space-y-2">
                {recentLogs.map((entry) => (
                  <LogRow key={entry.id} entry={entry} />
                ))}
              </div>
            ) : (
              <div className="rounded-lg border border-border/80 bg-background px-3 py-4 text-sm text-muted-foreground">
                {t("dashboard.noRecentLogs")}
              </div>
            )}
          </SectionCard>
        </div>

        <div className="grid gap-5 xl:grid-cols-3">
          <SectionCard
            title={t("dashboard.sourceCoverage")}
            description={t("dashboard.sourceCoverageDesc", {
              count: registries.length,
            })}
          >
            {sortedRepositories.length > 0 ? (
              <div className="space-y-2">
                {sortedRepositories.map((repository) => (
                  <div
                    key={repository.id}
                    className="flex items-center justify-between gap-3 rounded-lg border border-border/80 bg-background px-3 py-2"
                  >
                    <div className="min-w-0">
                      <div className="truncate text-sm font-medium">
                        {repository.name}
                      </div>
                      <div className="mt-1 text-xs text-muted-foreground">
                        {repository.source_type}
                      </div>
                    </div>
                    <span className="rounded-full bg-muted px-2 py-0.5 text-xs font-medium tabular-nums">
                      {getRepositorySkillCount(repository)}
                    </span>
                  </div>
                ))}
              </div>
            ) : (
              <div className="rounded-lg border border-border/80 bg-background px-3 py-4 text-sm text-muted-foreground">
                {isCentralLoading
                  ? t("dashboard.loadingCentral")
                  : t("dashboard.noSources")}
              </div>
            )}
          </SectionCard>

          <SectionCard
            title={t("dashboard.platformCoverage")}
            description={t("dashboard.platformCoverageDesc")}
          >
            {visiblePlatformTargets.length > 0 ? (
              <div className="space-y-2">
                {visiblePlatformTargets.slice(0, 6).map((agent) => {
                  const countAgentId = isUniversalPlatformTarget(agent)
                    ? agent.install_agent_id
                    : agent.id;
                  return (
                    <div
                      key={agent.id}
                      className="flex items-center justify-between gap-3 rounded-lg border border-border/80 bg-background px-3 py-2"
                    >
                      <div className="flex min-w-0 items-center gap-2">
                        <PlatformIcon
                          agentId={agent.id}
                          className="size-4 shrink-0 text-primary"
                        />
                        <span className="truncate text-sm font-medium">
                          {isUniversalPlatformTarget(agent)
                            ? t("platformTargets.universalShortLabel")
                            : agent.display_name}
                        </span>
                      </div>
                      <span className="truncate text-xs text-muted-foreground">
                        {skillsByAgent[countAgentId] ?? 0} /{" "}
                        {getPlatformTargetPathHint(agent)}
                      </span>
                    </div>
                  );
                })}
              </div>
            ) : (
              <div className="rounded-lg border border-border/80 bg-background px-3 py-4 text-sm text-muted-foreground">
                {t("sidebar.noPlatforms")}
              </div>
            )}
          </SectionCard>

          <SectionCard
            title={t("dashboard.quickActions")}
            description={t("dashboard.quickActionsDesc")}
          >
            <div className="grid gap-2">
              <Button
                variant="outline"
                className="justify-start"
                onClick={() => navigate("/central")}
              >
                <Blocks className="size-4" />
                {t("sidebar.centralSkills")}
              </Button>
              <Button
                variant="outline"
                className="justify-start"
                onClick={() => navigate("/discover")}
              >
                <Radar className="size-4" />
                {t("sidebar.discovered")}
              </Button>
              <Button
                variant="outline"
                className="justify-start"
                data-testid="dashboard-action-marketplace"
                onClick={() => navigate("/marketplace")}
              >
                <Store className="size-4" />
                {t("marketplace.title")}
              </Button>
              <Button
                variant="outline"
                className="justify-start"
                onClick={() => navigate("/logs")}
              >
                <ScrollText className="size-4" />
                {t("logs.title")}
              </Button>
            </div>
            <div className="mt-3 flex items-start gap-2 rounded-lg border border-border/80 bg-muted/20 px-3 py-2 text-xs leading-5 text-muted-foreground">
              <CircleDot className="mt-0.5 size-3.5 shrink-0 text-primary" />
              {t("dashboard.quickActionsNote")}
            </div>
          </SectionCard>
        </div>
      </div>
    </div>
  );
}
