import { useTranslation } from "react-i18next";

import {
  ChartStateRow,
  PanelHeader,
  ProgressRow,
} from "@/components/dashboard/DashboardPanels";
import { Card, CardContent } from "@/components/ui/card";
import type { CentralTopTag } from "@/types";

interface TopTagsPanelProps {
  topTags: CentralTopTag[];
  isLoading: boolean;
  error: string | null;
  onRetry: () => void;
}

export function TopTagsPanel({
  topTags,
  isLoading,
  error,
  onRetry,
}: TopTagsPanelProps) {
  const { t } = useTranslation();
  const maxCount = Math.max(1, ...topTags.map((tag) => tag.count));

  return (
    <Card className="surface-glass overflow-hidden rounded-2xl border-0 bg-transparent p-0 shadow-none">
      <PanelHeader
        title={t("dashboard.topTags.title")}
        description={t("dashboard.topTags.description")}
      />
      <CardContent className="space-y-3 p-4">
        {isLoading ? (
          <ChartStateRow kind="loading" />
        ) : error ? (
          <ChartStateRow kind="error" onRetry={onRetry} />
        ) : topTags.length > 0 ? (
          topTags.map((tag) => (
            <ProgressRow
              key={tag.id}
              label={tag.name}
              count={tag.count}
              total={maxCount}
            />
          ))
        ) : (
          <div className="rounded-md border border-border/80 bg-background px-3 py-3 text-sm text-muted-foreground">
            {t("dashboard.topTags.empty")}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
