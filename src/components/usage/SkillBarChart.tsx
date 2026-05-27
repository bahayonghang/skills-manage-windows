import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { ArrowDownUp, ArrowUpDown } from "lucide-react";

import type { SkillCount } from "@/types/usage";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { useUsageStore } from "@/stores/usageStore";

type SortMode = "count" | "alpha" | "recent";

interface SkillBarChartProps {
  skills: SkillCount[];
  className?: string;
  /** 单元格点击行为：默认 → 后端 resolve skill_id → /skill/:id；resolve 失败回退 onSkillClick */
  onSkillClick?: (skill: string) => void;
}

const SORT_KEYS: SortMode[] = ["count", "alpha", "recent"];

const DEFAULT_DIRECTION: Record<SortMode, "asc" | "desc"> = {
  count: "desc",
  alpha: "asc",
  recent: "desc",
};

/**
 * 横向技能频次柱状图。
 * - 三种排序：count / alpha / recent，sort 按钮循环切换
 * - direction 按 Tab 切换 asc / desc
 * - 单击行：先 invoke `usage_resolve_skill_id` 拿到中央库 skill_id，
 *   有则跳 `/skill/:id`；无则触发 onSkillClick 回退（用于内嵌详情）
 *
 * 与 skilled CLI 的 `s` 键循环一致。
 */
export function SkillBarChart({ skills, className, onSkillClick }: SkillBarChartProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const resolveSkillId = useUsageStore((s) => s.resolveSkillId);

  const [sortMode, setSortMode] = useState<SortMode>("count");
  const [direction, setDirection] = useState<"asc" | "desc">("desc");

  const sorted = useMemo(() => sortSkills(skills, sortMode, direction), [skills, sortMode, direction]);
  const max = sorted.length > 0 ? Math.max(...sorted.map((s) => s.count), 1) : 1;

  function cycleSort() {
    const next = SORT_KEYS[(SORT_KEYS.indexOf(sortMode) + 1) % SORT_KEYS.length]!;
    setSortMode(next);
    setDirection(DEFAULT_DIRECTION[next]);
  }

  function toggleDirection() {
    setDirection((d) => (d === "asc" ? "desc" : "asc"));
  }

  async function handleClick(name: string) {
    const id = await resolveSkillId(name);
    if (id) {
      navigate(`/skill/${id}`);
      return;
    }
    onSkillClick?.(name);
  }

  if (sorted.length === 0) {
    return (
      <div
        className={cn(
          "rounded border border-dashed border-border/60 px-3 py-8 text-center text-xs text-muted-foreground",
          className
        )}
      >
        {t("skillUsage.empty.noData")}
      </div>
    );
  }

  return (
    <div className={cn("flex h-full flex-col", className)}>
      {/* Toolbar */}
      <div className="mb-2 flex items-center justify-between text-xs text-muted-foreground">
        <span data-testid="bar-chart-count">{sorted.length}</span>
        <div className="flex items-center gap-1">
          <Button size="sm" variant="ghost" onClick={cycleSort} data-testid="sort-cycle">
            <ArrowUpDown className="mr-1 size-3.5" />
            {t(`skillUsage.sort.${sortMode}`)}
          </Button>
          <Button
            size="sm"
            variant="ghost"
            onClick={toggleDirection}
            data-testid="sort-direction"
            aria-label={direction === "asc" ? "ascending" : "descending"}
          >
            <ArrowDownUp className={cn("size-3.5", direction === "asc" && "rotate-180")} />
          </Button>
        </div>
      </div>

      {/* Bars */}
      <ul className="flex-1 space-y-0.5 overflow-y-auto pr-1" data-testid="bar-chart-list">
        {sorted.map((s, idx) => {
          const widthPct = Math.max((s.count / max) * 100, 2);
          return (
            <li key={s.skill}>
              <button
                type="button"
                onClick={() => void handleClick(s.skill)}
                data-testid={`bar-row-${s.skill}`}
                className={cn(
                  "group flex w-full items-center gap-2 rounded px-1.5 py-1 text-left text-sm hover:bg-muted/50"
                )}
              >
                <span className="w-32 shrink-0 truncate" title={s.skill}>
                  {s.skill}
                </span>
                <span className="relative h-4 flex-1 overflow-hidden rounded bg-muted/40">
                  <span
                    className={cn(
                      "absolute left-0 top-0 block h-full rounded bg-gradient-to-r from-primary/90 to-primary/55",
                      "transition-[width] duration-300"
                    )}
                    style={{ width: `${widthPct}%` }}
                    aria-hidden
                  />
                </span>
                <span className="w-12 shrink-0 text-right text-xs tabular-nums text-muted-foreground">
                  {s.count.toLocaleString()}
                </span>
                {/* idx 用于排序变更后的可访问性提示，但 UI 不显式出 */}
                <span className="sr-only">rank {idx + 1}</span>
              </button>
            </li>
          );
        })}
      </ul>
    </div>
  );
}

function sortSkills(skills: SkillCount[], mode: SortMode, dir: "asc" | "desc"): SkillCount[] {
  const arr = [...skills];
  arr.sort((a, b) => {
    let cmp = 0;
    switch (mode) {
      case "count":
        cmp = a.count - b.count;
        break;
      case "alpha":
        cmp = a.skill.localeCompare(b.skill);
        break;
      case "recent":
        cmp = a.lastUsedMs - b.lastUsedMs;
        break;
    }
    return dir === "asc" ? cmp : -cmp;
  });
  return arr;
}
