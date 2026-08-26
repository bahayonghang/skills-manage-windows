import { Search, X } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import type { SkillsCliGroupBy } from "@/pages/skillsCliViewModel";
import type { SkillsCliInstallTarget } from "@/types";

export interface SkillsCliToolbarProps {
  query: string;
  onQueryChange: (query: string) => void;
  groupBy: SkillsCliGroupBy;
  onGroupByChange: (groupBy: SkillsCliGroupBy) => void;
  platformFilter: string | null;
  onPlatformFilterChange: (platformId: string | null) => void;
  unlinkedOnly: boolean;
  onUnlinkedOnlyChange: (value: boolean) => void;
  selectMode: boolean;
  onSelectModeChange: (value: boolean) => void;
  targets: readonly SkillsCliInstallTarget[];
  onExportAll?: () => void;
  isExporting?: boolean;
}

const GROUP_OPTIONS: { id: SkillsCliGroupBy; labelKey: string }[] = [
  { id: "repo", labelKey: "skillsCli.groupByRepo" },
  { id: "platform", labelKey: "skillsCli.groupByPlatform" },
  { id: "status", labelKey: "skillsCli.groupByStatus" },
  { id: "none", labelKey: "skillsCli.groupByNone" },
];

const ICON_HIT =
  "relative size-8 after:absolute after:left-1/2 after:top-1/2 after:size-10 after:-translate-x-1/2 after:-translate-y-1/2 after:content-['']";

export function SkillsCliToolbar({
  query,
  onQueryChange,
  groupBy,
  onGroupByChange,
  platformFilter,
  onPlatformFilterChange,
  unlinkedOnly,
  onUnlinkedOnlyChange,
  selectMode,
  onSelectModeChange,
  targets,
  onExportAll,
  isExporting = false,
}: SkillsCliToolbarProps) {
  const { t } = useTranslation();
  const exportDisabled = onExportAll == null || isExporting;

  return (
    <div
      data-testid="skills-cli-toolbar"
      className="flex flex-wrap items-center gap-2"
    >
      <div className="relative min-w-[12rem] flex-1">
        <Search
          className="pointer-events-none absolute left-2.5 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground"
          aria-hidden
        />
        <Input
          value={query}
          onChange={(event) => onQueryChange(event.target.value)}
          placeholder={t("skillsCli.searchPlaceholder")}
          aria-label={t("skillsCli.searchAria")}
          className="pl-8 pr-8"
        />
        {query ? (
          <button
            type="button"
            className={cn(
              ICON_HIT,
              "absolute right-0.5 top-1/2 -translate-y-1/2 rounded-md text-muted-foreground focus-visible:opacity-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
            )}
            onClick={() => onQueryChange("")}
            aria-label={t("skillsCli.searchClear")}
          >
            <X className="mx-auto size-3.5" />
          </button>
        ) : null}
      </div>
      <div
        className="flex flex-wrap gap-1"
        role="group"
        aria-label={t("skillsCli.groupBy")}
      >
        {GROUP_OPTIONS.map((option) => (
          <button
            key={option.id}
            type="button"
            aria-pressed={groupBy === option.id}
            className={cn(
              "rounded-md border px-2 py-1 text-xs font-medium",
              groupBy === option.id
                ? "border-primary bg-primary/10 text-primary-text"
                : "border-border text-muted-foreground hover:bg-accent/40",
            )}
            onClick={() => onGroupByChange(option.id)}
          >
            {t(option.labelKey)}
          </button>
        ))}
      </div>
      <div
        className="flex flex-wrap gap-1"
        role="group"
        aria-label={t("skillsCli.platformFilter")}
      >
        {targets.map((target) => (
          <button
            key={target.id}
            type="button"
            aria-pressed={platformFilter === target.id}
            aria-label={t("skillsCli.filterChip", { name: target.displayName })}
            className={cn(
              "rounded-full border px-2 py-1 text-xs font-medium",
              platformFilter === target.id
                ? "border-primary bg-primary/10 text-primary-text"
                : "border-border text-muted-foreground hover:bg-accent/40",
            )}
            onClick={() =>
              onPlatformFilterChange(
                platformFilter === target.id ? null : target.id,
              )
            }
          >
            {target.displayName}
          </button>
        ))}
      </div>
      <button
        type="button"
        aria-pressed={unlinkedOnly}
        className={cn(
          "rounded-md border px-2 py-1 text-xs font-medium",
          unlinkedOnly
            ? "border-primary bg-primary/10 text-primary-text"
            : "border-border text-muted-foreground hover:bg-accent/40",
        )}
        onClick={() => onUnlinkedOnlyChange(!unlinkedOnly)}
      >
        {t("skillsCli.unlinkedOnly")}
      </button>
      <button
        type="button"
        aria-pressed={selectMode}
        className={cn(
          "rounded-md border px-2 py-1 text-xs font-medium",
          selectMode
            ? "border-primary bg-primary/10 text-primary-text"
            : "border-border text-muted-foreground hover:bg-accent/40",
        )}
        onClick={() => onSelectModeChange(!selectMode)}
      >
        {t("skillsCli.selectMode")}
      </button>
      <Button
        type="button"
        variant="outline"
        disabled={exportDisabled}
        onClick={() => onExportAll?.()}
        aria-label={t("skillsCli.exportAll")}
      >
        {t("skillsCli.exportAll")}
      </Button>
    </div>
  );
}
