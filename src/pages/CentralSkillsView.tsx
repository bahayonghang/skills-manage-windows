import { useDeferredValue, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";

import { BatchDeleteCentralSkillPreviewResult, CentralSkillUpdateState, DeleteSkillRepositoryPreview, SkillDetail, SkillRepositoryWithStats, SkillWithLinks } from "@/types";
import { markAppPerformance } from "@/lib/performance";
import { useResizableWidth } from "@/hooks/useResizableWidth";
import { DEFAULT_PLATFORM_CATEGORY_VISIBILITY } from "@/lib/platformVisibility";
import { CentralSkillsShell } from "@/components/central/CentralSkillsShell";
import { useCentralSkillsActions } from "@/pages/centralSkillsActions";
import {
  useCentralSkillsDerivedData,
  useCentralSkillsStoreBindings,
  type CentralCategorizeTab,
  type CentralSortDirection,
  type CentralSortField,
} from "@/pages/centralSkillsViewModel";
import { usePlatformStore } from "@/stores/platformStore";

const CENTRAL_FILTER_DEFAULT_WIDTH = 286;
const CENTRAL_FILTER_MIN_WIDTH = 220;
const CENTRAL_FILTER_MAX_WIDTH = 460;
const CENTRAL_CATEGORIZE_DEFAULT_WIDTH = 392;
const CENTRAL_CATEGORIZE_MIN_WIDTH = 336;
const CENTRAL_CATEGORIZE_MAX_WIDTH = 640;

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
  const [remoteMissingStates, setRemoteMissingStates] = useState<CentralSkillUpdateState[]>([]);
  const [remoteMissingPreview, setRemoteMissingPreview] =
    useState<BatchDeleteCentralSkillPreviewResult | null>(null);
  const [repositoryDeleteTarget, setRepositoryDeleteTarget] =
    useState<SkillRepositoryWithStats | null>(null);
  const [repositoryDeletePreview, setRepositoryDeletePreview] =
    useState<DeleteSkillRepositoryPreview | null>(null);
  const {
    width: filterSidebarWidth,
    startResize: startFilterSidebarResize,
    handleResizeKeyDown: handleFilterSidebarResizeKeyDown,
  } = useResizableWidth({
    defaultWidth: CENTRAL_FILTER_DEFAULT_WIDTH,
    minWidth: CENTRAL_FILTER_MIN_WIDTH,
    maxWidth: CENTRAL_FILTER_MAX_WIDTH,
  });
  const {
    width: categorizeSidebarWidth,
    startResize: startCategorizeSidebarResize,
    handleResizeKeyDown: handleCategorizeSidebarResizeKeyDown,
  } = useResizableWidth({
    defaultWidth: CENTRAL_CATEGORIZE_DEFAULT_WIDTH,
    minWidth: CENTRAL_CATEGORIZE_MIN_WIDTH,
    maxWidth: CENTRAL_CATEGORIZE_MAX_WIDTH,
    resizeFrom: "left",
  });
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
  const repositoryFilterName = useMemo(() => {
    if (repositoryFilter === "all") return null;
    return repositories.find((repo) => repo.id === repositoryFilter)?.name ?? null;
  }, [repositoryFilter, repositories]);
  const repositorySkillIds = useMemo(() => {
    if (repositoryFilter === "all") return [];
    return sortedSkills
      .filter((skill) => (skill.repository?.id ?? null) === repositoryFilter)
      .map((skill) => skill.id);
  }, [repositoryFilter, sortedSkills]);
  const checkButtonScope: "selected" | "repository" | "all" =
    selectedSkillIds.length > 0
      ? "selected"
      : repositoryFilter !== "all"
        ? "repository"
        : "all";
  const checkButtonTargetSkillIds =
    checkButtonScope === "selected"
      ? selectedSkillIds
      : checkButtonScope === "repository"
        ? repositorySkillIds
        : sortedSkills.map((skill) => skill.id);
  const checkButtonScopedSkillIds: string[] | undefined =
    checkButtonScope === "all" ? undefined : checkButtonTargetSkillIds;
  const checkButtonLabel =
    checkButtonScope === "selected"
      ? t("central.checkUpdatesSelected", { count: checkButtonTargetSkillIds.length })
      : checkButtonScope === "repository"
        ? t("central.checkUpdatesRepository", {
            repo: repositoryFilterName ?? "",
            count: checkButtonTargetSkillIds.length,
          })
        : t("central.checkUpdatesAll", { count: skills.length });
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
      selectedSkillIds,
      sortedSkills,
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
      setManualSelectedTagIds,
      setManualTagQuery,
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

  return (
    <CentralSkillsShell
      centralSkillsDir={centralSkillsDir}
      dialogs={{
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
        loadCentralSkills,
        previewSkillportStateImport,
        remoteMissingError,
        remoteMissingPreview,
        remoteMissingStates,
        repositoryDeletePreview,
        repositoryDeletePreviewError,
        repositoryDeleteTarget,
        selectedSkillIds,
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
        onRemoteMissingDialogOpenChange: handleRemoteMissingDialogOpenChange,
        onRepositoryDeleteDialogOpenChange: handleRepositoryDeleteDialogOpenChange,
        onResetGitHubImport: resetGitHubImport,
        onResolveRemoteMissing: handleResolveRemoteMissing,
      }}
      filterSidebar={{
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
        onRepositoryDelete: (repository) => {
          void handleRepositoryDeleteClick(repository);
        },
      }}
      tagSearch={{
        setCategorizeTab,
        tagCounts,
        uncategorizedCount,
        aiReviewCount: aiTagReviews.length,
        totalSkillCount: skills.length,
      }}
      isCheckingUpdates={isCheckingUpdates}
      isLoading={isLoading}
      listContent={{
        availableInstallAgents,
        contentRef,
        filteredSkills,
        isLoading,
        isSearchActive,
        onDelete: (skill) => {
          void handleDeleteClick(skill);
        },
        onDetail: handleOpenDrawer,
        onInstallTo: handleInstallClick,
        onTogglePlatform: handleTogglePlatform,
        onToggleSelection: handleToggleSelection,
        onUpdateCentral: (skillIds) => {
          void handleUpdateSkills(skillIds);
        },
        searchQuery,
        selectedSkillIdSet,
        setDetailButtonRef,
        skillsCount: skills.length,
        sortedSkills,
        togglingAgentId: togglingAgentId ?? null,
        updateStatuses,
        updatingSkillIds,
      }}
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
      updateButton={{
        disabled: updateTargetSkillIds.length === 0 || updatingSkillIds.length > 0,
        label:
          selectedSkillIds.length > 0
            ? t("central.updateSelected", { count: updateTargetSkillIds.length })
            : t("central.updateAvailable", { count: updateTargetSkillIds.length }),
        targetSkillIds: updateTargetSkillIds,
      }}
      aiProgress={{
        aiTagJob,
        onCancel: () => {
          void handleCancelAiTagJob();
        },
        onViewReviews: () => {
          setCategorizeTab("review");
          setTagFilter("ai-review");
        },
      }}
      categorizePanel={{
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
        sortedSkillCount: sortedSkills.length,
        startCategorizeSidebarResize,
        handleCategorizeSidebarResizeKeyDown,
        onAcceptReview: (review) => {
          void handleAcceptReview(review);
        },
        onApplyManualTags: () => {
          void handleApplyManualTags();
        },
        onApplyManualTagsToReview: (review) => {
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
        onSkipReview: (review) => {
          void handleSkipReview(review);
        },
        onToggleManualTag: handleToggleManualTag,
      }}
      updateProgress={{
        isDismissible: isUpdateProgressDismissible,
        updateJob,
        updateProgressKey,
        updateProgressRatio,
        onCancel: () => {
          void handleCancelCentralUpdates();
        },
        onDismiss: setDismissedUpdateProgressKey,
      }}
      checkButton={{
        label: checkButtonLabel,
        disabled:
          isCheckingUpdates ||
          updateJob.status === "running" ||
          updateJob.status === "cancelling" ||
          checkButtonTargetSkillIds.length === 0,
        onClick: () => {
          void handleCheckUpdates(checkButtonScopedSkillIds);
        },
      }}
      onRefresh={() => {
        void handleRefresh();
      }}
      onUpdateSkills={(skillIds) => {
        void handleUpdateSkills(skillIds);
      }}
    />
  );
}
