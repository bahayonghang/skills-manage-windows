import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { ChevronRight } from "lucide-react";

import { PanelHeader } from "@/components/dashboard/DashboardPanels";
import { Card, CardContent } from "@/components/ui/card";
import { cn } from "@/lib/utils";
import type { DashboardQueueItem } from "@/pages/dashboardViewModel";

type QueueFilter = "all" | "review" | "metadata";

interface WorkQueuePanelProps {
  onNavigate: (path: string) => void;
  queueItems: DashboardQueueItem[];
  activeJob: { completed: number; total: number } | null;
}

const FILTERS: { key: QueueFilter; labelKey: string }[] = [
  { key: "all", labelKey: "dashboard.queue.tabs.all" },
  { key: "review", labelKey: "dashboard.queue.tabs.review" },
  { key: "metadata", labelKey: "dashboard.queue.tabs.metadata" },
];

const dashboardControlMotion =
  "transition-[scale,background-color,border-color,color] duration-150 ease-out active:scale-[0.96]";

function queueDestination(key: string): string {
  switch (key) {
    case "ai":
      return "/central?filter=ai-review";
    case "uncategorized":
      return "/central?filter=uncategorized";
    case "unassigned":
      return "/central?filter=unassigned";
    case "updates":
      return "/central?filter=updates";
    default:
      return "/central";
  }
}

export function WorkQueuePanel({
  onNavigate,
  queueItems,
  activeJob,
}: WorkQueuePanelProps) {
  const { t } = useTranslation();
  const [filter, setFilter] = useState<QueueFilter>("all");

  const visible = useMemo(() => {
    const withCount = queueItems.filter((item) => item.count > 0);
    if (filter === "all") return withCount;
    return withCount.filter((item) => item.kind === filter);
  }, [queueItems, filter]);

  return (
    <Card className="surface-glass overflow-hidden rounded-3xl border-0 bg-transparent p-0 shadow-none">
      <PanelHeader
        title={t("dashboard.workQueues", { count: visible.length })}
        description={t("dashboard.workQueuesDesc")}
        action={
          <div
            role="tablist"
            aria-label={t("dashboard.queue.tabs.ariaLabel")}
            className="flex items-center gap-1 rounded-full border border-border/70 bg-card/60 p-0.5"
          >
            {FILTERS.map((option) => {
              const active = option.key === filter;
              return (
                <button
                  key={option.key}
                  role="tab"
                  type="button"
                  aria-selected={active}
                  onClick={() => setFilter(option.key)}
                  className={cn(
                    "min-h-10 rounded-full px-3 py-1 text-[0.7rem] font-medium",
                    dashboardControlMotion,
                    active
                      ? "bg-primary/15 text-primary-text"
                      : "text-muted-foreground hover:text-foreground",
                  )}
                >
                  {t(option.labelKey)}
                </button>
              );
            })}
          </div>
        }
      />
      <CardContent className="p-0">
        {visible.length > 0 ? (
          <ul className="divide-y divide-border/70">
            {visible.map((item) => (
              <li key={item.key}>
                <button
                  type="button"
                  onClick={() => onNavigate(queueDestination(item.key))}
                  className={cn(
                    "grid w-full grid-cols-[2.75rem_minmax(0,1fr)_auto] items-center gap-3 px-4 py-3 text-left hover:bg-muted/25",
                    dashboardControlMotion,
                  )}
                >
                  <span className="grid size-10 place-items-center rounded-xl border border-primary/25 bg-primary/10 font-display text-base font-semibold tabular-nums text-primary-text">
                    {item.count}
                  </span>
                  <span className="min-w-0">
                    <span className="block truncate text-sm font-medium">
                      {item.label}
                    </span>
                    <span className="mt-1 block truncate text-xs text-muted-foreground">
                      {item.description}
                    </span>
                  </span>
                  <ChevronRight className="size-4 text-muted-foreground" />
                </button>
              </li>
            ))}
          </ul>
        ) : (
          <div className="px-4 py-6 text-sm text-muted-foreground">
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
  );
}
