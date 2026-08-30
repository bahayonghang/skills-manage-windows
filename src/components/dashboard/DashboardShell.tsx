import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";

import { ActivityPanel } from "@/components/dashboard/sections/ActivityPanel";
import { AgentsPanel } from "@/components/dashboard/sections/AgentsPanel";
import { HealthOrbit } from "@/components/dashboard/sections/HealthOrbit";
import { LogsPanel } from "@/components/dashboard/sections/LogsPanel";
import { StatusHeader } from "@/components/dashboard/sections/StatusHeader";
import { TopTagsPanel } from "@/components/dashboard/sections/TopTagsPanel";
import { WorkQueuePanel } from "@/components/dashboard/sections/WorkQueuePanel";
import type { DashboardViewModel } from "@/pages/dashboardViewModel";

interface DashboardShellProps {
  onNavigate: (path: string) => void;
  viewModel: DashboardViewModel;
  extra?: ReactNode;
}

const GRID_TWO_COL =
  "grid gap-5 xl:grid-cols-[minmax(0,1.25fr)_minmax(20rem,0.85fr)]";

export function DashboardShell({
  onNavigate,
  viewModel,
  extra,
}: DashboardShellProps) {
  const { t } = useTranslation();

  return (
    <div className="bg-orbit h-full min-h-0 overflow-hidden">
      <div
        data-testid="dashboard-scroll-region"
        className="scrollbar-subtle flex h-full min-h-0 min-w-0 max-w-full flex-col overflow-y-auto overflow-x-hidden overscroll-contain"
      >
        <div className="min-h-0 min-w-0 flex-1 space-y-5 p-3 pb-6 sm:p-5 sm:pb-8">
          <StatusHeader
            onNavigate={onNavigate}
            scanState={viewModel.scanState}
            scanStateLabel={viewModel.scanStateLabel}
            lastScanLabel={viewModel.lastScanLabel}
            centralTotal={viewModel.centralTotal}
            sourceRepositoryCount={viewModel.sourceRepositoryCount}
            enabledTargetsCount={viewModel.enabledTargetsCount}
            visibleTargetsCount={viewModel.visiblePlatformTargets.length}
            quickMigratePath={viewModel.quickMigratePath}
            quickMigrateDescription={viewModel.quickMigrateDescription}
          />

          {viewModel.loadError && (
            <div
              className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-sm text-destructive-text"
              title={viewModel.loadError}
            >
              {t("dashboard.loadWarning")}
            </div>
          )}

          <WorkQueuePanel
            onNavigate={onNavigate}
            queueItems={viewModel.queueItems}
            activeJob={viewModel.activeJob}
          />

          <div className={GRID_TWO_COL}>
            <HealthOrbit readiness={viewModel.readiness} />
            <AgentsPanel
              onNavigate={onNavigate}
              visiblePlatformTargets={viewModel.visiblePlatformTargets}
              enabledTargetsCount={viewModel.enabledTargetsCount}
              skillsByAgent={viewModel.skillsByAgent}
            />
          </div>

          <div className={GRID_TWO_COL}>
            <ActivityPanel
              onNavigate={onNavigate}
              dailyCounts={viewModel.dailyCounts}
              isLoading={viewModel.isDailyCountsLoading}
              error={viewModel.dailyCountsError}
              onRetry={viewModel.retryDailyCounts}
            />
            <TopTagsPanel
              topTags={viewModel.topTags}
              isLoading={viewModel.isTopTagsLoading}
              error={viewModel.topTagsError}
              onRetry={viewModel.retryTopTags}
            />
          </div>

          <LogsPanel
            onNavigate={onNavigate}
            recentLogs={viewModel.recentLogs}
            isLogsLoading={viewModel.isLogsLoading}
            logTotal={viewModel.logTotal}
          />
          {extra}
        </div>
      </div>
    </div>
  );
}
