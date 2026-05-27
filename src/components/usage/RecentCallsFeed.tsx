import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";

import type { SkillCall } from "@/types/usage";
import { timeAgo, projectShort } from "@/lib/timeAgo";
import { cn } from "@/lib/utils";
import { useUsageStore } from "@/stores/usageStore";

interface RecentCallsFeedProps {
  calls: SkillCall[];
  className?: string;
  limit?: number;
}

/**
 * 最近调用 feed —— 按时间倒序展示最多 N 条 SkillCall。
 *
 * 单元素布局：[time-ago] [skill 链接] [source pill] [project short]
 *
 * 点击 skill 链接走 SkillBarChart 同款逻辑：resolveSkillId → /skill/:id；
 * 没匹配到中央库时只是高亮该名（不动作）。
 */
export function RecentCallsFeed({
  calls,
  className,
  limit = 20,
}: RecentCallsFeedProps) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const resolveSkillId = useUsageStore((s) => s.resolveSkillId);

  const list = calls.slice(0, limit);

  if (list.length === 0) {
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

  async function handleClick(skill: string) {
    const id = await resolveSkillId(skill);
    if (id) navigate(`/skill/${id}`);
  }

  return (
    <ul
      data-testid="recent-feed"
      className={cn("divide-y divide-border/60 text-sm", className)}
    >
      {list.map((c) => (
        <li
          key={`${c.skill}-${c.timestampMs}-${c.sessionId}`}
          data-testid={`recent-row-${c.skill}-${c.timestampMs}`}
          className="grid grid-cols-[3.5rem_minmax(0,1fr)_minmax(0,1fr)_5.5rem] items-center gap-2 px-3 py-1.5"
        >
          <span className="text-xs tabular-nums text-muted-foreground" title={new Date(c.timestampMs).toISOString()}>
            {timeAgo(c.timestampMs)}
          </span>
          <button
            type="button"
            onClick={() => void handleClick(c.skill)}
            className="truncate text-left text-foreground hover:text-primary hover:underline"
          >
            {c.skill}
          </button>
          <span
            className="truncate text-xs text-muted-foreground"
            title={c.project}
          >
            {projectShort(c.project)}
          </span>
          <span
            className={cn(
              "justify-self-end rounded-full border border-border bg-muted/40 px-2 py-0.5",
              "text-[10px] uppercase tracking-wide text-muted-foreground"
            )}
          >
            {c.source}
          </span>
        </li>
      ))}
    </ul>
  );
}
