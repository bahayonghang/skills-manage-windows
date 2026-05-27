import { useTranslation } from "react-i18next";

import { PanelHeader, ProgressRow } from "@/components/dashboard/DashboardPanels";
import { Card, CardContent } from "@/components/ui/card";

interface ProgressBreakdownProps {
  centralTotal: number;
  uncategorizedCount: number;
  aiReviewCount: number;
  updatesAvailable: number;
  unassignedSourceCount: number;
}

export function ProgressBreakdown({
  centralTotal,
  uncategorizedCount,
  aiReviewCount,
  updatesAvailable,
  unassignedSourceCount,
}: ProgressBreakdownProps) {
  const { t } = useTranslation();

  return (
    <Card className="surface-glass overflow-hidden rounded-3xl border-0 bg-transparent p-0 shadow-none">
      <PanelHeader
        title={t("dashboard.health.title")}
        description={t("dashboard.health.description")}
      />
      <CardContent className="space-y-3 p-4">
        <ProgressRow
          testId="dashboard-metric-uncategorized"
          label={t("dashboard.metrics.uncategorized")}
          count={uncategorizedCount}
          total={centralTotal}
          tone="primary"
        />
        <ProgressRow
          testId="dashboard-metric-ai"
          label={t("dashboard.metrics.aiReviews")}
          count={aiReviewCount}
          total={centralTotal}
          tone="primary"
        />
        <ProgressRow
          testId="dashboard-metric-updates"
          label={t("dashboard.metrics.updates")}
          count={updatesAvailable}
          total={centralTotal}
          tone="primary"
        />
        <ProgressRow
          testId="dashboard-metric-unassigned"
          label={t("dashboard.metrics.unassigned")}
          count={unassignedSourceCount}
          total={centralTotal}
          tone="muted"
        />
      </CardContent>
    </Card>
  );
}
