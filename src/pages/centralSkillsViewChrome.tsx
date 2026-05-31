import { useCallback, useMemo, type Dispatch, type SetStateAction } from "react";
import { CentralSavedViewsSection } from "@/components/central/CentralSavedViewsSection";
import { CentralTagGroupsSection } from "@/components/central/CentralTagGroupsSection";
import type { CentralSkillUpdateState, SkillWithLinks } from "@/types";

export function useCentralSkillsViewChrome({
  savedViewsBridge,
  tagGroupsBridge,
  skills,
  updateStatuses,
  selectedSkillIds,
  setSelectedSkillIds,
  visibleCurrentViewSkills,
}: {
  savedViewsBridge: {
    savedViews: Parameters<typeof CentralSavedViewsSection>[0]["savedViews"];
    activeSavedViewId: Parameters<typeof CentralSavedViewsSection>[0]["activeSavedViewId"];
    v2QueryString: Parameters<typeof CentralSavedViewsSection>[0]["currentQueryString"];
    canSaveCurrent: Parameters<typeof CentralSavedViewsSection>[0]["canSaveCurrent"];
    handleApplySavedView: Parameters<typeof CentralSavedViewsSection>[0]["onApply"];
    handleSaveCurrentView: Parameters<typeof CentralSavedViewsSection>[0]["onSaveCurrent"];
    handleRenameSavedView: Parameters<typeof CentralSavedViewsSection>[0]["onRename"];
    handleDeleteSavedView: Parameters<typeof CentralSavedViewsSection>[0]["onDelete"];
    handleTogglePinSavedView: Parameters<typeof CentralSavedViewsSection>[0]["onTogglePin"];
  };
  tagGroupsBridge: {
    tagGroups: Parameters<typeof CentralTagGroupsSection>[0]["tagGroups"];
    handleCreateTagGroup: () => void;
    handleRenameTagGroup: Parameters<typeof CentralTagGroupsSection>[0]["onRename"];
    handleDeleteTagGroup: Parameters<typeof CentralTagGroupsSection>[0]["onDelete"];
  };
  skills: SkillWithLinks[];
  updateStatuses: Record<string, CentralSkillUpdateState>;
  selectedSkillIds: string[];
  setSelectedSkillIds: Dispatch<SetStateAction<string[]>>;
  visibleCurrentViewSkills: SkillWithLinks[];
}) {
  const savedViewsSlot = (
    <CentralSavedViewsSection
      savedViews={savedViewsBridge.savedViews}
      activeSavedViewId={savedViewsBridge.activeSavedViewId}
      currentQueryString={savedViewsBridge.v2QueryString}
      canSaveCurrent={savedViewsBridge.canSaveCurrent}
      onApply={savedViewsBridge.handleApplySavedView}
      onSaveCurrent={savedViewsBridge.handleSaveCurrentView}
      onRename={savedViewsBridge.handleRenameSavedView}
      onDelete={savedViewsBridge.handleDeleteSavedView}
      onTogglePin={savedViewsBridge.handleTogglePinSavedView}
    />
  );

  const tagGroupsSlot = (
    <CentralTagGroupsSection
      tagGroups={tagGroupsBridge.tagGroups}
      onCreate={() => tagGroupsBridge.handleCreateTagGroup()}
      onRename={tagGroupsBridge.handleRenameTagGroup}
      onDelete={tagGroupsBridge.handleDeleteTagGroup}
    />
  );

  const repoUpdateCounts = useMemo(() => {
    const acc: Record<string, number> = {};
    for (const skill of skills) {
      if (updateStatuses[skill.id]?.status === "update_available" && skill.repository?.id) {
        acc[skill.repository.id] = (acc[skill.repository.id] ?? 0) + 1;
      }
    }
    return acc;
  }, [skills, updateStatuses]);

  const handleClearSelection = useCallback(() => setSelectedSkillIds([]), [setSelectedSkillIds]);
  const handleSelectCurrentResults = useCallback(() => {
    setSelectedSkillIds(visibleCurrentViewSkills.map((skill) => skill.id));
  }, [setSelectedSkillIds, visibleCurrentViewSkills]);

  const selectionControlsProps = useMemo(
    () => ({
      selectedCount: selectedSkillIds.length,
      currentResultCount: visibleCurrentViewSkills.length,
      onSelectCurrentResults: handleSelectCurrentResults,
      onClearSelection: handleClearSelection,
    }),
    [handleClearSelection, handleSelectCurrentResults, selectedSkillIds.length, visibleCurrentViewSkills.length],
  );

  return {
    savedViewsSlot,
    tagGroupsSlot,
    repoUpdateCounts,
    selectionControlsProps,
    handleClearSelection,
    handleSelectCurrentResults,
  };
}
