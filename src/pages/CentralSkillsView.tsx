import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { Dispatch, SetStateAction } from "react";
import { useTranslation } from "react-i18next";
import {
  BatchDeleteCentralSkillPreviewResult,
  CentralSkillUpdateState,
  DeleteSkillRepositoryPreview,
  SkillDetail,
  SkillRepositoryWithStats,
  SkillWithLinks,
} from "@/types";
import type { CentralRepositorySyncPreview } from "@/types/centralRepositorySync";
import { markAppPerformance } from "@/lib/performance";
import { DEFAULT_PLATFORM_CATEGORY_VISIBILITY } from "@/lib/platformVisibility";
import { CentralSidebarHeader } from "@/components/central/CentralSidebarHeader";
import { CentralSkillsShell } from "@/components/central/CentralSkillsShell";
import { CentralStoreLocationDialog } from "@/components/central/CentralStoreLocationDialog";
import { CommandPalette } from "@/components/central/CommandPalette";
import { useCentralViewStateUrl } from "@/hooks/useCentralViewStateUrl";
import { useCentralSkillsActions } from "@/pages/centralSkillsActions";
import { getCentralSkillsCheckButtonState } from "@/pages/centralSkillsCheckButton";
import { useCentralSkillsFacets } from "@/pages/centralSkillsFacets";
import {
  addUniqueToCentralViewState,
  useCentralSavedViewsBridge,
} from "@/pages/centralSavedViewsBridge";
import { useCentralTagGroupsBridge } from "@/pages/centralTagGroupsBridge";
import { useCentralPaletteActions } from "@/pages/centralPaletteActions";
import {
  useCentralSkillsDerivedData,
  useCentralSkillsStoreBindings,
  type CentralCategorizeTab,
  type CentralSortDirection,
  type CentralSortField,
} from "@/pages/centralSkillsViewModel";
import { usePlatformStore } from "@/stores/platformStore";
import { useCentralInstalledSkillsFilterBridge } from "@/pages/centralInstalledSkillsFilterBridge";
import { useCentralSkillsLayoutSizing } from "@/pages/centralSkillsLayoutSizing";
import {
  createCentralStoreLocationControls,
  useCentralStoreLocationApplied,
} from "@/pages/centralStoreLocationView";
import { useCentralUpdateCheckModeController } from "@/pages/centralUpdateCheckModeController";

export function CentralSkillsView() {
  const { t } = useTranslation();
  const {
    skills,
    agents,
    repositories,
    tags,
    aiTagReviews,
    aiTagJob,
    updateStatuses,
    updateJob,
    portabilityJob,
    aiTaggingAvailable,
    isRemoteTarget,
    centralSkillsDir,
    isLoading,
    loadCentralSkills,
    subscribeAiTagProgress,
    subscribeUpdateProgress,
    subscribePortabilityProgress,
    cancelSkillportStatePortability,
    isMetadataUpdating,
    isSuggestingTags,
    isCheckingUpdates,
    updatingSkillIds,
    isInstalling,
    isDeleting,
    togglingAgentId,
    refreshCounts,
    availableInstallAgents,
    githubImport,
    resetGitHubImport,
    exportSkillportState,
    previewSkillportStateImport,
    importSkillportState,
    setRepositoryPinned,
    previewCentralStoreLocationChange,
    applyCentralStoreLocationChange,
  } = useCentralSkillsStoreBindings(t);

  const [selectedSkillIds, setSelectedSkillIds] = useState<string[]>([]);
  const [categorizeTab, setCategorizeTab] =
    useState<CentralCategorizeTab>("manual");
  const [manualTagQuery, setManualTagQuery] = useState("");
  const [manualSelectedTagIds, setManualSelectedTagIds] = useState<string[]>(
    [],
  );
  const [installTargetSkill, setInstallTargetSkill] =
    useState<SkillWithLinks | null>(null);
  const [deleteTargetSkill, setDeleteTargetSkill] =
    useState<SkillWithLinks | null>(null);
  const [deletePreview, setDeletePreview] = useState<SkillDetail | null>(null);
  const [batchDeletePreview, setBatchDeletePreview] =
    useState<BatchDeleteCentralSkillPreviewResult | null>(null);
  const [pendingUpdateStates, setPendingUpdateStates] = useState<
    CentralSkillUpdateState[]
  >([]);
  const [queuedRemoteMissingStates, setQueuedRemoteMissingStates] = useState<
    CentralSkillUpdateState[]
  >([]);
  const [remoteMissingStates, setRemoteMissingStates] = useState<
    CentralSkillUpdateState[]
  >([]);
  const [remoteMissingPreview, setRemoteMissingPreview] =
    useState<BatchDeleteCentralSkillPreviewResult | null>(null);
  const [repositorySyncPreview, setRepositorySyncPreview] =
    useState<CentralRepositorySyncPreview | null>(null);
  const [queuedRepositorySyncPreview, setQueuedRepositorySyncPreview] =
    useState<CentralRepositorySyncPreview | null>(null);
  const [repositorySyncDeletePreview, setRepositorySyncDeletePreview] =
    useState<BatchDeleteCentralSkillPreviewResult | null>(null);
  const [repositoryDeleteTarget, setRepositoryDeleteTarget] =
    useState<SkillRepositoryWithStats | null>(null);
  const [repositoryDeletePreview, setRepositoryDeletePreview] =
    useState<DeleteSkillRepositoryPreview | null>(null);
  const {
    filterSidebarWidth,
    handleFilterSidebarResizeKeyDown,
    startFilterSidebarResize,
  } = useCentralSkillsLayoutSizing();
  const [isDeletePreviewLoading, setIsDeletePreviewLoading] = useState(false);
  const [isBatchDeletePreviewLoading, setIsBatchDeletePreviewLoading] =
    useState(false);
  const [isRemoteMissingPreviewLoading, setIsRemoteMissingPreviewLoading] =
    useState(false);
  const [isRepositorySyncPreviewLoading, setIsRepositorySyncPreviewLoading] =
    useState(false);
  const [isResolvingRemoteMissing, setIsResolvingRemoteMissing] =
    useState(false);
  const [isApplyingRepositorySync, setIsApplyingRepositorySync] =
    useState(false);
  const [
    isRepositoryDeletePreviewLoading,
    setIsRepositoryDeletePreviewLoading,
  ] = useState(false);
  const [deletePreviewError, setDeletePreviewError] = useState<string | null>(
    null,
  );
  const [batchDeletePreviewError, setBatchDeletePreviewError] = useState<
    string | null
  >(null);
  const [remoteMissingError, setRemoteMissingError] = useState<string | null>(
    null,
  );
  const [repositorySyncError, setRepositorySyncError] = useState<string | null>(
    null,
  );
  const [repositoryDeletePreviewError, setRepositoryDeletePreviewError] =
    useState<string | null>(null);
  const [isDialogOpen, setIsDialogOpen] = useState(false);
  const [isDeleteDialogOpen, setIsDeleteDialogOpen] = useState(false);
  const [isBatchInstallDialogOpen, setIsBatchInstallDialogOpen] =
    useState(false);
  const [isBatchDeleteDialogOpen, setIsBatchDeleteDialogOpen] = useState(false);
  const [isUpdateConfirmDialogOpen, setIsUpdateConfirmDialogOpen] =
    useState(false);
  const [isRemoteMissingDialogOpen, setIsRemoteMissingDialogOpen] =
    useState(false);
  const [isRepositorySyncDialogOpen, setIsRepositorySyncDialogOpen] =
    useState(false);
  const [isRepositoryDeleteDialogOpen, setIsRepositoryDeleteDialogOpen] =
    useState(false);
  const [isStoreLocationDialogOpen, setIsStoreLocationDialogOpen] =
    useState(false);
  const [drawerSkillId, setDrawerSkillId] = useState<string | null>(null);
  const [isDrawerOpen, setIsDrawerOpen] = useState(false);
  const [isGitHubImportOpen, setIsGitHubImportOpen] = useState(false);
  const [isPlatformManageOpen, setIsPlatformManageOpen] = useState(false);
  const [isPortabilityOpen, setIsPortabilityOpen] = useState(false);
  const [isCategorizeDrawerOpen, setIsCategorizeDrawerOpen] = useState(false);
  const [isTaskCenterOpen, setIsTaskCenterOpen] = useState(false);
  const [githubRepoUrl, setGitHubRepoUrl] = useState("");
  const contentRef = useRef<HTMLDivElement | null>(null);
  const detailButtonRefs = useRef<Record<string, HTMLButtonElement | null>>({});
  const hasMarkedCentralListReady = useRef(false);
  const categoryVisibility =
    usePlatformStore((state) => state.categoryVisibility) ??
    DEFAULT_PLATFORM_CATEGORY_VISIBILITY;
  const setCategoryVisibility = usePlatformStore(
    (state) => state.setCategoryVisibility,
  );
  const setAgentEnabled = usePlatformStore((state) => state.setAgentEnabled);
  const addCustomAgent = usePlatformStore((state) => state.addCustomAgent);
  const updateCustomAgent = usePlatformStore(
    (state) => state.updateCustomAgent,
  );
  const removeCustomAgent = usePlatformStore(
    (state) => state.removeCustomAgent,
  );

  const [viewState, setViewState] = useCentralViewStateUrl();
  const v2 = useCentralSkillsFacets({
    skills,
    repositories,
    tags,
    aiTagReviews,
    updateStatuses,
    state: viewState,
  });
  const {
    canCreateManualTag,
    filteredManualTags,
    selectedSkillIdSet,
    updateAvailableSkillIds,
    updateTargetSkillIds,
  } = useCentralSkillsDerivedData({
    skills,
    tags,
    updateStatuses,
    selectedSkillIds,
    manualTagQuery,
  });

  // ─── 旧动作模块仍接收 setRepositoryFilter（StateSetter<string>），这里包装成对 viewState.repos 的转换。
  const setRepositoryFilter = useCallback<Dispatch<SetStateAction<string>>>(
    (value) => {
      setViewState((prev) => {
        const currentRepo = prev.repos[0] ?? "all";
        const nextValue =
          typeof value === "function" ? value(currentRepo) : value;
        return {
          ...prev,
          repos: nextValue === "all" ? [] : [nextValue],
        };
      });
    },
    [setViewState],
  );

  const sortFieldOptions: Array<{ value: CentralSortField; label: string }> =
    useMemo(
      () => [
        { value: "name", label: t("central.sortByName") },
        { value: "createdAt", label: t("central.sortByCreatedAt") },
        { value: "updatedAt", label: t("central.sortByUpdatedAt") },
        {
          value: "installedPlatformCount",
          label: t("central.sortByInstalledPlatformCount"),
        },
      ],
      [t],
    );

  const sortDirectionOptions: Array<{
    value: CentralSortDirection;
    label: string;
  }> = useMemo(
    () => [
      { value: "asc", label: t("central.sortAscending") },
      { value: "desc", label: t("central.sortDescending") },
    ],
    [t],
  );

  const {
    installedSkillsFilterProps,
    isInstalledSkillsFilterActive,
    visibleCurrentViewSkills,
    visibleFilteredSkills,
  } = useCentralInstalledSkillsFilterBridge({
    availableInstallAgents,
    currentViewSkills: v2.sortedSkills,
    filteredSkills: v2.filteredSkills,
    selectedSkillIds,
    setIsBatchInstallDialogOpen,
    setSelectedSkillIds,
  });

  useEffect(() => {
    const visibleIds = new Set(
      visibleCurrentViewSkills.map((skill) => skill.id),
    );
    setSelectedSkillIds((current) => {
      const next = current.filter((skillId) => visibleIds.has(skillId));
      return next.length === current.length ? current : next;
    });
  }, [visibleCurrentViewSkills]);

  // ─── Saved Views ────────────────────────────────────────────
  const savedViewsBridge = useCentralSavedViewsBridge({
    enabled: true,
    v2ViewState: viewState,
    setV2ViewState: setViewState,
    t,
  });

  // ─── Tag Groups ─────────────────────────────────────────────
  const tagGroupsBridge = useCentralTagGroupsBridge({ enabled: true, t });

  // ─── Command palette state + actions ───────────────────────
  const [commandPaletteOpen, setCommandPaletteOpen] = useState(false);
  const hasCurrentFilters =
    viewState.repos.length > 0 ||
    viewState.tags.length > 0 ||
    v2.isSearchActive ||
    isInstalledSkillsFilterActive;
  const hasNonRepositoryFilters =
    viewState.tags.length > 0 ||
    v2.isSearchActive ||
    isInstalledSkillsFilterActive;
  const checkButtonState = getCentralSkillsCheckButtonState({
    currentViewSkills: visibleCurrentViewSkills,
    hasCurrentFilters,
    hasNonRepositoryFilters,
    repositories,
    selectedSkillIds,
    selectedRepoIds: viewState.repos,
    sortedSkills: visibleCurrentViewSkills,
    t,
    totalSkillCount: skills.length,
  });

  // Load central skills on mount.
  useEffect(() => {
    loadCentralSkills();
  }, [loadCentralSkills]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;
    subscribeAiTagProgress().then((unsubscribe) => {
      if (disposed) {
        unsubscribe();
        return;
      }
      unlisten = unsubscribe;
    });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [subscribeAiTagProgress]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;
    subscribeUpdateProgress().then((unsubscribe) => {
      if (disposed) {
        unsubscribe();
        return;
      }
      unlisten = unsubscribe;
    });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [subscribeUpdateProgress]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;
    subscribePortabilityProgress().then((unsubscribe) => {
      if (disposed) {
        unsubscribe();
        return;
      }
      unlisten = unsubscribe;
    });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [subscribePortabilityProgress]);

  useEffect(() => {
    if (!v2.isSearchActive || !contentRef.current) return;
    contentRef.current.scrollTop = 0;
  }, [v2.isSearchActive, viewState.q]);

  useEffect(() => {
    if (!isLoading && skills.length > 0 && !hasMarkedCentralListReady.current) {
      hasMarkedCentralListReady.current = true;
      markAppPerformance("central_list_ready");
    }
  }, [isLoading, skills.length]);

  const { actions: paletteActions, groupByOptions } = useCentralPaletteActions({
    t,
    viewState,
    setViewState,
    canSaveCurrent: savedViewsBridge.canSaveCurrent,
    onSaveCurrentView: savedViewsBridge.handleSaveCurrentView,
    onCreateTagGroup: tagGroupsBridge.handleCreateTagGroup,
  });

  const installableImportedSkills = useMemo(() => {
    if (!githubImport.importResult) return [];
    const importedIds = new Set(
      githubImport.importResult.importedSkills.map(
        (skill) => skill.importedSkillId,
      ),
    );
    return skills.filter((skill) => importedIds.has(skill.id));
  }, [githubImport.importResult, skills]);

  const {
    handleAcceptReview,
    handleAfterImportSuccess,
    handleApplyManualTags,
    handleApplyManualTagsToReview,
    handleBatchDeleteCentralSkills,
    handleBatchDeleteClick,
    handleBatchDeleteDialogOpenChange,
    handleBatchInstallCentralSkills,
    handleBulkSuggestTags,
    handleCancelAiTagJob,
    handleCancelCentralUpdates,
    handleRepositorySyncDialogOpenChange,
    handleApplyRepositorySync,
    handleConfirmUpdateSkills,
    handleCreateManualTag,
    handleDeleteCentralSkill,
    handleDeleteClick,
    handleDeleteDialogOpenChange,
    handleDeleteSkillRepository,
    handleGitHubImport,
    handleGitHubPreview,
    handleInstall,
    handleInstallClick,
    handleInstallImportedSkill,
    handleOpenDrawer,
    handleRemoteMissingDialogOpenChange,
    handleUpdateConfirmDialogOpenChange,
    handleRepositoryDeleteClick,
    handleRepositoryDeleteDialogOpenChange,
    handleResolveRemoteMissing,
    handleSkipReview,
    handleToggleManualTag,
    handleTogglePlatform,
    handleToggleSelection,
    handleUpdateSkills,
    setDetailButtonRef,
  } = useCentralSkillsActions({
    detailButtonRefs,
    t,
    state: {
      deleteTargetSkill,
      githubRepoUrl,
      manualSelectedTagIds,
      manualTagQuery,
      repositoryDeleteTarget,
      repositoryFilter: viewState.repos[0] ?? "all",
      queuedRemoteMissingStates,
      queuedRepositorySyncPreview,
      selectedSkillIds,
      currentViewSkills: visibleCurrentViewSkills,
    },
    setters: {
      setBatchDeletePreview,
      setBatchDeletePreviewError,
      setDeletePreview,
      setDeletePreviewError,
      setDeleteTargetSkill,
      setDrawerSkillId,
      setInstallTargetSkill,
      setIsBatchDeleteDialogOpen,
      setIsBatchDeletePreviewLoading,
      setIsDeleteDialogOpen,
      setIsDeletePreviewLoading,
      setIsDialogOpen,
      setIsDrawerOpen,
      setIsRemoteMissingDialogOpen,
      setIsRemoteMissingPreviewLoading,
      setIsRepositorySyncDialogOpen,
      setIsRepositorySyncPreviewLoading,
      setIsApplyingRepositorySync,
      setIsRepositoryDeleteDialogOpen,
      setIsRepositoryDeletePreviewLoading,
      setIsResolvingRemoteMissing,
      setIsUpdateConfirmDialogOpen,
      setManualSelectedTagIds,
      setManualTagQuery,
      setPendingUpdateStates,
      setQueuedRemoteMissingStates,
      setQueuedRepositorySyncPreview,
      setRemoteMissingError,
      setRemoteMissingPreview,
      setRemoteMissingStates,
      setRepositorySyncDeletePreview,
      setRepositorySyncError,
      setRepositorySyncPreview,
      setRepositoryDeletePreview,
      setRepositoryDeletePreviewError,
      setRepositoryDeleteTarget,
      setRepositoryFilter,
      setSelectedSkillIds,
    },
  });

  const handleToggleSelectionPreservingScroll = useCallback(
    (skillId: string) => {
      const scrollContainer = contentRef.current;
      const scrollTop = scrollContainer?.scrollTop;

      handleToggleSelection(skillId);

      if (scrollTop === undefined) return;

      window.requestAnimationFrame(() => {
        if (contentRef.current) {
          contentRef.current.scrollTop = scrollTop;
        }
      });
    },
    [handleToggleSelection],
  );

  // ─── 共享 props ────────────────────────────────────────────────
  const dialogsProps = {
    agents,
    availableInstallAgents,
    batchDeletePreview,
    batchDeletePreviewError,
    deletePreview,
    deletePreviewError,
    deleteTargetSkill,
    detailButtonRefs,
    drawerSkillId,
    githubImport,
    githubRepoUrl,
    importSkillportState,
    installTargetSkill,
    installableImportedSkills,
    isBatchDeleteDialogOpen,
    isBatchDeletePreviewLoading,
    isBatchInstallDialogOpen,
    isDeleteDialogOpen,
    isDeletePreviewLoading,
    isDeleting,
    isDialogOpen,
    isDrawerOpen,
    isGitHubImportOpen,
    isPlatformManageOpen,
    isInstalling,
    isPortabilityOpen,
    isRemoteMissingDialogOpen,
    isRemoteMissingPreviewLoading,
    isRepositorySyncDialogOpen,
    isRepositorySyncPreviewLoading,
    isApplyingRepositorySync,
    isRepositoryDeleteDialogOpen,
    isRepositoryDeletePreviewLoading,
    isResolvingRemoteMissing,
    isUpdatingSkills: updatingSkillIds.length > 0,
    isUpdateConfirmDialogOpen,
    loadCentralSkills,
    pendingUpdateStates,
    previewSkillportStateImport,
    remoteMissingError,
    remoteMissingPreview,
    remoteMissingStates,
    repositorySyncError,
    repositorySyncPreview,
    repositorySyncDeletePreview,
    repositoryDeletePreview,
    repositoryDeletePreviewError,
    repositoryDeleteTarget,
    selectedSkillIds,
    skills,
    setDrawerSkillId,
    setGithubRepoUrl: setGitHubRepoUrl,
    setIsBatchInstallDialogOpen,
    setIsDialogOpen,
    setIsDrawerOpen,
    setIsGitHubImportOpen,
    setIsPlatformManageOpen,
    setIsPortabilityOpen,
    exportSkillportState,
    portabilityJob,
    cancelSkillportStatePortability,
    platformManagement: {
      agents,
      categoryVisibility,
      addCustomAgent,
      updateCustomAgent,
      removeCustomAgent,
      setCategoryVisibility,
      setAgentEnabled,
      refreshAfterPlatformChange: async () => {
        await loadCentralSkills();
        await refreshCounts();
      },
    },
    onAfterImportSuccess: handleAfterImportSuccess,
    onBatchDeleteCentralSkills: handleBatchDeleteCentralSkills,
    onBatchDeleteDialogOpenChange: handleBatchDeleteDialogOpenChange,
    onBatchInstallCentralSkills: handleBatchInstallCentralSkills,
    onDeleteCentralSkill: handleDeleteCentralSkill,
    onDeleteDialogOpenChange: handleDeleteDialogOpenChange,
    onDeleteSkillRepository: handleDeleteSkillRepository,
    onGitHubImport: handleGitHubImport,
    onGitHubPreview: handleGitHubPreview,
    onInstall: handleInstall,
    onInstallFromDrawer: handleInstallClick,
    onInstallImportedSkill: handleInstallImportedSkill,
    onRefreshCounts: refreshCounts,
    onConfirmUpdateSkills: handleConfirmUpdateSkills,
    onRemoteMissingDialogOpenChange: handleRemoteMissingDialogOpenChange,
    onRepositorySyncDialogOpenChange: handleRepositorySyncDialogOpenChange,
    onApplyRepositorySync: handleApplyRepositorySync,
    onUpdateConfirmDialogOpenChange: handleUpdateConfirmDialogOpenChange,
    onRepositoryDeleteDialogOpenChange: handleRepositoryDeleteDialogOpenChange,
    onResetGitHubImport: resetGitHubImport,
    onResolveRemoteMissing: handleResolveRemoteMissing,
  };

  const listContentProps = {
    availableInstallAgents,
    contentRef,
    filteredSkills: visibleFilteredSkills,
    isLoading,
    isSearchActive: v2.isSearchActive || isInstalledSkillsFilterActive,
    viewMode: viewState.view,
    viewDensity: viewState.density,
    onDelete: (skill: SkillWithLinks) => {
      void handleDeleteClick(skill);
    },
    onDetail: handleOpenDrawer,
    onInstallTo: handleInstallClick,
    onTogglePlatform: handleTogglePlatform,
    onToggleSelection: handleToggleSelectionPreservingScroll,
    onUpdateCentral: (skillIds: string[]) => {
      void handleUpdateSkills(skillIds);
    },
    searchQuery: viewState.q,
    selectedSkillIdSet,
    setDetailButtonRef,
    skillsCount: skills.length,
    sortedSkills: visibleCurrentViewSkills,
    togglingAgentId: togglingAgentId ?? null,
    updateStatuses,
    updatingSkillIds,
  };

  const updateButtonProps = {
    disabled: updateTargetSkillIds.length === 0 || updatingSkillIds.length > 0,
    label:
      selectedSkillIds.length > 0
        ? t("central.updateSelected", { count: updateTargetSkillIds.length })
        : t("central.updateAvailable", { count: updateTargetSkillIds.length }),
    targetSkillIds: updateTargetSkillIds,
  };

  const handleViewAiReviews = useCallback(() => {
    setIsTaskCenterOpen(false);
    setCategorizeTab("review");
    setIsCategorizeDrawerOpen(true);
  }, []);

  const taskCenterProps = {
    open: isTaskCenterOpen,
    onOpenChange: setIsTaskCenterOpen,
    aiTagJob,
    updateJob,
    portabilityJob,
    onCancelAiTag: () => {
      void handleCancelAiTagJob();
    },
    onCancelUpdate: () => {
      void handleCancelCentralUpdates();
    },
    onCancelPortability: () => {
      void cancelSkillportStatePortability();
    },
    onViewAiReviews: handleViewAiReviews,
  };

  const handleOpenCategorizeDrawer = useCallback(
    (tab: CentralCategorizeTab) => {
      setCategorizeTab(tab);
      setIsCategorizeDrawerOpen(true);
    },
    [],
  );

  const handleClearSelection = useCallback(() => setSelectedSkillIds([]), []);

  const handleSelectCurrentResults = useCallback(() => {
    setSelectedSkillIds(visibleCurrentViewSkills.map((skill) => skill.id));
  }, [visibleCurrentViewSkills]);

  const categorizePanelProps = {
    aiTagJob,
    aiTagReviews,
    aiTaggingAvailable,
    canCreateManualTag,
    categorizeTab,
    filteredManualTags,
    isMetadataUpdating,
    isSuggestingTags,
    manualSelectedTagIds,
    manualTagQuery,
    selectedSkillCount: selectedSkillIds.length,
    sortedSkillCount: visibleCurrentViewSkills.length,
    onAcceptReview: (review: Parameters<typeof handleAcceptReview>[0]) => {
      void handleAcceptReview(review);
    },
    onApplyManualTags: () => {
      void handleApplyManualTags();
    },
    onApplyManualTagsToReview: (
      review: Parameters<typeof handleApplyManualTagsToReview>[0],
    ) => {
      void handleApplyManualTagsToReview(review);
    },
    onBulkSuggestTags: () => {
      void handleBulkSuggestTags();
    },
    onCreateManualTag: () => {
      void handleCreateManualTag();
    },
    onSetCategorizeTab: setCategorizeTab,
    onSetManualTagQuery: setManualTagQuery,
    onSkipReview: (review: Parameters<typeof handleSkipReview>[0]) => {
      void handleSkipReview(review);
    },
    onToggleManualTag: handleToggleManualTag,
  };

  const bulkBarProps = {
    selectedCount: selectedSkillIds.length,
    isInstalling,
    isDeleting,
    isAiBusy: isSuggestingTags || aiTagJob.status === "running",
    aiTaggingAvailable,
    onBatchInstall: () => setIsBatchInstallDialogOpen(true),
    onBatchDelete: () => {
      void handleBatchDeleteClick();
    },
    onClearSelection: handleClearSelection,
  };

  const selectionControlsProps = {
    selectedCount: selectedSkillIds.length,
    currentResultCount: visibleCurrentViewSkills.length,
    onSelectCurrentResults: handleSelectCurrentResults,
    onClearSelection: handleClearSelection,
  };

  const categorizeDrawerProps = {
    open: isCategorizeDrawerOpen,
    onOpenChange: setIsCategorizeDrawerOpen,
    onOpenManual: () => handleOpenCategorizeDrawer("manual"),
    onOpenAiSuggest: () => handleOpenCategorizeDrawer("ai"),
  };

  const checkButtonProps = {
    disabled:
      isCheckingUpdates ||
      updateJob.status === "running" ||
      updateJob.status === "cancelling",
  };
  const updateCheckMode = useCentralUpdateCheckModeController({
    checkButtonState,
    repositories,
    disabled: checkButtonProps.disabled,
  });

  const sidebarHeaderSlot = (
    <CentralSidebarHeader
      savedViewsBridge={savedViewsBridge}
      tagGroupsBridge={tagGroupsBridge}
    />
  );

  const handleCentralStoreLocationApplied = useCentralStoreLocationApplied({
    loadCentralSkills,
    refreshCounts,
    t,
  });

  return (
    <>
      <CentralSkillsShell
        t={t}
        centralSkillsDir={centralSkillsDir}
        isCheckingUpdates={isCheckingUpdates}
        filterSidebarWidth={filterSidebarWidth}
        startFilterSidebarResize={startFilterSidebarResize}
        handleFilterSidebarResizeKeyDown={handleFilterSidebarResizeKeyDown}
        viewState={viewState}
        setViewState={setViewState}
        queryAst={v2.queryAst}
        facetCounts={v2.facetCounts}
        repositories={repositories}
        tags={tags}
        sortFieldOptions={sortFieldOptions}
        sortDirectionOptions={sortDirectionOptions}
        groupByOptions={groupByOptions}
        installedSkillsFilter={installedSkillsFilterProps}
        listContent={listContentProps}
        categorizePanel={categorizePanelProps}
        bulkBar={bulkBarProps}
        selectionControls={selectionControlsProps}
        categorizeDrawer={categorizeDrawerProps}
        taskCenter={taskCenterProps}
        dialogs={dialogsProps}
        centralStoreLocation={createCentralStoreLocationControls({
          isRemoteTarget,
          setIsStoreLocationDialogOpen,
          t,
        })}
        setIsGitHubImportOpen={setIsGitHubImportOpen}
        setIsPlatformManageOpen={setIsPlatformManageOpen}
        setIsPortabilityOpen={setIsPortabilityOpen}
        onUpdateSkills={(skillIds) => {
          void handleUpdateSkills(skillIds);
        }}
        updateAvailableSkillCount={updateAvailableSkillIds.length}
        updateButton={updateButtonProps}
        checkButton={updateCheckMode.checkButton}
        onOpenPalette={() => setCommandPaletteOpen(true)}
        savedViewsSlot={sidebarHeaderSlot}
        tagGroups={tagGroupsBridge.tagGroups}
        onAssignTagToGroup={tagGroupsBridge.handleAssignTagToGroup}
        onDeleteRepository={(repo) => {
          void handleRepositoryDeleteClick(repo);
        }}
        onToggleRepositoryPin={(repo) => {
          void setRepositoryPinned(repo.id, !repo.pinned);
        }}
      />
      <CommandPalette
        open={commandPaletteOpen}
        onOpenChange={setCommandPaletteOpen}
        savedViews={savedViewsBridge.savedViews}
        tags={tags}
        repositories={repositories}
        actions={paletteActions}
        onSelectSavedView={savedViewsBridge.handleApplySavedView}
        onSelectTag={(tag) =>
          addUniqueToCentralViewState(viewState, setViewState, "tags", tag.id)
        }
        onSelectRepository={(repo) =>
          addUniqueToCentralViewState(viewState, setViewState, "repos", repo.id)
        }
      />
      <CentralStoreLocationDialog
        open={isStoreLocationDialogOpen}
        onOpenChange={setIsStoreLocationDialogOpen}
        t={t}
        currentPath={centralSkillsDir}
        preview={previewCentralStoreLocationChange}
        apply={applyCentralStoreLocationChange}
        onApplied={handleCentralStoreLocationApplied}
      />
      {updateCheckMode.dialog}
    </>
  );
}
