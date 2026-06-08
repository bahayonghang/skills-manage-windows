import { useMemo, type RefObject } from "react";
import type { TFunction } from "i18next";

import {
  CentralSkillEmptyState,
  CentralSkillFirstVisitEmptyState,
} from "@/components/central/CentralSkillEmptyStates";
import { UnifiedSkillCard } from "@/components/skill/UnifiedSkillCard";
import { VirtualizedGrid } from "@/components/ui/virtualized-grid";
import { VirtualizedList } from "@/components/ui/virtualized-list";
import { useSkillExplanationSummaries } from "@/hooks/useSkillExplanationSummaries";
import { useSkillCallCounts } from "@/hooks/useSkillCallCounts";
import { statusAccentOf } from "@/lib/centralSkillCardStatus";
import {
  CENTRAL_SKILL_CARD_GRID_GAP,
  CENTRAL_SKILL_CARD_MAX_COLUMNS,
  CENTRAL_SKILL_CARD_MIN_WIDTH,
  centralSkillCardGridTemplateColumns,
} from "@/lib/centralSkillGrid";
import type { PlatformTarget } from "@/lib/platformTargetGroups";
import type { ViewDensity, ViewMode } from "@/lib/centralViewState";
import { getRepoDotColor } from "@/lib/tagColor";
import { cn } from "@/lib/utils";
import type { CentralSkillUpdateState, SkillWithLinks } from "@/types";

// 卡片高度常量 —— 必须与 UnifiedSkillCard 的 min-h 保持一致，
// 否则虚拟列表会出现间隙或重叠。
const LIST_ITEM_HEIGHT_COMFORTABLE = 196;
const LIST_ITEM_HEIGHT_COMPACT = 148;
// grid 卡含 footer 分隔区（repo·调用数｜平台点）+ 底部 tag 行；高度贴合内容，消除中部死白。
const GRID_ITEM_HEIGHT_COMFORTABLE = 192;
const GRID_ITEM_HEIGHT_COMPACT = 172;

function listItemHeight(density: ViewDensity): number {
  return density === "compact"
    ? LIST_ITEM_HEIGHT_COMPACT
    : LIST_ITEM_HEIGHT_COMFORTABLE;
}

function gridItemHeight(density: ViewDensity): number {
  return density === "compact"
    ? GRID_ITEM_HEIGHT_COMPACT
    : GRID_ITEM_HEIGHT_COMFORTABLE;
}

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

  // 搜索激活时强制 list 单列（更易扫读结果）；其他场景遵循 viewMode。
  const effectiveView: ViewMode = isSearchActive ? "list" : viewMode;
  const cardDensity = viewDensity;

  function renderListCard(skill: SkillWithLinks) {
    const skillTags = (skill.tags ?? []).map((tag) => ({
      id: tag.id,
      name: tag.name,
      color: tag.color,
    }));
    const allTags = (tags ?? []).map((tag) => ({
      id: tag.id,
      name: tag.name,
      color: tag.color,
    }));
    const status = updateStatuses[skill.id]?.status;
    const statusAccent = statusAccentOf(status);
    const statusChipLabel =
      statusAccent && status ? t(`central.updateStatus.${status}`) : undefined;

    return (
      <UnifiedSkillCard
        key={skill.id}
        name={skill.name}
        description={skill.description}
        aiSummary={aiSummaries[skill.id]}
        usageBadge={usageCounts?.[skill.name]}
        statusAccent={statusAccent}
        statusChipLabel={statusChipLabel}
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
        detailButtonRef={(node) => setDetailButtonRef(skill.id, node)}
        editableTags={
          onAddSkillTag && onCreateSkillTag && onRemoveSkillTag
            ? {
                tags: skillTags,
                allTags,
                onAdd: (tagId) => onAddSkillTag(skill.id, tagId),
                onCreate: (name) => onCreateSkillTag(skill.id, name),
                onRemove: (tagId) => onRemoveSkillTag(skill.id, tagId),
              }
            : undefined
        }
        density={cardDensity}
      />
    );
  }

  function renderGridCard(skill: SkillWithLinks) {
    const skillTags = (skill.tags ?? []).map((tag) => ({
      id: tag.id,
      name: tag.name,
      color: tag.color,
    }));
    const allTags = (tags ?? []).map((tag) => ({
      id: tag.id,
      name: tag.name,
      color: tag.color,
    }));
    const status = updateStatuses[skill.id]?.status;
    const statusAccent = statusAccentOf(status);
    const statusChipLabel =
      statusAccent && status ? t(`central.updateStatus.${status}`) : undefined;

    return (
      <UnifiedSkillCard
        key={skill.id}
        name={skill.name}
        description={skill.description}
        aiSummary={aiSummaries[skill.id]}
        usageBadge={usageCounts?.[skill.name]}
        statusAccent={statusAccent}
        statusChipLabel={statusChipLabel}
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
        detailButtonRef={(node) => setDetailButtonRef(skill.id, node)}
        platformIcons={{
          agents: availableInstallAgents,
          linkedAgents: skill.linked_agents,
          lockedAgentIds: skill.shared_root_agents,
          skillId: skill.id,
          onToggle: onTogglePlatform,
          togglingAgentId,
        }}
        editableTags={
          onAddSkillTag && onCreateSkillTag && onRemoveSkillTag
            ? {
                tags: skillTags,
                allTags,
                onAdd: (tagId) => onAddSkillTag(skill.id, tagId),
                onCreate: (name) => onCreateSkillTag(skill.id, name),
                onRemove: (tagId) => onRemoveSkillTag(skill.id, tagId),
              }
            : undefined
        }
        footer={{
          repoName: skill.repository?.name,
          repoColor: skill.repository?.name
            ? getRepoDotColor(skill.repository.name)
            : undefined,
        }}
        density={cardDensity}
      />
    );
  }

  return (
    <div
      ref={contentRef}
      data-testid="central-skill-list-scroll"
      className={cn(
        "scrollbar-subtle flex-1 overflow-auto p-6",
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
            itemHeight={listItemHeight(cardDensity)}
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
          itemHeight={gridItemHeight(cardDensity)}
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
