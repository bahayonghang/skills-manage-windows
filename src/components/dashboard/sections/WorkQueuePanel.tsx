import { useTranslation } from "react-i18next";
import { ChevronRight } from "lucide-react";

import { PanelHeader } from "@/components/dashboard/DashboardPanels";
import { Card, CardContent } from "@/components/ui/card";
import { cn } from "@/lib/utils";
import type { DashboardQueueItem } from "@/pages/dashboardViewModel";

interface WorkQueuePanelProps {
  onNavigate: (path: string) => void;
  queueItems: DashboardQueueItem[];
  activeJob: { completed: number; total: number } | null;
}

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
  const pendingCount = queueItems.filter((item) => item.count > 0).length;

  return (
    <Card className="surface-glass overflow-hidden rounded-2xl border-0 bg-transparent p-0 shadow-none">
      <PanelHeader
        title={t("dashboard.workQueues", { count: pendingCount })}
        description={t("dashboard.workQueuesDesc")}
      />
      <CardContent className="p-2">
        <ul className="grid grid-cols-2 gap-1 xl:grid-cols-4">
          {queueItems.map((item) => (
            <li key={item.key}>
              <button
                type="button"
                onClick={() => onNavigate(queueDestination(item.key))}
                className={cn(
                  "focus-ring flex h-full w-full items-center gap-3 rounded-lg px-3 py-2.5 text-left hover:bg-muted/25",
                  dashboardControlMotion,
                )}
              >
                <span
                  className={cn(
                    "grid size-9 shrink-0 place-items-center rounded-xl border text-sm font-semibold tabular-nums",
                    item.count > 0
                      ? "border-primary/25 bg-primary/10 text-primary-text"
                      : "border-border/70 bg-muted/30 text-muted-foreground",
                  )}
                >
                  {item.count}
                </span>
                <span className="min-w-0">
                  <span className="block truncate text-sm font-medium">
                    {item.label}
                  </span>
                  <span className="mt-0.5 block truncate text-xs text-muted-foreground">
                    {item.description}
                  </span>
                </span>
                <ChevronRight className="ml-auto size-4 shrink-0 text-muted-foreground" />
              </button>
            </li>
          ))}
        </ul>
        {activeJob && (
          <div className="mt-2 rounded-md border border-border/70 bg-muted/20 px-3 py-2 text-xs text-muted-foreground">
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
