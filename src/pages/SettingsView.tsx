import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useLocation } from "react-router-dom";
import i18n from "@/i18n";

import { ACCENT_NAMES } from "@/stores/themeStore";
import { AddDirectoryDialog } from "@/components/settings/AddDirectoryDialog";
import { AboutSettingsSection } from "@/components/settings/AboutSettingsSection";
import { AiSettingsSection } from "@/components/settings/AiSettingsSection";
import { AppearanceSettingsSection } from "@/components/settings/AppearanceSettingsSection";
import { CustomPlatformsSettingsSection } from "@/components/settings/CustomPlatformsSettingsSection";
import { GitHubPatSettingsSection } from "@/components/settings/GitHubPatSettingsSection";
import { LocalRemoteSyncDialog } from "@/components/settings/LocalRemoteSyncDialog";
import { PlatformDialog } from "@/components/settings/PlatformDialog";
import { PlatformVisibilitySettingsSection } from "@/components/settings/PlatformVisibilitySettingsSection";
import {
  RemoteTargetsSettingsSection,
  type SshTargetFormState,
  type WslTargetFormState,
} from "@/components/settings/RemoteTargetsSettingsSection";
import { ScanDirectoriesSettingsSection } from "@/components/settings/ScanDirectoriesSettingsSection";
import {
  SettingsTableOfContents,
  type TocEntry,
} from "@/components/settings/SettingsTableOfContents";
import { AI_PROVIDERS } from "@/data/aiProviders";
import { createSettingsViewActions } from "@/pages/settingsViewActions";
import { useSettingsViewBindings } from "@/pages/settingsViewBindings";
import { useLocalRemoteSyncStore } from "@/stores/localRemoteSyncStore";
import { isRemoteLikeTarget } from "@/lib/targetKind";
import {
  CTP_VAR_MAP,
  EMPTY_SSH_TARGET_FORM,
  EMPTY_WSL_TARGET_FORM,
  FLAVOR_COLORS,
  FLAVOR_ORDER,
  REPO_URL,
  getAiProviderViewModel,
  getCustomAgents,
  getNormalizedPlatformVisibilityQuery,
  getPlatformVisibilityGroups,
  isPlatformVisibilitySearchActive,
  resolveSettingsDbPath,
} from "@/pages/settingsViewModel";
import type { AgentWithStatus } from "@/types";

const APP_VERSION = __APP_VERSION__;

const SETTINGS_TOC_ENTRIES: readonly TocEntry[] = [
  { id: "appearance-section", labelKey: "appearance" },
  { id: "remote-targets-section", labelKey: "remoteTargets" },
  { id: "custom-platforms-section", labelKey: "customPlatforms" },
  { id: "platform-visibility-section", labelKey: "platformVisibility" },
  { id: "github-pat-section", labelKey: "githubPat" },
  { id: "ai-section", labelKey: "ai" },
  { id: "scan-directories-section", labelKey: "scanDirectories" },
  { id: "about-section", labelKey: "about" },
];

export function SettingsView() {
  const { t } = useTranslation();
  const location = useLocation();
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
    switchAiProvider,
    clearAiApiKey,
    testAiConnection,
    agents,
    categoryVisibility,
    setCategoryVisibility,
    setAgentEnabled,
    rescan,
    refreshCounts,
    loadCentralSkills,
    loadMarketplaceRegistries,
    selectedMarketplaceRegistryId,
    loadMarketplaceSkills,
    targets,
    activeTarget,
    wslDistributions,
    isLoadingTargets,
    isLoadingWslDistributions,
    isCreatingTarget,
    updatingTargetId,
    testingTargetId,
    updatingPasswordTargetId,
    switchingTargetId,
    deletingTargetId,
    wslDistributionError,
    loadTargets,
    loadWslDistributions,
    createSshTarget,
    updateSshTarget,
    testSshTarget,
    createWslTarget,
    updateWslTarget,
    testWslTarget,
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
  const [wslTargetForm, setWslTargetForm] =
    useState<WslTargetFormState>(EMPTY_WSL_TARGET_FORM);
  const [editingTargetId, setEditingTargetId] = useState<string | null>(null);
  const [sshTargetEditForm, setSshTargetEditForm] =
    useState<SshTargetFormState>(EMPTY_SSH_TARGET_FORM);
  const [wslTargetEditForm, setWslTargetEditForm] =
    useState<WslTargetFormState>(EMPTY_WSL_TARGET_FORM);
  const [sshTargetPasswordUpdates, setSshTargetPasswordUpdates] = useState<
    Record<string, string>
  >({});
  const [targetMessage, setTargetMessage] = useState<{
    type: "success" | "error";
    text: string;
  } | null>(null);
  const [localRemoteSyncTargetId, setLocalRemoteSyncTargetId] = useState<string | null>(null);
  const consumedSettingsActionRef = useRef<string | null>(null);
  const localRemoteSyncPreview = useLocalRemoteSyncStore((state) => state.preview);
  const localRemoteSyncResult = useLocalRemoteSyncStore((state) => state.result);
  const isLocalRemoteSyncPreviewing = useLocalRemoteSyncStore(
    (state) => state.isPreviewing
  );
  const isLocalRemoteSyncApplying = useLocalRemoteSyncStore((state) => state.isApplying);
  const localRemoteSyncError = useLocalRemoteSyncStore((state) => state.error);
  const previewLocalRemoteSync = useLocalRemoteSyncStore((state) => state.previewSync);
  const applyLocalRemoteSync = useLocalRemoteSyncStore((state) => state.applySync);
  const resetLocalRemoteSync = useLocalRemoteSyncStore((state) => state.reset);

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
    void loadWslDistributions().catch(() => undefined);
  }, [loadScanDirectories, loadGitHubPat, loadTargets, loadWslDistributions]);

  useEffect(() => {
    if (wslTargetForm.distribution.trim() || wslDistributions.length !== 1) {
      return;
    }

    const [distribution] = wslDistributions;
    setWslTargetForm((current) => ({
      ...current,
      distribution: distribution.name,
      label: current.label.trim() ? current.label : distribution.name,
    }));
  }, [wslDistributions, wslTargetForm.distribution]);

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
    void switchAiProvider(id);
  }

  const {
    updateSshTargetFormField,
    updateSshTargetEditFormField,
    updateWslTargetFormField,
    updateWslTargetEditFormField,
    updateExistingTargetPassword,
    handleStartEditTarget,
    handleCancelEditTarget,
    handleCreateSshTarget,
    handleCreateWslTarget,
    handleTestNewSshTarget,
    handleTestNewWslTarget,
    handleTestExistingTarget,
    handleUpdateSshTarget,
    handleUpdateWslTarget,
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
    wslTargetForm,
    wslTargetEditForm,
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
    loadMarketplaceRegistries,
    loadMarketplaceSkills,
    createSshTarget,
    updateSshTarget,
    testSshTarget,
    createWslTarget,
    updateWslTarget,
    testWslTarget,
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
    setWslTargetForm,
    setWslTargetEditForm,
    setSshTargetPasswordUpdates,
  });

  const resolvedActiveTarget = activeTarget ?? targets[0]!;
  const firstRemoteTarget = targets.find(isRemoteLikeTarget);
  const localRemoteSyncTarget = targets.find(
    (target) => target.id === localRemoteSyncTargetId
  );

  useEffect(() => {
    const params = new URLSearchParams(location.search);
    const section = params.get("section");
    const shouldFocusRemoteTargets =
      section === "remote-targets" || location.hash === "#remote-targets-section";

    if (!shouldFocusRemoteTargets) return;

    window.requestAnimationFrame(() => {
      document
        .getElementById("remote-targets-section")
        ?.scrollIntoView({ block: "start", behavior: "smooth" });
    });
  }, [location.hash, location.search]);

  useEffect(() => {
    const params = new URLSearchParams(location.search);
    if (
      params.get("section") !== "remote-targets" ||
      params.get("action") !== "local-remote-sync"
    ) {
      return;
    }

    const actionKey = `${location.pathname}${location.search}`;
    if (consumedSettingsActionRef.current === actionKey) return;
    consumedSettingsActionRef.current = actionKey;

    const selectedTarget = isRemoteLikeTarget(resolvedActiveTarget)
      ? resolvedActiveTarget
      : firstRemoteTarget;

    if (!selectedTarget) {
      setTargetMessage({
        type: "error",
        text: t("settings.localRemoteSync.noRemoteTarget"),
      });
      return;
    }

    resetLocalRemoteSync();
    setLocalRemoteSyncTargetId(selectedTarget.id);
  }, [
    firstRemoteTarget,
    location.pathname,
    location.search,
    resetLocalRemoteSync,
    resolvedActiveTarget,
    t,
  ]);

  function handleLocalRemoteSyncOpenChange(open: boolean) {
    if (!open) {
      setLocalRemoteSyncTargetId(null);
      resetLocalRemoteSync();
    }
  }

  async function handlePreviewLocalRemoteSync() {
    if (!localRemoteSyncTargetId) return;
    await previewLocalRemoteSync({ targetId: localRemoteSyncTargetId });
  }

  async function handleApplyLocalRemoteSync() {
    if (!localRemoteSyncTargetId) return;
    await applyLocalRemoteSync({ targetId: localRemoteSyncTargetId });
    await loadTargets();
    if (localRemoteSyncTargetId === activeTarget?.id) {
      await Promise.allSettled([loadCentralSkills(), refreshCounts()]);
    }
  }

  return (
    <div className="flex flex-col h-full">
      <div className="border-b border-border px-6 py-4">
        <h1 className="text-xl font-semibold">{t("settings.title")}</h1>
      </div>

      <div className="flex-1 overflow-auto p-6 space-y-6">
        <SettingsTableOfContents entries={SETTINGS_TOC_ENTRIES} />

        <section id="appearance-section" className="scroll-mt-24">
          <AppearanceSettingsSection />
        </section>

        <section id="remote-targets-section" className="scroll-mt-24">
          <RemoteTargetsSettingsSection
            activeTarget={resolvedActiveTarget}
            deletingTargetId={deletingTargetId}
            editingTargetId={editingTargetId}
            isCreatingTarget={isCreatingTarget}
            isLoadingTargets={isLoadingTargets}
            isLoadingWslDistributions={isLoadingWslDistributions}
            switchingTargetId={switchingTargetId}
            sshTargetEditForm={sshTargetEditForm}
            sshTargetForm={sshTargetForm}
            sshTargetPasswordUpdates={sshTargetPasswordUpdates}
            targetMessage={targetMessage}
            targets={targets}
            testingTargetId={testingTargetId}
            updatingPasswordTargetId={updatingPasswordTargetId}
            updatingTargetId={updatingTargetId}
            wslDistributionError={wslDistributionError}
            wslDistributions={wslDistributions}
            wslTargetEditForm={wslTargetEditForm}
            wslTargetForm={wslTargetForm}
            onCancelEditTarget={handleCancelEditTarget}
            onCreateSshTarget={() => {
              void handleCreateSshTarget();
            }}
            onCreateWslTarget={() => {
              void handleCreateWslTarget();
            }}
            onDeleteTarget={(targetId) => {
              void handleDeleteTarget(targetId);
            }}
            onOpenLocalRemoteSync={(targetId) => {
              resetLocalRemoteSync();
              setLocalRemoteSyncTargetId(targetId);
            }}
            onRefreshWslDistributions={() => {
              void loadWslDistributions().catch(() => undefined);
            }}
            onStartEditTarget={handleStartEditTarget}
            onSwitchTarget={(targetId) => {
              void handleSwitchTarget(targetId);
            }}
            onTestExistingTarget={(target) => {
              void handleTestExistingTarget(target);
            }}
            onTestNewSshTarget={() => {
              void handleTestNewSshTarget();
            }}
            onTestNewWslTarget={() => {
              void handleTestNewWslTarget();
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
            onUpdateWslTarget={(target) => {
              void handleUpdateWslTarget(target);
            }}
            onUpdateWslTargetEditForm={updateWslTargetEditFormField}
            onUpdateWslTargetForm={updateWslTargetFormField}
          />
        </section>

        <section id="custom-platforms-section" className="scroll-mt-24">
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
        </section>

        <section id="platform-visibility-section" className="scroll-mt-24">
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
        </section>

        <section id="github-pat-section" className="scroll-mt-24">
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
        </section>

        <section id="ai-section" className="scroll-mt-24">
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
        </section>

        <section id="scan-directories-section" className="scroll-mt-24">
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
        </section>

        <section id="about-section" className="scroll-mt-24">
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
        </section>
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

      <LocalRemoteSyncDialog
        open={Boolean(localRemoteSyncTargetId)}
        targetLabel={localRemoteSyncTarget?.label ?? ""}
        preview={localRemoteSyncPreview}
        result={localRemoteSyncResult}
        isPreviewing={isLocalRemoteSyncPreviewing}
        isApplying={isLocalRemoteSyncApplying}
        error={localRemoteSyncError}
        onOpenChange={handleLocalRemoteSyncOpenChange}
        onPreview={() => {
          void handlePreviewLocalRemoteSync();
        }}
        onApply={() => {
          void handleApplyLocalRemoteSync();
        }}
      />
    </div>
  );
}
