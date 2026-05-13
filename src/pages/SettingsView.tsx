import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import i18n from "@/i18n";

import { ACCENT_NAMES } from "@/stores/themeStore";
import { AddDirectoryDialog } from "@/components/settings/AddDirectoryDialog";
import { AboutSettingsSection } from "@/components/settings/AboutSettingsSection";
import { AiSettingsSection } from "@/components/settings/AiSettingsSection";
import { CustomPlatformsSettingsSection } from "@/components/settings/CustomPlatformsSettingsSection";
import { GitHubPatSettingsSection } from "@/components/settings/GitHubPatSettingsSection";
import { PlatformDialog } from "@/components/settings/PlatformDialog";
import { PlatformVisibilitySettingsSection } from "@/components/settings/PlatformVisibilitySettingsSection";
import {
  RemoteTargetsSettingsSection,
  type SshTargetFormState,
} from "@/components/settings/RemoteTargetsSettingsSection";
import { ScanDirectoriesSettingsSection } from "@/components/settings/ScanDirectoriesSettingsSection";
import { AI_PROVIDERS } from "@/data/aiProviders";
import { createSettingsViewActions } from "@/pages/settingsViewActions";
import { useSettingsViewBindings } from "@/pages/settingsViewBindings";
import {
  CTP_VAR_MAP,
  EMPTY_SSH_TARGET_FORM,
  FLAVOR_COLORS,
  FLAVOR_ORDER,
  REPO_URL,
  getAiProviderViewModel,
  getCustomAgents,
  getNextAiProviderPatch,
  getNormalizedPlatformVisibilityQuery,
  getPlatformVisibilityGroups,
  isPlatformVisibilitySearchActive,
  resolveSettingsDbPath,
} from "@/pages/settingsViewModel";
import type { AgentWithStatus } from "@/types";

const APP_VERSION = __APP_VERSION__;

export function SettingsView() {
  const { t } = useTranslation();
  const {
    scanDirectories,
    isLoadingScanDirs,
    loadScanDirectories,
    addScanDirectory,
    removeScanDirectory,
    toggleScanDirectory,
    addCustomAgent,
    updateCustomAgent,
    removeCustomAgent,
    githubPatState,
    isLoadingGitHubPat,
    isSavingGitHubPat,
    isTestingGitHubPat,
    loadGitHubPat,
    saveGitHubPat,
    clearGitHubPat,
    testGitHubPat,
    aiSettings,
    aiApiKeyState,
    aiSettingsLoaded,
    isLoadingAiSettings,
    aiSaveStatus,
    aiSaveError,
    aiTesting,
    aiTestResult,
    loadAiSettings,
    updateAiSettings,
    clearAiApiKey,
    testAiConnection,
    agents,
    categoryVisibility,
    setCategoryVisibility,
    setAgentEnabled,
    rescan,
    refreshCounts,
    loadCentralSkills,
    refreshDiscoverCounts,
    loadMarketplaceRegistries,
    selectedMarketplaceRegistryId,
    loadMarketplaceSkills,
    targets,
    activeTarget,
    isLoadingTargets,
    isCreatingTarget,
    updatingTargetId,
    testingTargetId,
    updatingPasswordTargetId,
    switchingTargetId,
    deletingTargetId,
    loadTargets,
    createSshTarget,
    updateSshTarget,
    testSshTarget,
    updateSshTargetPassword,
    deleteTarget,
    switchTarget,
    flavor,
    setFlavor,
    accent,
    setAccent,
  } = useSettingsViewBindings();

  const customAgents = useMemo(() => getCustomAgents(agents), [agents]);
  const dbPathDisplay = useMemo(
    () => resolveSettingsDbPath(agents, scanDirectories),
    [agents, scanDirectories]
  );

  const [platformVisibilityQuery, setPlatformVisibilityQuery] = useState("");
  const [showAiTestDetails, setShowAiTestDetails] = useState(false);
  const [isAddDirOpen, setIsAddDirOpen] = useState(false);
  const [showBuiltinDirs, setShowBuiltinDirs] = useState(false);
  const [isPlatformDialogOpen, setIsPlatformDialogOpen] = useState(false);
  const [editingPlatform, setEditingPlatform] = useState<AgentWithStatus | null>(null);
  const [removingDir, setRemovingDir] = useState<string | null>(null);
  const [removingAgent, setRemovingAgent] = useState<string | null>(null);
  const [scanDirError, setScanDirError] = useState<string | null>(null);
  const [platformError, setPlatformError] = useState<string | null>(null);
  const [githubPatInput, setGitHubPatInput] = useState("");
  const [githubPatMessage, setGitHubPatMessage] = useState<{
    type: "success" | "error";
    text: string;
    detail?: string | null;
  } | null>(null);
  const [sshTargetForm, setSshTargetForm] =
    useState<SshTargetFormState>(EMPTY_SSH_TARGET_FORM);
  const [editingTargetId, setEditingTargetId] = useState<string | null>(null);
  const [sshTargetEditForm, setSshTargetEditForm] =
    useState<SshTargetFormState>(EMPTY_SSH_TARGET_FORM);
  const [sshTargetPasswordUpdates, setSshTargetPasswordUpdates] = useState<
    Record<string, string>
  >({});
  const [targetMessage, setTargetMessage] = useState<{
    type: "success" | "error";
    text: string;
  } | null>(null);

  const { resolvedUrl } = useMemo(
    () => getAiProviderViewModel(aiSettings, AI_PROVIDERS),
    [aiSettings]
  );
  const lang = i18n.language;

  useEffect(() => {
    if (!aiSettingsLoaded && !isLoadingAiSettings) {
      void loadAiSettings();
    }
  }, [aiSettingsLoaded, isLoadingAiSettings, loadAiSettings]);

  useEffect(() => {
    loadScanDirectories();
    loadGitHubPat();
    loadTargets();
  }, [loadScanDirectories, loadGitHubPat, loadTargets]);

  useEffect(() => {
    setGitHubPatInput("");
  }, [githubPatState.configured]);

  const normalizedPlatformVisibilityQuery = useMemo(
    () => getNormalizedPlatformVisibilityQuery(platformVisibilityQuery),
    [platformVisibilityQuery]
  );
  const platformVisibilitySearchActive = useMemo(
    () => isPlatformVisibilitySearchActive(normalizedPlatformVisibilityQuery),
    [normalizedPlatformVisibilityQuery]
  );
  const platformVisibilityGroups = useMemo(
    () =>
      getPlatformVisibilityGroups({
        agents,
        categoryVisibility,
        normalizedQuery: normalizedPlatformVisibilityQuery,
        t,
      }),
    [agents, categoryVisibility, normalizedPlatformVisibilityQuery, t]
  );
  function handleProviderChange(id: string) {
    updateAiSettings(getNextAiProviderPatch(id, AI_PROVIDERS, aiSettings));
  }

  const {
    updateSshTargetFormField,
    updateSshTargetEditFormField,
    updateExistingTargetPassword,
    handleStartEditTarget,
    handleCancelEditTarget,
    handleCreateSshTarget,
    handleTestNewSshTarget,
    handleTestExistingTarget,
    handleUpdateSshTarget,
    handleUpdateTargetPassword,
    handleSwitchTarget,
    handleDeleteTarget,
    handleAddDirectory,
    handleRemoveDirectory,
    handleToggleDirectory,
    handleOpenAddPlatform,
    handleOpenEditPlatform,
    handleAddPlatform,
    handleEditPlatform,
    handleRemovePlatform,
    handleToggleCategory,
    handleTogglePlatformVisibility,
    handleSaveGitHubPat,
    handleClearGitHubPat,
    handleTestGitHubPat,
  } = createSettingsViewActions({
    t,
    githubPatInput,
    sshTargetForm,
    sshTargetEditForm,
    sshTargetPasswordUpdates,
    editingPlatform,
    selectedMarketplaceRegistryId,
    addScanDirectory,
    removeScanDirectory,
    toggleScanDirectory,
    addCustomAgent,
    updateCustomAgent,
    removeCustomAgent,
    saveGitHubPat,
    clearGitHubPat,
    testGitHubPat,
    rescan,
    refreshCounts,
    loadCentralSkills,
    refreshDiscoverCounts,
    loadMarketplaceRegistries,
    loadMarketplaceSkills,
    createSshTarget,
    updateSshTarget,
    testSshTarget,
    updateSshTargetPassword,
    deleteTarget,
    switchTarget,
    setCategoryVisibility,
    setAgentEnabled,
    setEditingPlatform,
    setPlatformError,
    setIsPlatformDialogOpen,
    setRemovingAgent,
    setScanDirError,
    setRemovingDir,
    setGitHubPatInput,
    setGitHubPatMessage,
    setTargetMessage,
    setSshTargetForm,
    setEditingTargetId,
    setSshTargetEditForm,
    setSshTargetPasswordUpdates,
  });

  const resolvedActiveTarget = activeTarget ?? targets[0]!;

  return (
    <div className="flex flex-col h-full">
      <div className="border-b border-border px-6 py-4">
        <h1 className="text-xl font-semibold">{t("settings.title")}</h1>
      </div>

      <div className="flex-1 overflow-auto p-6 space-y-6">
        <RemoteTargetsSettingsSection
          activeTarget={resolvedActiveTarget}
          deletingTargetId={deletingTargetId}
          editingTargetId={editingTargetId}
          isCreatingTarget={isCreatingTarget}
          isLoadingTargets={isLoadingTargets}
          switchingTargetId={switchingTargetId}
          sshTargetEditForm={sshTargetEditForm}
          sshTargetForm={sshTargetForm}
          sshTargetPasswordUpdates={sshTargetPasswordUpdates}
          targetMessage={targetMessage}
          targets={targets}
          testingTargetId={testingTargetId}
          updatingPasswordTargetId={updatingPasswordTargetId}
          updatingTargetId={updatingTargetId}
          onCancelEditTarget={handleCancelEditTarget}
          onCreateSshTarget={() => {
            void handleCreateSshTarget();
          }}
          onDeleteTarget={(targetId) => {
            void handleDeleteTarget(targetId);
          }}
          onStartEditTarget={handleStartEditTarget}
          onSwitchTarget={(targetId) => {
            void handleSwitchTarget(targetId);
          }}
          onTestExistingTarget={(targetId) => {
            void handleTestExistingTarget(targetId);
          }}
          onTestNewSshTarget={() => {
            void handleTestNewSshTarget();
          }}
          onUpdateExistingTargetPassword={updateExistingTargetPassword}
          onUpdateSshTarget={(target) => {
            void handleUpdateSshTarget(target);
          }}
          onUpdateSshTargetEditForm={updateSshTargetEditFormField}
          onUpdateSshTargetForm={updateSshTargetFormField}
          onUpdateTargetPassword={(target) => {
            void handleUpdateTargetPassword(target);
          }}
        />

        <CustomPlatformsSettingsSection
          customAgents={customAgents}
          platformError={platformError}
          removingAgent={removingAgent}
          onAddPlatform={handleOpenAddPlatform}
          onEditPlatform={handleOpenEditPlatform}
          onRemovePlatform={(agentId) => {
            void handleRemovePlatform(agentId);
          }}
        />

        <PlatformVisibilitySettingsSection
          groups={platformVisibilityGroups}
          isSearchActive={platformVisibilitySearchActive}
          normalizedQuery={normalizedPlatformVisibilityQuery}
          query={platformVisibilityQuery}
          onQueryChange={setPlatformVisibilityQuery}
          onToggleCategory={(category, visible) => {
            void handleToggleCategory(category, visible);
          }}
          onTogglePlatform={(agentId, enabled) => {
            void handleTogglePlatformVisibility(agentId, enabled);
          }}
        />

        <GitHubPatSettingsSection
          githubPatState={githubPatState}
          githubPatInput={githubPatInput}
          githubPatMessage={githubPatMessage}
          isLoadingGitHubPat={isLoadingGitHubPat}
          isSavingGitHubPat={isSavingGitHubPat}
          isTestingGitHubPat={isTestingGitHubPat}
          onClear={() => {
            void handleClearGitHubPat();
          }}
          onInputChange={setGitHubPatInput}
          onSave={() => {
            void handleSaveGitHubPat();
          }}
          onTest={() => {
            void handleTestGitHubPat();
          }}
        />

        <AiSettingsSection
          aiSaveError={aiSaveError}
          aiSaveStatus={aiSaveStatus}
          aiApiKeyState={aiApiKeyState}
          aiSettings={aiSettings}
          aiTestResult={aiTestResult}
          aiTesting={aiTesting}
          isLoadingAiSettings={isLoadingAiSettings}
          lang={lang}
          resolvedUrl={resolvedUrl}
          showAiTestDetails={showAiTestDetails}
          onClearApiKey={() => {
            void clearAiApiKey();
          }}
          onProviderChange={handleProviderChange}
          onSetShowAiTestDetails={setShowAiTestDetails}
          onTestConnection={async () => {
            setShowAiTestDetails(false);
            await testAiConnection();
          }}
          onUpdateAiSettings={updateAiSettings}
        />

        <ScanDirectoriesSettingsSection
          isLoadingScanDirs={isLoadingScanDirs}
          removingDir={removingDir}
          scanDirError={scanDirError}
          scanDirectories={scanDirectories}
          showBuiltinDirs={showBuiltinDirs}
          onAddDirectory={() => setIsAddDirOpen(true)}
          onRemoveDirectory={(path) => {
            void handleRemoveDirectory(path);
          }}
          onToggleBuiltinDirs={() => setShowBuiltinDirs((value) => !value)}
          onToggleDirectory={(path, active) => {
            void handleToggleDirectory(path, active);
          }}
        />

        <AboutSettingsSection
          accent={accent}
          accentNames={ACCENT_NAMES}
          appVersion={APP_VERSION}
          ctpVarMap={CTP_VAR_MAP}
          dbPathDisplay={dbPathDisplay}
          flavor={flavor}
          flavorColors={FLAVOR_COLORS}
          flavorOrder={FLAVOR_ORDER}
          repoUrl={REPO_URL}
          onSetAccent={setAccent}
          onSetFlavor={setFlavor}
        />
      </div>

      <AddDirectoryDialog
        open={isAddDirOpen}
        onOpenChange={setIsAddDirOpen}
        onAdd={handleAddDirectory}
      />

      <PlatformDialog
        open={isPlatformDialogOpen}
        onOpenChange={setIsPlatformDialogOpen}
        platform={editingPlatform}
        onAdd={handleAddPlatform}
        onEdit={handleEditPlatform}
      />
    </div>
  );
}
