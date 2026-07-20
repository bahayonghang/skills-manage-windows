import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { ChevronRight } from "lucide-react";

import {
  ChartStateRow,
  PanelHeader,
} from "@/components/dashboard/DashboardPanels";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import type { DailyOperationCount } from "@/types";

interface ActivityPanelProps {
  onNavigate: (path: string) => void;
  dailyCounts: DailyOperationCount[];
  isLoading: boolean;
  error: string | null;
  onRetry: () => void;
}

const CHART_WIDTH = 560;
const CHART_HEIGHT = 120;
const CHART_PADDING_X = 4;
const BAR_GAP = 6;
/** 零值日渲染高度：基线细条（低不透明度），保证 14 桶全部可见。 */
const MIN_BAR_HEIGHT = 2;
const MAX_BAR_HEIGHT = CHART_HEIGHT - 8;

const dayLabelFormatter = new Intl.DateTimeFormat(undefined, {
  month: "short",
  day: "2-digit",
});

function formatDayLabel(dateKey: string) {
  const date = new Date(`${dateKey}T00:00:00`);
  if (Number.isNaN(date.getTime())) return dateKey;
  return dayLabelFormatter.format(date);
}

function localTodayKey() {
  const now = new Date();
  const month = `${now.getMonth() + 1}`.padStart(2, "0");
  const day = `${now.getDate()}`.padStart(2, "0");
  return `${now.getFullYear()}-${month}-${day}`;
}

export function ActivityPanel({
  onNavigate,
  dailyCounts,
  isLoading,
  error,
  onRetry,
}: ActivityPanelProps) {
  const { t } = useTranslation();
  const total = useMemo(
    () => dailyCounts.reduce((sum, bucket) => sum + bucket.count, 0),
    [dailyCounts],
  );
  const maxCount = useMemo(
    () => Math.max(0, ...dailyCounts.map((bucket) => bucket.count)),
    [dailyCounts],
  );
  const todayKey = localTodayKey();
  const startLabel =
    dailyCounts.length > 0 ? formatDayLabel(dailyCounts[0].date) : "";
  const endLabel =
    dailyCounts.length > 0
      ? formatDayLabel(dailyCounts[dailyCounts.length - 1].date)
      : "";

  const barCount = dailyCounts.length;
  const barWidth =
    barCount > 0
      ? (CHART_WIDTH - CHART_PADDING_X * 2 - BAR_GAP * (barCount - 1)) /
        barCount
      : 0;

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
        {isLoading ? (
          <ChartStateRow kind="loading" />
        ) : error ? (
          <ChartStateRow kind="error" onRetry={onRetry} />
        ) : barCount === 0 ? (
          <div className="rounded-md border border-border/80 bg-background px-3 py-4 text-sm text-muted-foreground">
            {t("dashboard.activity.empty")}
          </div>
        ) : (
          <>
            <div className="flex items-baseline justify-between gap-3">
              <div>
                <span className="font-display text-3xl font-semibold tabular-nums">
                  {total}
                </span>
                <span className="ml-2 text-xs font-medium uppercase tracking-wide text-muted-foreground">
                  {t("dashboard.activity.ops")}
                </span>
              </div>
              <span className="text-xs text-muted-foreground">
                {startLabel} - {endLabel}
              </span>
            </div>
            <svg
              viewBox={`0 0 ${CHART_WIDTH} ${CHART_HEIGHT}`}
              role="img"
              aria-label={t("dashboard.activity.chartAria", {
                days: barCount,
                total,
              })}
              className="h-28 w-full text-primary-text"
            >
              {dailyCounts.map((bucket, index) => {
                const height =
                  bucket.count <= 0 || maxCount <= 0
                    ? MIN_BAR_HEIGHT
                    : Math.max(
                        MIN_BAR_HEIGHT + 2,
                        (bucket.count / maxCount) * MAX_BAR_HEIGHT,
                      );
                const x = CHART_PADDING_X + index * (barWidth + BAR_GAP);
                const y = CHART_HEIGHT - height;
                const isToday = bucket.date === todayKey;

                return (
                  <g key={bucket.date}>
                    <rect
                      x={x}
                      y={y}
                      width={barWidth}
                      height={height}
                      rx={2}
                      fill="currentColor"
                      opacity={bucket.count > 0 ? 1 : 0.3}
                      aria-current={isToday ? "date" : undefined}
                    >
                      <title>
                        {t("dashboard.activity.barTitle", {
                          date: formatDayLabel(bucket.date),
                          count: bucket.count,
                        })}
                      </title>
                    </rect>
                    {isToday && (
                      <rect
                        aria-hidden="true"
                        x={Math.max(0, x - 2)}
                        y={Math.max(0, y - 2)}
                        width={barWidth + 4}
                        height={height + 4}
                        rx={3}
                        fill="none"
                        stroke="currentColor"
                        strokeWidth={1.5}
                        pointerEvents="none"
                      />
                    )}
                  </g>
                );
              })}
            </svg>
          </>
        )}
      </CardContent>
    </Card>
  );
}
