import { useMemo } from "react";

import type { DayCount } from "@/types/usage";
import { cn } from "@/lib/utils";

interface ActivityHeatmapProps {
  /** 16w * 7d = 112 cells, ascending by date, starting on a Monday. */
  days: DayCount[];
  className?: string;
}

const WEEKDAY_LABELS = ["Mon", "Wed", "Fri"]; // sparse labels

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
  const max = useMemo(
    () => Math.max(1, ...days.map((d) => d.count)),
    [days]
  );

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
    <div className={cn("flex gap-2", className)}>
      {/* Weekday labels (sparse) */}
      <div className="flex flex-col justify-between py-0.5 text-[10px] text-muted-foreground">
        {WEEKDAY_LABELS.map((d) => (
          <span key={d}>{d}</span>
        ))}
      </div>

      {/* Heatmap grid */}
      <div
        data-testid="heatmap-grid"
        className="grid auto-cols-min grid-flow-col grid-rows-7 gap-[2px]"
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
                "block size-3 rounded-[2px]",
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
