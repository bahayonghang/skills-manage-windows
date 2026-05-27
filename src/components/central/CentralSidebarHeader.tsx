import { CentralSavedViewsSection } from "./CentralSavedViewsSection";
import { CentralTagGroupsSection } from "./CentralTagGroupsSection";
import type { UseCentralV2SavedViewsBridgeResult } from "@/pages/centralSavedViewsBridge";
import type { UseCentralV2TagGroupsBridgeResult } from "@/pages/centralTagGroupsBridge";

interface CentralSidebarHeaderProps {
  savedViewsBridge: UseCentralV2SavedViewsBridgeResult;
  tagGroupsBridge: UseCentralV2TagGroupsBridgeResult;
}

/**
 * Sidebar 顶部组合段落（M3）。把 Saved Views + Tag Groups 收成一个 slot，
 * 让父层只传一个 `sidebarHeaderSlot` 即可。每段内部仍是独立的 FacetSection。
 */
export function CentralSidebarHeader({
  savedViewsBridge,
  tagGroupsBridge,
}: CentralSidebarHeaderProps) {
  return (
    <>
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
      <CentralTagGroupsSection
        tagGroups={tagGroupsBridge.tagGroups}
        onCreate={() => tagGroupsBridge.handleCreateTagGroup()}
        onRename={tagGroupsBridge.handleRenameTagGroup}
        onDelete={tagGroupsBridge.handleDeleteTagGroup}
      />
    </>
  );
}
