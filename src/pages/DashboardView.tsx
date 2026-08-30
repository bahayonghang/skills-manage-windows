import { useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";

import { DashboardShell } from "@/components/dashboard/DashboardShell";
import { InventoryCensus } from "@/components/skillsCli/InventoryCensus";
import { isLocalTarget } from "@/lib/targetKind";
import { useDashboardBindings, useDashboardBootstrap } from "@/pages/dashboardBindings";
import { RECENT_LOG_LIMIT } from "@/pages/dashboardUtils";
import { useDashboardViewModel } from "@/pages/dashboardViewModel";
import { useSkillsCliStore } from "@/stores/skillsCliStore";
import { useTargetStore } from "@/stores/targetStore";

function LocalSkillsCliCensus() {
  const { t } = useTranslation();
  const activeTarget = useTargetStore((state) => state.activeTarget);
  const isLocal = isLocalTarget(activeTarget);
  const skills = useSkillsCliStore((state) => state.skills);
  const targets = useSkillsCliStore((state) => state.targets);
  const loadAll = useSkillsCliStore((state) => state.loadAll);

  useEffect(() => {
    if (!isLocal) {
      return;
    }
    void loadAll();
  }, [isLocal, loadAll]);

  if (!isLocal) {
    return null;
  }

  return (
    <section data-testid="dashboard-skills-cli-census" className="space-y-2">
      <h2 className="text-sm font-medium">{t("skillsCli.dashboardHeading")}</h2>
      <InventoryCensus skills={skills} targets={targets} />
    </section>
  );
}

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

  return (
    <DashboardShell
      viewModel={viewModel}
      onNavigate={navigate}
      extra={<LocalSkillsCliCensus />}
    />
  );
}
