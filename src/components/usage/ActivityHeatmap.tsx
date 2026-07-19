import { useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { cn } from "@/lib/utils";
import type { DayCount } from "@/types/usage";

interface ActivityHeatmapProps {
  days: DayCount[];
  compact?: boolean;
  className?: string;
}

const LABELED_WEEKDAY_ROWS = new Set([0, 2, 4]);

export function ActivityHeatmap({
  days,
  compact = false,
  className,
}: ActivityHeatmapProps) {
  const { t, i18n } = useTranslation();
  const [focusIndex, setFocusIndex] = useState(0);
  const cells = useRef<Array<HTMLButtonElement | null>>([]);
  const nonZeroCounts = useMemo(
    () => days.map((day) => day.count).filter((count) => count > 0),
    [days],
  );
  const weekdayLabels = useMemo(() => {
    const mondayUtc = Date.UTC(2024, 0, 1);
    const formatter = new Intl.DateTimeFormat(i18n.language || undefined, {
      weekday: "short",
      timeZone: "UTC",
    });
    return Array.from({ length: 7 }, (_, index) =>
      LABELED_WEEKDAY_ROWS.has(index)
        ? formatter.format(new Date(mondayUtc + index * 86_400_000))
        : "",
    );
  }, [i18n.language]);
  const monthLabels = useMemo(
    () => buildMonthLabels(days, i18n.language),
    [days, i18n.language],
  );

  if (days.length === 0) {
    return (
      <div
        className={cn(
          "flex min-h-32 items-center justify-center text-xs text-muted-foreground",
          className,
        )}
      >
        {t("skillUsage.empty.noActivity")}
      </div>
    );
  }

  function moveFocus(index: number, delta: number) {
    const next = Math.max(0, Math.min(days.length - 1, index + delta));
    setFocusIndex(next);
    cells.current[next]?.focus();
  }

  return (
    <div
      data-testid="heatmap-shell"
      className={cn("min-w-0", compact ? "py-1" : "py-3", className)}
    >
      <div className="overflow-x-auto pb-1">
        <div className={cn("min-w-[22rem]", !compact && "px-1")}>
          <div className="ml-9 grid grid-cols-16 gap-1 text-ui-micro text-muted-foreground">
            {monthLabels.map((label, index) => (
              <span
                key={`${label}-${index}`}
                className="whitespace-nowrap overflow-visible"
              >
                {label}
              </span>
            ))}
          </div>
          <div className="mt-1 flex gap-2">
            <div className="grid w-7 shrink-0 grid-rows-7 items-center text-ui-micro text-muted-foreground">
              {weekdayLabels.map((label, index) => (
                <span key={`${label}-${index}`}>{label}</span>
              ))}
            </div>
            <div
              data-testid="heatmap-grid"
              role="grid"
              aria-label={t("skillUsage.heatmap.ariaLabel")}
              className="grid flex-1 auto-cols-fr grid-flow-col grid-rows-7 gap-1"
            >
              {days.map((day, index) => {
                const level = heatLevelFor(day.count, nonZeroCounts);
                const label = t("skillUsage.heatmap.cellLabel", {
                  date: day.date,
                  count: day.count,
                });
                return (
                  <button
                    key={day.date}
                    ref={(element) => {
                      cells.current[index] = element;
                    }}
                    type="button"
                    role="gridcell"
                    tabIndex={index === focusIndex ? 0 : -1}
                    data-testid={`heatmap-cell-${day.date}`}
                    data-level={level}
                    title={label}
                    aria-label={label}
                    onFocus={() => setFocusIndex(index)}
                    onKeyDown={(event) => {
                      const delta =
                        event.key === "ArrowRight"
                          ? 7
                          : event.key === "ArrowLeft"
                            ? -7
                            : event.key === "ArrowDown"
                              ? 1
                              : event.key === "ArrowUp"
                                ? -1
                                : 0;
                      if (delta !== 0) {
                        event.preventDefault();
                        moveFocus(index, delta);
                      }
                    }}
                    className={cn(
                      "block aspect-square w-full rounded-[3px] border border-transparent focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1 focus-visible:ring-offset-background",
                      CELL_LEVEL_CLASS[level],
                    )}
                  />
                );
              })}
            </div>
          </div>
        </div>
      </div>
      <div className="mt-2 flex items-center justify-end gap-1.5 text-ui-micro text-muted-foreground">
        <span>{t("skillUsage.heatmap.less")}</span>
        {([0, 1, 2, 3, 4, 5] as const).map((level) => (
          <span
            key={level}
            aria-hidden
            className={cn(
              "size-2.5 rounded-[2px] border border-transparent",
              CELL_LEVEL_CLASS[level],
            )}
          />
        ))}
        <span>{t("skillUsage.heatmap.more")}</span>
      </div>
    </div>
  );
}

function buildMonthLabels(days: DayCount[], locale: string): string[] {
  const formatter = new Intl.DateTimeFormat(locale || undefined, {
    month: "short",
  });
  let previousMonth = "";
  return Array.from({ length: 16 }, (_, week) => {
    const day = days[week * 7];
    if (!day) return "";
    const date = new Date(`${day.date}T12:00:00`);
    const monthKey = `${date.getFullYear()}-${date.getMonth()}`;
    if (monthKey === previousMonth) return "";
    previousMonth = monthKey;
    return formatter.format(date);
  });
}

function heatLevelFor(
  count: number,
  nonZeroCounts: number[],
): 0 | 1 | 2 | 3 | 4 | 5 {
  if (count <= 0) return 0;
  if (nonZeroCounts.length < 5) {
    const max = Math.max(1, ...nonZeroCounts);
    return Math.max(1, Math.min(5, Math.ceil((count / max) * 5))) as
      | 1
      | 2
      | 3
      | 4
      | 5;
  }

  const sorted = [...nonZeroCounts].sort((a, b) => a - b);
  const quantile = (value: number) => {
    const position = (sorted.length - 1) * value;
    const lower = Math.floor(position);
    const upper = Math.ceil(position);
    const lowerValue = sorted[lower] ?? 0;
    const upperValue = sorted[upper] ?? lowerValue;
    return lowerValue + (upperValue - lowerValue) * (position - lower);
  };
  if (count <= quantile(0.25)) return 1;
  if (count <= quantile(0.5)) return 2;
  if (count <= quantile(0.75)) return 3;
  if (count <= quantile(0.9)) return 4;
  return 5;
}

const CELL_LEVEL_CLASS: Record<0 | 1 | 2 | 3 | 4 | 5, string> = {
  0: "border-border/60 bg-muted/25",
  1: "bg-chart-2/20",
  2: "bg-chart-2/35",
  3: "bg-chart-2/55",
  4: "bg-chart-2/75",
  5: "bg-chart-2",
};
