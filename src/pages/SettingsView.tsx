import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useLocation, useNavigate } from "react-router-dom";
import i18n from "@/i18n";

import {
  type SshTargetFormState,
  type WslTargetFormState,
} from "@/components/settings/RemoteTargetsSettingsSection";
import {
  getCanonicalSettingsPagePath,
  getSettingsPageById,
  isSettingsPageId,
  normalizeSettingsSubpath,
  resolveSettingsPageId,
} from "@/components/settings/settingsPages";
import { SettingsSideNav } from "@/components/settings/SettingsSideNav";
import { AI_PROVIDERS } from "@/data/aiProviders";
import { createSettingsViewActions } from "@/pages/settingsViewActions";
import { useSettingsViewBindings } from "@/pages/settingsViewBindings";
import { SettingsPageSections } from "@/pages/settingsPageSections";
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

export function SettingsView() {
  const { t } = useTranslation();
  const location = useLocation();
  const navigate = useNavigate();
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

  const activePageId = useMemo(
    () =>
      resolveSettingsPageId({
        pathname: location.pathname,
        search: location.search,
        hash: location.hash,
      }),
    [location.hash, location.pathname, location.search]
  );
  const activePage = getSettingsPageById(activePageId);
  const settingsSubpath = normalizeSettingsSubpath(location.pathname);
  const isKnownSettingsSubpath = !settingsSubpath || isSettingsPageId(settingsSubpath);

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

  useEffect(() => {
    const nextPath = getCanonicalSettingsPagePath(activePageId);
    if (location.pathname !== nextPath) {
      navigate(`${nextPath}${location.search}${location.hash}`, { replace: true });
    }
  }, [activePageId, location.hash, location.pathname, location.search, navigate]);

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
    if (!activePage.sectionIds.includes("remote-targets")) return;

    window.requestAnimationFrame(() => {
      document
        .getElementById("remote-targets-section")
        ?.scrollIntoView({ block: "start", behavior: "smooth" });
    });
  }, [activePage.sectionIds, location.hash, location.search]);

  useEffect(() => {
    const params = new URLSearchParams(location.search);
    if (
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

  const PageIcon = activePage.icon;

  return (
    <div className="flex h-full min-h-0 flex-col bg-background">
      <div className="border-b border-border px-6 py-4">
        <p className="text-xs font-medium uppercase tracking-[0.2em] text-muted-foreground">
          {t("settings.pages.eyebrow")}
        </p>
        <h1 className="mt-1 text-xl font-semibold">{t("settings.title")}</h1>
      </div>

      <div className="flex min-h-0 flex-1 flex-col lg:flex-row">
        <SettingsSideNav activePageId={activePageId} />
        <main className="min-h-0 flex-1 overflow-auto px-4 py-6 sm:px-6 lg:px-8 xl:px-10">
          <div className="mx-auto w-full max-w-7xl space-y-6">
            <header className="border-b border-border/80 pb-5">
              <div className="flex items-start gap-3">
                <span className="mt-0.5 grid size-10 shrink-0 place-items-center rounded-2xl border border-border bg-card text-primary shadow-sm">
                  <PageIcon className="size-5" aria-hidden="true" />
                </span>
                <div className="min-w-0">
                  <h2 className="text-2xl font-semibold tracking-tight">
                    {t(activePage.titleKey)}
                  </h2>
                  <p className="mt-1 max-w-3xl text-sm leading-6 text-muted-foreground">
                    {t(activePage.descriptionKey)}
                  </p>
                  {!isKnownSettingsSubpath ? (
                    <p className="mt-2 text-xs text-muted-foreground">
                      {t("settings.pages.legacyFallback")}
                    </p>
                  ) : null}
                </div>
              </div>
            </header>

            <SettingsPageSections
              accent={accent}
              activeTarget={resolvedActiveTarget}
              aiApiKeyState={aiApiKeyState}
              aiSaveError={aiSaveError}
              aiSaveStatus={aiSaveStatus}
              aiSettings={aiSettings}
              aiTestResult={aiTestResult}
              aiTesting={aiTesting}
              appVersion={APP_VERSION}
              ctpVarMap={CTP_VAR_MAP}
              customAgents={customAgents}
              dbPathDisplay={dbPathDisplay}
              deletingTargetId={deletingTargetId}
              editingPlatform={editingPlatform}
              editingTargetId={editingTargetId}
              flavor={flavor}
              flavorColors={FLAVOR_COLORS}
              flavorOrder={FLAVOR_ORDER}
              githubPatInput={githubPatInput}
              githubPatMessage={githubPatMessage}
              githubPatState={githubPatState}
              isAddDirOpen={isAddDirOpen}
              isApplyingLocalRemoteSync={isLocalRemoteSyncApplying}
              isCreatingTarget={isCreatingTarget}
              isLoadingAiSettings={isLoadingAiSettings}
              isLoadingGitHubPat={isLoadingGitHubPat}
              isLoadingScanDirs={isLoadingScanDirs}
              isLoadingTargets={isLoadingTargets}
              isLoadingWslDistributions={isLoadingWslDistributions}
              isPlatformDialogOpen={isPlatformDialogOpen}
              isPreviewingLocalRemoteSync={isLocalRemoteSyncPreviewing}
              isSavingGitHubPat={isSavingGitHubPat}
              isTestingGitHubPat={isTestingGitHubPat}
              lang={lang}
              localRemoteSyncError={localRemoteSyncError}
              localRemoteSyncPreview={localRemoteSyncPreview}
              localRemoteSyncResult={localRemoteSyncResult}
              localRemoteSyncTarget={localRemoteSyncTarget}
              normalizedPlatformVisibilityQuery={normalizedPlatformVisibilityQuery}
              page={activePage}
              platformError={platformError}
              platformVisibilityGroups={platformVisibilityGroups}
              platformVisibilityQuery={platformVisibilityQuery}
              platformVisibilitySearchActive={platformVisibilitySearchActive}
              removingAgent={removingAgent}
              removingDir={removingDir}
              repoUrl={REPO_URL}
              resolvedUrl={resolvedUrl}
              scanDirError={scanDirError}
              scanDirectories={scanDirectories}
              showAiTestDetails={showAiTestDetails}
              showBuiltinDirs={showBuiltinDirs}
              sshTargetEditForm={sshTargetEditForm}
              sshTargetForm={sshTargetForm}
              sshTargetPasswordUpdates={sshTargetPasswordUpdates}
              switchingTargetId={switchingTargetId}
              targetMessage={targetMessage}
              targets={targets}
              testingTargetId={testingTargetId}
              updatingPasswordTargetId={updatingPasswordTargetId}
              updatingTargetId={updatingTargetId}
              wslDistributionError={wslDistributionError}
              wslDistributions={wslDistributions}
              wslTargetEditForm={wslTargetEditForm}
              wslTargetForm={wslTargetForm}
              onAddDirectory={handleAddDirectory}
              onAddPlatform={handleAddPlatform}
              onApplyLocalRemoteSync={() => {
                void handleApplyLocalRemoteSync();
              }}
              onCancelEditTarget={handleCancelEditTarget}
              onClearAiApiKey={() => {
                void clearAiApiKey();
              }}
              onClearGitHubPat={() => {
                void handleClearGitHubPat();
              }}
              onCreateSshTarget={() => {
                void handleCreateSshTarget();
              }}
              onCreateWslTarget={() => {
                void handleCreateWslTarget();
              }}
              onDeleteTarget={(targetId) => {
                void handleDeleteTarget(targetId);
              }}
              onEditPlatform={handleEditPlatform}
              onGitHubPatInputChange={setGitHubPatInput}
              onLocalRemoteSyncOpenChange={handleLocalRemoteSyncOpenChange}
              onOpenAddDirectory={() => setIsAddDirOpen(true)}
              onOpenAddDirectoryChange={setIsAddDirOpen}
              onOpenAddPlatform={handleOpenAddPlatform}
              onOpenEditPlatform={handleOpenEditPlatform}
              onOpenLocalRemoteSync={(targetId) => {
                resetLocalRemoteSync();
                setLocalRemoteSyncTargetId(targetId);
              }}
              onPlatformDialogOpenChange={setIsPlatformDialogOpen}
              onPreviewLocalRemoteSync={() => {
                void handlePreviewLocalRemoteSync();
              }}
              onProviderChange={handleProviderChange}
              onRefreshWslDistributions={() => {
                void loadWslDistributions().catch(() => undefined);
              }}
              onRemoveDirectory={(path) => {
                void handleRemoveDirectory(path);
              }}
              onRemovePlatform={(agentId) => {
                void handleRemovePlatform(agentId);
              }}
              onSaveGitHubPat={() => {
                void handleSaveGitHubPat();
              }}
              onSetAccent={setAccent}
              onSetFlavor={setFlavor}
              onSetPlatformVisibilityQuery={setPlatformVisibilityQuery}
              onSetShowAiTestDetails={setShowAiTestDetails}
              onStartEditTarget={handleStartEditTarget}
              onSwitchTarget={(targetId) => {
                void handleSwitchTarget(targetId);
              }}
              onTestAiConnection={async () => {
                setShowAiTestDetails(false);
                await testAiConnection();
              }}
              onTestExistingTarget={(target) => {
                void handleTestExistingTarget(target);
              }}
              onTestGitHubPat={() => {
                void handleTestGitHubPat();
              }}
              onTestNewSshTarget={() => {
                void handleTestNewSshTarget();
              }}
              onTestNewWslTarget={() => {
                void handleTestNewWslTarget();
              }}
              onToggleBuiltinDirs={() => setShowBuiltinDirs((value) => !value)}
              onToggleCategory={(category, visible) => {
                void handleToggleCategory(category, visible);
              }}
              onToggleDirectory={(path, active) => {
                void handleToggleDirectory(path, active);
              }}
              onTogglePlatformVisibility={(agentId, enabled) => {
                void handleTogglePlatformVisibility(agentId, enabled);
              }}
              onUpdateAiSettings={updateAiSettings}
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
          </div>
        </main>
      </div>
    </div>
  );
}
