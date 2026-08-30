import { useTranslation } from "react-i18next";

import type { SkillsCliGlobalSkill, SkillsCliInstallTarget } from "@/types";

interface InventoryCensusProps {
  skills: SkillsCliGlobalSkill[];
  targets: SkillsCliInstallTarget[];
}

/** Normalized source buckets in display order; mirrors the backend enum. */
const SOURCE_BUCKETS = [
  "github",
  "gitlab",
  "git",
  "mintlify",
  "huggingface",
  "local",
  "well-known",
  "unknown",
] as const;

/**
 * Theme-token color per bucket. Legend dots and donut segments read the same
 * value, so the palette stays theme-aware without arbitrary hex colors.
 */
const BUCKET_COLOR: Record<string, string> = {
  github: "var(--chart-1)",
  gitlab: "var(--chart-2)",
  git: "var(--chart-3)",
  mintlify: "var(--chart-4)",
  huggingface: "var(--chart-5)",
  local: "var(--chart-2)",
  "well-known": "var(--chart-4)",
  unknown: "var(--muted-foreground)",
};

// Compact platform bars: fixed pixel geometry, never scaled by the container.
const ROW_HEIGHT = 16;
const BAR_HEIGHT = 8;
const LABEL_WIDTH = 88;
const BAR_AREA_WIDTH = 120;
const COUNT_WIDTH = 22;
const COLUMN_GAP = 6;
const CHART_PADDING_Y = 2;

// Donut geometry.
const DONUT_SIZE = 120;
const DONUT_RADIUS = 44;
const DONUT_STROKE = 14;

interface CensusRow {
  key: string;
  label: string;
  count: number;
  color: string;
}

function SourceDonut({ rows, total }: { rows: CensusRow[]; total: number }) {
  const { t } = useTranslation();
  const circumference = 2 * Math.PI * DONUT_RADIUS;
  let accumulatedLength = 0;
  return (
    <svg
      width={DONUT_SIZE}
      height={DONUT_SIZE}
      viewBox={`0 0 ${DONUT_SIZE} ${DONUT_SIZE}`}
      role="img"
      aria-label={t("skillsCli.census.donutAria", { count: total })}
    >
      <g transform={`rotate(-90 ${DONUT_SIZE / 2} ${DONUT_SIZE / 2})`}>
        {rows.map((row) => {
          const fraction = total > 0 ? row.count / total : 0;
          const segmentLength = fraction * circumference;
          const dashOffset = -accumulatedLength;
          accumulatedLength += segmentLength;
          return (
            <circle
              key={row.key}
              cx={DONUT_SIZE / 2}
              cy={DONUT_SIZE / 2}
              r={DONUT_RADIUS}
              fill="none"
              stroke={row.color}
              strokeWidth={DONUT_STROKE}
              strokeDasharray={`${segmentLength} ${circumference - segmentLength}`}
              strokeDashoffset={dashOffset}
            >
              <title>
                {t("skillsCli.census.donutTitle", {
                  label: row.label,
                  count: row.count,
                  percent: Math.round(fraction * 100),
                })}
              </title>
            </circle>
          );
        })}
      </g>
      <text
        x={DONUT_SIZE / 2}
        y={DONUT_SIZE / 2 + 2}
        textAnchor="middle"
        fontSize={18}
        fontWeight={600}
        className="fill-foreground"
      >
        {total}
      </text>
      <text
        x={DONUT_SIZE / 2}
        y={DONUT_SIZE / 2 + 16}
        textAnchor="middle"
        fontSize={9}
        className="fill-muted-foreground"
      >
        {t("skillsCli.census.donutCenterLabel")}
      </text>
    </svg>
  );
}

function PlatformBarChart({
  ariaLabel,
  rows,
}: {
  ariaLabel: string;
  rows: CensusRow[];
}) {
  const { t } = useTranslation();
  if (rows.length === 0) {
    return null;
  }
  const width =
    LABEL_WIDTH + COLUMN_GAP + BAR_AREA_WIDTH + COLUMN_GAP + COUNT_WIDTH;
  const height = rows.length * ROW_HEIGHT + CHART_PADDING_Y * 2;
  const maxCount = Math.max(...rows.map((row) => row.count), 1);
  return (
    <svg
      width={width}
      height={height}
      role="img"
      aria-label={ariaLabel}
      className="text-primary-text"
    >
      {rows.map((row, index) => {
        const rowTop = CHART_PADDING_Y + index * ROW_HEIGHT;
        const barY = rowTop + (ROW_HEIGHT - BAR_HEIGHT) / 2;
        const barWidth =
          row.count > 0
            ? Math.max((row.count / maxCount) * BAR_AREA_WIDTH, 2)
            : 1;
        const textY = rowTop + (ROW_HEIGHT + BAR_HEIGHT) / 2 - 1;
        return (
          <g key={row.key}>
            <text
              x={0}
              y={textY}
              fontSize={9}
              className="fill-muted-foreground"
            >
              {row.label}
            </text>
            <rect
              x={LABEL_WIDTH + COLUMN_GAP}
              y={barY}
              width={barWidth}
              height={BAR_HEIGHT}
              rx={2}
              fill="currentColor"
              opacity={row.count > 0 ? 1 : 0.3}
            >
              <title>
                {t("skillsCli.census.barTitle", {
                  label: row.label,
                  count: row.count,
                })}
              </title>
            </rect>
            <text
              x={LABEL_WIDTH + COLUMN_GAP + BAR_AREA_WIDTH + COLUMN_GAP}
              y={textY}
              fontSize={9}
              className="fill-foreground tabular-nums"
            >
              {row.count}
            </text>
          </g>
        );
      })}
    </svg>
  );
}

export function InventoryCensus({ skills, targets }: InventoryCensusProps) {
  const { t } = useTranslation();
  if (skills.length === 0) {
    return (
      <p
        data-testid="skills-cli-census-empty"
        className="text-sm text-muted-foreground"
      >
        {t("skillsCli.census.empty")}
      </p>
    );
  }

  const linkedCount = skills.filter((skill) => skill.agents.length >= 1).length;
  const sourceKindPresence: Partial<
    Record<SkillsCliGlobalSkill["sourceTypeBucket"], true>
  > = {};
  for (const skill of skills) {
    sourceKindPresence[skill.sourceTypeBucket] = true;
  }
  const sourceKindCount = Object.keys(sourceKindPresence).length;

  const kpis = [
    { label: t("skillsCli.census.kpiInstalled"), value: skills.length },
    { label: t("skillsCli.census.kpiLinked"), value: linkedCount },
    { label: t("skillsCli.census.kpiSourceKinds"), value: sourceKindCount },
  ];

  const platformRows: CensusRow[] = targets.map((target) => ({
    key: target.id,
    label: target.displayName,
    count: skills.filter((skill) =>
      skill.agents.includes(target.displayName),
    ).length,
    color: "currentColor",
  }));
  const donutRows: CensusRow[] = SOURCE_BUCKETS.flatMap((bucket) => {
    const count = skills.filter(
      (skill) => skill.sourceTypeBucket === bucket,
    ).length;
    if (count === 0) {
      return [];
    }
    const i18nKey = bucket === "well-known" ? "wellKnown" : bucket;
    return [
      {
        key: bucket,
        label: t(`skillsCli.census.bucket.${i18nKey}`),
        count,
        color: BUCKET_COLOR[bucket] ?? BUCKET_COLOR.unknown,
      },
    ];
  });

  return (
    <section className="rounded-lg border border-border p-4">
      <div
        aria-label={t("skillsCli.census.kpiAria")}
        className="flex flex-wrap gap-x-10 gap-y-2 border-b border-border pb-3"
      >
        {kpis.map((kpi) => (
          <div key={kpi.label}>
            <div className="text-xs text-muted-foreground">{kpi.label}</div>
            <div className="mt-0.5 text-lg font-semibold tabular-nums text-foreground">
              {kpi.value.toLocaleString()}
            </div>
          </div>
        ))}
      </div>
      <div className="mt-4 grid gap-6 md:grid-cols-[auto_minmax(0,1fr)]">
        <figure className="flex items-center gap-4">
          <SourceDonut rows={donutRows} total={skills.length} />
          <figcaption className="space-y-1.5">
            <div className="text-xs font-medium">
              {t("skillsCli.census.sourceHeading")}
            </div>
            {donutRows.map((row) => (
              <div key={row.key} className="flex items-center gap-1.5 text-xs">
                <span
                  aria-hidden
                  className="size-2.5 rounded-full"
                  style={{ background: row.color }}
                />
                <span>{row.label}</span>
                <span className="tabular-nums text-muted-foreground">
                  {row.count}
                </span>
              </div>
            ))}
          </figcaption>
        </figure>
        <figure className="space-y-2">
          <figcaption className="text-xs font-medium">
            {t("skillsCli.census.platformHeading")}
          </figcaption>
          <PlatformBarChart
            ariaLabel={t("skillsCli.census.platformAria", {
              count: platformRows.length,
            })}
            rows={platformRows}
          />
        </figure>
      </div>
    </section>
  );
}
