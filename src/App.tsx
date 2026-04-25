import { Routes, Route, Navigate } from "react-router-dom";
import { AppShell } from "@/components/layout/AppShell";
import { PlatformView } from "@/pages/PlatformView";
import { CentralSkillsView } from "@/pages/CentralSkillsView";
import { SkillDetailPage } from "@/pages/SkillDetailPage";
import { CollectionsListView } from "@/pages/CollectionsListView";
import { MarketplaceView } from "@/pages/MarketplaceView";
import { SettingsView } from "@/pages/SettingsView";
import { DiscoverView } from "@/pages/DiscoverView";

function App() {
  return (
    <Routes>
      <Route path="/" element={<AppShell />}>
        {/* Default redirect to Central Skills */}
        <Route index element={<Navigate to="/central" replace />} />
        {/* Platform view: lists skills for a specific agent */}
        <Route path="platform/:agentId" element={<PlatformView />} />
        {/* Central Skills: canonical ~/.skillsmanage/skills/ view */}
        <Route path="central" element={<CentralSkillsView />} />
        {/* Skill detail page */}
        <Route path="skill/:skillId" element={<SkillDetailPage />} />
        {/* Collections */}
        <Route path="collections" element={<CollectionsListView />} />
        {/* Marketplace */}
        <Route path="marketplace" element={<MarketplaceView />} />
        {/* Discover project skills */}
        <Route path="discover" element={<DiscoverView />} />
        {/* Discover filtered by project */}
        <Route path="discover/:projectPath" element={<DiscoverView />} />
        {/* Settings */}
        <Route path="settings" element={<SettingsView />} />
      </Route>
    </Routes>
  );
}

export default App;
