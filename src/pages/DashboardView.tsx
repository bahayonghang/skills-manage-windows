import { useEffect, useMemo, useRef, type ReactNode } from "react";
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
import type { OperationLogEntry, SkillWithLinks } from "@/types";

const RECENT_LOG_LIMIT = 5;
const ACTIVITY_DAY_COUNT = 14;
const TOP_TAG_LIMIT = 6;

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

function formatTime(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat(undefined, {
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

function formatDateLabel(value: Date) {
  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "2-digit",
  }).format(value);
}

function startOfUtcDay(value: Date) {
  const date = new Date(value);
  date.setUTCHours(0, 0, 0, 0);
  return date;
}

function dayKey(value: Date) {
  return startOfUtcDay(value).toISOString().slice(0, 10);
}

function ratio(count: number, total: number) {
  if (total <= 0) return 0;
  return Math.min(1, Math.max(0, count / total));
}

function logStatusClass(status: string) {
  switch (status) {
    case "failed":
      return "border-destructive/30 bg-destructive/10 text-destructive";
    case "partial":
    case "cancelled":
      return "border-primary/30 bg-primary/10 text-primary";
    default:
      return "border-border bg-muted/40 text-muted-foreground";
  }
}

function logDotClass(status: string) {
  switch (status) {
    case "failed":
      return "bg-destructive";
    case "partial":
    case "cancelled":
      return "bg-primary";
    default:
      return "bg-muted-foreground";
  }
}

function heatCellClass(count: number, max: number) {
  if (count <= 0) return "bg-muted/45";
  const level = Math.ceil((count / Math.max(max, 1)) * 4);
  if (level >= 4) return "bg-primary";
  if (level === 3) return "bg-primary/70";
  if (level === 2) return "bg-primary/40";
  return "bg-primary/20";
}

function buildActivitySummary(entries: OperationLogEntry[]) {
  const validDates = entries
    .map((entry) => new Date(entry.createdAt))
    .filter((date) => !Number.isNaN(date.getTime()));
  const endDate = startOfUtcDay(
    validDates.length > 0
      ? new Date(Math.max(...validDates.map((date) => date.getTime())))
      : new Date()
  );
  const startDate = new Date(endDate);
  startDate.setUTCDate(endDate.getUTCDate() - (ACTIVITY_DAY_COUNT - 1));

  const buckets = Array.from({ length: ACTIVITY_DAY_COUNT }, (_, index) => {
    const date = new Date(startDate);
    date.setUTCDate(startDate.getUTCDate() + index);
    return {
      key: dayKey(date),
      date,
      count: 0,
    };
  });
  const indexByKey = new Map(buckets.map((bucket, index) => [bucket.key, index]));

  for (const entry of entries) {
    const date = new Date(entry.createdAt);
    if (Number.isNaN(date.getTime())) continue;
    const index = indexByKey.get(dayKey(date));
    if (index === undefined) continue;
    buckets[index].count += 1;
  }

  const total = buckets.reduce((sum, bucket) => sum + bucket.count, 0);
  const max = buckets.reduce((current, bucket) => Math.max(current, bucket.count), 0);

  return {
    buckets,
    endLabel: formatDateLabel(endDate),
    max,
    startLabel: formatDateLabel(startDate),
    total,
  };
}

function buildTopTags(skills: SkillWithLinks[]) {
  const counts = new Map<string, { id: string; name: string; count: number }>();

  for (const skill of skills) {
    for (const tag of skill.tags ?? []) {
      if (tag.id === "uncategorized") continue;
      const existing = counts.get(tag.id);
      counts.set(tag.id, {
        id: tag.id,
        name: tag.name,
        count: (existing?.count ?? 0) + 1,
      });
    }
  }

  return [...counts.values()]
    .sort((left, right) => right.count - left.count || left.name.localeCompare(right.name))
    .slice(0, TOP_TAG_LIMIT);
}

function PanelHeader({
  title,
  description,
  action,
}: {
  title: string;
  description?: string;
  action?: ReactNode;
}) {
  return (
    <CardHeader className="border-b border-border/70 px-4 py-3">
      <div className="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
        <div className="min-w-0">
          <CardTitle className="truncate text-sm font-semibold">
            {title}
          </CardTitle>
          {description && (
            <CardDescription className="mt-1 max-w-full break-words text-xs leading-5">
              {description}
            </CardDescription>
          )}
        </div>
        {action && <div className="shrink-0 self-start">{action}</div>}
      </div>
    </CardHeader>
  );
}

function StatusTile({
  icon,
  label,
  value,
  detail,
  testId,
}: {
  icon: ReactNode;
  label: string;
  value: string;
  detail?: string;
  testId?: string;
}) {
  return (
    <div
      data-testid={testId}
      className="flex min-w-0 items-start gap-3 border-b border-border/70 bg-card px-3 py-3 last:border-b-0 sm:border-b-0 sm:border-r sm:last:border-r-0"
    >
      <span className="grid size-8 shrink-0 place-items-center rounded-md border border-border/80 bg-muted/40 text-primary">
        {icon}
      </span>
      <div className="min-w-0">
        <div className="text-[0.68rem] font-medium uppercase tracking-wide text-muted-foreground">
          {label}
        </div>
        <div className="mt-1 truncate text-sm font-semibold">{value}</div>
        {detail && (
          <div className="mt-1 truncate text-xs text-muted-foreground">
            {detail}
          </div>
        )}
      </div>
    </div>
  );
}

function ProgressRow({
  label,
  count,
  total,
  tone = "default",
  testId,
}: {
  label: string;
  count: number;
  total: number;
  tone?: "default" | "primary" | "muted" | "destructive";
  testId?: string;
}) {
  const scale = ratio(count, total);
  const fillClass =
    tone === "destructive"
      ? "bg-destructive"
      : tone === "muted"
        ? "bg-muted-foreground/55"
        : "bg-primary";

  return (
    <div
      data-testid={testId}
      className="grid grid-cols-[minmax(7.5rem,0.9fr)_minmax(4.5rem,1fr)_2.5rem] items-center gap-3"
    >
      <span className="min-w-0 text-xs font-medium leading-4 text-muted-foreground">
        {label}
      </span>
      <span className="h-1.5 overflow-hidden rounded-full bg-muted">
        <span
          className={cn("block h-full origin-left rounded-full", fillClass)}
          style={{ transform: `scaleX(${scale})` }}
        />
      </span>
      <span className="text-right text-xs font-semibold tabular-nums">
        {count}
      </span>
    </div>
  );
}

function StatButton({
  icon,
  label,
  value,
  description,
  onClick,
  testId,
  emphasized = false,
}: {
  icon: ReactNode;
  label: string;
  value: number | string;
  description: string;
  onClick: () => void;
  testId?: string;
  emphasized?: boolean;
}) {
  return (
    <button
      type="button"
      data-testid={testId}
      onClick={onClick}
      className={cn(
        "flex min-h-[6.75rem] min-w-0 flex-col rounded-md border px-3 py-3 text-left transition-colors",
        emphasized
          ? "border-primary/35 bg-primary/10 hover:border-primary/55"
          : "border-border/80 bg-background hover:border-primary/40 hover:bg-muted/25"
      )}
    >
      <span className="flex items-center justify-between gap-2 text-[0.68rem] font-medium uppercase tracking-wide text-muted-foreground">
        <span className={cn("truncate", emphasized && "text-primary")}>
          {label}
        </span>
        <span className={cn("shrink-0", emphasized && "text-primary")}>
          {icon}
        </span>
      </span>
      <span className="mt-2 text-2xl font-semibold tabular-nums tracking-tight">
        {value}
      </span>
      <span className="mt-1 line-clamp-2 text-xs leading-5 text-muted-foreground">
        {description}
      </span>
    </button>
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
      className="grid w-full grid-cols-[2.5rem_minmax(0,1fr)_auto] items-center gap-3 border-t border-border/70 px-4 py-3 text-left transition-colors first:border-t-0 hover:bg-muted/25"
    >
      <span className="grid size-9 place-items-center rounded-md border border-primary/25 bg-primary/10 text-sm font-semibold tabular-nums text-primary">
        {count}
      </span>
      <span className="min-w-0">
        <span className="block truncate text-sm font-medium">{label}</span>
        <span className="mt-1 block truncate text-xs text-muted-foreground">
          {description}
        </span>
      </span>
      <span className="inline-flex items-center gap-1 rounded-md border border-border bg-background px-2 py-1 text-xs font-medium text-muted-foreground">
        <ChevronRight className="size-3" />
      </span>
    </button>
  );
}

function LogRow({
  entry,
  statusLabel,
}: {
  entry: OperationLogEntry;
  statusLabel: string;
}) {
  return (
    <div className="grid grid-cols-[0.875rem_minmax(0,1fr)_auto] items-start gap-3 py-2">
      <span
        className={cn(
          "mt-1.5 size-2 rounded-full ring-4 ring-muted/35",
          logDotClass(entry.status)
        )}
      />
      <div className="min-w-0">
        <div className="truncate text-sm font-medium">{entry.summary}</div>
        <div className="mt-1 flex flex-wrap items-center gap-x-2 gap-y-1 text-xs text-muted-foreground">
          <span>{entry.action}</span>
          <span className="h-3 w-px bg-border" />
          <span>{entry.targetLabel ?? entry.targetKind}</span>
        </div>
      </div>
      <div className="flex shrink-0 items-center gap-2">
        <span
          className={cn(
            "rounded-md border px-2 py-0.5 text-[0.68rem] font-medium",
            logStatusClass(entry.status)
          )}
        >
          {statusLabel}
        </span>
        <span className="hidden w-12 text-right text-xs text-muted-foreground sm:block">
          {formatTime(entry.createdAt)}
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
