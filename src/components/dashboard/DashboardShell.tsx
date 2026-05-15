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
  ScrollText,
  Server,
  Store,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import {
  LogRow,
  PanelHeader,
  ProgressRow,
  QueueRow,
  StatButton,
  StatusTile,
} from "@/components/dashboard/DashboardPanels";
import { PlatformIcon } from "@/components/platform/PlatformIcon";
import {
  getPlatformTargetPathHint,
  isUniversalPlatformTarget,
} from "@/lib/platformTargetGroups";
import { cn } from "@/lib/utils";
import { heatCellClass } from "@/pages/dashboardUtils";
import type { DashboardViewModel } from "@/pages/dashboardViewModel";

interface DashboardShellProps {
  onNavigate: (path: string) => void;
  viewModel: DashboardViewModel;
}

export function DashboardShell({
  onNavigate,
  viewModel,
}: DashboardShellProps) {
  const { t } = useTranslation();

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
              onClick={() => onNavigate("/central")}
            >
              <Blocks className="size-4" />
              {t("sidebar.centralSkills")}
            </Button>
            <Button
              variant="outline"
              size="sm"
              onClick={() => onNavigate("/logs")}
            >
              <ScrollText className="size-4" />
              {t("logs.title")}
            </Button>
            <Button
              variant="outline"
              size="sm"
              data-testid="dashboard-action-marketplace"
              onClick={() => onNavigate("/marketplace")}
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
              value={viewModel.targetLabel}
              detail={viewModel.targetDescription}
            />
            <StatusTile
              icon={<Database className="size-4" />}
              label={t("dashboard.centralPath")}
              value={viewModel.centralPath}
            />
            <StatusTile
              icon={<Clock3 className="size-4" />}
              label={t("dashboard.lastScan")}
              value={viewModel.lastScanLabel}
            />
            <StatusTile
              icon={
                viewModel.isPlatformLoading || viewModel.isPlatformRefreshing ? (
                  <Loader2 className="size-4 animate-spin" />
                ) : viewModel.scanState === "error" ? (
                  <AlertCircle className="size-4 text-destructive" />
                ) : (
                  <CheckCircle2 className="size-4" />
                )
              }
              label={t("dashboard.scanStateLabel")}
              value={viewModel.scanStateLabel}
              detail={
                viewModel.isPlatformRefreshing
                  ? t("dashboard.scanStateDetail.refreshing")
                  : t("dashboard.scanStateDetail.idle")
              }
            />
          </div>
        </Card>

        {viewModel.loadError && (
          <div
            className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive"
            title={viewModel.loadError}
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
                  onClick={() => onNavigate("/central")}
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
                      {viewModel.centralTotal}
                    </span>
                    <span className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
                      {t("dashboard.health.centralUnit")}
                    </span>
                  </div>
                  <p className="mt-3 max-w-xl break-words text-sm leading-6 text-muted-foreground">
                    {viewModel.healthSummary}
                  </p>
                </div>
                <div className="space-y-3">
                  <ProgressRow
                    testId="dashboard-metric-uncategorized"
                    label={t("dashboard.metrics.uncategorized")}
                    count={viewModel.uncategorizedCount}
                    total={viewModel.centralTotal}
                    tone="primary"
                  />
                  <ProgressRow
                    testId="dashboard-metric-ai"
                    label={t("dashboard.metrics.aiReviews")}
                    count={viewModel.aiReviewCount}
                    total={viewModel.centralTotal}
                    tone="primary"
                  />
                  <ProgressRow
                    testId="dashboard-metric-updates"
                    label={t("dashboard.metrics.updates")}
                    count={viewModel.updatesAvailable}
                    total={viewModel.centralTotal}
                    tone="primary"
                  />
                  <ProgressRow
                    testId="dashboard-metric-unassigned"
                    label={t("dashboard.metrics.unassigned")}
                    count={viewModel.unassignedSourceCount}
                    total={viewModel.centralTotal}
                    tone="muted"
                  />
                </div>
              </div>
              <div className="grid min-w-0 gap-3 sm:grid-cols-2">
                <StatButton
                  testId="dashboard-metric-collections"
                  icon={<Layers className="size-4" />}
                  label={t("dashboard.metrics.collections")}
                  value={viewModel.resolvedCollectionCount}
                  description={t("dashboard.metrics.collectionsDesc")}
                  onClick={() => onNavigate("/collections")}
                />
                <StatButton
                  icon={<FolderGit2 className="size-4" />}
                  label={t("dashboard.health.sources")}
                  value={viewModel.sourceRepositoryCount}
                  description={t("dashboard.health.sourcesDesc", {
                    count: viewModel.registriesCount,
                  })}
                  onClick={() => onNavigate("/marketplace")}
                />
                <StatButton
                  testId="dashboard-metric-targets"
                  icon={<Server className="size-4" />}
                  label={t("dashboard.metrics.enabledTargets")}
                  value={viewModel.enabledTargets.length}
                  description={t("dashboard.metrics.enabledTargetsDesc")}
                  onClick={() => onNavigate("/settings")}
                  emphasized={viewModel.enabledTargets.length > 0}
                />
              </div>
            </CardContent>
          </Card>

          <Card className="overflow-hidden border-border/90 bg-card/95">
            <PanelHeader
              title={t("dashboard.platforms.title", {
                enabled: viewModel.enabledTargets.length,
                total: viewModel.visiblePlatformTargets.length,
              })}
              description={t("dashboard.platforms.description")}
              action={
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => onNavigate("/settings")}
                >
                  {t("dashboard.platforms.manage")}
                  <ChevronRight className="size-3.5" />
                </Button>
              }
            />
            <CardContent className="p-0">
              {viewModel.visiblePlatformTargets.length > 0 ? (
                <div>
                  {viewModel.visiblePlatformTargets.slice(0, 7).map((agent) => {
                    const countAgentId = isUniversalPlatformTarget(agent)
                      ? agent.install_agent_id
                      : agent.id;
                    const pathHint = getPlatformTargetPathHint(agent);

                    return (
                      <button
                        key={agent.id}
                        type="button"
                        onClick={() => onNavigate(`/platform/${agent.id}`)}
                        className="grid w-full grid-cols-[2rem_minmax(0,1fr)_auto_auto] items-center gap-3 border-t border-border/70 px-4 py-3 text-left transition-colors first:border-t-0 hover:bg-muted/25"
                        aria-label={t("dashboard.platforms.openLabel", {
                          name: isUniversalPlatformTarget(agent)
                            ? t("platformTargets.universalShortLabel")
                            : agent.display_name,
                        })}
                      >
                        <span className="grid size-8 place-items-center rounded-md border border-border/80 bg-background text-primary">
                          <PlatformIcon agentId={agent.id} className="size-4" />
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
                          {viewModel.skillsByAgent[countAgentId] ?? 0}
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
              count: viewModel.activeQueueItems.length,
            })}
            description={t("dashboard.workQueuesDesc")}
          />
          <CardContent className="p-0">
            {viewModel.activeQueueItems.length > 0 ? (
              <div>
                {viewModel.activeQueueItems.map((item) => (
                  <QueueRow
                    key={item.key}
                    label={item.label}
                    count={item.count}
                    description={item.description}
                    onClick={() => onNavigate("/central")}
                  />
                ))}
              </div>
            ) : (
              <div className="px-4 py-5 text-sm text-muted-foreground">
                {t("dashboard.queue.empty")}
              </div>
            )}

            {viewModel.activeJob && (
              <div className="border-t border-border/70 bg-muted/20 px-4 py-3 text-xs text-muted-foreground">
                <span className="font-medium text-foreground">
                  {t("dashboard.activeJobs")}
                </span>
                <span className="ml-2">
                  {t("dashboard.jobProgress", {
                    completed: viewModel.activeJob.completed,
                    total: viewModel.activeJob.total,
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
              description={t("dashboard.recentLogsDesc", {
                count: viewModel.logTotal,
              })}
              action={
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => onNavigate("/logs")}
                >
                  {t("dashboard.viewAllLogs")}
                  <ChevronRight className="size-3.5" />
                </Button>
              }
            />
            <CardContent className="px-4 py-3">
              {viewModel.isLogsLoading ? (
                <div className="flex items-center gap-2 rounded-md border border-border/80 bg-background px-3 py-4 text-sm text-muted-foreground">
                  <Loader2 className="size-4 animate-spin" />
                  {t("dashboard.loadingLogs")}
                </div>
              ) : viewModel.recentLogs.length > 0 ? (
                <div className="divide-y divide-border/70">
                  {viewModel.recentLogs.map((entry) => (
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
                  onClick={() => onNavigate("/logs")}
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
                    {viewModel.activity.total}
                  </span>
                  <span className="ml-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">
                    {t("dashboard.activity.ops")}
                  </span>
                </div>
                <span className="text-xs text-muted-foreground">
                  {viewModel.activity.startLabel} - {viewModel.activity.endLabel}
                </span>
              </div>

              <div
                className="grid grid-cols-[repeat(14,minmax(0,1fr))] gap-1"
                aria-hidden="true"
              >
                {viewModel.activity.buckets.map((bucket) => (
                  <div
                    key={bucket.key}
                    className={cn(
                      "aspect-square rounded-sm border border-border/40",
                      heatCellClass(bucket.count, viewModel.activity.max),
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
                {viewModel.topTags.length > 0 ? (
                  <div className="flex flex-wrap gap-2">
                    {viewModel.topTags.map((tag, index) => (
                      <span
                        key={tag.id}
                        className={cn(
                          "inline-flex items-center gap-2 rounded-full border px-2.5 py-1 text-xs",
                          index === 0
                            ? "border-primary/30 bg-primary/10 text-primary"
                            : "border-border bg-background text-muted-foreground",
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
