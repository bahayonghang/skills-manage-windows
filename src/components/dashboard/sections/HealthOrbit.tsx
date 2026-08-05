import { useTranslation } from "react-i18next";
import type { CSSProperties, ReactNode } from "react";
import { CheckCircle2, CircleAlert, CircleDashed } from "lucide-react";

import { cn } from "@/lib/utils";
import type { DashboardReadiness } from "@/types";

interface HealthOrbitProps {
  readiness: DashboardReadiness;
}

interface FactorProps {
  label: string;
  value: number;
  tone: "primary" | "accent" | "neutral" | "success";
}

type ReadinessStatus = "ready" | "review" | "triage";

const FACTOR_TONE_CLASS: Record<FactorProps["tone"], string> = {
  primary: "from-primary to-primary/55",
  accent: "from-chart-1 to-chart-1/45",
  neutral: "from-muted-foreground/70 to-muted-foreground/35",
  success: "from-success to-success/45",
};

const STATUS_CLASS: Record<ReadinessStatus, string> = {
  ready: "border-success/30 bg-success/10 text-success-foreground",
  review: "border-primary/35 bg-primary/10 text-primary-text",
  triage: "border-destructive/30 bg-destructive/10 text-destructive-text",
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

function FactorRail({ label, value, tone }: FactorProps) {
  const safeValue = Math.max(0, Math.min(1, value));
  const percent = toPercent(safeValue);

  return (
    <div className="py-1.5">
      <div className="flex items-center justify-between gap-3">
        <span className="min-w-0 truncate text-xs font-medium text-muted-foreground">
          {label}
        </span>
        <span className="font-display text-xs font-semibold tabular-nums">
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

export function HealthOrbit({ readiness }: HealthOrbitProps) {
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
      className="readiness-card surface-glass rounded-2xl p-5"
      aria-label={t("dashboard.readiness.label")}
    >
      <div className="relative z-10 flex h-full flex-col gap-4">
        <div className="flex items-start justify-between gap-4">
          <div className="min-w-0">
            <div className="text-xs font-semibold uppercase tracking-[0.12em] text-muted-foreground">
              {t("dashboard.readiness.label")}
            </div>
            <div
              className="readiness-score-plaque mt-3 inline-flex items-end gap-2 rounded-xl px-4 py-3"
              aria-label={t("dashboard.readiness.ariaLabel", { score })}
              role="img"
              style={
                {
                  "--score-angle": `${Math.round((score / 100) * 360)}deg`,
                } as CSSProperties
              }
            >
              <span className="font-display text-display-score font-semibold leading-none tabular-nums">
                {score}
              </span>
              <span className="mb-2 text-xs font-semibold uppercase tracking-[0.12em] text-muted-foreground">
                {t("dashboard.readiness.scoreUnit")}
              </span>
            </div>
          </div>
          <span
            className={cn(
              "inline-flex shrink-0 items-center gap-1.5 rounded-full border px-2.5 py-1 text-xs font-semibold uppercase tracking-wide",
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
          className="grid gap-1"
          aria-label={t("dashboard.readiness.factors.ariaLabel")}
        >
          {factors.map((factor) => (
            <FactorRail key={factor.label} {...factor} />
          ))}
        </div>
      </div>
    </section>
  );
}
