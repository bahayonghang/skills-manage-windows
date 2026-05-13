import { useTranslation } from "react-i18next";
import { Activity, AlertCircle, CheckCircle2, Timer } from "lucide-react";

import { Card } from "@/components/ui/card";
import { StatusTile } from "@/components/dashboard/DashboardPanels";
import {
  LogsKpi,
  formatDurationMs,
  formatPercent,
} from "@/components/logs/logsUtils";

interface LogsKpiRowProps {
  kpi: LogsKpi;
  total: number;
  loadedCount: number;
}

export function LogsKpiRow({ kpi, total, loadedCount }: LogsKpiRowProps) {
  const { t } = useTranslation();
  const isSampled = total > loadedCount;
  const sampledLabel = isSampled
    ? t("logs.kpi.basedOn", { count: loadedCount })
    : undefined;
  const failuresDetail =
    kpi.failures > 0
      ? t("logs.kpi.failuresBreakdown", {
          failed: kpi.failed,
          partial: kpi.partial,
        })
      : sampledLabel;

  return (
    <Card className="overflow-hidden border-border/90 bg-card/95 gap-0 py-0">
      <div className="grid sm:grid-cols-2 xl:grid-cols-4">
        <StatusTile
          testId="logs-kpi-total"
          icon={<Activity className="size-4" />}
          label={t("logs.kpi.total")}
          value={String(total)}
          detail={sampledLabel}
        />
        <StatusTile
          testId="logs-kpi-success-rate"
          icon={<CheckCircle2 className="size-4" />}
          label={t("logs.kpi.successRate")}
          value={formatPercent(kpi.successRate)}
          detail={sampledLabel}
        />
        <StatusTile
          testId="logs-kpi-failures"
          icon={<AlertCircle className="size-4" />}
          label={t("logs.kpi.failures")}
          value={String(kpi.failures)}
          detail={failuresDetail}
        />
        <StatusTile
          testId="logs-kpi-avg-duration"
          icon={<Timer className="size-4" />}
          label={t("logs.kpi.avgDuration")}
          value={formatDurationMs(kpi.avgDurationMs)}
          detail={sampledLabel}
        />
      </div>
    </Card>
  );
}
