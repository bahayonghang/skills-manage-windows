import { useTranslation } from "react-i18next";

import type { DashboardReadiness } from "@/types";

interface HealthOrbitProps {
  readiness: DashboardReadiness;
  centralTotal: number;
  sourceRepositoryCount: number;
  enabledAgentsCount: number;
}

interface MiniProps {
  label: string;
  value: number | string;
  testId?: string;
}

function MiniStat({ label, value, testId }: MiniProps) {
  return (
    <div
      data-testid={testId}
      className="rounded-2xl border border-border/70 bg-card/60 px-3 py-2.5"
    >
      <div className="text-[0.65rem] font-semibold uppercase tracking-wide text-muted-foreground">
        {label}
      </div>
      <div className="mt-1 font-display text-xl font-semibold tabular-nums tracking-tight">
        {value}
      </div>
    </div>
  );
}

export function HealthOrbit({
  readiness,
  centralTotal,
  sourceRepositoryCount,
  enabledAgentsCount,
}: HealthOrbitProps) {
  const { t } = useTranslation();
  const score = Math.max(0, Math.min(100, Math.round(readiness.score)));

  return (
    <section
      className="surface-glass rounded-3xl p-5"
      aria-label={t("dashboard.readiness.label")}
    >
      <div className="flex flex-col items-center gap-5">
        <div
          className="ring-readiness w-[min(100%,17rem)]"
          style={
            {
              "--score": `${score}%`,
            } as React.CSSProperties
          }
          aria-label={t("dashboard.readiness.ariaLabel", { score })}
          role="img"
        >
          <div>
            <div className="font-display text-5xl font-semibold tabular-nums tracking-tight">
              {score}
            </div>
            <div className="mt-1 text-[0.65rem] font-semibold uppercase tracking-[0.18em] text-muted-foreground">
              {t("dashboard.readiness.label")}
            </div>
          </div>
        </div>
        <div className="grid w-full grid-cols-3 gap-2">
          <MiniStat
            label={t("dashboard.readiness.miniCentral")}
            value={centralTotal}
          />
          <MiniStat
            label={t("dashboard.readiness.miniSources")}
            value={sourceRepositoryCount}
          />
          <MiniStat
            testId="dashboard-metric-targets"
            label={t("dashboard.readiness.miniAgents")}
            value={enabledAgentsCount}
          />
        </div>
      </div>
    </section>
  );
}
