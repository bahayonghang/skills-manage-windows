import { useTranslation } from "react-i18next";

import { cn } from "@/lib/utils";
import type { UsageKpis } from "@/types/usage";

interface UsageMetricStripProps {
  kpis: UsageKpis;
  singlePlatform?: boolean;
  className?: string;
}

export function UsageMetricStrip({
  kpis,
  singlePlatform = false,
  className,
}: UsageMetricStripProps) {
  const { t } = useTranslation();
  const metrics = [
    [t("skillUsage.kpi.calls"), kpis.totalCalls],
    [t("skillUsage.kpi.skills"), kpis.uniqueSkills],
    [t("skillUsage.kpi.projects"), kpis.uniqueProjects],
    [
      t(singlePlatform ? "skillUsage.kpi.sessions" : "skillUsage.kpi.sources"),
      singlePlatform ? kpis.uniqueSessions : kpis.uniqueSources,
    ],
  ] as const;

  return (
    <section
      aria-label={t("skillUsage.range.allRecorded")}
      className={cn("border-y border-border bg-muted/15 px-4 py-3", className)}
    >
      <div className="flex flex-wrap gap-y-2">
        {metrics.map(([label, value], index) => (
          <div
            key={label}
            className={cn(
              "min-w-0 px-5 first:pl-0",
              index > 0 && "border-l border-border",
            )}
          >
            <div className="truncate text-xs text-muted-foreground">
              {label}
            </div>
            <div className="mt-0.5 text-xl font-semibold tabular-nums text-foreground">
              {value.toLocaleString()}
            </div>
          </div>
        ))}
      </div>
    </section>
  );
}
