import { useMemo, useSyncExternalStore, type RefObject } from "react";
import type { TFunction } from "i18next";

import {
  CentralSkillEmptyState,
  CentralSkillFirstVisitEmptyState,
} from "@/components/central/CentralSkillEmptyStates";
import { buildCentralSkillCardProps } from "@/components/central/centralSkillCardProps";
import { UnifiedSkillCard } from "@/components/skill/UnifiedSkillCard";
import { VirtualizedGrid } from "@/components/ui/virtualized-grid";
import { VirtualizedList } from "@/components/ui/virtualized-list";
import { useSkillExplanationSummaries } from "@/hooks/useSkillExplanationSummaries";
import { useSkillCallCounts } from "@/hooks/useSkillCallCounts";
import {
  CENTRAL_SKILL_CARD_GRID_GAP,
  CENTRAL_SKILL_CARD_MAX_COLUMNS,
  CENTRAL_SKILL_CARD_MIN_WIDTH,
  centralVirtualItemHeight,
  centralSkillCardGridTemplateColumns,
} from "@/lib/centralSkillGrid";
import {
  getAppliedFontScale,
  subscribeAppliedFontScale,
} from "@/lib/displayFont";
import type { PlatformTarget } from "@/lib/platformTargetGroups";
import type { ViewDensity, ViewMode } from "@/lib/centralViewState";
import { getRepoDotColor } from "@/lib/tagColor";
import { cn } from "@/lib/utils";
import type { CentralSkillUpdateState, SkillWithLinks } from "@/types";

export function CentralSkillListContent({
  availableInstallAgents,
  contentRef,
  filteredSkills,
  isLoading,
  isSearchActive,
  viewMode = "grid",
  viewDensity = "comfortable",
  onAddSkillTag,
  onCreateSkillTag,
  onDelete,
  onDetail,
  onInstallTo,
  onUninstallFromPlatforms,
  onRemoveSkillTag,
  onTogglePlatform,
  onToggleSelection,
  onUpdateCentral,
  searchQuery,
  selectedCount,
  selectedSkillIdSet,
  setDetailButtonRef,
  skillsCount,
  sortedSkills,
  t,
  tags,
  togglingAgentId,
  updateStatuses,
  updatingSkillIds,
}: {
  availableInstallAgents: PlatformTarget[];
  contentRef: RefObject<HTMLDivElement | null>;
  filteredSkills: SkillWithLinks[];
  isLoading: boolean;
  isSearchActive: boolean;
  viewMode?: ViewMode;
  viewDensity?: ViewDensity;
  onAddSkillTag?: (skillId: string, tagId: string) => void;
  onCreateSkillTag?: (skillId: string, name: string) => void;
  onDelete: (skill: SkillWithLinks) => void;
  onDetail: (skillId: string) => void;
  onInstallTo: (skill: SkillWithLinks) => void;
  onUninstallFromPlatforms: (skill: SkillWithLinks) => void;
  onRemoveSkillTag?: (skillId: string, tagId: string) => void;
  onTogglePlatform: (skillId: string, agentId: string) => Promise<void>;
  onToggleSelection: (skillId: string) => void;
  onUpdateCentral: (skillIds: string[]) => void;
  searchQuery: string;
  selectedCount: number;
  selectedSkillIdSet: Set<string>;
  setDetailButtonRef: (skillId: string, node: HTMLButtonElement | null) => void;
  skillsCount: number;
  sortedSkills: SkillWithLinks[];
  t: TFunction;
  tags?: readonly { id: string; name: string; color?: string | null }[];
  togglingAgentId: string | null;
  updateStatuses: Record<string, CentralSkillUpdateState>;
  updatingSkillIds: string[];
}) {
  const summarySkillIds = useMemo(
    () => sortedSkills.map((skill) => skill.id),
    [sortedSkills],
  );
  const aiSummaries = useSkillExplanationSummaries(summarySkillIds, "zh");
  const skillNamesForUsage = useMemo(
    () => Array.from(new Set(sortedSkills.map((s) => s.name))),
    [sortedSkills],
  );
  const usageCounts = useSkillCallCounts(skillNamesForUsage, 30);
  const fontScale = useSyncExternalStore(
    subscribeAppliedFontScale,
    getAppliedFontScale,
    () => 1,
  );

  // 搜索激活时强制 list 单列（更易扫读结果）；其他场景遵循 viewMode。
  const effectiveView: ViewMode = isSearchActive ? "list" : viewMode;
  const cardDensity = viewDensity;

  function buildCardProps(skill: SkillWithLinks) {
    return buildCentralSkillCardProps(skill, {
      aiSummaries,
      usageCounts,
      selectedSkillIdSet,
      updateStatuses,
      updatingSkillIds,
      tags,
      t,
      density: cardDensity,
      setDetailButtonRef,
      onToggleSelection,
      onDetail,
      onInstallTo,
      onUninstallFromPlatforms,
      onUpdateCentral,
      onDelete,
      onAddSkillTag,
      onCreateSkillTag,
      onRemoveSkillTag,
    });
  }

  function renderListCard(skill: SkillWithLinks) {
    return <UnifiedSkillCard key={skill.id} {...buildCardProps(skill)} />;
  }

  function renderGridCard(skill: SkillWithLinks) {
    return (
      <UnifiedSkillCard
        key={skill.id}
        {...buildCardProps(skill)}
        platformIcons={{
          agents: availableInstallAgents,
          linkedAgents: skill.linked_agents,
          lockedAgentIds: skill.shared_root_agents,
          skillId: skill.id,
          onToggle: onTogglePlatform,
          togglingAgentId,
        }}
        footer={{
          repoName: skill.repository?.name,
          repoColor: skill.repository?.name
            ? getRepoDotColor(skill.repository.name)
            : undefined,
        }}
      />
    );
  }

  return (
    <div
      ref={contentRef}
      data-testid="central-skill-list-scroll"
      className={cn(
        "scrollbar-subtle min-w-0 flex-1 overflow-auto p-4 sm:p-6",
        selectedCount > 0 && "pb-28",
      )}
    >
      {isLoading ? (
        <CentralSkillEmptyState message={t("central.loading")} />
      ) : skillsCount === 0 ? (
        <CentralSkillFirstVisitEmptyState />
      ) : filteredSkills.length === 0 ? (
        <CentralSkillEmptyState
          message={t("central.noMatch", { query: searchQuery })}
        />
      ) : effectiveView === "list" ? (
        sortedSkills.length > 60 ? (
          <VirtualizedList
            items={sortedSkills}
            itemHeight={centralVirtualItemHeight("list", cardDensity, fontScale)}
            itemGap={12}
            overscan={8}
            scrollContainerRef={contentRef}
            itemKey={(skill) => skill.id}
            renderItem={(skill) => renderListCard(skill)}
          />
        ) : (
          <div className="space-y-3">
            {sortedSkills.map((skill) => renderListCard(skill))}
          </div>
        )
      ) : sortedSkills.length > 40 ? (
        <VirtualizedGrid
          items={sortedSkills}
          itemHeight={centralVirtualItemHeight("grid", cardDensity, fontScale)}
          rowGap={CENTRAL_SKILL_CARD_GRID_GAP}
          columnGap={CENTRAL_SKILL_CARD_GRID_GAP}
          overscanRows={3}
          minColumnWidth={CENTRAL_SKILL_CARD_MIN_WIDTH}
          maxColumns={CENTRAL_SKILL_CARD_MAX_COLUMNS}
          scrollContainerRef={contentRef}
          itemKey={(skill) => skill.id}
          renderItem={(skill) => renderGridCard(skill)}
        />
      ) : (
        <div
          className="grid gap-4"
          style={{
            gridTemplateColumns: centralSkillCardGridTemplateColumns(),
          }}
        >
          {sortedSkills.map((skill) => renderGridCard(skill))}
        </div>
      )}
    </div>
  );
}
