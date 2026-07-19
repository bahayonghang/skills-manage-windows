import { useMemo } from "react";
import { useTranslation } from "react-i18next";

import { Card, CardContent } from "@/components/ui/card";
import { PanelHeader } from "@/components/dashboard/DashboardPanels";
import {
  buildActivitySummary,
  heatCellClass,
} from "@/pages/dashboardUtils";
import { OperationLogEntry } from "@/types";
import { cn } from "@/lib/utils";

interface LogsActivityCardProps {
  entries: OperationLogEntry[];
  selectedDayKey: string | null;
  onSelectDay: (dayKey: string | null, date: Date) => void;
}

export function LogsActivityCard({
  entries,
  selectedDayKey,
  onSelectDay,
}: LogsActivityCardProps) {
  const { t } = useTranslation();
  const activity = useMemo(() => buildActivitySummary(entries), [entries]);

  return (
    <Card className="overflow-hidden border-border/90 bg-card/95 gap-0 py-0">
      <PanelHeader
        title={t("logs.activity.title")}
        description={t("logs.activity.description", {
          count: activity.total,
        })}
      />
      <CardContent className="space-y-3 px-4 py-4">
        <div className="flex items-baseline justify-between text-xs text-muted-foreground">
          <span>{activity.startLabel}</span>
          <span>{activity.endLabel}</span>
        </div>
        <div
          role="group"
          aria-label={t("logs.activity.title")}
          className="grid grid-cols-[repeat(14,minmax(0,1fr))] gap-1"
        >
          {activity.buckets.map((bucket) => {
            const isActive = selectedDayKey === bucket.key;
            return (
              <button
                key={bucket.key}
                type="button"
                aria-pressed={isActive}
                aria-label={t("logs.activity.dayLabel", {
                  date: bucket.key,
                  count: bucket.count,
                })}
                title={`${bucket.key}: ${bucket.count}`}
                onClick={() =>
                  onSelectDay(isActive ? null : bucket.key, bucket.date)
                }
                className={cn(
                  "aspect-square rounded-sm border transition-colors",
                  heatCellClass(bucket.count, activity.max),
                  isActive
                    ? "border-primary ring-2 ring-primary/40"
                    : "border-border/40 hover:border-primary/60",
                )}
              />
            );
          })}
        </div>
        <div className="flex items-center justify-between text-ui-meta text-muted-foreground">
          <span>{t("logs.activity.less")}</span>
          <span>{t("logs.activity.more")}</span>
        </div>
      </CardContent>
    </Card>
  );
}
