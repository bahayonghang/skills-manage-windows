import type { RefObject } from "react";
import { ChevronDown, ChevronRight } from "lucide-react";
import { useState } from "react";
import type { TFunction } from "i18next";

import { UnifiedSkillCard } from "@/components/skill/UnifiedSkillCard";
import type { PlatformTarget } from "@/lib/platformTargetGroups";
import type { SkillGroup } from "@/lib/centralGrouping";
import type { CentralSkillUpdateState, SkillWithLinks } from "@/types";

/**
 * V2 列表分组渲染（M4）。
 *
 * 当 `groupBy !== "none"` 时由 Shell 用此组件替代 `CentralSkillListContent`。
 * - 每组一个折叠 header（计数 + ChevronDown/Right）。
 * - 默认全部展开；用户可逐组折叠（hover 不显示按钮，整行可点）。
 * - 不接 VirtualizedGrid：分组本身是"显式信息密度"，超大列表场景靠用户切回
 *   `groupBy=none` 即可。
 */

export interface CentralGroupedSkillListProps {
  contentRef: RefObject<HTMLDivElement | null>;
  groups: SkillGroup[];
  availableInstallAgents: PlatformTarget[];
  selectedSkillIdSet: Set<string>;
  updateStatuses: Record<string, CentralSkillUpdateState>;
  updatingSkillIds: string[];
  togglingAgentId: string | null;
  setDetailButtonRef: (skillId: string, node: HTMLButtonElement | null) => void;
  onToggleSelection: (skillId: string) => void;
  onDetail: (skillId: string) => void;
  onInstallTo: (skill: SkillWithLinks) => void;
  onUpdateCentral: (skillIds: string[]) => void;
  onDelete: (skill: SkillWithLinks) => void;
  onTogglePlatform: (skillId: string, agentId: string) => Promise<void>;
  t: TFunction;
}

export function CentralGroupedSkillList({
  contentRef,
  groups,
  availableInstallAgents,
  selectedSkillIdSet,
  updateStatuses,
  updatingSkillIds,
  togglingAgentId,
  setDetailButtonRef,
  onToggleSelection,
  onDetail,
  onInstallTo,
  onUpdateCentral,
  onDelete,
  onTogglePlatform,
  t,
}: CentralGroupedSkillListProps) {
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({});

  return (
    <div ref={contentRef} className="scrollbar-subtle flex-1 overflow-auto p-6">
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
                className="sticky top-0 z-10 mb-3 flex w-full items-center gap-2 rounded-md border border-border/60 bg-background/95 px-3 py-2 text-left text-sm font-medium backdrop-blur"
                data-testid={`group-header-${group.key}`}
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
                  data-testid={`group-body-${group.key}`}
                >
                  {group.skills.map((skill) => (
                    <UnifiedSkillCard
                      key={skill.id}
                      name={skill.name}
                      description={skill.description}
                      checkbox={{
                        checked: selectedSkillIdSet.has(skill.id),
                        onChange: () => onToggleSelection(skill.id),
                      }}
                      tags={(skill.tags ?? []).map((tag) => ({
                        key: tag.id,
                        label: tag.name,
                      }))}
                      publisher={skill.repository?.name}
                      updateStatus={
                        updateStatuses[skill.id]
                          ? {
                              ...updateStatuses[skill.id],
                              isUpdating: updatingSkillIds.includes(skill.id),
                            }
                          : undefined
                      }
                      onDetail={() => onDetail(skill.id)}
                      onInstallTo={() => onInstallTo(skill)}
                      onUpdateCentral={() => onUpdateCentral([skill.id])}
                      onDeleteFromCentral={() => onDelete(skill)}
                      detailButtonRef={(node) =>
                        setDetailButtonRef(skill.id, node)
                      }
                      platformIcons={{
                        agents: availableInstallAgents,
                        linkedAgents: skill.linked_agents,
                        lockedAgentIds: skill.shared_root_agents,
                        skillId: skill.id,
                        onToggle: onTogglePlatform,
                        togglingAgentId,
                      }}
                      className="h-[212px]"
                    />
                  ))}
                </div>
              )}
            </section>
          );
        })}
        {groups.length === 0 && (
          <p className="text-center text-sm text-muted-foreground">
            {t("central.v2.groupByEmpty")}
          </p>
        )}
      </div>
    </div>
  );
}
