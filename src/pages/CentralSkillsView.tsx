import { useDeferredValue, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { BatchDeleteCentralSkillPreviewResult, CentralSkillUpdateState, DeleteSkillRepositoryPreview, SkillDetail, SkillRepositoryWithStats, SkillWithLinks } from "@/types";
import { markAppPerformance } from "@/lib/performance";
import { DEFAULT_PLATFORM_CATEGORY_VISIBILITY } from "@/lib/platformVisibility";
import { CentralSkillsShell } from "@/components/central/CentralSkillsShell";
import { CentralSidebarV2Header } from "@/components/central/v2/CentralSidebarV2Header";
import { CentralSkillsShellV2 } from "@/components/central/v2/CentralSkillsShellV2";
import { CommandPaletteV2 } from "@/components/central/v2/CommandPaletteV2";
import { useFeatureFlag } from "@/lib/featureFlags";
import { useCentralViewStateUrl } from "@/hooks/useCentralViewStateUrl";
import { useCentralSkillsActions } from "@/pages/centralSkillsActions";
import { getCentralSkillsCheckButtonState } from "@/pages/centralSkillsCheckButton";
import { useCentralSkillsViewModelV2 } from "@/pages/centralSkillsViewModelV2";
import { addUniqueToCentralViewState, useCentralV2SavedViewsBridge } from "@/pages/centralV2SavedViewsBridge";
import { useCentralV2TagGroupsBridge } from "@/pages/centralV2TagGroupsBridge";
import { useCentralV2PaletteActions } from "@/pages/centralV2PaletteActions";
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
  } = useCentralSkillsStoreBindings(t);

  const [sortField, setSortField] = useState<CentralSortField>("name");
  const [sortDirection, setSortDirection] = useState<CentralSortDirection>("asc");
  const [searchQuery, setSearchQuery] = useState("");
  const [repositoryFilter, setRepositoryFilter] = useState("all");
  const [tagFilter, setTagFilter] = useState("all");
  const [selectedSkillIds, setSelectedSkillIds] = useState<string[]>([]);
  const [categorizeTab, setCategorizeTab] = useState<CentralCategorizeTab>("manual");
  const [manualTagQuery, setManualTagQuery] = useState("");
  const [manualSelectedTagIds, setManualSelectedTagIds] = useState<string[]>([]);
  const [installTargetSkill, setInstallTargetSkill] =
    useState<SkillWithLinks | null>(null);
  const [deleteTargetSkill, setDeleteTargetSkill] =
    useState<SkillWithLinks | null>(null);
  const [deletePreview, setDeletePreview] = useState<SkillDetail | null>(null);
  const [batchDeletePreview, setBatchDeletePreview] =
    useState<BatchDeleteCentralSkillPreviewResult | null>(null);
  const [pendingUpdateStates, setPendingUpdateStates] = useState<CentralSkillUpdateState[]>([]);
  const [queuedRemoteMissingStates, setQueuedRemoteMissingStates] =
    useState<CentralSkillUpdateState[]>([]);
  const [remoteMissingStates, setRemoteMissingStates] = useState<CentralSkillUpdateState[]>([]);
  const [remoteMissingPreview, setRemoteMissingPreview] =
    useState<BatchDeleteCentralSkillPreviewResult | null>(null);
  const [repositoryDeleteTarget, setRepositoryDeleteTarget] =
    useState<SkillRepositoryWithStats | null>(null);
  const [repositoryDeletePreview, setRepositoryDeletePreview] =
    useState<DeleteSkillRepositoryPreview | null>(null);
  const {
    categorizeSidebarWidth,
    filterSidebarWidth,
    handleCategorizeSidebarResizeKeyDown,
    handleFilterSidebarResizeKeyDown,
    startCategorizeSidebarResize,
    startFilterSidebarResize,
  } = useCentralSkillsLayoutSizing();
  const [isDeletePreviewLoading, setIsDeletePreviewLoading] = useState(false);
  const [isBatchDeletePreviewLoading, setIsBatchDeletePreviewLoading] = useState(false);
  const [isRemoteMissingPreviewLoading, setIsRemoteMissingPreviewLoading] = useState(false);
  const [isResolvingRemoteMissing, setIsResolvingRemoteMissing] = useState(false);
  const [isRepositoryDeletePreviewLoading, setIsRepositoryDeletePreviewLoading] = useState(false);
  const [deletePreviewError, setDeletePreviewError] = useState<string | null>(null);
  const [batchDeletePreviewError, setBatchDeletePreviewError] = useState<string | null>(null);
  const [remoteMissingError, setRemoteMissingError] = useState<string | null>(null);
  const [repositoryDeletePreviewError, setRepositoryDeletePreviewError] = useState<string | null>(null);
  const [isDialogOpen, setIsDialogOpen] = useState(false);
  const [isDeleteDialogOpen, setIsDeleteDialogOpen] = useState(false);
  const [isBatchInstallDialogOpen, setIsBatchInstallDialogOpen] = useState(false);
  const [isBatchDeleteDialogOpen, setIsBatchDeleteDialogOpen] = useState(false);
  const [isUpdateConfirmDialogOpen, setIsUpdateConfirmDialogOpen] = useState(false);
  const [isRemoteMissingDialogOpen, setIsRemoteMissingDialogOpen] = useState(false);
  const [isRepositoryDeleteDialogOpen, setIsRepositoryDeleteDialogOpen] = useState(false);
  const [drawerSkillId, setDrawerSkillId] = useState<string | null>(null);
  const [isDrawerOpen, setIsDrawerOpen] = useState(false);
  const [isGitHubImportOpen, setIsGitHubImportOpen] = useState(false);
  const [isPlatformManageOpen, setIsPlatformManageOpen] = useState(false);
  const [isPortabilityOpen, setIsPortabilityOpen] = useState(false);
  const [dismissedUpdateProgressKey, setDismissedUpdateProgressKey] = useState<string | null>(null);
  const [githubRepoUrl, setGitHubRepoUrl] = useState("");
  const contentRef = useRef<HTMLDivElement | null>(null);
  const detailButtonRefs = useRef<Record<string, HTMLButtonElement | null>>({});
  const hasMarkedCentralListReady = useRef(false);
  const deferredSearchQuery = useDeferredValue(searchQuery);
  const categoryVisibility =
    usePlatformStore((state) => state.categoryVisibility) ?? DEFAULT_PLATFORM_CATEGORY_VISIBILITY;
  const setCategoryVisibility = usePlatformStore((state) => state.setCategoryVisibility);
  const setAgentEnabled = usePlatformStore((state) => state.setAgentEnabled);
  const addCustomAgent = usePlatformStore((state) => state.addCustomAgent);
  const updateCustomAgent = usePlatformStore((state) => state.updateCustomAgent);
  const removeCustomAgent = usePlatformStore((state) => state.removeCustomAgent);
  const effectiveSearchQuery =
    skills.length > 80 ? deferredSearchQuery : searchQuery;
  const {
    canCreateManualTag,
    filteredManualTags,
    filteredSkills,
    isSearchActive,
    normalizedSearchQuery,
    selectedSkillIdSet,
    skillIdsKey,
    sortedSkills,
    tagCounts,
    uncategorizedCount,
    updateAvailableSkillIds,
    updateTargetSkillIds,
  } = useCentralSkillsDerivedData({
    skills,
    tags,
    aiTagReviews,
    updateStatuses,
    selectedSkillIds,
    searchQuery,
    effectiveSearchQuery,
    manualTagQuery,
    repositoryFilter,
    tagFilter,
    sortField,
    sortDirection,
  });

  // ─── Central V2 (M1) ───────────────────────────────────────────────
  // 新布局通过 feature flag 切换。V2 view-model 与 V1 并存，state 各自维护，
  // 保证 V1 行为完全不受影响。
  const v2EnabledFromFlag = useFeatureFlag("central.newLayout");
  const [v2OverrideClassic, setV2OverrideClassic] = useState(false);
  const v2Enabled = v2EnabledFromFlag && !v2OverrideClassic;
  const [v2ViewState, setV2ViewState] = useCentralViewStateUrl({ disabled: !v2Enabled });
  const v2 = useCentralSkillsViewModelV2({
    skills,
    repositories,
    tags,
    aiTagReviews,
    updateStatuses,
    state: v2ViewState,
  });
  const currentViewSkills = v2Enabled ? v2.sortedSkills : sortedSkills;
  const {
    installedSkillsFilterProps,
    isInstalledSkillsFilterActive,
    visibleCurrentViewSkills,
    visibleFilteredSkills,
    visibleV2FilteredSkills,
  } = useCentralInstalledSkillsFilterBridge({
    availableInstallAgents,
    currentViewSkills,
    filteredSkills,
    selectedSkillIds,
    setIsBatchInstallDialogOpen,
    setSelectedSkillIds,
    v2FilteredSkills: v2.filteredSkills,
  });

  // ─── Saved Views (M2) ────────────────────────────────────────
  const savedViewsBridge = useCentralV2SavedViewsBridge({
    enabled: v2Enabled,
    v2ViewState,
    setV2ViewState,
    t,
  });

  // ─── Tag Groups (M3) ─────────────────────────────────────────
  const tagGroupsBridge = useCentralV2TagGroupsBridge({ enabled: v2Enabled, t });

  // ─── Command palette state (M2) + actions (M6) ───────────────────────
  const [commandPaletteOpen, setCommandPaletteOpen] = useState(false);
  const updateProgressKey = useMemo(
    () =>
      [
        updateJob.phase,
        updateJob.status,
        updateJob.total,
        updateJob.completed,
        updateJob.succeeded,
        updateJob.skipped,
        updateJob.failed,
        updateJob.error ?? "",
      ].join(":"),
    [
      updateJob.phase,
      updateJob.status,
      updateJob.total,
      updateJob.completed,
      updateJob.succeeded,
      updateJob.skipped,
      updateJob.failed,
      updateJob.error,
    ]
  );
  const isUpdateProgressDismissible =
    updateJob.status === "completed" ||
    updateJob.status === "failed" ||
    updateJob.status === "cancelled";
  const checkButtonState = getCentralSkillsCheckButtonState({
    currentViewSkills: visibleCurrentViewSkills,
    repositories,
    repositoryFilter,
    selectedSkillIds,
    sortedSkills: visibleCurrentViewSkills,
    t,
    totalSkillCount: skills.length,
    v2Enabled,
    v2HasCurrentFilters:
      v2ViewState.repos.length > 0 ||
      v2ViewState.tags.length > 0 ||
      v2.isSearchActive ||
      isInstalledSkillsFilterActive,
  });
  const shouldShowUpdateProgress =
    updateJob.status !== "idle" &&
    (!isUpdateProgressDismissible || dismissedUpdateProgressKey !== updateProgressKey);
  const updateProgressRatio =
    updateJob.total > 0 ? Math.min(1, updateJob.completed / updateJob.total) : 0;
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
    if (!isSearchActive || !contentRef.current) return;
    contentRef.current.scrollTop = 0;
  }, [isSearchActive, normalizedSearchQuery]);

  useEffect(() => {
    if (updateJob.status === "running") {
      setDismissedUpdateProgressKey(null);
    }
  }, [updateJob.phase, updateJob.status]);

  useEffect(() => {
    const visibleIds = new Set(skillIdsKey ? skillIdsKey.split("\0") : []);
    setSelectedSkillIds((current) => {
      const next = current.filter((skillId) => visibleIds.has(skillId));
      return next.length === current.length ? current : next;
    });
  }, [skillIdsKey]);

  useEffect(() => {
    if (!isLoading && skills.length > 0 && !hasMarkedCentralListReady.current) {
      hasMarkedCentralListReady.current = true;
      markAppPerformance("central_list_ready");
    }
  }, [isLoading, skills.length]);

  const sortFieldOptions: Array<{ value: CentralSortField; label: string }> = [
    { value: "name", label: t("central.sortByName") },
    { value: "createdAt", label: t("central.sortByCreatedAt") },
    { value: "updatedAt", label: t("central.sortByUpdatedAt") },
  ];

  const sortDirectionOptions: Array<{ value: CentralSortDirection; label: string }> = [
    { value: "asc", label: t("central.sortAscending") },
    { value: "desc", label: t("central.sortDescending") },
  ];

  const { actions: paletteActions, groupByOptions } = useCentralV2PaletteActions({
    t, viewState: v2ViewState, setViewState: setV2ViewState,
    canSaveCurrent: savedViewsBridge.canSaveCurrent,
    onSaveCurrentView: savedViewsBridge.handleSaveCurrentView,
    onCreateTagGroup: tagGroupsBridge.handleCreateTagGroup,
    onSwitchToClassic: () => setV2OverrideClassic(true),
  });

  const installableImportedSkills = useMemo(() => {
    if (!githubImport.importResult) return [];
    const importedIds = new Set(
      githubImport.importResult.importedSkills.map((skill) => skill.importedSkillId)
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
    handleCheckUpdates,
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
    handleRefresh,
    handleRemoteMissingDialogOpenChange,
    handleUpdateConfirmDialogOpenChange,
    handleRepositoryDeleteClick,
    handleRepositoryDeleteDialogOpenChange,
    handleResolveRemoteMissing,
    handleSelectCurrentFilter,
    handleSelectUncategorized,
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
      repositoryFilter,
      queuedRemoteMissingStates,
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
      setIsRepositoryDeleteDialogOpen,
      setIsRepositoryDeletePreviewLoading,
      setIsResolvingRemoteMissing,
      setIsUpdateConfirmDialogOpen,
      setManualSelectedTagIds,
      setManualTagQuery,
      setPendingUpdateStates,
      setQueuedRemoteMissingStates,
      setRemoteMissingError,
      setRemoteMissingPreview,
      setRemoteMissingStates,
      setRepositoryDeletePreview,
      setRepositoryDeletePreviewError,
      setRepositoryDeleteTarget,
      setRepositoryFilter,
      setSelectedSkillIds,
    },
  });

  // ─── 共享 props（V1/V2 Shell 公用），保证两个 Shell 渲染一致的对话框、列表、进度 ───
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
        onInstallImportedSkill: handleInstallImportedSkill,
        onRefreshCounts: refreshCounts,
        onConfirmUpdateSkills: handleConfirmUpdateSkills,
        onRemoteMissingDialogOpenChange: handleRemoteMissingDialogOpenChange,
        onUpdateConfirmDialogOpenChange: handleUpdateConfirmDialogOpenChange,
        onRepositoryDeleteDialogOpenChange: handleRepositoryDeleteDialogOpenChange,
        onResetGitHubImport: resetGitHubImport,
        onResolveRemoteMissing: handleResolveRemoteMissing,
  };

  const filterSidebarProps = {
        filterSidebarWidth,
        isDeleting,
        isRepositoryDeletePreviewLoading,
        repositoryDeleteTargetId: repositoryDeleteTarget?.id,
        repositoryFilter,
        repositories,
        setRepositoryFilter,
        skillsCount: skills.length,
        startFilterSidebarResize,
        handleFilterSidebarResizeKeyDown,
        onRepositoryDelete: (repository: SkillRepositoryWithStats) => {
          void handleRepositoryDeleteClick(repository);
        },
  };

  const tagSearchProps = {
    setCategorizeTab,
    tagCounts,
    uncategorizedCount,
    aiReviewCount: aiTagReviews.length,
    totalSkillCount: skills.length,
  };

  const listContentProps = {
        availableInstallAgents,
        contentRef,
        filteredSkills: visibleFilteredSkills,
        isLoading,
        isSearchActive,
        onDelete: (skill: SkillWithLinks) => {
          void handleDeleteClick(skill);
        },
        onDetail: handleOpenDrawer,
        onInstallTo: handleInstallClick,
        onTogglePlatform: handleTogglePlatform,
        onToggleSelection: handleToggleSelection,
        onUpdateCentral: (skillIds: string[]) => {
          void handleUpdateSkills(skillIds);
        },
        searchQuery,
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

  const aiProgressProps = {
    aiTagJob,
    onCancel: () => {
      void handleCancelAiTagJob();
    },
    onViewReviews: () => {
      setCategorizeTab("review");
      setTagFilter("ai-review");
    },
  };

  const categorizePanelProps = {
    aiTagJob,
    aiTagReviews,
    aiTaggingAvailable,
    canCreateManualTag,
    categorizeSidebarWidth,
    categorizeTab,
    filteredManualTags,
    isDeleting,
    isInstalling,
    isMetadataUpdating,
    isSuggestingTags,
    manualSelectedTagIds,
    manualTagQuery,
    selectedSkillCount: selectedSkillIds.length,
    sortedSkillCount: visibleCurrentViewSkills.length,
    startCategorizeSidebarResize,
    handleCategorizeSidebarResizeKeyDown,
    onAcceptReview: (review: Parameters<typeof handleAcceptReview>[0]) => {
      void handleAcceptReview(review);
    },
    onApplyManualTags: () => {
      void handleApplyManualTags();
    },
    onApplyManualTagsToReview: (review: Parameters<typeof handleApplyManualTagsToReview>[0]) => {
      void handleApplyManualTagsToReview(review);
    },
    onBatchDelete: () => {
      void handleBatchDeleteClick();
    },
    onBatchInstallOpen: () => setIsBatchInstallDialogOpen(true),
    onBulkSuggestTags: () => {
      void handleBulkSuggestTags();
    },
    onClearSelection: () => setSelectedSkillIds([]),
    onCreateManualTag: () => {
      void handleCreateManualTag();
    },
    onSelectCurrentFilter: handleSelectCurrentFilter,
    onSelectUncategorized: handleSelectUncategorized,
    onSetCategorizeTab: setCategorizeTab,
    onSetManualTagQuery: setManualTagQuery,
    onSkipReview: (review: Parameters<typeof handleSkipReview>[0]) => {
      void handleSkipReview(review);
    },
    onToggleManualTag: handleToggleManualTag,
  };

  const updateProgressProps = {
    isDismissible: isUpdateProgressDismissible,
    updateJob,
    updateProgressKey,
    updateProgressRatio,
    onCancel: () => {
      void handleCancelCentralUpdates();
    },
    onDismiss: setDismissedUpdateProgressKey,
  };

  const checkButtonProps = {
    label: checkButtonState.label,
    disabled:
      isCheckingUpdates ||
      updateJob.status === "running" ||
      updateJob.status === "cancelling" ||
      checkButtonState.targetSkillIds.length === 0,
    onClick: () => {
      void handleCheckUpdates(checkButtonState.scopedSkillIds);
    },
  };

  if (v2Enabled) {
    // V2 Shell：listContent 用 V2 view-model 的 sortedSkills/filteredSkills/searchQuery/isSearchActive
    const v2ListContentProps = {
      ...listContentProps,
      filteredSkills: visibleV2FilteredSkills,
      sortedSkills: visibleCurrentViewSkills,
      searchQuery: v2ViewState.q,
      isSearchActive: v2.isSearchActive || isInstalledSkillsFilterActive,
    };
    const sidebarHeaderSlot = (
      <CentralSidebarV2Header
        savedViewsBridge={savedViewsBridge}
        tagGroupsBridge={tagGroupsBridge}
      />
    );
    return (
      <>
        <CentralSkillsShellV2
          t={t}
          centralSkillsDir={centralSkillsDir}
          isLoading={isLoading}
          isCheckingUpdates={isCheckingUpdates}
          filterSidebarWidth={filterSidebarWidth}
          startFilterSidebarResize={startFilterSidebarResize}
          handleFilterSidebarResizeKeyDown={handleFilterSidebarResizeKeyDown}
          viewState={v2ViewState}
          setViewState={setV2ViewState}
          queryAst={v2.queryAst}
          facetCounts={v2.facetCounts}
          repositories={repositories}
          tags={tags}
          sortFieldOptions={sortFieldOptions}
          sortDirectionOptions={sortDirectionOptions}
          groupByOptions={groupByOptions}
          installedSkillsFilter={installedSkillsFilterProps}
          listContent={v2ListContentProps}
          categorizePanel={categorizePanelProps}
          shouldShowCategorizePanel={skills.length > 0}
          aiProgress={aiProgressProps}
          updateProgress={updateProgressProps}
          shouldShowUpdateProgress={shouldShowUpdateProgress}
          dialogs={dialogsProps}
          setIsGitHubImportOpen={setIsGitHubImportOpen}
          setIsPlatformManageOpen={setIsPlatformManageOpen}
          setIsPortabilityOpen={setIsPortabilityOpen}
          onRefresh={() => {
            void handleRefresh();
          }}
          onUpdateSkills={(skillIds) => {
            void handleUpdateSkills(skillIds);
          }}
          updateAvailableSkillCount={updateAvailableSkillIds.length}
          updateButton={updateButtonProps}
          checkButton={checkButtonProps}
          onSwitchToClassic={() => setV2OverrideClassic(true)}
          onOpenPalette={() => setCommandPaletteOpen(true)}
          savedViewsSlot={sidebarHeaderSlot}
          tagGroups={tagGroupsBridge.tagGroups} onAssignTagToGroup={tagGroupsBridge.handleAssignTagToGroup}
        />
        <CommandPaletteV2
          open={commandPaletteOpen}
          onOpenChange={setCommandPaletteOpen}
          savedViews={savedViewsBridge.savedViews}
          tags={tags}
          repositories={repositories}
          actions={paletteActions}
          onSelectSavedView={savedViewsBridge.handleApplySavedView}
          onSelectTag={(tag) => addUniqueToCentralViewState(v2ViewState, setV2ViewState, "tags", tag.id)}
          onSelectRepository={(repo) => addUniqueToCentralViewState(v2ViewState, setV2ViewState, "repos", repo.id)}
        />
      </>
    );
  }

  return (
    <CentralSkillsShell
      centralSkillsDir={centralSkillsDir}
      dialogs={dialogsProps}
      filterSidebar={filterSidebarProps}
      tagSearch={tagSearchProps}
      isCheckingUpdates={isCheckingUpdates}
      isLoading={isLoading}
      listContent={listContentProps}
      installedSkillsFilter={installedSkillsFilterProps}
      searchQuery={searchQuery}
      setIsGitHubImportOpen={setIsGitHubImportOpen}
      setIsPlatformManageOpen={setIsPlatformManageOpen}
      setIsPortabilityOpen={setIsPortabilityOpen}
      setRepositoryFilter={setRepositoryFilter}
      setSearchQuery={setSearchQuery}
      setSortDirection={setSortDirection}
      setSortField={setSortField}
      setTagFilter={setTagFilter}
      shouldShowCategorizePanel={skills.length > 0}
      shouldShowUpdateProgress={shouldShowUpdateProgress}
      sortDirection={sortDirection}
      sortDirectionOptions={sortDirectionOptions}
      sortField={sortField}
      sortFieldOptions={sortFieldOptions}
      tagFilter={tagFilter}
      tags={tags}
      t={t}
      updateAvailableSkillCount={updateAvailableSkillIds.length}
      updateButton={updateButtonProps}
      aiProgress={aiProgressProps}
      categorizePanel={categorizePanelProps}
      updateProgress={updateProgressProps}
      checkButton={checkButtonProps}
      onRefresh={() => {
        void handleRefresh();
      }}
      onUpdateSkills={(skillIds) => {
        void handleUpdateSkills(skillIds);
      }}
      onSwitchToNew={v2EnabledFromFlag ? () => setV2OverrideClassic(false) : undefined}
    />
  );
}
