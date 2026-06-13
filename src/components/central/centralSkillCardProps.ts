import type { TFunction } from "i18next";

import type { UnifiedSkillCardProps } from "@/components/skill/UnifiedSkillCard";
import { statusAccentOf } from "@/lib/centralSkillCardStatus";
import { getVisibleSkillTags } from "@/lib/centralTags";
import type { CentralSkillUpdateState, SkillWithLinks } from "@/types";

interface CentralSkillCardTag {
  id: string;
  name: string;
  color?: string | null;
}

interface CentralSkillCardPropsContext {
  aiSummaries: Record<string, string | null | undefined>;
  usageCounts?: Record<string, number>;
  selectedSkillIdSet: Set<string>;
  updateStatuses: Record<string, CentralSkillUpdateState>;
  updatingSkillIds: string[];
  tags?: readonly CentralSkillCardTag[];
  t: TFunction;
  density: UnifiedSkillCardProps["density"];
  setDetailButtonRef: (skillId: string, node: HTMLButtonElement | null) => void;
  onToggleSelection: (skillId: string) => void;
  onDetail: (skillId: string) => void;
  onInstallTo: (skill: SkillWithLinks) => void;
  onUpdateCentral: (skillIds: string[]) => void;
  onDelete: (skill: SkillWithLinks) => void;
  onAddSkillTag?: (skillId: string, tagId: string) => void;
  onCreateSkillTag?: (skillId: string, name: string) => void;
  onRemoveSkillTag?: (skillId: string, tagId: string) => void;
}

export function buildCentralSkillCardProps(
  skill: SkillWithLinks,
  context: CentralSkillCardPropsContext,
): UnifiedSkillCardProps {
  const skillTags = getVisibleSkillTags(skill.tags ?? []).map((tag) => ({
    id: tag.id,
    name: tag.name,
    color: tag.color,
  }));
  const allTags = getVisibleSkillTags(context.tags ?? []).map((tag) => ({
    id: tag.id,
    name: tag.name,
    color: tag.color,
  }));
  const status = context.updateStatuses[skill.id]?.status;
  const statusAccent = statusAccentOf(status);
  const statusChipLabel =
    statusAccent && status
      ? context.t(`central.updateStatus.${status}`)
      : undefined;

  return {
    name: skill.name,
    description: skill.description,
    aiSummary: context.aiSummaries[skill.id],
    usageBadge: context.usageCounts?.[skill.name],
    statusAccent,
    statusChipLabel,
    checkbox: {
      checked: context.selectedSkillIdSet.has(skill.id),
      onChange: () => context.onToggleSelection(skill.id),
    },
    tags: skillTags.map((tag) => ({
      key: tag.id,
      label: tag.name,
    })),
    publisher: skill.repository?.name,
    updateStatus: context.updateStatuses[skill.id]
      ? {
          ...context.updateStatuses[skill.id],
          isUpdating: context.updatingSkillIds.includes(skill.id),
        }
      : undefined,
    onDetail: () => context.onDetail(skill.id),
    onInstallTo: () => context.onInstallTo(skill),
    onUpdateCentral: () => context.onUpdateCentral([skill.id]),
    onDeleteFromCentral: () => context.onDelete(skill),
    detailButtonRef: (node) => context.setDetailButtonRef(skill.id, node),
    editableTags:
      context.onAddSkillTag &&
      context.onCreateSkillTag &&
      context.onRemoveSkillTag
        ? {
            tags: skillTags,
            allTags,
            onAdd: (tagId) => context.onAddSkillTag?.(skill.id, tagId),
            onCreate: (name) => context.onCreateSkillTag?.(skill.id, name),
            onRemove: (tagId) => context.onRemoveSkillTag?.(skill.id, tagId),
          }
        : undefined,
    density: context.density,
  };
}
