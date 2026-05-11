import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { Bookmark, Pencil, Pin, PinOff, Plus, Save, Trash2 } from "lucide-react";

import { Button } from "@/components/ui/button";
import { FacetItem } from "./FacetItem";
import { FacetSection } from "./FacetSection";
import type { SavedView } from "@/types";

export interface CentralSavedViewsSectionProps {
  /** 全部 saved views，已按 pinned-first / sort_order 排序。 */
  savedViews: SavedView[];
  /** 当前命中的 saved view id（基于 query 字符串相等判定）。 */
  activeSavedViewId: string | null;
  /** 当前 view state 的 URL 序列化字符串（不含前导 `?`），用于快速保存。 */
  currentQueryString: string;
  /** 是否启用「保存当前视图」按钮。无任何条件变化时禁用避免空操作。 */
  canSaveCurrent: boolean;

  onApply: (view: SavedView) => void;
  onSaveCurrent: (defaultName: string) => void;
  onRename: (view: SavedView) => void;
  onDelete: (view: SavedView) => void;
  onTogglePin: (view: SavedView) => void;
}

/**
 * Sidebar 顶部 Saved Views 段落。复用 FacetSection 容器与 FacetItem 行布局，
 * 行尾用 dropdown menu 收口 rename / delete / pin / unpin。
 */
export function CentralSavedViewsSection({
  savedViews,
  activeSavedViewId,
  currentQueryString,
  canSaveCurrent,
  onApply,
  onSaveCurrent,
  onRename,
  onDelete,
  onTogglePin,
}: CentralSavedViewsSectionProps) {
  const { t } = useTranslation();
  const totalCount = savedViews.length;

  // 用 useMemo 避免每次重渲染都新建 array
  const itemElements = useMemo(
    () =>
      savedViews.map((view) => {
        const isActive = view.id === activeSavedViewId;
        const isCurrent = !isActive && view.query === currentQueryString;
        return (
          <FacetItem
            key={view.id}
            label={view.name}
            active={isActive}
            icon={view.pinned ? <Pin className="size-3" /> : <Bookmark className="size-3" />}
            description={isCurrent ? t("central.v2.savedViewsApplied") : undefined}
            onClick={() => onApply(view)}
            testId={`saved-view-${view.id}`}
            trailingAction={
              <span className="flex shrink-0 items-center gap-0.5 opacity-0 transition-opacity group-hover:opacity-100 focus-within:opacity-100">
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  className="size-6"
                  aria-label={view.pinned ? t("central.v2.savedViewsUnpin") : t("central.v2.savedViewsPin")}
                  title={view.pinned ? t("central.v2.savedViewsUnpin") : t("central.v2.savedViewsPin")}
                  onClick={(e) => {
                    e.stopPropagation();
                    onTogglePin(view);
                  }}
                >
                  {view.pinned ? <PinOff className="size-3" /> : <Pin className="size-3" />}
                </Button>
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  className="size-6"
                  aria-label={t("central.v2.savedViewsRename")}
                  title={t("central.v2.savedViewsRename")}
                  onClick={(e) => {
                    e.stopPropagation();
                    onRename(view);
                  }}
                >
                  <Pencil className="size-3" />
                </Button>
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  className="size-6 text-destructive hover:text-destructive"
                  aria-label={t("central.v2.savedViewsDelete")}
                  title={t("central.v2.savedViewsDelete")}
                  onClick={(e) => {
                    e.stopPropagation();
                    onDelete(view);
                  }}
                >
                  <Trash2 className="size-3" />
                </Button>
              </span>
            }
          />
        );
      }),
    [savedViews, activeSavedViewId, currentQueryString, onApply, onRename, onDelete, onTogglePin, t],
  );

  return (
    <FacetSection
      title={t("central.v2.savedViews")}
      count={totalCount}
      testId="central-saved-views"
    >
      <div className="flex flex-col gap-1">
        {savedViews.length === 0 ? (
          <div className="px-2 py-1.5 text-xs text-muted-foreground">
            {t("central.v2.savedViewsEmpty")}
          </div>
        ) : (
          itemElements
        )}
        <Button
          type="button"
          variant="ghost"
          size="sm"
          disabled={!canSaveCurrent}
          onClick={() => onSaveCurrent(t("central.v2.savedViewsNameDefault"))}
          className="mt-1 h-7 justify-start gap-2 text-xs"
        >
          {savedViews.length === 0 ? (
            <Save className="size-3.5" />
          ) : (
            <Plus className="size-3.5" />
          )}
          {t("central.v2.savedViewsSaveCurrent")}
        </Button>
      </div>
    </FacetSection>
  );
}
