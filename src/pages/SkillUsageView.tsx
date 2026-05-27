import { useTranslation } from "react-i18next";
import { Loader2, RefreshCw, BarChart3, Globe2, Monitor, AlertTriangle } from "lucide-react";

import { Button } from "@/components/ui/button";
import { ActivityHeatmap } from "@/components/usage/ActivityHeatmap";
import { KpiStrip } from "@/components/usage/KpiStrip";
import { ProviderHealthList } from "@/components/usage/ProviderHealthList";
import { RecentCallsFeed } from "@/components/usage/RecentCallsFeed";
import { SkillBarChart } from "@/components/usage/SkillBarChart";
import { useUsageBindings, useUsageBootstrap } from "@/pages/skillUsageBindings";
import { timeAgo } from "@/lib/timeAgo";
import { cn } from "@/lib/utils";
import type { UsageScopeInfo } from "@/types/usage";

/**
 * Skill Usage 页面 lean 入口。Task 7 起接入真实 KpiStrip / ProviderHealthList，
 * Task 8-10 会继续把 SkillBarChart / ActivityHeatmap / RecentCallsFeed 替换进来。
 */
export function SkillUsageView() {
  const { t } = useTranslation();
  useUsageBootstrap();
  const { overview, providers, recent, scope, refreshing, error, lastRefreshMs, refresh } =
    useUsageBindings();

  const kpis = overview?.kpis ?? {
    totalCalls: 0,
    uniqueSkills: 0,
    uniqueProjects: 0,
    uniqueSources: 0,
  };

  return (
    <div className="flex h-full min-h-0 flex-col bg-background">
      {/* Header */}
      <div className="flex items-center justify-between border-b border-border px-5 py-4">
        <div className="flex items-center gap-3">
          <BarChart3 className="size-5 text-primary" />
          <h1 className="text-lg font-semibold">{t("skillUsage.title")}</h1>
          {scope && <ScopeBadge scope={scope} />}
        </div>
        <div className="flex items-center gap-3">
          <span className="text-xs text-muted-foreground">
            {lastRefreshMs
              ? t("skillUsage.lastRefreshed", { time: timeAgo(lastRefreshMs) })
              : t("skillUsage.neverRefreshed")}
          </span>
          <Button
            size="sm"
            variant="outline"
            onClick={() => void refresh(true)}
            disabled={refreshing}
          >
            {refreshing ? (
              <Loader2 className="size-4 animate-spin" />
            ) : (
              <RefreshCw className="size-4" />
            )}
            <span className="ml-1.5">{t("skillUsage.refresh")}</span>
          </Button>
        </div>
      </div>

      {/* Body */}
      <div
        data-testid="skill-usage-scroll-region"
        className="scrollbar-subtle flex-1 overflow-y-auto p-5"
      >
        {error && (
          <div
            role="alert"
            className="mb-4 rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive"
          >
            {error}
          </div>
        )}
        {scope?.isRemote && !scope.remoteReachable && (
          <div
            role="alert"
            data-testid="remote-unreachable-banner"
            className="mb-4 flex items-start gap-2 rounded-md border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-sm text-amber-700 dark:text-amber-400"
          >
            <AlertTriangle className="mt-0.5 size-4 shrink-0" />
            <span>{t("skillUsage.remoteUnreachable", { host: scope.label })}</span>
          </div>
        )}

        <KpiStrip kpis={kpis} className="mb-5" />

        <div className="grid gap-5 lg:grid-cols-2">
          <Panel title={t("skillUsage.panels.topSkills")}>
            <div className="h-[28rem]">
              <SkillBarChart skills={overview?.topSkills ?? []} />
            </div>
          </Panel>
          <Panel title={t("skillUsage.panels.heatmap")}>
            <div className="h-[28rem]">
              <ActivityHeatmap days={overview?.heatmap ?? []} className="h-full" />
            </div>
          </Panel>
          <Panel title={t("skillUsage.panels.recent")}>
            <RecentCallsFeed calls={recent} />
          </Panel>
          <Panel title={t("skillUsage.panels.providers")}>
            <ProviderHealthList providers={providers} />
          </Panel>
        </div>
      </div>
    </div>
  );
}

function Panel({
  title,
  children,
}: {
  title: string;
  children: React.ReactNode;
}) {
  return (
    <div className="rounded-lg border border-border bg-card p-4">
      <h2 className="mb-3 text-sm font-semibold">{title}</h2>
      {children}
    </div>
  );
}

function ScopeBadge({ scope }: { scope: UsageScopeInfo }) {
  const Icon = scope.isRemote ? Globe2 : Monitor;
  const tone = scope.isRemote
    ? scope.remoteReachable
      ? "border-emerald-500/40 bg-emerald-500/10 text-emerald-600 dark:text-emerald-400"
      : "border-amber-500/40 bg-amber-500/10 text-amber-600 dark:text-amber-400"
    : "border-border bg-muted text-muted-foreground";
  return (
    <span
      data-testid="scope-badge"
      data-scope-kind={scope.isRemote ? "remote" : "local"}
      className={cn(
        "inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-xs",
        tone
      )}
      title={scope.isRemote ? scope.label : "Local target"}
    >
      <Icon className="size-3" />
      <span className="max-w-[14rem] truncate">
        {scope.isRemote ? `Remote (${scope.label})` : "Local"}
      </span>
    </span>
  );
}
