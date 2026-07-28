import { useRef } from "react";
import { useTranslation } from "react-i18next";
import {
  AlertTriangle,
  BarChart3,
  ChevronDown,
  Globe2,
  Loader2,
  Monitor,
  RefreshCw,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import { ActivityHeatmap } from "@/components/usage/ActivityHeatmap";
import { PlatformFilterBar } from "@/components/usage/PlatformFilterBar";
import { ProviderHealthList } from "@/components/usage/ProviderHealthList";
import { RecentCallsFeed } from "@/components/usage/RecentCallsFeed";
import { SkillUsageDetailPanel } from "@/components/usage/SkillUsageDetailPanel";
import { SkillUsageTable } from "@/components/usage/SkillUsageTable";
import { UsageMetricStrip } from "@/components/usage/UsageMetricStrip";
import { formatUsageRelativeTime } from "@/components/usage/usageFormat";
import { cn } from "@/lib/utils";
import {
  useUsageBindings,
  useUsageBootstrap,
} from "@/pages/skillUsageBindings";
import type { UsageScopeInfo } from "@/types/usage";

export function SkillUsageView() {
  const { t, i18n } = useTranslation();
  const detailTriggerRef = useRef<HTMLButtonElement | null>(null);
  useUsageBootstrap();
  const {
    overview,
    providers,
    recent,
    detail,
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
    refresh,
    selectSource,
    loadDetail,
    clearDetail,
  } = useUsageBindings();

  const kpis = overview?.kpis ?? {
    totalCalls: 0,
    uniqueSkills: 0,
    uniqueProjects: 0,
    uniqueSources: 0,
    uniqueSessions: 0,
  };
  const initialLoading = overview === null && error === null;
  const availableProviders = providers.filter(
    (provider) => provider.available,
  ).length;

  const selectSkill = (skill: string, trigger: HTMLButtonElement) => {
    detailTriggerRef.current = trigger;
    void loadDetail(skill);
  };
  const closeDetail = () => {
    clearDetail();
    requestAnimationFrame(() => detailTriggerRef.current?.focus());
  };

  return (
    <div className="flex h-full min-h-0 flex-col bg-background">
      <header className="border-b border-border px-4 py-3 sm:px-5">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="flex min-w-0 items-center gap-3">
            <BarChart3 className="size-5 shrink-0 text-primary" />
            <div className="min-w-0">
              <h1 className="truncate text-base font-semibold text-foreground">
                {t("skillUsage.title")}
              </h1>
              <p className="text-xs text-muted-foreground">
                {t("skillUsage.range.allRecorded")}
              </p>
            </div>
            {scope && <ScopeBadge scope={scope} />}
          </div>
          <div className="flex items-center gap-2">
            <span className="hidden text-xs text-muted-foreground sm:inline">
              {lastRefreshMs
                ? t("skillUsage.lastRefreshed", {
                    time: formatUsageRelativeTime(lastRefreshMs, i18n.language),
                  })
                : t("skillUsage.neverRefreshed")}
            </span>
            <Button
              type="button"
              size="icon-sm"
              variant="outline"
              onClick={() => void refresh(true)}
              disabled={refreshing}
              title={t("skillUsage.refresh")}
              aria-label={t("skillUsage.refresh")}
            >
              {refreshing ? (
                <Loader2 className="size-4 animate-spin motion-reduce:animate-none" />
              ) : (
                <RefreshCw className="size-4" />
              )}
            </Button>
          </div>
        </div>

        <PlatformFilterBar
          providers={providers}
          selected={selectedSource}
          onSelect={(source) => void selectSource(source)}
          className="mt-3"
        />
      </header>

      <main
        data-testid="skill-usage-scroll-region"
        className="scrollbar-subtle flex-1 overflow-y-auto px-4 py-4 sm:px-5"
      >
        {error && !overview && (
          <StatusBanner tone="error">{error}</StatusBanner>
        )}
        {refreshError && usedCachedData && (
          <StatusBanner tone="warning">
            {t("skillUsage.showingCachedAfterError")}
            {lastRefreshMs
              ? ` · ${t("skillUsage.lastRefreshed", {
                  time: formatUsageRelativeTime(lastRefreshMs, i18n.language),
                })}`
              : ""}
          </StatusBanner>
        )}
        {scope?.isRemote && !scope.remoteReachable && (
          <StatusBanner tone="warning" testId="remote-unreachable-banner">
            {t(
              usedCachedData
                ? "skillUsage.remoteUnreachableCached"
                : "skillUsage.remoteUnreachableEmpty",
              { host: scope.label },
            )}
          </StatusBanner>
        )}

        {!initialLoading && (
          <UsageMetricStrip
            kpis={kpis}
            singlePlatform={selectedSource !== null}
            className="mb-4"
          />
        )}

        {initialLoading ? (
          <UsageSkeleton />
        ) : (
          <div
            aria-busy={loading}
            className={cn(
              "grid min-w-0 gap-4 xl:grid-cols-[minmax(0,1.55fr)_minmax(22rem,0.85fr)]",
              loading && "opacity-70",
            )}
          >
            <UsageSection
              title={t("skillUsage.panels.topSkills")}
              className="min-h-[32rem] xl:row-span-3"
              fill
            >
              <SkillUsageTable
                skills={overview?.topSkills ?? []}
                selectedSkill={selectedSkill}
                onSelect={selectSkill}
                className="h-full min-h-[24rem]"
              />
            </UsageSection>

            {selectedSkill ? (
              <UsageSection title={t("skillUsage.panels.detail")}>
                <SkillUsageDetailPanel
                  detail={detail}
                  loading={detailLoading}
                  onClose={closeDetail}
                />
              </UsageSection>
            ) : (
              <UsageSection
                title={t("skillUsage.panels.recent")}
                range={t("skillUsage.range.latestCalls", { count: 20 })}
              >
                <div className="scrollbar-subtle max-h-[26rem] overflow-y-auto">
                  <RecentCallsFeed calls={recent} onSelect={selectSkill} />
                </div>
              </UsageSection>
            )}

            <UsageSection
              title={t("skillUsage.panels.heatmap")}
              range={t("skillUsage.range.lastWeeks", { count: 16 })}
            >
              <ActivityHeatmap days={overview?.heatmap ?? []} />
            </UsageSection>

            {selectedSkill && (
              <UsageSection
                title={t("skillUsage.panels.recent")}
                range={t("skillUsage.range.latestCalls", { count: 20 })}
              >
                <div className="scrollbar-subtle max-h-[26rem] overflow-y-auto">
                  <RecentCallsFeed calls={recent} onSelect={selectSkill} />
                </div>
              </UsageSection>
            )}
          </div>
        )}

        <details className="group mt-4 border-y border-border bg-muted/10">
          <summary className="flex cursor-pointer list-none items-center justify-between gap-3 px-3 py-3 text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring">
            <span className="font-medium text-foreground">
              {t("skillUsage.panels.providers")}
            </span>
            <span className="flex items-center gap-2 text-xs text-muted-foreground">
              {t("skillUsage.providerSummary", {
                available: availableProviders,
                total: providers.length,
              })}
              <ChevronDown className="size-4 transition-transform group-open:rotate-180" />
            </span>
          </summary>
          <ProviderHealthList
            providers={providers}
            activeSource={selectedSource}
            onSelect={(source) => void selectSource(source)}
            className="border-t border-border"
          />
        </details>
      </main>
    </div>
  );
}

function UsageSection({
  title,
  range,
  className,
  fill = false,
  children,
}: {
  title: string;
  range?: string;
  className?: string;
  /** true 时卡片纵向 flex，children 填满剩余高度（供表格内部滚动用） */
  fill?: boolean;
  children: React.ReactNode;
}) {
  return (
    <section
      className={cn(
        "min-w-0 overflow-hidden rounded-md border border-border bg-card",
        fill && "flex flex-col",
        className,
      )}
    >
      <div className="flex min-h-11 shrink-0 items-center justify-between gap-3 border-b border-border px-3 py-2">
        <h2 className="text-sm font-semibold text-foreground">{title}</h2>
        {range && (
          <span className="text-xs text-muted-foreground">{range}</span>
        )}
      </div>
      {fill ? <div className="min-h-0 flex-1">{children}</div> : children}
    </section>
  );
}

function StatusBanner({
  tone,
  testId,
  children,
}: {
  tone: "warning" | "error";
  testId?: string;
  children: React.ReactNode;
}) {
  return (
    <div
      role="alert"
      data-testid={testId}
      className={cn(
        "mb-4 flex items-start gap-2 rounded-md border px-3 py-2 text-sm",
        tone === "error"
          ? "border-destructive/35 bg-destructive/10 text-destructive-text"
          : "border-warning/35 bg-warning/10 text-warning-foreground",
      )}
    >
      <AlertTriangle className="mt-0.5 size-4 shrink-0" />
      <span>{children}</span>
    </div>
  );
}

function ScopeBadge({ scope }: { scope: UsageScopeInfo }) {
  const { t } = useTranslation();
  const Icon = scope.isRemote ? Globe2 : Monitor;
  return (
    <span
      data-testid="scope-badge"
      data-scope-kind={scope.isRemote ? "remote" : "local"}
      className="hidden max-w-56 items-center gap-1.5 rounded-md border border-border bg-muted/40 px-2 py-1 text-xs text-muted-foreground md:inline-flex"
      title={scope.label}
    >
      <Icon className="size-3 shrink-0" />
      <span className="truncate">
        {scope.isRemote
          ? t("skillUsage.scope.remote", { label: scope.label })
          : t("skillUsage.scope.local")}
      </span>
    </span>
  );
}

function UsageSkeleton() {
  const { t } = useTranslation();
  return (
    <div className="relative">
      <div className="animate-pulse motion-reduce:animate-none">
        {/* KPI 条形态骨架：占位与 UsageMetricStrip 同形，避免真实组件渲染 0 值 */}
        <div className="mb-4 flex flex-wrap gap-x-10 gap-y-2 border-y border-border bg-muted/15 px-4 py-3">
          {[0, 1, 2, 3].map((index) => (
            <div key={index} className="space-y-1.5">
              <div className="h-3 w-14 rounded bg-muted/40" />
              <div className="h-6 w-20 rounded bg-muted/50" />
            </div>
          ))}
        </div>
        <div className="grid gap-4 xl:grid-cols-[minmax(0,1.55fr)_minmax(22rem,0.85fr)]">
          <div className="h-[34rem] rounded-md border border-border bg-muted/30 xl:row-span-2" />
          <div className="h-56 rounded-md border border-border bg-muted/25" />
          <div className="h-64 rounded-md border border-border bg-muted/20" />
        </div>
      </div>
      <div
        role="status"
        className="absolute inset-0 z-10 flex items-center justify-center"
      >
        <span className="flex items-center gap-2 rounded-md border border-border bg-card px-4 py-2 text-sm text-muted-foreground shadow-sm">
          <Loader2
            aria-hidden
            className="size-4 animate-spin motion-reduce:animate-none"
          />
          {t("skillUsage.scanning")}
        </span>
      </div>
    </div>
  );
}
