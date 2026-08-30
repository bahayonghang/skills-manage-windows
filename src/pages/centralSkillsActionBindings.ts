import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { ComponentProps, Dispatch, SetStateAction } from "react";
import type { TFunction } from "i18next";

import type { CentralSkillListContent } from "@/components/central/CentralSkillListContent";
import { useCentralBatchUninstallView } from "@/pages/centralBatchUninstallView";
import { useCentralSkillsActions } from "@/pages/centralSkillsActions";
import { DEFAULT_PLATFORM_CATEGORY_VISIBILITY } from "@/lib/platformVisibility";
import type { CentralViewState } from "@/lib/centralViewState";
import { useImportIntentBindings } from "@/stores/importIntentStore";
import { usePlatformStore } from "@/stores/platformStore";
import type {
  AgentWithStatus,
  BatchDeleteCentralSkillPreviewResult,
  CentralSkillUpdateState,
  DeleteCentralSkillPreview,
  DeleteSkillRepositoryPreview,
  SkillRepositoryWithStats,
  SkillWithLinks,
} from "@/types";
import type { CentralRepositorySyncPreview } from "@/types/centralRepositorySync";

type StateSetter<T> = Dispatch<SetStateAction<T>>;
type SetCentralViewState = (
  next: CentralViewState | ((prev: CentralViewState) => CentralViewState),
) => void;

type CentralSkillsListBindings = Pick<
  ComponentProps<typeof CentralSkillListContent>,
  | "availableInstallAgents"
  | "filteredSkills"
  | "isLoading"
  | "isSearchActive"
  | "searchQuery"
  | "selectedSkillIdSet"
  | "tags"
  | "togglingAgentId"
  | "updateStatuses"
  | "updatingSkillIds"
  | "viewDensity"
  | "viewMode"
>;

export function useCentralSkillsActionState() {
  const [selectedSkillIds, setSelectedSkillIds] = useState<string[]>([]);
  const [manualTagQuery, setManualTagQuery] = useState("");
  const [manualSelectedTagIds, setManualSelectedTagIds] = useState<string[]>(
    [],
  );
  const [installTargetSkill, setInstallTargetSkill] =
    useState<SkillWithLinks | null>(null);
  const [deleteTargetSkill, setDeleteTargetSkill] =
    useState<SkillWithLinks | null>(null);
  const [deletePreview, setDeletePreview] = useState<DeleteCentralSkillPreview | null>(null);
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
  const [isRepositoryDeletePreviewLoading, setIsRepositoryDeletePreviewLoading] =
    useState(false);
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
    useState<string | null
  >(null);
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
  const [drawerSkillId, setDrawerSkillId] = useState<string | null>(null);
  const [isDrawerOpen, setIsDrawerOpen] = useState(false);
  const {
    githubBranch,
    githubRepoUrl,
    isGitHubImportOpen,
    setGithubBranch,
    setGithubRepoUrl,
    setIsGitHubImportOpen,
  } = useImportIntentBindings();
  const [isPlatformManageOpen, setIsPlatformManageOpen] = useState(false);
  const [isPortabilityOpen, setIsPortabilityOpen] = useState(false);
  const contentRef = useRef<HTMLDivElement | null>(null);
  const detailButtonRefs = useRef<Record<string, HTMLButtonElement | null>>({});
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

  return {
    addCustomAgent,
    batchDeletePreview,
    batchDeletePreviewError,
    categoryVisibility,
    contentRef,
    deletePreview,
    deletePreviewError,
    deleteTargetSkill,
    detailButtonRefs,
    drawerSkillId,
    githubBranch,
    githubRepoUrl,
    installTargetSkill,
    isApplyingRepositorySync,
    isBatchDeleteDialogOpen,
    isBatchDeletePreviewLoading,
    isBatchInstallDialogOpen,
    isDeleteDialogOpen,
    isDeletePreviewLoading,
    isDialogOpen,
    isDrawerOpen,
    isGitHubImportOpen,
    isPlatformManageOpen,
    isPortabilityOpen,
    isRemoteMissingDialogOpen,
    isRemoteMissingPreviewLoading,
    isRepositoryDeleteDialogOpen,
    isRepositoryDeletePreviewLoading,
    isRepositorySyncDialogOpen,
    isRepositorySyncPreviewLoading,
    isResolvingRemoteMissing,
    isUpdateConfirmDialogOpen,
    manualSelectedTagIds,
    manualTagQuery,
    pendingUpdateStates,
    queuedRemoteMissingStates,
    queuedRepositorySyncPreview,
    remoteMissingError,
    remoteMissingPreview,
    remoteMissingStates,
    removeCustomAgent,
    repositoryDeletePreview,
    repositoryDeletePreviewError,
    repositoryDeleteTarget,
    repositorySyncDeletePreview,
    repositorySyncError,
    repositorySyncPreview,
    selectedSkillIds,
    setAgentEnabled,
    setBatchDeletePreview,
    setBatchDeletePreviewError,
    setDeletePreview,
    setDeletePreviewError,
    setDeleteTargetSkill,
    setDrawerSkillId,
    setGithubBranch,
    setGithubRepoUrl,
    setInstallTargetSkill,
    setIsApplyingRepositorySync,
    setIsBatchDeleteDialogOpen,
    setIsBatchDeletePreviewLoading,
    setIsBatchInstallDialogOpen,
    setIsDeleteDialogOpen,
    setIsDeletePreviewLoading,
    setIsDialogOpen,
    setIsDrawerOpen,
    setIsGitHubImportOpen,
    setIsPlatformManageOpen,
    setIsPortabilityOpen,
    setIsRemoteMissingDialogOpen,
    setIsRemoteMissingPreviewLoading,
    setIsRepositoryDeleteDialogOpen,
    setIsRepositoryDeletePreviewLoading,
    setIsRepositorySyncDialogOpen,
    setIsRepositorySyncPreviewLoading,
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
    setRepositoryDeletePreview,
    setRepositoryDeletePreviewError,
    setRepositoryDeleteTarget,
    setRepositorySyncDeletePreview,
    setRepositorySyncError,
    setRepositorySyncPreview,
    setSelectedSkillIds,
    setCategoryVisibility,
    updateCustomAgent,
  };
}

type CentralSkillsActionState = ReturnType<typeof useCentralSkillsActionState>;

interface CentralSkillsActionBindingsProps {
  actionState: CentralSkillsActionState;
  currentViewSkills: SkillWithLinks[];
  githubImportResult: {
    importedSkills: Array<{ importedSkillId: string }>;
  } | null;
  list: CentralSkillsListBindings;
  platform: {
    agents: AgentWithStatus[];
    loadCentralSkills: () => Promise<void>;
    refreshCounts: () => Promise<void>;
  };
  repositoryFilter: string;
  setViewState: SetCentralViewState;
  skills: SkillWithLinks[];
  t: TFunction;
  updateTargetSkillIds: string[];
  updatingSkillIds: string[];
}

export function useCentralSkillsActionBindings({
  actionState,
  currentViewSkills,
  githubImportResult,
  list,
  platform,
  repositoryFilter,
  setViewState,
  skills,
  t,
  updateTargetSkillIds,
  updatingSkillIds,
}: CentralSkillsActionBindingsProps) {
  const { setSelectedSkillIds } = actionState;
  const setRepositoryFilter = useCallback<StateSetter<string>>(
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

  useEffect(() => {
    const visibleIds = new Set(currentViewSkills.map((skill) => skill.id));
    setSelectedSkillIds((current) => {
      const next = current.filter((skillId) => visibleIds.has(skillId));
      return next.length === current.length ? current : next;
    });
  }, [currentViewSkills, setSelectedSkillIds]);

  const actions = useCentralSkillsActions({
    detailButtonRefs: actionState.detailButtonRefs,
    t,
    state: {
      deleteTargetSkill: actionState.deleteTargetSkill,
      githubRepoUrl: actionState.githubRepoUrl,
      manualSelectedTagIds: actionState.manualSelectedTagIds,
      manualTagQuery: actionState.manualTagQuery,
      repositoryDeleteTarget: actionState.repositoryDeleteTarget,
      repositoryFilter,
      queuedRemoteMissingStates: actionState.queuedRemoteMissingStates,
      queuedRepositorySyncPreview: actionState.queuedRepositorySyncPreview,
      selectedSkillIds: actionState.selectedSkillIds,
      currentViewSkills,
    },
    setters: {
      setBatchDeletePreview: actionState.setBatchDeletePreview,
      setBatchDeletePreviewError: actionState.setBatchDeletePreviewError,
      setDeletePreview: actionState.setDeletePreview,
      setDeletePreviewError: actionState.setDeletePreviewError,
      setDeleteTargetSkill: actionState.setDeleteTargetSkill,
      setDrawerSkillId: actionState.setDrawerSkillId,
      setInstallTargetSkill: actionState.setInstallTargetSkill,
      setIsBatchDeleteDialogOpen: actionState.setIsBatchDeleteDialogOpen,
      setIsBatchDeletePreviewLoading:
        actionState.setIsBatchDeletePreviewLoading,
      setIsDeleteDialogOpen: actionState.setIsDeleteDialogOpen,
      setIsDeletePreviewLoading: actionState.setIsDeletePreviewLoading,
      setIsDialogOpen: actionState.setIsDialogOpen,
      setIsDrawerOpen: actionState.setIsDrawerOpen,
      setIsRemoteMissingDialogOpen: actionState.setIsRemoteMissingDialogOpen,
      setIsRemoteMissingPreviewLoading:
        actionState.setIsRemoteMissingPreviewLoading,
      setIsRepositorySyncDialogOpen:
        actionState.setIsRepositorySyncDialogOpen,
      setIsRepositorySyncPreviewLoading:
        actionState.setIsRepositorySyncPreviewLoading,
      setIsApplyingRepositorySync: actionState.setIsApplyingRepositorySync,
      setIsRepositoryDeleteDialogOpen:
        actionState.setIsRepositoryDeleteDialogOpen,
      setIsRepositoryDeletePreviewLoading:
        actionState.setIsRepositoryDeletePreviewLoading,
      setIsResolvingRemoteMissing: actionState.setIsResolvingRemoteMissing,
      setIsUpdateConfirmDialogOpen:
        actionState.setIsUpdateConfirmDialogOpen,
      setManualSelectedTagIds: actionState.setManualSelectedTagIds,
      setManualTagQuery: actionState.setManualTagQuery,
      setPendingUpdateStates: actionState.setPendingUpdateStates,
      setQueuedRemoteMissingStates:
        actionState.setQueuedRemoteMissingStates,
      setQueuedRepositorySyncPreview:
        actionState.setQueuedRepositorySyncPreview,
      setRemoteMissingError: actionState.setRemoteMissingError,
      setRemoteMissingPreview: actionState.setRemoteMissingPreview,
      setRemoteMissingStates: actionState.setRemoteMissingStates,
      setRepositorySyncDeletePreview:
        actionState.setRepositorySyncDeletePreview,
      setRepositorySyncError: actionState.setRepositorySyncError,
      setRepositorySyncPreview: actionState.setRepositorySyncPreview,
      setRepositoryDeletePreview: actionState.setRepositoryDeletePreview,
      setRepositoryDeletePreviewError:
        actionState.setRepositoryDeletePreviewError,
      setRepositoryDeleteTarget: actionState.setRepositoryDeleteTarget,
      setRepositoryFilter,
      setSelectedSkillIds: actionState.setSelectedSkillIds,
    },
  });
  const { handleBatchUninstallCentralSkills, handleToggleSelection } = actions;

  const handleToggleSelectionPreservingScroll = useCallback(
    (skillId: string) => {
      const scrollContainer = actionState.contentRef.current;
      const scrollTop = scrollContainer?.scrollTop;

      handleToggleSelection(skillId);

      if (scrollTop === undefined) return;

      window.requestAnimationFrame(() => {
        if (actionState.contentRef.current) {
          actionState.contentRef.current.scrollTop = scrollTop;
        }
      });
    },
    [actionState.contentRef, handleToggleSelection],
  );

  const batchUninstall = useCentralBatchUninstallView({
    selectedSkillIds: actionState.selectedSkillIds,
    skills,
    onConfirm: handleBatchUninstallCentralSkills,
  });

  const installableImportedSkills = useMemo(() => {
    if (!githubImportResult) return [];
    const importedIds = new Set(
      githubImportResult.importedSkills.map(
        (skill) => skill.importedSkillId,
      ),
    );
    return skills.filter((skill) => importedIds.has(skill.id));
  }, [githubImportResult, skills]);

  const listContentProps = {
    ...list,
    contentRef: actionState.contentRef,
    onDelete: (skill: SkillWithLinks) => {
      void actions.handleDeleteClick(skill);
    },
    onDetail: actions.handleOpenDrawer,
    onInstallTo: actions.handleInstallClick,
    onUninstallFromPlatforms: (skill: SkillWithLinks) =>
      batchUninstall.openForSkill(skill.id),
    onTogglePlatform: actions.handleTogglePlatform,
    onToggleSelection: handleToggleSelectionPreservingScroll,
    onUpdateCentral: (skillIds: string[]) => {
      void actions.handleUpdateSkills(skillIds);
    },
    onAddSkillTag: (skillId: string, tagId: string) => {
      void actions.handleAddSkillTag(skillId, tagId);
    },
    onCreateSkillTag: (skillId: string, name: string) => {
      void actions.handleCreateSkillTag(skillId, name);
    },
    onRemoveSkillTag: (skillId: string, tagId: string) => {
      void actions.handleRemoveSkillTag(skillId, tagId);
    },
    setDetailButtonRef: actions.setDetailButtonRef,
    skillsCount: skills.length,
    sortedSkills: currentViewSkills,
  };

  const platformManagement = {
    agents: platform.agents,
    categoryVisibility: actionState.categoryVisibility,
    addCustomAgent: actionState.addCustomAgent,
    updateCustomAgent: actionState.updateCustomAgent,
    removeCustomAgent: actionState.removeCustomAgent,
    setCategoryVisibility: actionState.setCategoryVisibility,
    setAgentEnabled: actionState.setAgentEnabled,
    refreshAfterPlatformChange: async () => {
      await platform.loadCentralSkills();
      await platform.refreshCounts();
    },
  };

  const updateButtonProps = {
    disabled: updateTargetSkillIds.length === 0 || updatingSkillIds.length > 0,
    label:
      actionState.selectedSkillIds.length > 0
        ? t("central.updateSelected", { count: updateTargetSkillIds.length })
        : t("central.updateAvailable", { count: updateTargetSkillIds.length }),
    targetSkillIds: updateTargetSkillIds,
  };

  return {
    ...actionState,
    ...actions,
    batchUninstall,
    handleToggleSelectionPreservingScroll,
    installableImportedSkills,
    listContentProps,
    platformManagement,
    updateButtonProps,
  };
}
