import { useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";

import { DashboardShell } from "@/components/dashboard/DashboardShell";
import { useDashboardBindings, useDashboardBootstrap } from "@/pages/dashboardBindings";
import { RECENT_LOG_LIMIT } from "@/pages/dashboardUtils";
import { useDashboardViewModel } from "@/pages/dashboardViewModel";

export function DashboardView() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const bindings = useDashboardBindings();

  useDashboardBootstrap({
    refreshDashboardSummary: bindings.refreshDashboardSummary,
    loadTopTags: bindings.loadTopTags,
    loadDailyCounts: bindings.loadDailyCounts,
    loadLogs: bindings.loadLogs,
    subscribeAiTagProgress: bindings.subscribeAiTagProgress,
    subscribeUpdateProgress: bindings.subscribeUpdateProgress,
    scanGeneration: bindings.scanGeneration,
    recentLogLimit: RECENT_LOG_LIMIT,
  });

  const viewModel = useDashboardViewModel({
    t,
    ...bindings,
  });

  return <DashboardShell viewModel={viewModel} onNavigate={navigate} />;
}
