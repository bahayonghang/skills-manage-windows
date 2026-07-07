import { useTranslation } from "react-i18next";
import { ChevronRight } from "lucide-react";

import { PanelHeader } from "@/components/dashboard/DashboardPanels";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { cn } from "@/lib/utils";
import { buildSparklinePath, heatCellClass } from "@/pages/dashboardUtils";
import type { DashboardViewModel } from "@/pages/dashboardViewModel";

interface ActivityPanelProps {
  onNavigate: (path: string) => void;
  activity: DashboardViewModel["activity"];
  sparkline: DashboardViewModel["sparkline"];
  topTags: DashboardViewModel["topTags"];
}

export function ActivityPanel({
  onNavigate,
  activity,
  sparkline,
  topTags,
}: ActivityPanelProps) {
  const { t } = useTranslation();
  const sparkPath = buildSparklinePath(sparkline.points, {
    width: 220,
    height: 78,
    padding: 6,
  });

  return (
    <Card className="surface-glass overflow-hidden rounded-2xl border-0 bg-transparent p-0 shadow-none">
      <PanelHeader
        title={t("dashboard.activity.title")}
        description={t("dashboard.activity.description")}
        action={
          <Button variant="ghost" size="sm" onClick={() => onNavigate("/logs")}>
            {t("dashboard.activity.logs")}
            <ChevronRight className="size-3.5" />
          </Button>
        }
      />
      <CardContent className="space-y-4 p-4">
        <div className="flex items-baseline justify-between gap-3">
          <div>
            <span className="font-display text-3xl font-semibold tabular-nums">
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
        <div className="grid items-center gap-4 lg:grid-cols-[minmax(0,1fr)_minmax(0,11rem)]">
          <div
            className="grid grid-cols-[repeat(14,minmax(0,1fr))] gap-1"
            aria-hidden="true"
          >
            {activity.buckets.map((bucket) => (
              <div
                key={bucket.key}
                className={cn(
                  "aspect-square rounded-sm border border-border/40",
                  heatCellClass(bucket.count, activity.max),
                )}
                title={`${bucket.key}: ${bucket.count}`}
              />
            ))}
          </div>
          {sparkPath ? (
            <svg
              viewBox="0 0 220 78"
              role="img"
              aria-label={t("dashboard.sparkline.ariaLabel")}
              className="h-20 w-full text-primary-text"
            >
              <path
                d={sparkPath}
                fill="none"
                stroke="currentColor"
                strokeWidth={2.5}
                strokeLinecap="round"
                strokeLinejoin="round"
              />
            </svg>
          ) : (
            <div className="grid h-20 place-items-center rounded-xl border border-dashed border-border/60 text-[0.7rem] text-muted-foreground">
              {t("dashboard.sparkline.empty")}
            </div>
          )}
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
                      ? "border-primary/30 bg-primary/10 text-primary-text"
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
  );
}
