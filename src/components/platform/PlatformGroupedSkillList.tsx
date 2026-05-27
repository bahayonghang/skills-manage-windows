import { useState, type ReactNode } from "react";
import { ChevronDown, ChevronRight } from "lucide-react";

import type { ScannedSkill } from "@/types";
import type { PlatformSkillGroup } from "@/lib/platformSkillViewModel";

export function PlatformGroupedSkillList({
  groups,
  renderSkillCard,
}: {
  groups: PlatformSkillGroup[];
  renderSkillCard: (skill: ScannedSkill) => ReactNode;
}) {
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({});

  return (
    <div className="space-y-6">
      {groups.map((group) => {
        const isCollapsed = collapsed[group.key] ?? false;
        const Caret = isCollapsed ? ChevronRight : ChevronDown;
        return (
          <section key={group.key} aria-label={group.label}>
            <button
              type="button"
              onClick={() =>
                setCollapsed((prev) => ({ ...prev, [group.key]: !isCollapsed }))
              }
              aria-expanded={!isCollapsed}
              data-testid={`platform-group-header-${group.key}`}
              className="sticky top-0 z-10 mb-3 flex w-full items-center gap-2 rounded-md border border-border/60 bg-background/95 px-3 py-2 text-left text-sm font-medium backdrop-blur"
            >
              <Caret className="size-4 shrink-0 text-muted-foreground" />
              <span className="min-w-0 flex-1 truncate">{group.label}</span>
              <span className="shrink-0 rounded-md border border-border/80 bg-muted/60 px-1.5 py-0.5 font-mono text-[10px] tabular-nums text-muted-foreground">
                {group.skills.length}
              </span>
            </button>
            {!isCollapsed && (
              <div
                className="grid grid-cols-1 gap-4 lg:grid-cols-2"
                data-testid={`platform-group-body-${group.key}`}
              >
                {group.skills.map((skill) => renderSkillCard(skill))}
              </div>
            )}
          </section>
        );
      })}
    </div>
  );
}
