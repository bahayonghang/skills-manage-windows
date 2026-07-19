import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { ArrowUpRight } from "lucide-react";

import { Button } from "@/components/ui/button";
import { projectShort } from "@/lib/timeAgo";
import { cn } from "@/lib/utils";
import { formatUsageRelativeTime } from "@/components/usage/usageFormat";
import type { RecentSkillCall } from "@/types/usage";

interface RecentCallsFeedProps {
  calls: RecentSkillCall[];
  onSelect: (skill: string, trigger: HTMLButtonElement) => void;
  className?: string;
  limit?: number;
}

export function RecentCallsFeed({
  calls,
  onSelect,
  className,
  limit = 20,
}: RecentCallsFeedProps) {
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const list = calls.slice(0, limit);

  if (list.length === 0) {
    return (
      <div className={cn("px-4 py-10 text-center text-xs text-muted-foreground", className)}>
        {t("skillUsage.empty.noData")}
      </div>
    );
  }

  return (
    <ul data-testid="recent-feed" className={cn("divide-y divide-border/60", className)}>
      {list.map((call) => (
        <li
          key={`${call.skill}-${call.timestampMs}-${call.sessionId}`}
          data-testid={`recent-row-${call.skill}-${call.timestampMs}`}
          className="grid grid-cols-[4rem_minmax(0,1fr)_minmax(5rem,0.8fr)_2rem] items-center gap-2 px-3 py-2"
        >
          <span
            className="text-xs tabular-nums text-muted-foreground"
            title={new Date(call.timestampMs).toLocaleString()}
          >
            {formatUsageRelativeTime(call.timestampMs, i18n.language)}
          </span>
          <button
            type="button"
            onClick={(event) => onSelect(call.skill, event.currentTarget)}
            className="min-w-0 text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
          >
            <span className="block truncate text-sm text-foreground hover:text-primary">
              {call.skill}
            </span>
            <span className="block truncate text-ui-meta text-muted-foreground">
              {call.source}
            </span>
          </button>
          <span className="truncate text-right text-xs text-muted-foreground">
            {projectShort(call.project)}
          </span>
          {call.resolvedSkillId ? (
            <Button
              type="button"
              size="icon-sm"
              variant="ghost"
              title={t("skillUsage.openSkill")}
              aria-label={t("skillUsage.openSkillNamed", { skill: call.skill })}
              onClick={() =>
                navigate(`/skill/${encodeURIComponent(call.resolvedSkillId!)}`)
              }
            >
              <ArrowUpRight className="size-3.5" />
            </Button>
          ) : (
            <span aria-hidden />
          )}
        </li>
      ))}
    </ul>
  );
}
