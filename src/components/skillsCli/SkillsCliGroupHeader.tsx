import { ChevronDown } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import type { SkillsCliBucket } from "@/pages/skillsCliViewModel";

export interface SkillsCliGroupHeaderProps {
  bucket: SkillsCliBucket;
  label: string;
  expanded: boolean;
  panelId: string;
  onToggle: () => void;
  updateCount?: number;
  onSelectAll?: () => void;
  onUpdateAll?: () => void;
}

export function SkillsCliGroupHeader({
  bucket,
  label,
  expanded,
  panelId,
  onToggle,
  updateCount = 0,
  onSelectAll,
  onUpdateAll,
}: SkillsCliGroupHeaderProps) {
  const { t } = useTranslation();
  const headerId = `skills-cli-group-${bucket.id}`;

  return (
    <div
      id={headerId}
      data-testid={`skills-cli-group-header-${bucket.id}`}
      className="sticky top-0 z-10 flex flex-wrap items-center gap-2 border-b border-border bg-background py-2"
    >
      <button
        type="button"
        className="flex min-w-0 flex-1 items-center gap-2 rounded-md text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        aria-expanded={expanded}
        aria-controls={panelId}
        onClick={onToggle}
      >
        <ChevronDown
          className={cn(
            "size-4 shrink-0 transition-transform",
            !expanded && "rotate-[-90deg]",
          )}
          aria-hidden
        />
        <span
          data-testid={`skills-cli-group-title-${bucket.id}`}
          className="truncate text-sm font-medium"
        >
          {label}
        </span>
      </button>
      <span
        data-testid={`skills-cli-group-count-${bucket.id}`}
        className="text-ui-meta font-normal tabular-nums text-muted-foreground"
      >
        {t("skillsCli.groupCount", {
          skills: bucket.skillCount,
          links: bucket.managedLinkCount,
        })}
      </span>
      <div
        data-testid={`skills-cli-group-actions-${bucket.id}`}
        className="flex flex-wrap items-center gap-2"
      >
        <span
          data-testid={`skills-cli-update-badge-${bucket.id}`}
          className={cn(
            "min-w-6 rounded-full px-1.5 py-0.5 text-center text-ui-micro tabular-nums",
            updateCount > 0
              ? "bg-warning/10 text-warning-foreground"
              : "text-muted-foreground",
          )}
          aria-label={t("skillsCli.updateBadge", { count: updateCount })}
        >
          {updateCount > 0 ? updateCount : ""}
        </span>
        {onSelectAll ? (
          <Button type="button" size="sm" variant="ghost" onClick={onSelectAll}>
            {t("skillsCli.selectAll")}
          </Button>
        ) : null}
        {onUpdateAll ? (
          <Button type="button" size="sm" variant="ghost" onClick={onUpdateAll}>
            {t("skillsCli.updateAll")}
          </Button>
        ) : null}
      </div>
    </div>
  );
}
