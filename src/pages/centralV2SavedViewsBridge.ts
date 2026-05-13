/**
 * `useCentralV2SavedViewsBridge` —— 把 saved views store 与 V2 view-state 之间的
 * 协作（计算当前 query / 命中哪条 saved view / 各种 CRUD handler）封装为单一
 * hook，CentralSkillsView 只负责注入 viewState/setViewState 与启用开关。
 */

import { useEffect, useMemo } from "react";
import type { TFunction } from "i18next";

import {
  parseCentralViewStateFromUrl,
  serializeCentralViewState,
  type CentralViewState,
} from "@/lib/centralViewState";
import { useSavedViewsStore } from "@/stores/savedViewsStore";
import type { SavedView } from "@/types";

export interface UseCentralV2SavedViewsBridgeArgs {
  /** 仅当为 true 时才会从后端拉取 saved views，避免在 V1 模式下白白请求。 */
  enabled: boolean;
  v2ViewState: CentralViewState;
  setV2ViewState: (state: CentralViewState) => void;
  t: TFunction;
}

export interface UseCentralV2SavedViewsBridgeResult {
  savedViews: SavedView[];
  /** 当前 view-state 的 URL 序列化（不含前导 `?`）。 */
  v2QueryString: string;
  /** 命中的 saved view id，无命中为 null。 */
  activeSavedViewId: string | null;
  /** 是否允许「保存当前视图」按钮可点。 */
  canSaveCurrent: boolean;
  handleApplySavedView: (view: SavedView) => void;
  handleSaveCurrentView: (defaultName: string) => void;
  handleRenameSavedView: (view: SavedView) => void;
  handleDeleteSavedView: (view: SavedView) => void;
  handleTogglePinSavedView: (view: SavedView) => void;
}

export function useCentralV2SavedViewsBridge({
  enabled,
  v2ViewState,
  setV2ViewState,
  t,
}: UseCentralV2SavedViewsBridgeArgs): UseCentralV2SavedViewsBridgeResult {
  const savedViews = useSavedViewsStore((s) => s.views);
  const loadSavedViews = useSavedViewsStore((s) => s.loadSavedViews);
  const createSavedView = useSavedViewsStore((s) => s.createSavedView);
  const updateSavedView = useSavedViewsStore((s) => s.updateSavedView);
  const deleteSavedView = useSavedViewsStore((s) => s.deleteSavedView);

  useEffect(() => {
    if (enabled) {
      void loadSavedViews();
    }
  }, [enabled, loadSavedViews]);

  const v2QueryString = useMemo(
    () => serializeCentralViewState(v2ViewState).toString(),
    [v2ViewState],
  );

  const activeSavedViewId = useMemo(() => {
    const hit = savedViews.find((view) => view.query === v2QueryString);
    return hit?.id ?? null;
  }, [savedViews, v2QueryString]);

  const canSaveCurrent = v2QueryString.length > 0 && activeSavedViewId === null;

  const handleApplySavedView = (view: SavedView) => {
    setV2ViewState(parseCentralViewStateFromUrl(`?${view.query}`));
  };

  const handleSaveCurrentView = (defaultName: string) => {
    const name = window.prompt(t("central.v2.savedViewsNamePlaceholder"), defaultName);
    if (!name || !name.trim()) return;
    void createSavedView({ name: name.trim(), query: v2QueryString });
  };

  const handleRenameSavedView = (view: SavedView) => {
    const name = window.prompt(t("central.v2.savedViewsNamePlaceholder"), view.name);
    if (!name || !name.trim() || name.trim() === view.name) return;
    void updateSavedView(view.id, { name: name.trim() });
  };

  const handleDeleteSavedView = (view: SavedView) => {
    if (!window.confirm(t("central.v2.savedViewsDeleteConfirm", { name: view.name }))) return;
    void deleteSavedView(view.id);
  };

  const handleTogglePinSavedView = (view: SavedView) => {
    void updateSavedView(view.id, { pinned: !view.pinned });
  };

  return {
    savedViews,
    v2QueryString,
    activeSavedViewId,
    canSaveCurrent,
    handleApplySavedView,
    handleSaveCurrentView,
    handleRenameSavedView,
    handleDeleteSavedView,
    handleTogglePinSavedView,
  };
}

/**
 * 把一个 id 加入 viewState 中的 `tags` 或 `repos` 列表（去重）。
 * 用于命令面板等需要追加单个 facet 的场景。
 */
export function addUniqueToCentralViewState(
  state: CentralViewState,
  setState: (next: CentralViewState) => void,
  kind: "tags" | "repos",
  id: string,
): void {
  const existing = state[kind];
  if (existing.includes(id)) return;
  setState({ ...state, [kind]: [...existing, id] });
}
