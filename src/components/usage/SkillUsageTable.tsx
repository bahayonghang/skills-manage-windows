import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { ArrowUpRight } from "lucide-react";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { formatUsageRelativeTime } from "@/components/usage/usageFormat";
import type {
  SkillUsageSummary,
  UsageSkillMatchStatus,
} from "@/types/usage";

type SortMode = "count" | "recent" | "name";

interface SkillUsageTableProps {
  skills: SkillUsageSummary[];
  selectedSkill?: string | null;
  onSelect: (skill: string, trigger: HTMLButtonElement) => void;
  className?: string;
}

export function SkillUsageTable({
  skills,
  selectedSkill,
  onSelect,
  className,
}: SkillUsageTableProps) {
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const [sortMode, setSortMode] = useState<SortMode>("count");
  const sorted = useMemo(() => sortSkills(skills, sortMode), [skills, sortMode]);

  if (skills.length === 0) {
    return (
      <div className={cn("px-4 py-12 text-center text-sm", className)}>
        <p className="text-foreground">{t("skillUsage.empty.title")}</p>
        <p className="mt-1 text-xs text-muted-foreground">
          {t("skillUsage.empty.noData")}
        </p>
      </div>
    );
  }

  return (
    <div className={cn("flex min-h-0 flex-col", className)}>
      <div className="flex flex-wrap items-center justify-between gap-2 border-b border-border px-3 py-2">
        <span className="text-xs text-muted-foreground">
          {t("skillUsage.rankingCount", { count: sorted.length })}
        </span>
        <div
          role="group"
          aria-label={t("skillUsage.sort.label")}
          className="inline-flex rounded-md border border-border p-0.5"
        >
          {(["count", "recent", "name"] as const).map((mode) => (
            <button
              key={mode}
              type="button"
              aria-pressed={sortMode === mode}
              onClick={() => setSortMode(mode)}
              className={cn(
                "min-h-7 rounded px-2.5 text-xs transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                sortMode === mode
                  ? "bg-foreground text-background"
                  : "text-muted-foreground hover:text-foreground",
              )}
            >
              {t(`skillUsage.sort.${mode}`)}
            </button>
          ))}
        </div>
      </div>

      <div className="grid grid-cols-[minmax(8rem,1fr)_3.5rem_4.5rem_2rem] gap-2 border-b border-border px-3 py-2 text-ui-meta text-muted-foreground md:grid-cols-[minmax(9rem,1fr)_5.5rem_4rem_4rem_6rem_5rem_2rem]">
        <span>{t("skillUsage.columns.skill")}</span>
        <span className="hidden md:block">{t("skillUsage.columns.match")}</span>
        <span className="text-right">{t("skillUsage.columns.calls")}</span>
        <span className="hidden text-right md:block">
          {t("skillUsage.columns.sessions")}
        </span>
        <span className="hidden text-right md:block">
          {t("skillUsage.columns.estimate")}
        </span>
        <span className="text-right">{t("skillUsage.columns.lastUsed")}</span>
        <span className="sr-only">{t("skillUsage.openSkill")}</span>
      </div>

      <ul className="min-h-0 flex-1 divide-y divide-border/70 overflow-y-auto">
        {sorted.map((item) => {
          const selected = item.skill === selectedSkill;
          return (
            <li
              key={item.skill}
              data-testid={`usage-row-${item.skill}`}
              className={cn(
                "grid grid-cols-[minmax(8rem,1fr)_3.5rem_4.5rem_2rem] items-stretch gap-2 px-3 md:grid-cols-[minmax(9rem,1fr)_5.5rem_4rem_4rem_6rem_5rem_2rem]",
                selected && "bg-primary/7",
              )}
            >
              <button
                type="button"
                aria-pressed={selected}
                onClick={(event) => onSelect(item.skill, event.currentTarget)}
                className="col-span-3 grid min-w-0 grid-cols-subgrid items-center py-2.5 text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring md:col-span-6"
              >
                <span className="min-w-0 pr-2">
                  <span className="block truncate text-sm font-medium text-foreground">
                    {item.skill}
                  </span>
                  <span className="mt-0.5 block text-ui-meta text-muted-foreground md:hidden">
                    {t(`skillUsage.match.${item.matchStatus}`)} · {item.projects}{" "}
                    {t("skillUsage.projectsShort")}
                  </span>
                </span>
                <MatchStatus status={item.matchStatus} className="hidden md:block" />
                <span className="text-right text-sm font-medium tabular-nums">
                  {item.count.toLocaleString()}
                </span>
                <span className="hidden text-right text-xs tabular-nums text-muted-foreground md:block">
                  {item.sessions.toLocaleString()}
                </span>
                <span
                  className="hidden text-right text-xs tabular-nums text-muted-foreground md:block"
                  title={
                    item.staticTokenEstimate === null
                      ? t("skillUsage.staticEstimateUnavailable")
                      : t("skillUsage.staticEstimateTooltip", {
                          bytes: item.staticByteCount?.toLocaleString() ?? "—",
                        })
                  }
                >
                  {item.staticTokenEstimate === null
                    ? "—"
                    : `~${item.staticTokenEstimate.toLocaleString()}`}
                </span>
                <span className="text-right text-xs tabular-nums text-muted-foreground">
                  {formatUsageRelativeTime(item.lastUsedMs, i18n.language)}
                </span>
              </button>
              <div className="flex items-center justify-end">
                {item.resolvedSkillId && (
                  <Button
                    type="button"
                    size="icon-sm"
                    variant="ghost"
                    title={t("skillUsage.openSkill")}
                    aria-label={t("skillUsage.openSkillNamed", {
                      skill: item.skill,
                    })}
                    onClick={() =>
                      navigate(`/skill/${encodeURIComponent(item.resolvedSkillId!)}`)
                    }
                  >
                    <ArrowUpRight className="size-3.5" />
                  </Button>
                )}
              </div>
            </li>
          );
        })}
      </ul>
    </div>
  );
}

function MatchStatus({
  status,
  className,
}: {
  status: UsageSkillMatchStatus;
  className?: string;
}) {
  const { t } = useTranslation();
  return (
    <span className={cn("truncate text-xs text-muted-foreground", className)}>
      {t(`skillUsage.match.${status}`)}
    </span>
  );
}

function sortSkills(
  skills: SkillUsageSummary[],
  mode: SortMode,
): SkillUsageSummary[] {
  return [...skills].sort((a, b) => {
    if (mode === "name") return a.skill.localeCompare(b.skill);
    if (mode === "recent") {
      return b.lastUsedMs - a.lastUsedMs || a.skill.localeCompare(b.skill);
    }
    return (
      b.count - a.count ||
      b.lastUsedMs - a.lastUsedMs ||
      a.skill.localeCompare(b.skill)
    );
  });
}
