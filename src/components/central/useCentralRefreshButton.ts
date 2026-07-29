import { useCallback } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";

import { useCentralSkillsStore } from "@/stores/centralSkillsStore";
import { usePlatformStore } from "@/stores/platformStore";

async function noopAsync(_options?: { throwOnError?: boolean }): Promise<void> {}

export interface CentralRefreshButtonState {
  refreshing: boolean;
  disabled: boolean;
  onClick: () => void;
}

/**
 * Central Skills 工具栏手动刷新按钮的状态与编排（design D3/D6）。
 *
 * 列表重取与计数刷新并行、互不阻断；任一失败 toast central.refreshError
 * （列表失败优先上报），成功路径无 toast。装配在 Shell 内部，
 * CentralSkillsView 零改动（sizecheck 冻结基线约束）。
 */
export function useCentralRefreshButton(): CentralRefreshButtonState {
  const { t } = useTranslation();
  const isRefreshingList =
    useCentralSkillsStore((state) => state.isRefreshingList) ?? false;
  const isLoading = useCentralSkillsStore((state) => state.isLoading) ?? false;
  const loadCentralSkills =
    useCentralSkillsStore((state) => state.loadCentralSkills) ?? noopAsync;
  const refreshCounts =
    usePlatformStore((state) => state.refreshCounts) ?? noopAsync;

  const refresh = useCallback(async () => {
    const [listResult, countsResult] = await Promise.allSettled([
      loadCentralSkills({ throwOnError: true }),
      refreshCounts(),
    ]);
    const failure =
      listResult.status === "rejected"
        ? listResult.reason
        : countsResult.status === "rejected"
          ? countsResult.reason
          : null;
    if (failure) {
      toast.error(t("central.refreshError", { error: String(failure) }));
    }
  }, [loadCentralSkills, refreshCounts, t]);

  return {
    refreshing: isRefreshingList,
    disabled: isRefreshingList || isLoading,
    onClick: () => {
      void refresh();
    },
  };
}
