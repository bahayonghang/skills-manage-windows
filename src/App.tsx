import type { ReactNode } from "react";
import { lazy, Suspense } from "react";
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
const DiscoverView = lazy(() =>
  import("@/pages/DiscoverView").then(({ DiscoverView }) => ({
    default: DiscoverView,
  }))
);
const OperationLogsView = lazy(() =>
  import("@/pages/OperationLogsView").then(({ OperationLogsView }) => ({
    default: OperationLogsView,
  }))
);

function lazyPage(element: ReactNode) {
  return <Suspense fallback={null}>{element}</Suspense>;
}

function App() {
  return (
    <Routes>
      <Route path="/" element={<AppShell />}>
        {/* Default redirect to Central Skills */}
        <Route index element={<Navigate to="/central" replace />} />
        {/* Platform view: lists skills for a specific agent */}
        <Route
          path="platform/:agentId"
          element={lazyPage(<PlatformView />)}
        />
        {/* Central Skills: canonical ~/.skillsmanage/skills/ view */}
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
        {/* Discover project skills */}
        <Route
          path="discover"
          element={lazyPage(<DiscoverView />)}
        />
        {/* Discover filtered by project */}
        <Route
          path="discover/:projectPath"
          element={lazyPage(<DiscoverView />)}
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
