import { useEffect, useMemo, useRef } from "react";
import { useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";
import {
  AlertCircle,
  Blocks,
  CheckCircle2,
  ChevronRight,
  Clock3,
  Database,
  FolderGit2,
  Layers,
  LayoutDashboard,
  Loader2,
  Radar,
  ScrollText,
  Server,
  Store,
} from "lucide-react";

import {
  Card,
  CardContent,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import {
  LogRow,
  PanelHeader,
  ProgressRow,
  QueueRow,
  StatButton,
  StatusTile,
} from "@/components/dashboard/DashboardPanels";
import { PlatformIcon } from "@/components/platform/PlatformIcon";
import { useCentralSkillsStore } from "@/stores/centralSkillsStore";
import { useCollectionStore } from "@/stores/collectionStore";
import { useMarketplaceStore } from "@/stores/marketplaceStore";
import { useOperationLogStore } from "@/stores/operationLogStore";
import { usePlatformStore } from "@/stores/platformStore";
import { useTargetStore } from "@/stores/targetStore";
import { isTauriRuntime } from "@/lib/tauri";
import {
  DEFAULT_PLATFORM_CATEGORY_VISIBILITY,
} from "@/lib/platformVisibility";
import {
  getPlatformTargetGroups,
  getPlatformTargetPathHint,
  isUniversalPlatformTarget,
} from "@/lib/platformTargetGroups";
import { cn } from "@/lib/utils";
import {
  buildActivitySummary,
  buildTopTags,
  EMPTY_DASHBOARD_CENTRAL_SUMMARY,
  formatDateTime,
  heatCellClass,
  RECENT_LOG_LIMIT,
} from "@/pages/dashboardUtils";

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
  const sourceRepositoryCount =
    repositories.length > 0
      ? repositories.length
      : dashboardCentralSummary.sourceRepositories.length;

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
  const activeJob =
    aiTagJob.status === "running"
      ? aiTagJob
      : updateJob.status === "running"
        ? updateJob
        : null;
  const activity = useMemo(
    () => buildActivitySummary(logEntries),
    [logEntries]
  );
  const topTags = useMemo(() => buildTopTags(centralSkills), [centralSkills]);
  const recentLogs = logEntries.slice(0, RECENT_LOG_LIMIT);
  const loadError =
    centralError ??
    collectionsError ??
    logsError ??
    (isTauriRuntime() ? marketplaceError : null);

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
  const healthSummary =
    centralTotal > 0
      ? t("dashboard.health.summary", {
          aiReviewCount,
          centralTotal,
          sourceRepositoryCount,
          uncategorizedCount,
        })
      : t("dashboard.health.emptySummary");

  return (
    <div className="flex h-full min-h-0 min-w-0 max-w-full flex-col overflow-y-auto overflow-x-hidden bg-background">
      <div className="min-w-0 shrink-0 border-b border-border px-3 py-4 sm:px-5">
        <div className="flex flex-col gap-3 xl:flex-row xl:items-end xl:justify-between">
          <div className="min-w-0">
            <div className="mb-2 inline-flex items-center gap-2 rounded-md border border-border bg-muted/25 px-2.5 py-1 text-xs font-medium text-muted-foreground">
              <LayoutDashboard className="size-3.5 text-primary" />
              {t("dashboard.eyebrow")}
            </div>
            <h1 className="text-2xl font-semibold tracking-tight">
              {t("dashboard.title")}{" "}
              <span className="font-mono text-sm font-normal text-muted-foreground">
                / {t("dashboard.localScope")}
              </span>
            </h1>
            <p className="mt-1 max-w-3xl break-words text-sm leading-6 text-muted-foreground">
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
            <Button
              variant="outline"
              size="sm"
              data-testid="dashboard-action-marketplace"
              onClick={() => navigate("/marketplace")}
            >
              <Store className="size-4" />
              {t("marketplace.title")}
            </Button>
          </div>
        </div>
      </div>

      <div className="min-h-0 min-w-0 flex-1 space-y-5 p-3 sm:p-5">
        <Card className="overflow-hidden border-border/90 bg-card/95">
          <div className="grid sm:grid-cols-2 xl:grid-cols-4">
            <StatusTile
              icon={<Server className="size-4" />}
              label={t("dashboard.target")}
              value={targetLabel}
              detail={targetDescription}
            />
            <StatusTile
              icon={<Database className="size-4" />}
              label={t("dashboard.centralPath")}
              value={centralPath}
            />
            <StatusTile
              icon={<Clock3 className="size-4" />}
              label={t("dashboard.lastScan")}
              value={lastScanLabel}
            />
            <StatusTile
              icon={
                isPlatformLoading || isPlatformRefreshing ? (
                  <Loader2 className="size-4 animate-spin" />
                ) : scanState === "error" ? (
                  <AlertCircle className="size-4 text-destructive" />
                ) : (
                  <CheckCircle2 className="size-4" />
                )
              }
              label={t("dashboard.scanStateLabel")}
              value={scanStateLabel}
              detail={
                isPlatformRefreshing
                  ? t("dashboard.scanStateDetail.refreshing")
                  : t("dashboard.scanStateDetail.idle")
              }
            />
          </div>
        </Card>

        {loadError && (
          <div
            className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive"
            title={loadError}
          >
            {t("dashboard.loadWarning")}
          </div>
        )}

        <div className="grid gap-5 xl:grid-cols-[minmax(0,1.35fr)_minmax(22rem,0.9fr)]">
          <Card className="overflow-hidden border-border/90 bg-card/95">
            <PanelHeader
              title={t("dashboard.health.title")}
              description={t("dashboard.health.description")}
              action={
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => navigate("/central")}
                >
                  {t("dashboard.health.openCentral")}
                  <ChevronRight className="size-3.5" />
                </Button>
              }
            />
            <CardContent className="grid gap-5 p-4 lg:grid-cols-[minmax(0,0.9fr)_minmax(0,1fr)]">
              <div className="min-w-0 space-y-4">
                <div>
                  <div
                    data-testid="dashboard-metric-central"
                    className="flex items-baseline gap-3"
                  >
                    <span className="text-5xl font-semibold tabular-nums tracking-tight">
                      {centralTotal}
                    </span>
                    <span className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
                      {t("dashboard.health.centralUnit")}
                    </span>
                  </div>
                  <p className="mt-3 max-w-xl break-words text-sm leading-6 text-muted-foreground">
                    {healthSummary}
                  </p>
                </div>
                <div className="space-y-3">
                  <ProgressRow
                    testId="dashboard-metric-uncategorized"
                    label={t("dashboard.metrics.uncategorized")}
                    count={uncategorizedCount}
                    total={centralTotal}
                    tone="primary"
                  />
                  <ProgressRow
                    testId="dashboard-metric-ai"
                    label={t("dashboard.metrics.aiReviews")}
                    count={aiReviewCount}
                    total={centralTotal}
                    tone="primary"
                  />
                  <ProgressRow
                    testId="dashboard-metric-updates"
                    label={t("dashboard.metrics.updates")}
                    count={updatesAvailable}
                    total={centralTotal}
                    tone="primary"
                  />
                  <ProgressRow
                    testId="dashboard-metric-unassigned"
                    label={t("dashboard.metrics.unassigned")}
                    count={unassignedSourceCount}
                    total={centralTotal}
                    tone="muted"
                  />
                </div>
              </div>
              <div className="grid min-w-0 gap-3 sm:grid-cols-2">
                <StatButton
                  testId="dashboard-metric-discovered"
                  icon={<Radar className="size-4" />}
                  label={t("dashboard.metrics.discovered")}
                  value={discoveredCount}
                  description={t("dashboard.metrics.discoveredDesc")}
                  onClick={() => navigate("/discover")}
                />
                <StatButton
                  testId="dashboard-metric-collections"
                  icon={<Layers className="size-4" />}
                  label={t("dashboard.metrics.collections")}
                  value={resolvedCollectionCount}
                  description={t("dashboard.metrics.collectionsDesc")}
                  onClick={() => navigate("/collections")}
                />
                <StatButton
                  icon={<FolderGit2 className="size-4" />}
                  label={t("dashboard.health.sources")}
                  value={sourceRepositoryCount}
                  description={t("dashboard.health.sourcesDesc", {
                    count: registries.length,
                  })}
                  onClick={() => navigate("/marketplace")}
                />
                <StatButton
                  testId="dashboard-metric-targets"
                  icon={<Server className="size-4" />}
                  label={t("dashboard.metrics.enabledTargets")}
                  value={enabledTargets.length}
                  description={t("dashboard.metrics.enabledTargetsDesc")}
                  onClick={() => navigate("/settings")}
                  emphasized={enabledTargets.length > 0}
                />
              </div>
            </CardContent>
          </Card>

          <Card className="overflow-hidden border-border/90 bg-card/95">
            <PanelHeader
              title={t("dashboard.platforms.title", {
                enabled: enabledTargets.length,
                total: visiblePlatformTargets.length,
              })}
              description={t("dashboard.platforms.description")}
              action={
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => navigate("/settings")}
                >
                  {t("dashboard.platforms.manage")}
                  <ChevronRight className="size-3.5" />
                </Button>
              }
            />
            <CardContent className="p-0">
              {visiblePlatformTargets.length > 0 ? (
                <div>
                  {visiblePlatformTargets.slice(0, 7).map((agent) => {
                    const countAgentId = isUniversalPlatformTarget(agent)
                      ? agent.install_agent_id
                      : agent.id;
                    const pathHint = getPlatformTargetPathHint(agent);

                    return (
                      <button
                        key={agent.id}
                        type="button"
                        onClick={() => navigate(`/platform/${agent.id}`)}
                        className="grid w-full grid-cols-[2rem_minmax(0,1fr)_auto_auto] items-center gap-3 border-t border-border/70 px-4 py-3 text-left transition-colors first:border-t-0 hover:bg-muted/25"
                        aria-label={t("dashboard.platforms.openLabel", {
                          name: isUniversalPlatformTarget(agent)
                            ? t("platformTargets.universalShortLabel")
                            : agent.display_name,
                        })}
                      >
                        <span className="grid size-8 place-items-center rounded-md border border-border/80 bg-background text-primary">
                          <PlatformIcon
                            agentId={agent.id}
                            className="size-4"
                          />
                        </span>
                        <span className="min-w-0">
                          <span className="flex min-w-0 items-center gap-2">
                            <span className="truncate text-sm font-medium">
                              {isUniversalPlatformTarget(agent)
                                ? t("platformTargets.universalShortLabel")
                                : agent.display_name}
                            </span>
                            <span className="hidden rounded border border-border bg-muted/40 px-1.5 py-0.5 text-[0.65rem] font-medium uppercase tracking-wide text-muted-foreground sm:inline">
                              {agent.is_enabled
                                ? t("dashboard.platforms.enabled")
                                : t("dashboard.platforms.hidden")}
                            </span>
                          </span>
                          <span className="mt-1 block truncate text-xs text-muted-foreground">
                            {pathHint || t("dashboard.platforms.noPath")}
                          </span>
                        </span>
                        <span className="text-sm font-semibold tabular-nums">
                          {skillsByAgent[countAgentId] ?? 0}
                        </span>
                        <ChevronRight className="size-4 text-muted-foreground" />
                      </button>
                    );
                  })}
                </div>
              ) : (
                <div className="px-4 py-6 text-sm text-muted-foreground">
                  {t("sidebar.noPlatforms")}
                </div>
              )}
            </CardContent>
          </Card>
        </div>

        <Card className="overflow-hidden border-border/90 bg-card/95">
          <PanelHeader
            title={t("dashboard.workQueues", {
              count: activeQueueItems.length,
            })}
            description={t("dashboard.workQueuesDesc")}
          />
          <CardContent className="p-0">
            {activeQueueItems.length > 0 ? (
              <div>
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
              <div className="px-4 py-5 text-sm text-muted-foreground">
                {t("dashboard.queue.empty")}
              </div>
            )}

            {activeJob && (
              <div className="border-t border-border/70 bg-muted/20 px-4 py-3 text-xs text-muted-foreground">
                <span className="font-medium text-foreground">
                  {t("dashboard.activeJobs")}
                </span>
                <span className="ml-2">
                  {t("dashboard.jobProgress", {
                    completed: activeJob.completed,
                    total: activeJob.total,
                  })}
                </span>
              </div>
            )}
          </CardContent>
        </Card>

        <div className="grid gap-5 xl:grid-cols-[minmax(0,1.25fr)_minmax(22rem,0.9fr)]">
          <Card className="overflow-hidden border-border/90 bg-card/95">
            <PanelHeader
              title={t("dashboard.recentLogs")}
              description={t("dashboard.recentLogsDesc", { count: logTotal })}
              action={
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => navigate("/logs")}
                >
                  {t("dashboard.viewAllLogs")}
                  <ChevronRight className="size-3.5" />
                </Button>
              }
            />
            <CardContent className="px-4 py-3">
              {isLogsLoading ? (
                <div className="flex items-center gap-2 rounded-md border border-border/80 bg-background px-3 py-4 text-sm text-muted-foreground">
                  <Loader2 className="size-4 animate-spin" />
                  {t("dashboard.loadingLogs")}
                </div>
              ) : recentLogs.length > 0 ? (
                <div className="divide-y divide-border/70">
                  {recentLogs.map((entry) => (
                    <LogRow
                      key={entry.id}
                      entry={entry}
                      statusLabel={t(`logs.status.${entry.status}`, {
                        defaultValue: entry.status,
                      })}
                    />
                  ))}
                </div>
              ) : (
                <div className="rounded-md border border-border/80 bg-background px-3 py-4 text-sm text-muted-foreground">
                  {t("dashboard.noRecentLogs")}
                </div>
              )}
            </CardContent>
          </Card>

          <Card className="overflow-hidden border-border/90 bg-card/95">
            <PanelHeader
              title={t("dashboard.activity.title")}
              description={t("dashboard.activity.description")}
              action={
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => navigate("/logs")}
                >
                  {t("dashboard.activity.logs")}
                  <ChevronRight className="size-3.5" />
                </Button>
              }
            />
            <CardContent className="space-y-4 p-4">
              <div className="flex items-baseline justify-between gap-3">
                <div>
                  <span className="text-3xl font-semibold tabular-nums tracking-tight">
                    {activity.total}
                  </span>
                  <span className="ml-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">
                    {t("dashboard.activity.ops")}
                  </span>
                </div>
                <span className="text-xs text-muted-foreground">
                  {activity.startLabel} - {activity.endLabel}
                </span>
              </div>

              <div
                className="grid grid-cols-[repeat(14,minmax(0,1fr))] gap-1"
                aria-hidden="true"
              >
                {activity.buckets.map((bucket) => (
                  <div
                    key={bucket.key}
                    className={cn(
                      "aspect-square rounded-sm border border-border/40",
                      heatCellClass(bucket.count, activity.max)
                    )}
                    title={`${bucket.key}: ${bucket.count}`}
                  />
                ))}
              </div>
              <div className="flex items-center justify-between gap-3 text-[0.68rem] text-muted-foreground">
                <span>{t("dashboard.activity.less")}</span>
                <span>{t("dashboard.activity.more")}</span>
              </div>

              <div className="border-t border-border/70 pt-4">
                <div className="mb-2 text-[0.68rem] font-medium uppercase tracking-wide text-muted-foreground">
                  {t("dashboard.activity.topTags")}
                </div>
                {topTags.length > 0 ? (
                  <div className="flex flex-wrap gap-2">
                    {topTags.map((tag, index) => (
                      <span
                        key={tag.id}
                        className={cn(
                          "inline-flex items-center gap-2 rounded-full border px-2.5 py-1 text-xs",
                          index === 0
                            ? "border-primary/30 bg-primary/10 text-primary"
                            : "border-border bg-background text-muted-foreground"
                        )}
                      >
                        <span className="max-w-28 truncate">{tag.name}</span>
                        <span className="font-mono tabular-nums">{tag.count}</span>
                      </span>
                    ))}
                  </div>
                ) : (
                  <div className="rounded-md border border-border/80 bg-background px-3 py-3 text-sm text-muted-foreground">
                    {t("dashboard.activity.noTags")}
                  </div>
                )}
              </div>
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  );
}
