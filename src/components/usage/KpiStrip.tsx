import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { Activity, Layers, FolderOpen, Wrench } from "lucide-react";

import type { UsageKpis } from "@/types/usage";
import { cn } from "@/lib/utils";

interface KpiStripProps {
  kpis: UsageKpis;
  className?: string;
}

/** 顶部 4 张卡：CALLS / SKILLS / PROJECTS / SOURCES。等价 skilled 顶部条。 */
export function KpiStrip({ kpis, className }: KpiStripProps) {
  const { t } = useTranslation();

  return (
    <div className={cn("grid grid-cols-2 gap-3 sm:grid-cols-4", className)}>
      <KpiCard
        label={t("skillUsage.kpi.calls")}
        value={kpis.totalCalls}
        icon={<Activity className="size-4" />}
        tone="primary"
      />
      <KpiCard
        label={t("skillUsage.kpi.skills")}
        value={kpis.uniqueSkills}
        icon={<Wrench className="size-4" />}
        tone="accent"
      />
      <KpiCard
        label={t("skillUsage.kpi.projects")}
        value={kpis.uniqueProjects}
        icon={<FolderOpen className="size-4" />}
        tone="neutral"
      />
      <KpiCard
        label={t("skillUsage.kpi.sources")}
        value={kpis.uniqueSources}
        icon={<Layers className="size-4" />}
        tone="success"
      />
    </div>
  );
}

interface KpiCardProps {
  label: string;
  value: number;
  icon: ReactNode;
  tone: "primary" | "accent" | "neutral" | "success";
}

const TONE_CLASS: Record<KpiCardProps["tone"], string> = {
  primary: "from-primary/14 via-primary/6 text-primary",
  accent: "from-chart-1/14 via-chart-1/6 text-chart-1",
  neutral: "from-muted/40 via-muted/20 text-muted-foreground",
  success: "from-emerald-500/14 via-emerald-500/6 text-emerald-500",
};

function KpiCard({ label, value, icon, tone }: KpiCardProps) {
  return (
    <div
      data-testid={`kpi-card-${label}`}
      className={cn(
        "rounded-lg border border-border bg-card px-4 py-3",
        "bg-gradient-to-br to-transparent",
        TONE_CLASS[tone]
      )}
    >
      <div className="flex items-center justify-between">
        <span className="text-[11px] uppercase tracking-wide text-muted-foreground">
          {label}
        </span>
        <span className="opacity-70">{icon}</span>
      </div>
      <div className="mt-1 text-2xl font-semibold tabular-nums text-foreground">
        {value.toLocaleString()}
      </div>
    </div>
  );
}
