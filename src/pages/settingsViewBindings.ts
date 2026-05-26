import { useSettingsStore } from "@/stores/settingsStore";
import { useThemeStore } from "@/stores/themeStore";
import { usePlatformStore } from "@/stores/platformStore";
import { useCentralSkillsStore } from "@/stores/centralSkillsStore";
import { useMarketplaceStore } from "@/stores/marketplaceStore";
import { useTargetStore } from "@/stores/targetStore";
import { DEFAULT_PLATFORM_CATEGORY_VISIBILITY } from "@/lib/platformVisibility";

async function noopAsync() {
  return undefined;
}

export function useSettingsViewBindings() {
  const scanDirectories = useSettingsStore((state) => state.scanDirectories);
  const isLoadingScanDirs = useSettingsStore((state) => state.isLoadingScanDirs);
  const loadScanDirectories = useSettingsStore((state) => state.loadScanDirectories);
  const addScanDirectory = useSettingsStore((state) => state.addScanDirectory);
  const removeScanDirectory = useSettingsStore((state) => state.removeScanDirectory);
  const toggleScanDirectory = useSettingsStore((state) => state.toggleScanDirectory);
  const addCustomAgent = usePlatformStore((state) => state.addCustomAgent);
  const updateCustomAgent = usePlatformStore((state) => state.updateCustomAgent);
  const removeCustomAgent = usePlatformStore((state) => state.removeCustomAgent);
  const githubPatState = useSettingsStore((state) => state.githubPatState);
  const isLoadingGitHubPat = useSettingsStore((state) => state.isLoadingGitHubPat);
  const isSavingGitHubPat = useSettingsStore((state) => state.isSavingGitHubPat);
  const isTestingGitHubPat = useSettingsStore((state) => state.isTestingGitHubPat);
  const loadGitHubPat = useSettingsStore((state) => state.loadGitHubPat);
  const revealGitHubPat = useSettingsStore((state) => state.revealGitHubPat);
  const saveGitHubPat = useSettingsStore((state) => state.saveGitHubPat);
  const clearGitHubPat = useSettingsStore((state) => state.clearGitHubPat);
  const testGitHubPat = useSettingsStore((state) => state.testGitHubPat);
  const aiSettings = useSettingsStore((state) => state.aiSettings);
  const aiApiKeyState = useSettingsStore((state) => state.aiApiKeyState);
  const aiSettingsLoaded = useSettingsStore((state) => state.aiSettingsLoaded);
  const isLoadingAiSettings = useSettingsStore((state) => state.isLoadingAiSettings);
  const aiSaveStatus = useSettingsStore((state) => state.aiSaveStatus);
  const aiSaveError = useSettingsStore((state) => state.aiSaveError);
  const aiTesting = useSettingsStore((state) => state.aiTesting);
  const aiTestResult = useSettingsStore((state) => state.aiTestResult);
  const loadAiSettings = useSettingsStore((state) => state.loadAiSettings);
  const updateAiSettings = useSettingsStore((state) => state.updateAiSettings);
  const switchAiProvider = useSettingsStore((state) => state.switchAiProvider);
  const revealAiApiKey = useSettingsStore((state) => state.revealAiApiKey);
  const clearAiApiKey = useSettingsStore((state) => state.clearAiApiKey);
  const testAiConnection = useSettingsStore((state) => state.testAiConnection);

  const agents = usePlatformStore((state) => state.agents);
  const categoryVisibility =
    usePlatformStore((state) => state.categoryVisibility) ??
    DEFAULT_PLATFORM_CATEGORY_VISIBILITY;
  const setCategoryVisibility =
    usePlatformStore((state) => state.setCategoryVisibility) ?? noopAsync;
  const setAgentEnabled =
    usePlatformStore((state) => state.setAgentEnabled) ?? noopAsync;
  const rescan = usePlatformStore((state) => state.rescan);
  const refreshCounts = usePlatformStore((state) => state.refreshCounts);

  const loadCentralSkills = useCentralSkillsStore((state) => state.loadCentralSkills);
  const loadMarketplaceRegistries = useMarketplaceStore((state) => state.loadRegistries);
  const selectedMarketplaceRegistryId = useMarketplaceStore(
    (state) => state.selectedRegistryId
  );
  const loadMarketplaceSkills = useMarketplaceStore((state) => state.loadSkills);

  const targets = useTargetStore((state) => state.targets);
  const activeTarget = useTargetStore((state) => state.activeTarget);
  const wslDistributions = useTargetStore((state) => state.wslDistributions);
  const isLoadingTargets = useTargetStore((state) => state.isLoading);
  const isLoadingWslDistributions = useTargetStore(
    (state) => state.isLoadingWslDistributions
  );
  const isCreatingTarget = useTargetStore((state) => state.isCreating);
  const updatingTargetId = useTargetStore((state) => state.updatingTargetId);
  const testingTargetId = useTargetStore((state) => state.testingTargetId);
  const updatingPasswordTargetId = useTargetStore(
    (state) => state.updatingPasswordTargetId
  );
  const switchingTargetId = useTargetStore((state) => state.switchingTargetId);
  const deletingTargetId = useTargetStore((state) => state.deletingTargetId);
  const wslDistributionError = useTargetStore(
    (state) => state.wslDistributionError
  );
  const loadTargets = useTargetStore((state) => state.loadTargets);
  const loadWslDistributions = useTargetStore(
    (state) => state.loadWslDistributions
  );
  const createSshTarget = useTargetStore((state) => state.createSshTarget);
  const updateSshTarget = useTargetStore((state) => state.updateSshTarget);
  const testSshTarget = useTargetStore((state) => state.testSshTarget);
  const createWslTarget = useTargetStore((state) => state.createWslTarget);
  const updateWslTarget = useTargetStore((state) => state.updateWslTarget);
  const testWslTarget = useTargetStore((state) => state.testWslTarget);
  const updateSshTargetPassword = useTargetStore(
    (state) => state.updateSshTargetPassword
  );
  const deleteTarget = useTargetStore((state) => state.deleteTarget);
  const switchTarget = useTargetStore((state) => state.switchTarget);

  const flavor = useThemeStore((state) => state.flavor);
  const setFlavor = useThemeStore((state) => state.setFlavor);
  const accent = useThemeStore((state) => state.accent);
  const setAccent = useThemeStore((state) => state.setAccent);

  return {
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
    revealGitHubPat,
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
    revealAiApiKey,
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
  };
}
