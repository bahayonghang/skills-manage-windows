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
    collectionsLength: bindings.collections.length,
    isCollectionsLoading: bindings.isCollectionsLoading,
    loadCollections: bindings.loadCollections,
    registriesLength: bindings.registries.length,
    isMarketplaceLoading: bindings.isMarketplaceLoading,
    loadRegistries: bindings.loadRegistries,
    loadLogs: bindings.loadLogs,
    subscribeAiTagProgress: bindings.subscribeAiTagProgress,
    subscribeUpdateProgress: bindings.subscribeUpdateProgress,
    recentLogLimit: RECENT_LOG_LIMIT,
  });

  const viewModel = useDashboardViewModel({
    t,
    ...bindings,
  });

  return <DashboardShell viewModel={viewModel} onNavigate={navigate} />;
}
