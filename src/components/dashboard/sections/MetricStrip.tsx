import type { ReactNode } from "react";

import { useTranslation } from "react-i18next";
import { Blocks, FolderGit2, Layers, Sparkles } from "lucide-react";

import { cn } from "@/lib/utils";

interface MetricStripProps {
  onNavigate: (path: string) => void;
  centralTotal: number;
  aiReviewCount: number;
  sourceRepositoryCount: number;
  collectionCount: number;
}

function MetricItem({
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
  value: number;
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
      title={description}
      className={cn(
        "focus-ring flex min-w-0 flex-1 items-center gap-2 rounded-lg px-3 py-2 text-left",
        "transition-[scale,background-color,color] duration-150 ease-out hover:bg-muted/25 active:scale-[0.96]",
      )}
    >
      <span
        className={cn(
          "shrink-0 text-muted-foreground",
          emphasized && "text-primary-text",
        )}
      >
        {icon}
      </span>
      <span
        className={cn(
          "shrink-0 text-base font-semibold leading-none tabular-nums",
          emphasized && "text-primary-text",
        )}
      >
        {value}
      </span>
      <span className="min-w-0 truncate text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
        {label}
      </span>
    </button>
  );
}

export function MetricStrip({
  onNavigate,
  centralTotal,
  aiReviewCount,
  sourceRepositoryCount,
  collectionCount,
}: MetricStripProps) {
  const { t } = useTranslation();

  return (
    <section
      className="grid grid-cols-2 rounded-xl bg-card p-1 shadow-sm ring-1 ring-border sm:flex sm:items-stretch sm:divide-x sm:divide-border/60"
      aria-label={t("dashboard.metricStrip.ariaLabel")}
    >
      <MetricItem
        testId="dashboard-metric-central"
        icon={<Blocks className="size-4" />}
        label={t("dashboard.metricStrip.central")}
        value={centralTotal}
        description={t("dashboard.metricStrip.centralDesc")}
        onClick={() => onNavigate("/central")}
        emphasized={centralTotal > 0}
      />
      <MetricItem
        testId="dashboard-metric-ai-stat"
        icon={<Sparkles className="size-4" />}
        label={t("dashboard.metricStrip.aiReviews")}
        value={aiReviewCount}
        description={t("dashboard.metricStrip.aiReviewsDesc")}
        onClick={() => onNavigate("/central?filter=ai-review")}
        emphasized={aiReviewCount > 0}
      />
      <MetricItem
        icon={<FolderGit2 className="size-4" />}
        label={t("dashboard.metricStrip.sources")}
        value={sourceRepositoryCount}
        description={t("dashboard.metricStrip.sourcesDesc")}
        onClick={() => onNavigate("/marketplace")}
      />
      <MetricItem
        testId="dashboard-metric-collections"
        icon={<Layers className="size-4" />}
        label={t("dashboard.metricStrip.collections")}
        value={collectionCount}
        description={t("dashboard.metricStrip.collectionsDesc")}
        onClick={() => onNavigate("/collections")}
      />
    </section>
  );
}
