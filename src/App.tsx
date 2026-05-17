import type { ReactNode } from "react";
import { lazy, Suspense } from "react";
import { useTranslation } from "react-i18next";
import { Routes, Route, Navigate } from "react-router-dom";
import { AppShell } from "@/components/layout/AppShell";

const PlatformView = lazy(() =>
  import("@/pages/PlatformView").then(({ PlatformView }) => ({
    default: PlatformView,
  }))
);
const CentralSkillsView = lazy(() =>
  import("@/pages/CentralSkillsView").then(({ CentralSkillsView }) => ({
    default: CentralSkillsView,
  }))
);
const SkillDetailPage = lazy(() =>
  import("@/pages/SkillDetailPage").then(({ SkillDetailPage }) => ({
    default: SkillDetailPage,
  }))
);
const CollectionsListView = lazy(() =>
  import("@/pages/CollectionsListView").then(({ CollectionsListView }) => ({
    default: CollectionsListView,
  }))
);
const MarketplaceView = lazy(() =>
  import("@/pages/MarketplaceView").then(({ MarketplaceView }) => ({
    default: MarketplaceView,
  }))
);
const SettingsView = lazy(() =>
  import("@/pages/SettingsView").then(({ SettingsView }) => ({
    default: SettingsView,
  }))
);
const ProjectsView = lazy(() =>
  import("@/pages/ProjectsView").then(({ ProjectsView }) => ({
    default: ProjectsView,
  }))
);
const ObsidianVaultView = lazy(() =>
  import("@/pages/ObsidianVaultView").then(({ ObsidianVaultView }) => ({
    default: ObsidianVaultView,
  }))
);
const OperationLogsView = lazy(() =>
  import("@/pages/OperationLogsView").then(({ OperationLogsView }) => ({
    default: OperationLogsView,
  }))
);
const DashboardView = lazy(() =>
  import("@/pages/DashboardView").then(({ DashboardView }) => ({
    default: DashboardView,
  }))
);

function RouteLoadingFallback() {
  const { t } = useTranslation();

  return (
    <div
      role="status"
      aria-live="polite"
      className="flex h-full items-center justify-center bg-background text-sm text-muted-foreground"
    >
      <div className="flex items-center gap-2 rounded-full border border-border bg-card/80 px-3 py-1.5 shadow-sm">
        <span className="size-2 animate-pulse rounded-full bg-primary" aria-hidden />
        <span>{t("common.loading")}</span>
      </div>
    </div>
  );
}

function lazyPage(element: ReactNode) {
  return <Suspense fallback={<RouteLoadingFallback />}>{element}</Suspense>;
}

function App() {
  return (
    <Routes>
      <Route path="/" element={<AppShell />}>
        {/* Default redirect to Dashboard */}
        <Route index element={<Navigate to="/dashboard" replace />} />
        {/* Dashboard: local-first operations overview */}
        <Route
          path="dashboard"
          element={lazyPage(<DashboardView />)}
        />
        {/* Platform view: lists skills for a specific agent */}
        <Route
          path="platform/:agentId"
          element={lazyPage(<PlatformView />)}
        />
        {/* Central Skills: canonical SkillPort library view */}
        <Route
          path="central"
          element={lazyPage(<CentralSkillsView />)}
        />
        {/* Skill detail page */}
        <Route
          path="skill/:skillId"
          element={lazyPage(<SkillDetailPage />)}
        />
        {/* Collections */}
        <Route
          path="collections"
          element={lazyPage(<CollectionsListView />)}
        />
        {/* Marketplace */}
        <Route
          path="marketplace"
          element={lazyPage(<MarketplaceView />)}
        />
        {/* Discover removed — redirect to Projects */}
        <Route
          path="discover"
          element={<Navigate to="/projects" replace />}
        />
        <Route
          path="discover/:projectPath"
          element={<Navigate to="/projects" replace />}
        />
        {/* Projects (project-level skill management) */}
        <Route
          path="projects"
          element={lazyPage(<ProjectsView />)}
        />
        <Route
          path="projects/:projectId"
          element={lazyPage(<ProjectsView />)}
        />
        <Route
          path="obsidian"
          element={lazyPage(<ObsidianVaultView />)}
        />
        <Route
          path="obsidian/:vaultId"
          element={lazyPage(<ObsidianVaultView />)}
        />
        {/* Operation logs */}
        <Route
          path="logs"
          element={lazyPage(<OperationLogsView />)}
        />
        {/* Settings */}
        <Route
          path="settings"
          element={lazyPage(<SettingsView />)}
        />
      </Route>
    </Routes>
  );
}

export default App;
