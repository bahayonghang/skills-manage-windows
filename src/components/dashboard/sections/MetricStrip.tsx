import { useTranslation } from "react-i18next";
import { Blocks, FolderGit2, Layers, Sparkles } from "lucide-react";

import { StatButton } from "@/components/dashboard/DashboardPanels";

interface MetricStripProps {
  onNavigate: (path: string) => void;
  centralTotal: number;
  aiReviewCount: number;
  sourceRepositoryCount: number;
  collectionCount: number;
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
      className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4"
      aria-label={t("dashboard.metricStrip.ariaLabel")}
    >
      <StatButton
        testId="dashboard-metric-central"
        icon={<Blocks className="size-4" />}
        label={t("dashboard.metricStrip.central")}
        value={centralTotal}
        description={t("dashboard.metricStrip.centralDesc")}
        onClick={() => onNavigate("/central")}
        emphasized={centralTotal > 0}
      />
      <StatButton
        testId="dashboard-metric-ai-stat"
        icon={<Sparkles className="size-4" />}
        label={t("dashboard.metricStrip.aiReviews")}
        value={aiReviewCount}
        description={t("dashboard.metricStrip.aiReviewsDesc")}
        onClick={() => onNavigate("/central?filter=ai-review")}
        emphasized={aiReviewCount > 0}
      />
      <StatButton
        icon={<FolderGit2 className="size-4" />}
        label={t("dashboard.metricStrip.sources")}
        value={sourceRepositoryCount}
        description={t("dashboard.metricStrip.sourcesDesc")}
        onClick={() => onNavigate("/marketplace")}
      />
      <StatButton
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
