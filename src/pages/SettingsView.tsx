import { useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";
import i18n from "@/i18n";

import { useSettingsStore } from "@/stores/settingsStore";
import { useThemeStore, ACCENT_NAMES } from "@/stores/themeStore";
import { usePlatformStore } from "@/stores/platformStore";
import { useCentralSkillsStore } from "@/stores/centralSkillsStore";
import { useDiscoverStore } from "@/stores/discoverStore";
import { useMarketplaceStore } from "@/stores/marketplaceStore";
import { useTargetStore } from "@/stores/targetStore";
import {
  DEFAULT_PLATFORM_CATEGORY_VISIBILITY,
  getPlatformCategoryKey,
  sortPlatformVisibilityAgents,
  type PlatformCategoryKey,
} from "@/lib/platformVisibility";
import { AddDirectoryDialog } from "@/components/settings/AddDirectoryDialog";
import { AboutSettingsSection } from "@/components/settings/AboutSettingsSection";
import { AiSettingsSection } from "@/components/settings/AiSettingsSection";
import { CustomPlatformsSettingsSection } from "@/components/settings/CustomPlatformsSettingsSection";
import { GitHubPatSettingsSection } from "@/components/settings/GitHubPatSettingsSection";
import { PlatformDialog } from "@/components/settings/PlatformDialog";
import { PlatformVisibilitySettingsSection } from "@/components/settings/PlatformVisibilitySettingsSection";
import { matchesPlatformVisibilityQuery } from "@/components/settings/platformVisibilityUtils";
import {
  RemoteTargetsSettingsSection,
  type SshTargetFormState,
} from "@/components/settings/RemoteTargetsSettingsSection";
import { ScanDirectoriesSettingsSection } from "@/components/settings/ScanDirectoriesSettingsSection";
import { AgentWithStatus, TargetSummary } from "@/types";
import { AI_PROVIDERS } from "@/data/aiProviders";
import {
  createPlatformTargetGroups,
} from "@/lib/platformTargetGroups";
import {
  CTP_VAR_MAP,
  EMPTY_SSH_TARGET_FORM,
  FLAVOR_COLORS,
  FLAVOR_ORDER,
  REPO_URL,
  resolveSettingsDbPath,
} from "@/pages/settingsViewModel";
const APP_VERSION = __APP_VERSION__;

export function SettingsView() {
  const { t } = useTranslation();

  // ── Store State ────────────────────────────────────────────────────────────

  const scanDirectories = useSettingsStore((s) => s.scanDirectories);
  const isLoadingScanDirs = useSettingsStore((s) => s.isLoadingScanDirs);
  const loadScanDirectories = useSettingsStore((s) => s.loadScanDirectories);
  const addScanDirectory = useSettingsStore((s) => s.addScanDirectory);
  const removeScanDirectory = useSettingsStore((s) => s.removeScanDirectory);
  const toggleScanDirectory = useSettingsStore((s) => s.toggleScanDirectory);
  const addCustomAgent = useSettingsStore((s) => s.addCustomAgent);
  const updateCustomAgent = useSettingsStore((s) => s.updateCustomAgent);
  const removeCustomAgent = useSettingsStore((s) => s.removeCustomAgent);
  const githubPat = useSettingsStore((s) => s.githubPat);
  const isLoadingGitHubPat = useSettingsStore((s) => s.isLoadingGitHubPat);
  const isSavingGitHubPat = useSettingsStore((s) => s.isSavingGitHubPat);
  const isTestingGitHubPat = useSettingsStore((s) => s.isTestingGitHubPat);
  const loadGitHubPat = useSettingsStore((s) => s.loadGitHubPat);
  const saveGitHubPat = useSettingsStore((s) => s.saveGitHubPat);
  const clearGitHubPat = useSettingsStore((s) => s.clearGitHubPat);
  const testGitHubPat = useSettingsStore((s) => s.testGitHubPat);

  const agents = usePlatformStore((s) => s.agents);
  const categoryVisibility = usePlatformStore((s) => s.categoryVisibility) ?? DEFAULT_PLATFORM_CATEGORY_VISIBILITY;
  const setCategoryVisibility = usePlatformStore((s) => s.setCategoryVisibility) ?? (async () => undefined);
  const setAgentEnabled = usePlatformStore((s) => s.setAgentEnabled) ?? (async () => undefined);
  const loadCentralSkills = useCentralSkillsStore((s) => s.loadCentralSkills);
  const refreshDiscoverCounts = useDiscoverStore((s) => s.refreshCounts);
  const loadMarketplaceRegistries = useMarketplaceStore((s) => s.loadRegistries);
  const selectedMarketplaceRegistryId = useMarketplaceStore((s) => s.selectedRegistryId);
  const loadMarketplaceSkills = useMarketplaceStore((s) => s.loadSkills);
  const targets = useTargetStore((s) => s.targets);
  const activeTarget = useTargetStore((s) => s.activeTarget);
  const isLoadingTargets = useTargetStore((s) => s.isLoading);
  const isCreatingTarget = useTargetStore((s) => s.isCreating);
  const updatingTargetId = useTargetStore((s) => s.updatingTargetId);
  const testingTargetId = useTargetStore((s) => s.testingTargetId);
  const updatingPasswordTargetId = useTargetStore((s) => s.updatingPasswordTargetId);
  const switchingTargetId = useTargetStore((s) => s.switchingTargetId);
  const deletingTargetId = useTargetStore((s) => s.deletingTargetId);
  const loadTargets = useTargetStore((s) => s.loadTargets);
  const createSshTarget = useTargetStore((s) => s.createSshTarget);
  const updateSshTarget = useTargetStore((s) => s.updateSshTarget);
  const testSshTarget = useTargetStore((s) => s.testSshTarget);
  const updateSshTargetPassword = useTargetStore((s) => s.updateSshTargetPassword);
  const deleteTarget = useTargetStore((s) => s.deleteTarget);
  const switchTarget = useTargetStore((s) => s.switchTarget);

  const flavor = useThemeStore((s) => s.flavor);
  const setFlavor = useThemeStore((s) => s.setFlavor);
  const accent = useThemeStore((s) => s.accent);
  const setAccent = useThemeStore((s) => s.setAccent);
  const rescan = usePlatformStore((s) => s.rescan);
  const refreshCounts = usePlatformStore((s) => s.refreshCounts);

  // Custom agents are those that are not built-in.
  const customAgents = agents.filter((a) => !a.is_builtin);
  const dbPathDisplay = useMemo(
    () => resolveSettingsDbPath(agents, scanDirectories),
    [agents, scanDirectories]
  );

  // ── Local State ────────────────────────────────────────────────────────────

  const [platformVisibilityQuery, setPlatformVisibilityQuery] = useState("");

  // AI Provider state is centralized in settingsStore so the page does not
  // perform direct Tauri IPC and rapid edits can be saved as one debounced batch.
  const aiSettings = useSettingsStore((s) => s.aiSettings);
  const aiSettingsLoaded = useSettingsStore((s) => s.aiSettingsLoaded);
  const isLoadingAiSettings = useSettingsStore((s) => s.isLoadingAiSettings);
  const aiSaveStatus = useSettingsStore((s) => s.aiSaveStatus);
  const aiSaveError = useSettingsStore((s) => s.aiSaveError);
  const aiTesting = useSettingsStore((s) => s.aiTesting);
  const aiTestResult = useSettingsStore((s) => s.aiTestResult);
  const loadAiSettings = useSettingsStore((s) => s.loadAiSettings);
  const updateAiSettings = useSettingsStore((s) => s.updateAiSettings);
  const testAiConnection = useSettingsStore((s) => s.testAiConnection);
  const aiProvider = aiSettings.provider;
  const aiRegion = aiSettings.region;
  const aiModel = aiSettings.model;
  const aiCustomUrl = aiSettings.customUrl;

  useEffect(() => {
    if (!aiSettingsLoaded && !isLoadingAiSettings) {
      void loadAiSettings();
    }
  }, [aiSettingsLoaded, isLoadingAiSettings, loadAiSettings]);

  // When provider or region changes, update model to default
  function handleProviderChange(id: string) {
    const p = AI_PROVIDERS.find((x) => x.id === id);
    updateAiSettings({
      provider: id,
      model: p?.defaultModel ?? aiModel,
      region: p && !p.regions.includes(aiRegion) ? p.regions[0] : aiRegion,
    });
  }

  const currentProvider = AI_PROVIDERS.find((p) => p.id === aiProvider);
  const resolvedUrl = aiProvider === "custom"
    ? aiCustomUrl
    : (currentProvider?.endpoints[aiRegion] ?? "");
  const lang = i18n.language;
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
  const [githubPatMessage, setGitHubPatMessage] = useState<{ type: "success" | "error"; text: string } | null>(null);
  const [sshTargetForm, setSshTargetForm] = useState<SshTargetFormState>(EMPTY_SSH_TARGET_FORM);
  const [editingTargetId, setEditingTargetId] = useState<string | null>(null);
  const [sshTargetEditForm, setSshTargetEditForm] =
    useState<SshTargetFormState>(EMPTY_SSH_TARGET_FORM);
  const [sshTargetPasswordUpdates, setSshTargetPasswordUpdates] = useState<Record<string, string>>({});
  const [targetMessage, setTargetMessage] = useState<{ type: "success" | "error"; text: string } | null>(null);
  const normalizedPlatformVisibilityQuery = useMemo(
    () => platformVisibilityQuery.trim().toLowerCase(),
    [platformVisibilityQuery]
  );
  const isPlatformVisibilitySearchActive =
    normalizedPlatformVisibilityQuery.length > 0;
  const allPlatformAgents = useMemo(
    () => agents.filter((agent) => agent.id !== "central"),
    [agents]
  );
  const platformVisibilityGroups = useMemo(() => {
    const groupConfigs = [
      {
        category: "coding" as const,
        title: t("sidebar.categoryCoding"),
        description: t("settings.platformGroupCodingDesc"),
        groupVisible: categoryVisibility.coding,
      },
      {
        category: "lobster" as const,
        title: t("sidebar.categoryLobster"),
        description: t("settings.platformGroupLobsterDesc"),
        groupVisible: categoryVisibility.lobster,
      },
    ];

    return groupConfigs
      .map((group) => {
        const groupAgents = sortPlatformVisibilityAgents(
          allPlatformAgents.filter(
            (agent) => getPlatformCategoryKey(agent.category) === group.category
          )
        );
        const groupedAgents = createPlatformTargetGroups(groupAgents, agents);
        const matchingAgents = groupedAgents.filter((agent) =>
          matchesPlatformVisibilityQuery(agent, normalizedPlatformVisibilityQuery)
        );

        return {
          ...group,
          enabledCount: groupAgents.filter((agent) => agent.is_enabled).length,
          totalCount: groupAgents.length,
          agents: matchingAgents,
        };
      })
      .filter(
        (group) =>
          !isPlatformVisibilitySearchActive || group.agents.length > 0
      );
  }, [
    allPlatformAgents,
    agents,
    categoryVisibility.coding,
    categoryVisibility.lobster,
    isPlatformVisibilitySearchActive,
    normalizedPlatformVisibilityQuery,
    t,
  ]);

  // ── Load on mount ──────────────────────────────────────────────────────────

  useEffect(() => {
    loadScanDirectories();
    loadGitHubPat();
    loadTargets();
  }, [loadScanDirectories, loadGitHubPat, loadTargets]);

  useEffect(() => {
    setGitHubPatInput(githubPat);
  }, [githubPat]);

  const isGitHubPatDirty = useMemo(() => githubPatInput.trim() !== githubPat, [githubPatInput, githubPat]);

  async function refreshAfterTargetChange() {
    await rescan();
    await Promise.allSettled([
      loadCentralSkills(),
      refreshDiscoverCounts(),
      loadMarketplaceRegistries().then(() => {
        if (selectedMarketplaceRegistryId) {
          return loadMarketplaceSkills(selectedMarketplaceRegistryId);
        }
      }),
    ]);
  }

  function updateSshTargetForm(field: keyof SshTargetFormState, value: string) {
    setSshTargetForm((current) => ({ ...current, [field]: value }));
  }

  function updateSshTargetEditForm(field: keyof SshTargetFormState, value: string) {
    setSshTargetEditForm((current) => ({ ...current, [field]: value }));
  }

  function updateExistingTargetPassword(targetId: string, value: string) {
    setSshTargetPasswordUpdates((current) => ({ ...current, [targetId]: value }));
  }

  function targetToSshTargetForm(target: TargetSummary): SshTargetFormState {
    return {
      label: target.label,
      host: target.host ?? "",
      username: target.username ?? "",
      port: String(target.port ?? 22),
      authMethod: target.authMethod ?? "key",
      keyPath: target.keyPath ?? "",
      password: "",
    };
  }

  function sshTargetPayload(form: SshTargetFormState, includeEmptyPassword: boolean) {
    const port = Number(form.port.trim() || "22");
    const authMethod = form.authMethod;
    const password = form.password.trim();
    return {
      label: form.label.trim(),
      host: form.host.trim(),
      username: form.username.trim(),
      port: Number.isFinite(port) ? port : 22,
      authMethod,
      keyPath: authMethod === "key" ? form.keyPath.trim() : null,
      password: authMethod === "password" && (includeEmptyPassword || password)
        ? form.password
        : null,
    };
  }

  function handleStartEditTarget(target: TargetSummary) {
    setTargetMessage(null);
    setEditingTargetId(target.id);
    setSshTargetEditForm(targetToSshTargetForm(target));
  }

  function handleCancelEditTarget() {
    setEditingTargetId(null);
    setSshTargetEditForm(EMPTY_SSH_TARGET_FORM);
  }

  async function handleCreateSshTarget() {
    setTargetMessage(null);
    try {
      const target = await createSshTarget(sshTargetPayload(sshTargetForm, true));
      setSshTargetForm(EMPTY_SSH_TARGET_FORM);
      setTargetMessage({ type: "success", text: t("targets.created", { label: target.label }) });
      toast.success(t("targets.created", { label: target.label }));
    } catch (err) {
      const text = String(err);
      setTargetMessage({ type: "error", text });
      toast.error(text);
    }
  }

  async function handleTestNewSshTarget() {
    setTargetMessage(null);
    try {
      const result = await testSshTarget(sshTargetPayload(sshTargetForm, true));
      const text = result.ok
        ? t("targets.testSucceeded", {
            home: result.remoteHome ?? "",
            os: result.remoteOs ?? "",
          })
        : result.message;
      setTargetMessage({ type: result.ok ? "success" : "error", text });
      if (result.ok) {
        toast.success(text);
      } else {
        toast.error(text);
      }
    } catch (err) {
      const text = String(err);
      setTargetMessage({ type: "error", text });
      toast.error(text);
    }
  }

  async function handleTestExistingTarget(targetId: string) {
    setTargetMessage(null);
    try {
      const password = sshTargetPasswordUpdates[targetId];
      const result = await testSshTarget({
        id: targetId,
        password: password?.trim() ? password : null,
      });
      const text = result.ok
        ? t("targets.testSucceeded", {
            home: result.remoteHome ?? "",
            os: result.remoteOs ?? "",
          })
        : result.message;
      setTargetMessage({ type: result.ok ? "success" : "error", text });
      if (result.ok) {
        toast.success(text);
      } else {
        toast.error(text);
      }
    } catch (err) {
      const text = String(err);
      setTargetMessage({ type: "error", text });
      toast.error(text);
    }
  }

  async function handleUpdateSshTarget(target: TargetSummary) {
    setTargetMessage(null);
    try {
      const updatedTarget = await updateSshTarget({
        id: target.id,
        ...sshTargetPayload(sshTargetEditForm, false),
      });
      setEditingTargetId(null);
      setSshTargetEditForm(EMPTY_SSH_TARGET_FORM);
      updateExistingTargetPassword(target.id, "");
      await refreshAfterTargetChange();
      const text = t("targets.updated", { label: updatedTarget.label });
      setTargetMessage({ type: "success", text });
      toast.success(text);
    } catch (err) {
      const text = String(err);
      setTargetMessage({ type: "error", text });
      toast.error(text);
    }
  }

  async function handleUpdateTargetPassword(target: TargetSummary) {
    const password = sshTargetPasswordUpdates[target.id] ?? "";
    if (!password.trim()) {
      const text = t("targets.passwordRequired");
      setTargetMessage({ type: "error", text });
      toast.error(text);
      return;
    }

    setTargetMessage(null);
    try {
      const result = await updateSshTargetPassword(target.id, password);
      const text = result.ok
        ? result.credentialStatus === "session"
          ? t("targets.passwordUpdatedSession", { label: target.label })
          : t("targets.passwordUpdated", { label: target.label })
        : result.message;
      setTargetMessage({ type: result.ok ? "success" : "error", text });
      if (result.ok) {
        updateExistingTargetPassword(target.id, "");
        toast.success(text);
      } else {
        toast.error(text);
      }
    } catch (err) {
      const text = String(err);
      setTargetMessage({ type: "error", text });
      toast.error(text);
    }
  }

  async function handleSwitchTarget(targetId: string) {
    setTargetMessage(null);
    try {
      const target = await switchTarget(targetId);
      await refreshAfterTargetChange();
      toast.success(t("targets.switched", { label: target.label }));
    } catch (err) {
      const text = String(err);
      setTargetMessage({ type: "error", text });
      toast.error(text);
    }
  }

  async function handleDeleteTarget(targetId: string) {
    setTargetMessage(null);
    try {
      await deleteTarget(targetId);
      await refreshAfterTargetChange();
      toast.success(t("targets.deleted"));
    } catch (err) {
      const text = String(err);
      setTargetMessage({ type: "error", text });
      toast.error(text);
    }
  }

  // ── Scan Directories Handlers ──────────────────────────────────────────────

  async function handleAddDirectory(path: string) {
    setScanDirError(null);
    try {
      await addScanDirectory(path);
      // Trigger rescan after adding a directory.
      await refreshCounts();
      toast.success(t("addDir.add") + " ✓");
    } catch (err) {
      setScanDirError(String(err));
      toast.error(String(err));
      throw err; // Re-throw so the dialog knows it failed
    }
  }

  async function handleRemoveDirectory(path: string) {
    setRemovingDir(path);
    setScanDirError(null);
    try {
      await removeScanDirectory(path);
      // Trigger rescan after removing a directory.
      await refreshCounts();
      toast.success(t("common.delete") + " ✓");
    } catch (err) {
      setScanDirError(String(err));
      toast.error(String(err));
    } finally {
      setRemovingDir(null);
    }
  }

  /**
   * Toggle the active state of a custom scan directory.
   * Persists the change to the backend via set_scan_directory_active command.
   */
  async function handleToggleDirectory(path: string, active: boolean) {
    setScanDirError(null);
    try {
      await toggleScanDirectory(path, active);
    } catch (err) {
      setScanDirError(String(err));
      toast.error(String(err));
    }
  }

  // ── Custom Platform Handlers ───────────────────────────────────────────────

  function handleOpenAddPlatform() {
    setEditingPlatform(null);
    setPlatformError(null);
    setIsPlatformDialogOpen(true);
  }

  function handleOpenEditPlatform(agent: AgentWithStatus) {
    setEditingPlatform(agent);
    setPlatformError(null);
    setIsPlatformDialogOpen(true);
  }

  async function handleAddPlatform(displayName: string, globalSkillsDir: string, category?: string) {
    setPlatformError(null);
    try {
      await addCustomAgent({
        display_name: displayName,
        global_skills_dir: globalSkillsDir,
        category: category || "coding",
      });
      // Refresh agents + rescan to show new platform in sidebar.
      await rescan();
      toast.success(t("platformDialog.add") + " ✓");
    } catch (err) {
      setPlatformError(String(err));
      toast.error(String(err));
      throw err;
    }
  }

  async function handleEditPlatform(displayName: string, globalSkillsDir: string, category?: string) {
    if (!editingPlatform) return;
    setPlatformError(null);
    try {
      await updateCustomAgent(editingPlatform.id, {
        display_name: displayName,
        global_skills_dir: globalSkillsDir,
        category: category || "coding",
      });
      // Refresh agents + rescan.
      await rescan();
      toast.success(t("platformDialog.save") + " ✓");
    } catch (err) {
      setPlatformError(String(err));
      toast.error(String(err));
      throw err;
    }
  }

  async function handleRemovePlatform(agentId: string) {
    setRemovingAgent(agentId);
    setPlatformError(null);
    try {
      await removeCustomAgent(agentId);
      // Refresh agents.
      await rescan();
      toast.success(t("common.delete") + " ✓");
    } catch (err) {
      setPlatformError(String(err));
      toast.error(String(err));
    } finally {
      setRemovingAgent(null);
    }
  }

  async function handleToggleCategory(category: PlatformCategoryKey, visible: boolean) {
    try {
      await setCategoryVisibility(category, visible);
    } catch (err) {
      toast.error(String(err));
    }
  }

  async function handleTogglePlatformVisibility(agentId: string, enabled: boolean) {
    try {
      await setAgentEnabled(agentId, enabled);
    } catch (err) {
      toast.error(String(err));
    }
  }

  async function handleSaveGitHubPat() {
    setGitHubPatMessage(null);
    try {
      await saveGitHubPat(githubPatInput);
      setGitHubPatMessage({
        type: "success",
        text: t("settings.githubPatSaved"),
      });
      toast.success(t("settings.githubPatSaved"));
    } catch (err) {
      const text = String(err);
      setGitHubPatMessage({ type: "error", text });
      toast.error(text);
    }
  }

  async function handleClearGitHubPat() {
    setGitHubPatMessage(null);
    try {
      await clearGitHubPat();
      setGitHubPatInput("");
      setGitHubPatMessage({
        type: "success",
        text: t("settings.githubPatCleared"),
      });
      toast.success(t("settings.githubPatCleared"));
    } catch (err) {
      const text = String(err);
      setGitHubPatMessage({ type: "error", text });
      toast.error(text);
    }
  }

  async function handleTestGitHubPat() {
    setGitHubPatMessage(null);
    try {
      const result = await testGitHubPat();
      setGitHubPatMessage({
        type: result.ok ? "success" : "error",
        text: result.message,
      });
      if (result.ok) {
        toast.success(result.message);
      } else {
        toast.error(result.message);
      }
    } catch (err) {
      const text = String(err);
      setGitHubPatMessage({ type: "error", text });
      toast.error(text);
    }
  }

  // ── Render ─────────────────────────────────────────────────────────────────

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="border-b border-border px-6 py-4">
        <h1 className="text-xl font-semibold">{t("settings.title")}</h1>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto p-6 space-y-6">

        <RemoteTargetsSettingsSection
          activeTarget={activeTarget}
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
          onUpdateSshTargetEditForm={updateSshTargetEditForm}
          onUpdateSshTargetForm={updateSshTargetForm}
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
          isSearchActive={isPlatformVisibilitySearchActive}
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
          githubPat={githubPat}
          githubPatInput={githubPatInput}
          githubPatMessage={githubPatMessage}
          isGitHubPatDirty={isGitHubPatDirty}
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
          aiSettings={aiSettings}
          aiTestResult={aiTestResult}
          aiTesting={aiTesting}
          isLoadingAiSettings={isLoadingAiSettings}
          lang={lang}
          resolvedUrl={resolvedUrl}
          showAiTestDetails={showAiTestDetails}
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

      {/* ── Dialogs ────────────────────────────────────────────────────────── */}
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
