import { useMemo } from "react";
import { useTranslation } from "react-i18next";

import type { DayCount } from "@/types/usage";
import { cn } from "@/lib/utils";

interface ActivityHeatmapProps {
  /** 16w * 7d = 112 cells, ascending by date, starting on a Monday. */
  days: DayCount[];
  className?: string;
}

const LABELED_WEEKDAY_ROWS = new Set([0, 2, 4]); // Mon / Wed / Fri

/**
 * GitHub 风格 16 周活动热力图。
 *
 * 后端 `aggregate::heatmap_grid_16w` 已经保证：
 * - 112 个连续日期，按日升序排列
 * - 第一项是 16 周前对应的周一
 *
 * 因此 cells 以 (week, day) 行主序排列，正好可以用 CSS Grid
 * `grid-flow-col` + `grid-rows-7` 按列优先渲染，每一列就是一周的 Mon→Sun。
 *
 * 5 级配色：lvl 0=空（border 灰），1..4 emerald 渐深。
 */
export function ActivityHeatmap({ days, className }: ActivityHeatmapProps) {
  const { i18n } = useTranslation();
  const max = useMemo(
    () => Math.max(1, ...days.map((d) => d.count)),
    [days]
  );
  const weekdayLabels = useMemo(() => {
    const mondayUtc = Date.UTC(2024, 0, 1); // 2024-01-01 is a Monday.
    const formatter = new Intl.DateTimeFormat(i18n.language || undefined, {
      weekday: "short",
      timeZone: "UTC",
    });

    return Array.from({ length: 7 }, (_, index) =>
      LABELED_WEEKDAY_ROWS.has(index)
        ? formatter.format(new Date(mondayUtc + index * 86_400_000))
        : ""
    );
  }, [i18n.language]);

  if (days.length === 0) {
    return (
      <div
        className={cn(
          "rounded border border-dashed border-border/60 px-3 py-8 text-center text-xs text-muted-foreground",
          className
        )}
      >
        —
      </div>
    );
  }

  return (
    <div
      data-testid="heatmap-shell"
      className={cn(
        "flex h-full min-h-[16rem] items-center justify-center gap-3 overflow-hidden",
        className
      )}
    >
      {/* Weekday labels (sparse) */}
      <div className="grid h-full min-h-[12rem] grid-rows-7 items-center py-1 text-[10px] text-muted-foreground">
        {weekdayLabels.map((d, index) => (
          <span key={`${d}-${index}`}>{d}</span>
        ))}
      </div>

      {/* Heatmap grid */}
      <div
        data-testid="heatmap-grid"
        className="grid w-full max-w-[40rem] auto-cols-fr grid-flow-col grid-rows-7 gap-1"
      >
        {days.map((d) => {
          const lvl = levelFor(d.count, max);
          return (
            <span
              key={d.date}
              data-testid={`heatmap-cell-${d.date}`}
              data-level={lvl}
              title={`${d.date} • ${d.count}`}
              className={cn(
                "block aspect-square w-full rounded-[4px]",
                CELL_LEVEL_CLASS[lvl]
              )}
            />
          );
        })}
      </div>
    </div>
  );
}

function levelFor(count: number, max: number): 0 | 1 | 2 | 3 | 4 {
  if (count <= 0) return 0;
  // 4 等分量化：max 大时用比例；max 小时直接用计数避免全部挤到 lvl4
  if (max <= 4) {
    if (count >= 4) return 4;
    return count as 1 | 2 | 3;
  }
  const ratio = count / max;
  if (ratio < 0.25) return 1;
  if (ratio < 0.5) return 2;
  if (ratio < 0.75) return 3;
  return 4;
}

const CELL_LEVEL_CLASS: Record<0 | 1 | 2 | 3 | 4, string> = {
  0: "bg-muted/30",
  1: "bg-emerald-500/25",
  2: "bg-emerald-500/45",
  3: "bg-emerald-500/70",
  4: "bg-emerald-500",
};
