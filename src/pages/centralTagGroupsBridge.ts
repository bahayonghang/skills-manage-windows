/**
 * `useCentralTagGroupsBridge` —— Tag Groups store 与 V2 UI 的桥接。
 *
 * 把 prompt/confirm 风格的命令式 CRUD 收口到一个 hook，CentralSkillsView 只
 * 关心是否启用与 t 注入。
 */

import { useEffect } from "react";
import type { TFunction } from "i18next";

import { useTagGroupsStore } from "@/stores/tagGroupsStore";
import type { TagGroup } from "@/types";

export interface UseCentralV2TagGroupsBridgeArgs {
  enabled: boolean;
  t: TFunction;
}

export interface UseCentralV2TagGroupsBridgeResult {
  tagGroups: TagGroup[];
  handleCreateTagGroup: (defaultName?: string) => void;
  handleRenameTagGroup: (group: TagGroup) => void;
  handleDeleteTagGroup: (group: TagGroup) => void;
  handleAssignTagToGroup: (tagId: string, groupId: string | null) => void;
}

export function useCentralTagGroupsBridge({
  enabled,
  t,
}: UseCentralV2TagGroupsBridgeArgs): UseCentralV2TagGroupsBridgeResult {
  const tagGroups = useTagGroupsStore((s) => s.groups);
  const loadTagGroups = useTagGroupsStore((s) => s.loadTagGroups);
  const createTagGroup = useTagGroupsStore((s) => s.createTagGroup);
  const updateTagGroup = useTagGroupsStore((s) => s.updateTagGroup);
  const deleteTagGroup = useTagGroupsStore((s) => s.deleteTagGroup);
  const setTagGroup = useTagGroupsStore((s) => s.setTagGroup);

  useEffect(() => {
    if (enabled) void loadTagGroups();
  }, [enabled, loadTagGroups]);

  const handleCreateTagGroup = (defaultName?: string) => {
    const name = window.prompt(
      t("central.v2.tagGroupsNamePlaceholder"),
      defaultName ?? t("central.v2.tagGroupsNameDefault"),
    );
    if (!name || !name.trim()) return;
    void createTagGroup({ name: name.trim() });
  };

  const handleRenameTagGroup = (group: TagGroup) => {
    const name = window.prompt(t("central.v2.tagGroupsNamePlaceholder"), group.name);
    if (!name || !name.trim() || name.trim() === group.name) return;
    void updateTagGroup(group.id, { name: name.trim() });
  };

  const handleDeleteTagGroup = (group: TagGroup) => {
    if (!window.confirm(t("central.v2.tagGroupsDeleteConfirm", { name: group.name }))) return;
    void deleteTagGroup(group.id);
  };

  const handleAssignTagToGroup = (tagId: string, groupId: string | null) => {
    void setTagGroup(tagId, groupId);
  };

  return {
    tagGroups,
    handleCreateTagGroup,
    handleRenameTagGroup,
    handleDeleteTagGroup,
    handleAssignTagToGroup,
  };
}
