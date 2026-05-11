/**
 * `useCentralV2PaletteActions` —— 把 V2 命令面板的 quick actions 组装在一处。
 *
 * 当前提供：
 * 1. 保存当前视图（仅当 saved-view 桥允许时）
 * 2. 创建标签分组
 * 3. 切换 group-by 模式（5 选其余 4）
 * 4. 切回经典布局
 */

import { useMemo } from "react";
import type { TFunction } from "i18next";

import type { CommandPaletteAction } from "@/components/central/v2/CommandPaletteV2";
import type { CentralViewState, GroupByMode } from "@/lib/centralViewState";

export interface GroupByOption {
  value: GroupByMode;
  label: string;
}

export interface UseCentralV2PaletteActionsArgs {
  t: TFunction;
  viewState: CentralViewState;
  setViewState: (next: CentralViewState) => void;
  canSaveCurrent: boolean;
  onSaveCurrentView: (defaultName: string) => void;
  onCreateTagGroup: () => void;
  onSwitchToClassic: () => void;
  /** 可选注入 group-by 选项；默认从 t 派生 5 种内置模式。 */
  groupByOptions?: ReadonlyArray<GroupByOption>;
}

export interface UseCentralV2PaletteActionsResult {
  actions: CommandPaletteAction[];
  groupByOptions: GroupByOption[];
}

export function useCentralV2PaletteActions({
  t,
  viewState,
  setViewState,
  canSaveCurrent,
  onSaveCurrentView,
  onCreateTagGroup,
  onSwitchToClassic,
  groupByOptions: groupByOptionsArg,
}: UseCentralV2PaletteActionsArgs): UseCentralV2PaletteActionsResult {
  const groupByOptions = useMemo<GroupByOption[]>(
    () =>
      groupByOptionsArg
        ? [...groupByOptionsArg]
        : [
            { value: "none", label: t("central.v2.groupByModeNone") },
            { value: "repository", label: t("central.v2.groupByModeRepository") },
            { value: "owner", label: t("central.v2.groupByModeOwner") },
            { value: "tag", label: t("central.v2.groupByModeTag") },
            { value: "status", label: t("central.v2.groupByModeStatus") },
          ],
    [t, groupByOptionsArg],
  );

  const actions = useMemo(() => {
    const items: CommandPaletteAction[] = [];

    if (canSaveCurrent) {
      items.push({
        id: "save-current-view",
        label: t("central.v2.paletteActionSaveCurrentView"),
        onSelect: () =>
          onSaveCurrentView(t("central.v2.savedViewsNamePlaceholder")),
      });
    }

    items.push({
      id: "create-tag-group",
      label: t("central.v2.paletteActionCreateTagGroup"),
      onSelect: onCreateTagGroup,
    });

    for (const opt of groupByOptions) {
      if (opt.value === viewState.group) continue;
      items.push({
        id: `group-by-${opt.value}`,
        label: t("central.v2.paletteActionGroupBy", { label: opt.label }),
        onSelect: () => setViewState({ ...viewState, group: opt.value }),
      });
    }

    items.push({
      id: "switch-to-classic",
      label: t("central.v2.paletteActionSwitchToClassic"),
      onSelect: onSwitchToClassic,
    });

    return items;
  }, [
    t,
    viewState,
    setViewState,
    canSaveCurrent,
    onSaveCurrentView,
    onCreateTagGroup,
    onSwitchToClassic,
    groupByOptions,
  ]);

  return { actions, groupByOptions };
}
