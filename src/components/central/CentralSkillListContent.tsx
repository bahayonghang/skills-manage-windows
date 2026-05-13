import type { RefObject } from "react";
import type { TFunction } from "i18next";

import { CentralSkillEmptyState, CentralSkillFirstVisitEmptyState } from "@/components/central/CentralSkillEmptyStates";
import { UnifiedSkillCard } from "@/components/skill/UnifiedSkillCard";
import { VirtualizedGrid } from "@/components/ui/virtualized-grid";
import { VirtualizedList } from "@/components/ui/virtualized-list";
import type { PlatformTarget } from "@/lib/platformTargetGroups";
import type {
  CentralSkillUpdateState,
  SkillWithLinks,
} from "@/types";

export function CentralSkillListContent({
  availableInstallAgents,
  contentRef,
  filteredSkills,
  isLoading,
  isSearchActive,
  onDelete,
  onDetail,
  onInstallTo,
  onTogglePlatform,
  onToggleSelection,
  onUpdateCentral,
  searchQuery,
  selectedSkillIdSet,
  setDetailButtonRef,
  skillsCount,
  sortedSkills,
  t,
  togglingAgentId,
  updateStatuses,
  updatingSkillIds,
}: {
  availableInstallAgents: PlatformTarget[];
  contentRef: RefObject<HTMLDivElement | null>;
  filteredSkills: SkillWithLinks[];
  isLoading: boolean;
  isSearchActive: boolean;
  onDelete: (skill: SkillWithLinks) => void;
  onDetail: (skillId: string) => void;
  onInstallTo: (skill: SkillWithLinks) => void;
  onTogglePlatform: (skillId: string, agentId: string) => Promise<void>;
  onToggleSelection: (skillId: string) => void;
  onUpdateCentral: (skillIds: string[]) => void;
  searchQuery: string;
  selectedSkillIdSet: Set<string>;
  setDetailButtonRef: (skillId: string, node: HTMLButtonElement | null) => void;
  skillsCount: number;
  sortedSkills: SkillWithLinks[];
  t: TFunction;
  togglingAgentId: string | null;
  updateStatuses: Record<string, CentralSkillUpdateState>;
  updatingSkillIds: string[];
}) {
  function renderSearchResult(skill: SkillWithLinks) {
    return (
      <UnifiedSkillCard
        key={skill.id}
        name={skill.name}
        description={skill.description}
        checkbox={{
          checked: selectedSkillIdSet.has(skill.id),
          onChange: () => onToggleSelection(skill.id),
        }}
        tags={(skill.tags ?? []).map((tag) => ({ key: tag.id, label: tag.name }))}
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
        detailButtonRef={(node) => setDetailButtonRef(skill.id, node)}
        className="h-[132px]"
      />
    );
  }

  function renderGridCard(skill: SkillWithLinks) {
    return (
      <UnifiedSkillCard
        key={skill.id}
        name={skill.name}
        description={skill.description}
        checkbox={{
          checked: selectedSkillIdSet.has(skill.id),
          onChange: () => onToggleSelection(skill.id),
        }}
        tags={(skill.tags ?? []).map((tag) => ({ key: tag.id, label: tag.name }))}
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
        detailButtonRef={(node) => setDetailButtonRef(skill.id, node)}
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
    );
  }

  return (
    <div ref={contentRef} className="scrollbar-subtle flex-1 overflow-auto p-6">
      {isLoading ? (
        <CentralSkillEmptyState message={t("central.loading")} />
      ) : skillsCount === 0 ? (
        <CentralSkillFirstVisitEmptyState />
      ) : filteredSkills.length === 0 ? (
        <CentralSkillEmptyState message={t("central.noMatch", { query: searchQuery })} />
      ) : isSearchActive ? (
        sortedSkills.length > 60 ? (
          <VirtualizedList
            items={sortedSkills}
            itemHeight={132}
            itemGap={12}
            overscan={8}
            scrollContainerRef={contentRef}
            itemKey={(skill) => skill.id}
            renderItem={(skill) => renderSearchResult(skill)}
          />
        ) : (
          <div className="space-y-3">
            {sortedSkills.map((skill) => renderSearchResult(skill))}
          </div>
        )
      ) : sortedSkills.length > 40 ? (
        <VirtualizedGrid
          items={sortedSkills}
          itemHeight={212}
          rowGap={16}
          columnGap={16}
          overscanRows={3}
          minColumnWidth={420}
          maxColumns={2}
          scrollContainerRef={contentRef}
          itemKey={(skill) => skill.id}
          renderItem={(skill) => renderGridCard(skill)}
        />
      ) : (
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
          {sortedSkills.map((skill) => renderGridCard(skill))}
        </div>
      )}
    </div>
  );
}
