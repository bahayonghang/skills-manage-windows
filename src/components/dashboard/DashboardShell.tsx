import { useTranslation } from "react-i18next";

import { ActivityPanel } from "@/components/dashboard/sections/ActivityPanel";
import { AgentsPanel } from "@/components/dashboard/sections/AgentsPanel";
import { HealthOrbit } from "@/components/dashboard/sections/HealthOrbit";
import { HeroSection } from "@/components/dashboard/sections/HeroSection";
import { LogsPanel } from "@/components/dashboard/sections/LogsPanel";
import { MetricStrip } from "@/components/dashboard/sections/MetricStrip";
import { ProgressBreakdown } from "@/components/dashboard/sections/ProgressBreakdown";
import { WorkQueuePanel } from "@/components/dashboard/sections/WorkQueuePanel";
import type { DashboardViewModel } from "@/pages/dashboardViewModel";

interface DashboardShellProps {
  onNavigate: (path: string) => void;
  viewModel: DashboardViewModel;
}

export function DashboardShell({
  onNavigate,
  viewModel,
}: DashboardShellProps) {
  const { t } = useTranslation();

  return (
    <div className="bg-orbit h-full min-h-0 overflow-hidden">
      <div
        data-testid="dashboard-scroll-region"
        className="scrollbar-subtle flex h-full min-h-0 min-w-0 max-w-full flex-col overflow-y-auto overflow-x-hidden overscroll-contain"
      >
        <div className="min-h-0 min-w-0 flex-1 space-y-5 p-3 pb-6 sm:p-5 sm:pb-8">
          <div className="grid gap-5 xl:grid-cols-[minmax(0,1.35fr)_minmax(20rem,0.85fr)]">
            <HeroSection
              onNavigate={onNavigate}
              uncategorizedCount={viewModel.uncategorizedCount}
              scanStateLabel={viewModel.scanStateLabel}
              scanState={viewModel.scanState}
              quickMigratePath={viewModel.quickMigratePath}
              quickMigrateDescription={viewModel.quickMigrateDescription}
            />
            <HealthOrbit
              readiness={viewModel.readiness}
              centralTotal={viewModel.centralTotal}
              sourceRepositoryCount={viewModel.sourceRepositoryCount}
              enabledAgentsCount={viewModel.enabledTargets.length}
            />
          </div>

          {viewModel.loadError && (
            <div
              className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive"
              title={viewModel.loadError}
            >
              {t("dashboard.loadWarning")}
            </div>
          )}

          <MetricStrip
            onNavigate={onNavigate}
            centralTotal={viewModel.centralTotal}
            aiReviewCount={viewModel.aiReviewCount}
            sourceRepositoryCount={viewModel.sourceRepositoryCount}
            collectionCount={viewModel.resolvedCollectionCount}
          />

          <div className="grid gap-5 xl:grid-cols-[minmax(0,1.18fr)_minmax(20rem,0.82fr)]">
            <ProgressBreakdown
              centralTotal={viewModel.centralTotal}
              uncategorizedCount={viewModel.uncategorizedCount}
              aiReviewCount={viewModel.aiReviewCount}
              updatesAvailable={viewModel.updatesAvailable}
              unassignedSourceCount={viewModel.unassignedSourceCount}
            />
            <AgentsPanel
              onNavigate={onNavigate}
              visiblePlatformTargets={viewModel.visiblePlatformTargets}
              enabledTargetsCount={viewModel.enabledTargets.length}
              skillsByAgent={viewModel.skillsByAgent}
            />
          </div>

          <WorkQueuePanel
            onNavigate={onNavigate}
            queueItems={viewModel.queueItems}
            activeJob={viewModel.activeJob}
          />

          <div className="grid gap-5 xl:grid-cols-[minmax(0,1.25fr)_minmax(20rem,0.85fr)]">
            <LogsPanel
              onNavigate={onNavigate}
              recentLogs={viewModel.recentLogs}
              isLogsLoading={viewModel.isLogsLoading}
              logTotal={viewModel.logTotal}
            />
            <ActivityPanel
              onNavigate={onNavigate}
              activity={viewModel.activity}
              sparkline={viewModel.sparkline}
              topTags={viewModel.topTags}
            />
          </div>
        </div>
      </div>
    </div>
  );
}
