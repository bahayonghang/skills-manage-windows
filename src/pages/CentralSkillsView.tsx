import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { markAppPerformance } from "@/lib/performance";
import { CentralSkillsShell } from "@/components/central/CentralSkillsShell";
import { CentralStoreLocationDialog } from "@/components/central/CentralStoreLocationDialog";
import { CommandPalette } from "@/components/central/CommandPalette";
import { useCentralViewStateUrl } from "@/hooks/useCentralViewStateUrl";
import {
  useCentralSkillsActionBindings,
  useCentralSkillsActionState,
} from "@/pages/centralSkillsActionBindings";
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
import { useCentralInstalledSkillsFilterBridge } from "@/pages/centralInstalledSkillsFilterBridge";
import { useCentralSkillsLayoutSizing } from "@/pages/centralSkillsLayoutSizing";
import { useCentralSkillsViewChrome } from "@/pages/centralSkillsViewChrome";
import {
  createCentralStoreLocationControls,
  useCentralStoreLocationApplied,
} from "@/pages/centralStoreLocationView";
import { useCentralUpdateCheckModeController } from "@/pages/centralUpdateCheckModeController";
import { useCentralAiTagDashboardView } from "@/pages/centralAiTagDashboardView";

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
    portabilityJob, activeTarget,
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
    exportSkillportState, saveSkillportStateExport,
    previewSkillportStateImport, previewSkillportStateImportFile,
    importSkillportState,
    setRepositoryPinned,
    previewCentralStoreLocationChange,
    applyCentralStoreLocationChange,
  } = useCentralSkillsStoreBindings(t);
  const actionState = useCentralSkillsActionState();
  const [categorizeTab, setCategorizeTab] = useState<CentralCategorizeTab>("manual");
  const {
    filterSidebarWidth,
    handleFilterSidebarResizeKeyDown,
    startFilterSidebarResize,
  } = useCentralSkillsLayoutSizing();
  const [isStoreLocationDialogOpen, setIsStoreLocationDialogOpen] =
    useState(false);
  const [isCategorizeDrawerOpen, setIsCategorizeDrawerOpen] = useState(false);
  const [isTaskCenterOpen, setIsTaskCenterOpen] = useState(false);
  const hasMarkedCentralListReady = useRef(false);

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
    selectedSkillIds: actionState.selectedSkillIds,
    manualTagQuery: actionState.manualTagQuery,
  });

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
    selectedSkillIds: actionState.selectedSkillIds,
    setIsBatchInstallDialogOpen: actionState.setIsBatchInstallDialogOpen,
    setSelectedSkillIds: actionState.setSelectedSkillIds,
  });

  const savedViewsBridge = useCentralSavedViewsBridge({
    enabled: true,
    v2ViewState: viewState,
    setV2ViewState: setViewState,
    t,
  });

  const tagGroupsBridge = useCentralTagGroupsBridge({ enabled: true, t });

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
    selectedSkillIds: actionState.selectedSkillIds,
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
    if (!v2.isSearchActive || !actionState.contentRef.current) return;
    actionState.contentRef.current.scrollTop = 0;
  }, [v2.isSearchActive, viewState.q, actionState.contentRef]);

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
  const actionBindings = useCentralSkillsActionBindings({
    actionState,
    currentViewSkills: visibleCurrentViewSkills,
    githubImportResult: githubImport.importResult,
    list: {
      availableInstallAgents,
      filteredSkills: visibleFilteredSkills,
      isLoading,
      isSearchActive: v2.isSearchActive || isInstalledSkillsFilterActive,
      searchQuery: viewState.q,
      selectedSkillIdSet,
      tags,
      togglingAgentId: togglingAgentId ?? null,
      updateStatuses,
      updatingSkillIds,
      viewDensity: viewState.density,
      viewMode: viewState.view,
    },
    platform: { agents, loadCentralSkills, refreshCounts },
    repositoryFilter: viewState.repos[0] ?? "all",
    setViewState,
    skills,
    t,
    updateTargetSkillIds,
    updatingSkillIds,
  });

  // ─── 共享 props ────────────────────────────────────────────────
  const dialogsProps = {
    agents, activeTarget,
    availableInstallAgents,
    batchDeletePreview: actionBindings.batchDeletePreview,
    batchDeletePreviewError: actionBindings.batchDeletePreviewError,
    deletePreview: actionBindings.deletePreview,
    deletePreviewError: actionBindings.deletePreviewError,
    deleteTargetSkill: actionBindings.deleteTargetSkill,
    detailButtonRefs: actionBindings.detailButtonRefs,
    drawerSkillId: actionBindings.drawerSkillId,
    githubImport,
    githubRepoUrl: actionBindings.githubRepoUrl,
    importSkillportState,
    installTargetSkill: actionBindings.installTargetSkill,
    installableImportedSkills: actionBindings.installableImportedSkills,
    isBatchDeleteDialogOpen: actionBindings.isBatchDeleteDialogOpen,
    isBatchDeletePreviewLoading: actionBindings.isBatchDeletePreviewLoading,
    isBatchInstallDialogOpen: actionBindings.isBatchInstallDialogOpen,
    isDeleteDialogOpen: actionBindings.isDeleteDialogOpen,
    isDeletePreviewLoading: actionBindings.isDeletePreviewLoading,
    isDeleting,
    isDialogOpen: actionBindings.isDialogOpen,
    isDrawerOpen: actionBindings.isDrawerOpen,
    isGitHubImportOpen: actionBindings.isGitHubImportOpen,
    isPlatformManageOpen: actionBindings.isPlatformManageOpen,
    isInstalling,
    isPortabilityOpen: actionBindings.isPortabilityOpen,
    isRemoteMissingDialogOpen: actionBindings.isRemoteMissingDialogOpen,
    isRemoteMissingPreviewLoading:
      actionBindings.isRemoteMissingPreviewLoading,
    isRepositorySyncDialogOpen: actionBindings.isRepositorySyncDialogOpen,
    isRepositorySyncPreviewLoading:
      actionBindings.isRepositorySyncPreviewLoading,
    isApplyingRepositorySync: actionBindings.isApplyingRepositorySync,
    isRepositoryDeleteDialogOpen:
      actionBindings.isRepositoryDeleteDialogOpen,
    isRepositoryDeletePreviewLoading:
      actionBindings.isRepositoryDeletePreviewLoading,
    isResolvingRemoteMissing: actionBindings.isResolvingRemoteMissing,
    isUpdatingSkills: updatingSkillIds.length > 0,
    isUpdateConfirmDialogOpen: actionBindings.isUpdateConfirmDialogOpen,
    loadCentralSkills,
    pendingUpdateStates: actionBindings.pendingUpdateStates,
    previewSkillportStateImport, previewSkillportStateImportFile,
    remoteMissingError: actionBindings.remoteMissingError,
    remoteMissingPreview: actionBindings.remoteMissingPreview,
    remoteMissingStates: actionBindings.remoteMissingStates,
    batchUninstall: actionBindings.batchUninstall.dialog,
    repositorySyncError: actionBindings.repositorySyncError,
    repositorySyncPreview: actionBindings.repositorySyncPreview,
    repositorySyncDeletePreview: actionBindings.repositorySyncDeletePreview,
    repositoryDeletePreview: actionBindings.repositoryDeletePreview,
    repositoryDeletePreviewError: actionBindings.repositoryDeletePreviewError,
    repositoryDeleteTarget: actionBindings.repositoryDeleteTarget,
    selectedSkillIds: actionBindings.selectedSkillIds,
    skills,
    setDrawerSkillId: actionBindings.setDrawerSkillId,
    setGithubRepoUrl: actionBindings.setGithubRepoUrl,
    setIsBatchInstallDialogOpen: actionBindings.setIsBatchInstallDialogOpen,
    setIsDialogOpen: actionBindings.setIsDialogOpen,
    setIsDrawerOpen: actionBindings.setIsDrawerOpen,
    setIsGitHubImportOpen: actionBindings.setIsGitHubImportOpen,
    setIsPlatformManageOpen: actionBindings.setIsPlatformManageOpen,
    setIsPortabilityOpen: actionBindings.setIsPortabilityOpen,
    exportSkillportState, saveSkillportStateExport,
    portabilityJob,
    cancelSkillportStatePortability,
    platformManagement: actionBindings.platformManagement,
    onAfterImportSuccess: actionBindings.handleAfterImportSuccess,
    onBatchDeleteCentralSkills: actionBindings.handleBatchDeleteCentralSkills,
    onBatchDeleteDialogOpenChange:
      actionBindings.handleBatchDeleteDialogOpenChange,
    onBatchInstallCentralSkills:
      actionBindings.handleBatchInstallCentralSkills,
    onDeleteCentralSkill: actionBindings.handleDeleteCentralSkill,
    onDeleteDialogOpenChange: actionBindings.handleDeleteDialogOpenChange,
    onDeleteSkillRepository: actionBindings.handleDeleteSkillRepository,
    onGitHubImport: actionBindings.handleGitHubImport,
    onGitHubPreview: actionBindings.handleGitHubPreview,
    onInstall: actionBindings.handleInstall,
    onInstallFromDrawer: actionBindings.handleInstallClick,
    onInstallImportedSkill: actionBindings.handleInstallImportedSkill,
    onRefreshCounts: refreshCounts,
    onConfirmUpdateSkills: actionBindings.handleConfirmUpdateSkills,
    onRemoteMissingDialogOpenChange:
      actionBindings.handleRemoteMissingDialogOpenChange,
    onRepositorySyncDialogOpenChange:
      actionBindings.handleRepositorySyncDialogOpenChange,
    onApplyRepositorySync: actionBindings.handleApplyRepositorySync,
    onUpdateConfirmDialogOpenChange:
      actionBindings.handleUpdateConfirmDialogOpenChange,
    onRepositoryDeleteDialogOpenChange:
      actionBindings.handleRepositoryDeleteDialogOpenChange,
    onResetGitHubImport: resetGitHubImport,
    onResolveRemoteMissing: actionBindings.handleResolveRemoteMissing,
  };

  const { aiTagProgressItems, aiTagRateProfile } =
    useCentralAiTagDashboardView({ aiTagJob, skills });

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
      void actionBindings.handleCancelAiTagJob();
    },
    onCancelUpdate: () => {
      void actionBindings.handleCancelCentralUpdates();
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

  const categorizePanelProps = {
    aiTagJob,
    aiTagProgressItems,
    aiTagRateProfile,
    aiTagReviews,
    aiTaggingAvailable,
    canCreateManualTag,
    categorizeTab,
    filteredManualTags,
    isMetadataUpdating,
    isSuggestingTags,
    manualSelectedTagIds: actionBindings.manualSelectedTagIds,
    manualTagQuery: actionBindings.manualTagQuery,
    selectedSkillCount: actionBindings.selectedSkillIds.length,
    sortedSkillCount: visibleCurrentViewSkills.length,
    onAcceptReview: (
      review: Parameters<typeof actionBindings.handleAcceptReview>[0],
    ) => {
      void actionBindings.handleAcceptReview(review);
    },
    onApplyManualTags: () => {
      void actionBindings.handleApplyManualTags();
    },
    onApplyManualTagsToReview: (
      review: Parameters<typeof actionBindings.handleApplyManualTagsToReview>[0],
    ) => {
      void actionBindings.handleApplyManualTagsToReview(review);
    },
    onBulkSuggestTags: () => {
      void actionBindings.handleBulkSuggestTags();
    },
    onCancelAiTag: () => {
      void actionBindings.handleCancelAiTagJob();
    },
    onCreateManualTag: () => {
      void actionBindings.handleCreateManualTag();
    },
    onSetCategorizeTab: setCategorizeTab,
    onSetManualTagQuery: actionBindings.setManualTagQuery,
    onSkipReview: (
      review: Parameters<typeof actionBindings.handleSkipReview>[0],
    ) => {
      void actionBindings.handleSkipReview(review);
    },
    onToggleManualTag: actionBindings.handleToggleManualTag,
  };

  const {
    savedViewsSlot,
    tagGroupsSlot,
    repoUpdateCounts,
    selectionControlsProps,
    handleClearSelection,
  } = useCentralSkillsViewChrome({
    savedViewsBridge,
    tagGroupsBridge,
    skills,
    updateStatuses,
    selectedSkillIds: actionBindings.selectedSkillIds,
    setSelectedSkillIds: actionBindings.setSelectedSkillIds,
    visibleCurrentViewSkills,
  });

  const bulkBarProps = {
    selectedCount: actionBindings.selectedSkillIds.length,
    isInstalling,
    ...actionBindings.batchUninstall.bulkBar,
    isDeleting,
    isAiBusy: isSuggestingTags || aiTagJob.status === "running",
    aiTaggingAvailable,
    onBatchInstall: () => actionBindings.setIsBatchInstallDialogOpen(true),
    onBatchDelete: () => {
      void actionBindings.handleBatchDeleteClick();
    },
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
        repoUpdateCounts={repoUpdateCounts}
        tags={tags}
        sortFieldOptions={sortFieldOptions}
        sortDirectionOptions={sortDirectionOptions}
        groupByOptions={groupByOptions}
        installedSkillsFilter={installedSkillsFilterProps}
        listContent={actionBindings.listContentProps}
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
        setIsGitHubImportOpen={actionBindings.setIsGitHubImportOpen}
        setIsPlatformManageOpen={actionBindings.setIsPlatformManageOpen}
        setIsPortabilityOpen={actionBindings.setIsPortabilityOpen}
        onUpdateSkills={(skillIds) => {
          void actionBindings.handleUpdateSkills(skillIds);
        }}
        updateAvailableSkillCount={updateAvailableSkillIds.length}
        updateButton={actionBindings.updateButtonProps}
        checkModeControl={updateCheckMode.modeControl}
        checkButton={updateCheckMode.checkButton}
        onOpenPalette={() => setCommandPaletteOpen(true)}
        savedViewsSlot={savedViewsSlot}
        topFiltersTagGroups={tagGroupsSlot}
        onDeleteRepository={(repo) => {
          void actionBindings.handleRepositoryDeleteClick(repo);
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
        tagCounts={v2.facetCounts.tags}
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
