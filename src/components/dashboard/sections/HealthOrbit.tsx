import { useTranslation } from "react-i18next";
import type { CSSProperties, ReactNode } from "react";
import { CheckCircle2, CircleAlert, CircleDashed } from "lucide-react";

import { cn } from "@/lib/utils";
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

interface FactorProps {
  label: string;
  value: number;
  tone: "primary" | "accent" | "neutral" | "success";
}

type ReadinessStatus = "ready" | "review" | "triage";

const FACTOR_TONE_CLASS: Record<FactorProps["tone"], string> = {
  primary: "from-primary to-primary/55",
  accent: "from-chart-1 to-primary/45",
  neutral: "from-muted-foreground/70 to-muted-foreground/35",
  success: "from-emerald-500 to-emerald-500/45",
};

const STATUS_CLASS: Record<ReadinessStatus, string> = {
  ready: "border-emerald-500/30 bg-emerald-500/10 text-emerald-600 dark:text-emerald-300",
  review: "border-primary/35 bg-primary/10 text-primary",
  triage: "border-destructive/30 bg-destructive/10 text-destructive",
};

const STATUS_ICON: Record<ReadinessStatus, ReactNode> = {
  ready: <CheckCircle2 className="size-3.5" />,
  review: <CircleDashed className="size-3.5" />,
  triage: <CircleAlert className="size-3.5" />,
};

function toPercent(value: number) {
  return `${Math.round(Math.max(0, Math.min(1, value)) * 100)}%`;
}

function getReadinessStatus(score: number): ReadinessStatus {
  if (score >= 80) return "ready";
  if (score >= 50) return "review";
  return "triage";
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

function FactorRail({ label, value, tone }: FactorProps) {
  const safeValue = Math.max(0, Math.min(1, value));
  const percent = toPercent(safeValue);

  return (
    <div className="readiness-rail rounded-2xl border border-border/60 bg-card/50 px-3 py-2.5">
      <div className="flex items-center justify-between gap-3">
        <span className="min-w-0 truncate text-xs font-medium text-muted-foreground">
          {label}
        </span>
        <span className="font-display text-xs font-semibold tabular-nums tracking-tight">
          {percent}
        </span>
      </div>
      <div className="mt-2 h-1.5 overflow-hidden rounded-full bg-muted/70">
        <span
          className={cn(
            "block h-full origin-left rounded-full bg-gradient-to-r transition-transform duration-500 ease-out",
            FACTOR_TONE_CLASS[tone],
          )}
          style={{ transform: `scaleX(${safeValue})` }}
        />
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
  const status = getReadinessStatus(score);
  const factors: FactorProps[] = [
    {
      label: t("dashboard.readiness.factors.categorized"),
      value: readiness.categorizedRatio,
      tone: "primary",
    },
    {
      label: t("dashboard.readiness.factors.described"),
      value: readiness.describedRatio,
      tone: "accent",
    },
    {
      label: t("dashboard.readiness.factors.sourced"),
      value: readiness.sourcedRatio,
      tone: "neutral",
    },
    {
      label: t("dashboard.readiness.factors.installHealth"),
      value: readiness.installHealthRatio,
      tone: "success",
    },
  ];

  return (
    <section
      className="readiness-card surface-glass rounded-3xl p-5"
      aria-label={t("dashboard.readiness.label")}
    >
      <div className="relative z-10 flex h-full flex-col gap-4">
        <div className="flex items-start justify-between gap-4">
          <div className="min-w-0">
            <div className="text-[0.68rem] font-semibold uppercase tracking-[0.22em] text-muted-foreground">
              {t("dashboard.readiness.label")}
            </div>
            <div
              className="readiness-score-plaque mt-3 inline-flex items-end gap-2 rounded-3xl px-4 py-3"
              aria-label={t("dashboard.readiness.ariaLabel", { score })}
              role="img"
              style={
                {
                  "--score-angle": `${Math.round((score / 100) * 360)}deg`,
                } as CSSProperties
              }
            >
              <span className="font-display text-6xl font-semibold leading-none tabular-nums tracking-[-0.08em]">
                {score}
              </span>
              <span className="mb-2 text-xs font-semibold uppercase tracking-[0.2em] text-muted-foreground">
                {t("dashboard.readiness.scoreUnit")}
              </span>
            </div>
          </div>
          <span
            className={cn(
              "inline-flex shrink-0 items-center gap-1.5 rounded-full border px-2.5 py-1 text-[0.68rem] font-semibold uppercase tracking-wide",
              STATUS_CLASS[status],
            )}
          >
            {STATUS_ICON[status]}
            {t(`dashboard.readiness.status.${status}`)}
          </span>
        </div>

        <p className="max-w-md text-xs leading-5 text-muted-foreground">
          {t("dashboard.readiness.description")}
        </p>

        <div
          className="grid gap-2"
          aria-label={t("dashboard.readiness.factors.ariaLabel")}
        >
          {factors.map((factor) => (
            <FactorRail key={factor.label} {...factor} />
          ))}
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
