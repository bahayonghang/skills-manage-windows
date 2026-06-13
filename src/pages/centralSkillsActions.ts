import type { Dispatch, RefObject, SetStateAction } from "react";
import type { TFunction } from "i18next";
import { toast } from "sonner";

import { formatBackendError } from "@/lib/backendError";
import { useCentralSkillsDeleteWorkflow } from "@/pages/centralSkillsDeleteWorkflow";
import { useCentralSkillsImportWorkflow } from "@/pages/centralSkillsImportWorkflow";
import { useCentralSkillsUpdateWorkflow } from "@/pages/centralSkillsUpdateWorkflow";
import { usePlatformStore } from "@/stores/platformStore";
import { useCentralSkillsStore } from "@/stores/centralSkillsStore";
import { useSkillDetailStore } from "@/stores/skillDetailStore";
import { useSkillStore } from "@/stores/skillStore";
import type {
  CentralBatchUninstallApplyResult,
  CentralBatchUninstallPreview,
} from "@/lib/centralBatchUninstall";
import type {
  BatchDeleteCentralSkillPreviewResult,
  BatchInstallResult,
  CentralBatchInstallResult,
  CentralSkillUpdateState,
  DeleteSkillRepositoryPreview,
  SkillAiTagReview,
  SkillDetail,
  SkillRepositoryWithStats,
  SkillWithLinks,
} from "@/types";
import type { CentralRepositorySyncPreview } from "@/types/centralRepositorySync";

type StateSetter<T> = Dispatch<SetStateAction<T>>;

export interface CentralSkillsActionsState {
  deleteTargetSkill: SkillWithLinks | null;
  githubRepoUrl: string;
  manualSelectedTagIds: string[];
  manualTagQuery: string;
  repositoryDeleteTarget: SkillRepositoryWithStats | null;
  repositoryFilter: string;
  queuedRemoteMissingStates: CentralSkillUpdateState[];
  queuedRepositorySyncPreview: CentralRepositorySyncPreview | null;
  selectedSkillIds: string[];
  currentViewSkills: SkillWithLinks[];
}

export interface CentralSkillsActionsSetters {
  setBatchDeletePreview: StateSetter<BatchDeleteCentralSkillPreviewResult | null>;
  setBatchDeletePreviewError: StateSetter<string | null>;
  setDeletePreview: StateSetter<SkillDetail | null>;
  setDeletePreviewError: StateSetter<string | null>;
  setDeleteTargetSkill: StateSetter<SkillWithLinks | null>;
  setDrawerSkillId: StateSetter<string | null>;
  setInstallTargetSkill: StateSetter<SkillWithLinks | null>;
  setIsBatchDeleteDialogOpen: StateSetter<boolean>;
  setIsBatchDeletePreviewLoading: StateSetter<boolean>;
  setIsDeleteDialogOpen: StateSetter<boolean>;
  setIsDeletePreviewLoading: StateSetter<boolean>;
  setIsDialogOpen: StateSetter<boolean>;
  setIsDrawerOpen: StateSetter<boolean>;
  setIsRemoteMissingDialogOpen: StateSetter<boolean>;
  setIsRemoteMissingPreviewLoading: StateSetter<boolean>;
  setIsRepositorySyncDialogOpen: StateSetter<boolean>;
  setIsRepositorySyncPreviewLoading: StateSetter<boolean>;
  setIsApplyingRepositorySync: StateSetter<boolean>;
  setIsRepositoryDeleteDialogOpen: StateSetter<boolean>;
  setIsRepositoryDeletePreviewLoading: StateSetter<boolean>;
  setIsResolvingRemoteMissing: StateSetter<boolean>;
  setIsUpdateConfirmDialogOpen: StateSetter<boolean>;
  setManualSelectedTagIds: StateSetter<string[]>;
  setManualTagQuery: StateSetter<string>;
  setPendingUpdateStates: StateSetter<CentralSkillUpdateState[]>;
  setQueuedRemoteMissingStates: StateSetter<CentralSkillUpdateState[]>;
  setQueuedRepositorySyncPreview: StateSetter<CentralRepositorySyncPreview | null>;
  setRemoteMissingError: StateSetter<string | null>;
  setRemoteMissingPreview: StateSetter<BatchDeleteCentralSkillPreviewResult | null>;
  setRemoteMissingStates: StateSetter<CentralSkillUpdateState[]>;
  setRepositorySyncDeletePreview: StateSetter<BatchDeleteCentralSkillPreviewResult | null>;
  setRepositorySyncError: StateSetter<string | null>;
  setRepositorySyncPreview: StateSetter<CentralRepositorySyncPreview | null>;
  setRepositoryDeletePreview: StateSetter<DeleteSkillRepositoryPreview | null>;
  setRepositoryDeletePreviewError: StateSetter<string | null>;
  setRepositoryDeleteTarget: StateSetter<SkillRepositoryWithStats | null>;
  setRepositoryFilter: StateSetter<string>;
  setSelectedSkillIds: StateSetter<string[]>;
}

export interface CentralSkillsActionsDeps {
  detailButtonRefs: RefObject<Record<string, HTMLButtonElement | null>>;
  state: CentralSkillsActionsState;
  setters: CentralSkillsActionsSetters;
  t: TFunction;
}

function skippedCount(
  result: BatchInstallResult | CentralBatchInstallResult,
): number {
  return result.skipped?.length ?? 0;
}

export function useCentralSkillsActions({
  detailButtonRefs,
  state,
  setters,
  t,
}: CentralSkillsActionsDeps) {
  const acceptAiTagReview = useCentralSkillsStore(
    (store) => store.acceptAiTagReview,
  );
  const assignSkillTags = useCentralSkillsStore(
    (store) => store.assignSkillTags,
  );
  const unassignSkillTags = useCentralSkillsStore(
    (store) => store.unassignSkillTags,
  );
  const batchInstallSkills = useCentralSkillsStore(
    (store) => store.batchInstallSkills,
  );
  const bulkSuggestSkillTags = useCentralSkillsStore(
    (store) => store.bulkSuggestSkillTags,
  );
  const cancelAiTagJob = useCentralSkillsStore((store) => store.cancelAiTagJob);
  const createTag = useCentralSkillsStore((store) => store.createTag);
  const installSkill = useCentralSkillsStore((store) => store.installSkill);
  const loadCentralSkills = useCentralSkillsStore(
    (store) => store.loadCentralSkills,
  );
  const skipAiTagReview = useCentralSkillsStore(
    (store) => store.skipAiTagReview,
  );
  const togglePlatformLink = useCentralSkillsStore(
    (store) => store.togglePlatformLink,
  );
  const currentDetail = useSkillDetailStore((store) => store.detail);
  const refreshDetailInstallations = useSkillDetailStore(
    (store) => store.refreshInstallations,
  );
  const batchUninstallSkillsFromAgent = useSkillStore(
    (store) => store.batchUninstallSkillsFromAgent,
  );
  const refreshCounts = usePlatformStore((store) => store.refreshCounts);

  const {
    deleteTargetSkill,
    githubRepoUrl,
    manualSelectedTagIds,
    manualTagQuery,
    repositoryDeleteTarget,
    repositoryFilter,
    queuedRemoteMissingStates,
    queuedRepositorySyncPreview,
    selectedSkillIds,
    currentViewSkills,
  } = state;
  const {
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
  } = setters;

  const deleteWorkflow = useCentralSkillsDeleteWorkflow({
    t,
    state: {
      deleteTargetSkill,
      repositoryDeleteTarget,
      repositoryFilter,
      selectedSkillIds,
    },
    setters: {
      setBatchDeletePreview,
      setBatchDeletePreviewError,
      setDeletePreview,
      setDeletePreviewError,
      setDeleteTargetSkill,
      setIsBatchDeleteDialogOpen,
      setIsBatchDeletePreviewLoading,
      setIsDeleteDialogOpen,
      setIsDeletePreviewLoading,
      setIsRepositoryDeleteDialogOpen,
      setIsRepositoryDeletePreviewLoading,
      setRepositoryDeletePreview,
      setRepositoryDeletePreviewError,
      setRepositoryDeleteTarget,
      setRepositoryFilter,
      setSelectedSkillIds,
    },
  });

  const updateWorkflow = useCentralSkillsUpdateWorkflow({
    t,
    state: {
      queuedRemoteMissingStates,
      queuedRepositorySyncPreview,
    },
    setters: {
      setIsUpdateConfirmDialogOpen,
      setIsRemoteMissingDialogOpen,
      setIsRemoteMissingPreviewLoading,
      setIsRepositorySyncDialogOpen,
      setIsRepositorySyncPreviewLoading,
      setIsApplyingRepositorySync,
      setIsResolvingRemoteMissing,
      setPendingUpdateStates,
      setQueuedRemoteMissingStates,
      setQueuedRepositorySyncPreview,
      setRemoteMissingError,
      setRemoteMissingPreview,
      setRemoteMissingStates,
      setRepositorySyncDeletePreview,
      setRepositorySyncError,
      setRepositorySyncPreview,
      setSelectedSkillIds,
    },
  });

  const importWorkflow = useCentralSkillsImportWorkflow({
    t,
    githubRepoUrl,
  });

  function handleInstallClick(skill: SkillWithLinks) {
    setInstallTargetSkill(skill);
    setIsDialogOpen(true);
  }

  function setDetailButtonRef(skillId: string, node: HTMLButtonElement | null) {
    detailButtonRefs.current[skillId] = node;
  }

  function handleOpenDrawer(skillId: string) {
    setDrawerSkillId(skillId);
    setIsDrawerOpen(true);
  }

  async function handleTogglePlatform(skillId: string, agentId: string) {
    try {
      await togglePlatformLink(skillId, agentId);
      await refreshCounts();
    } catch (err) {
      toast.error(t("central.installError", { error: String(err) }));
    }
  }

  async function handleInstall(
    skillId: string,
    agentIds: string[],
    method: "symlink" | "copy",
    projectPath?: string | null,
  ) {
    try {
      const result = (await installSkill(
        skillId,
        agentIds,
        method,
        projectPath,
      )) as BatchInstallResult;
      await refreshCounts();
      if (currentDetail?.id === skillId) {
        await refreshDetailInstallations(skillId);
      }
      if (result.failed.length > 0) {
        const failedNames = result.failed
          .map((failure) => `${failure.agent_id}: ${failure.error}`)
          .join("; ");
        toast.error(
          t("central.installPartialFailDetailed", {
            succeededCount: result.succeeded.length,
            skippedCount: skippedCount(result),
            failedCount: result.failed.length,
            platforms: failedNames,
          }),
        );
      } else if (skippedCount(result) > 0) {
        toast.success(
          t("central.installSkippedToast", {
            succeededCount: result.succeeded.length,
            skippedCount: skippedCount(result),
          }),
        );
      } else {
        toast.success(
          t("central.installSuccessToast", {
            succeededCount: result.succeeded.length,
          }),
        );
      }
      return result;
    } catch (err) {
      toast.error(t("central.installError", { error: String(err) }));
      throw err;
    }
  }

  async function handleBatchInstallCentralSkills(
    agentIds: string[],
    method: "symlink" | "copy",
    projectPath?: string | null,
  ) {
    const requestedSkillIds = [...selectedSkillIds];
    const requestedSkillCount = requestedSkillIds.length;
    const platformCount = agentIds.length;

    try {
      const result = (await batchInstallSkills(
        requestedSkillIds,
        agentIds,
        method,
        projectPath,
      )) as CentralBatchInstallResult;
      await refreshCounts();
      if (result.failed.length > 0) {
        toast.error(
          t("central.batchInstallPartialToast", {
            skillCount: requestedSkillCount,
            platformCount,
            succeededCount: result.succeeded.length,
            skippedCount: skippedCount(result),
            failedCount: result.failed.length,
          }),
        );
      } else if (skippedCount(result) > 0) {
        toast.success(
          t("central.batchInstallSkippedToast", {
            skillCount: requestedSkillCount,
            platformCount,
            succeededCount: result.succeeded.length,
            skippedCount: skippedCount(result),
          }),
        );
        setSelectedSkillIds([]);
      } else {
        toast.success(
          t("central.batchInstallSuccess", {
            skillCount: requestedSkillCount,
            platformCount,
          }),
        );
        setSelectedSkillIds([]);
      }
      return result;
    } catch (err) {
      toast.error(t("central.installError", { error: String(err) }));
      throw err;
    }
  }

  async function handleBatchUninstallCentralSkills(
    preview: CentralBatchUninstallPreview,
  ): Promise<CentralBatchUninstallApplyResult> {
    if (preview.groups.length === 0) {
      toast.info(t("central.batchUninstallNothingToDo"));
      return {
        succeeded: [],
        failed: [],
        skipped: preview.skippedSkills,
        sharedRootLinks: preview.sharedRootLinks,
      };
    }

    const succeeded: CentralBatchUninstallApplyResult["succeeded"] = [];
    const failed: CentralBatchUninstallApplyResult["failed"] = [];

    try {
      for (const group of preview.groups) {
        const result = await batchUninstallSkillsFromAgent(
          group.agentId,
          group.requests,
        );
        succeeded.push(
          ...result.succeeded.map((item) => ({
            skill_id: item.skill_id,
            agent_id: group.agentId,
          })),
        );
        failed.push(
          ...result.failed.map((item) => ({
            skill_id: item.skill_id,
            agent_id: group.agentId,
            error: item.error,
          })),
        );
      }

      await refreshCounts();
      await loadCentralSkills();
      if (
        currentDetail?.id &&
        preview.selectedSkillIds.includes(currentDetail.id)
      ) {
        await refreshDetailInstallations(currentDetail.id);
      }

      if (failed.length === 0) {
        toast.success(
          t("central.batchUninstallSuccess", {
            succeeded: succeeded.length,
            skipped: preview.skippedSkills.length,
          }),
        );
        setSelectedSkillIds([]);
      } else {
        setSelectedSkillIds(
          Array.from(new Set(failed.map((item) => item.skill_id))),
        );
        toast.error(
          t("central.batchUninstallPartial", {
            succeeded: succeeded.length,
            failed: failed.length,
          }),
        );
      }

      return {
        succeeded,
        failed,
        skipped: preview.skippedSkills,
        sharedRootLinks: preview.sharedRootLinks,
      };
    } catch (err) {
      toast.error(t("central.batchUninstallError", { error: String(err) }));
      throw err;
    }
  }

  function handleToggleSelection(skillId: string) {
    setSelectedSkillIds((current) =>
      current.includes(skillId)
        ? current.filter((id) => id !== skillId)
        : [...current, skillId],
    );
  }

  function handleSelectCurrentFilter() {
    setSelectedSkillIds(currentViewSkills.map((skill) => skill.id));
  }

  function handleSelectUncategorized() {
    setSelectedSkillIds(
      currentViewSkills
        .filter((skill) => {
          const skillTags = skill.tags ?? [];
          return (
            skillTags.length === 0 ||
            skillTags.some((tag) => tag.id === "uncategorized")
          );
        })
        .map((skill) => skill.id),
    );
  }

  function handleToggleManualTag(tagId: string) {
    setManualSelectedTagIds((current) =>
      current.includes(tagId)
        ? current.filter((id) => id !== tagId)
        : [...current, tagId],
    );
  }

  async function handleCreateManualTag() {
    const name = manualTagQuery.trim();
    if (!name || !createTag) return;
    try {
      const tag = await createTag(name);
      setManualSelectedTagIds((current) =>
        current.includes(tag.id) ? current : [...current, tag.id],
      );
      setManualTagQuery("");
      toast.success(t("central.tagCreated"));
    } catch (err) {
      toast.error(
        t("central.metadataError", { error: formatBackendError(err, t) }),
      );
    }
  }

  async function handleApplyManualTags() {
    if (
      !assignSkillTags ||
      selectedSkillIds.length === 0 ||
      manualSelectedTagIds.length === 0
    )
      return;
    try {
      await assignSkillTags(selectedSkillIds, manualSelectedTagIds);
      toast.success(
        t("central.tagsAssigned", { count: selectedSkillIds.length }),
      );
    } catch (err) {
      toast.error(
        t("central.metadataError", { error: formatBackendError(err, t) }),
      );
    }
  }

  async function handleAcceptReview(review: SkillAiTagReview) {
    if (!acceptAiTagReview) return;
    try {
      await acceptAiTagReview(review.skill_id, [review.tag.id]);
      toast.success(t("central.reviewAccepted"));
    } catch (err) {
      toast.error(
        t("central.metadataError", { error: formatBackendError(err, t) }),
      );
    }
  }

  async function handleApplyManualTagsToReview(review: SkillAiTagReview) {
    if (
      !assignSkillTags ||
      !skipAiTagReview ||
      manualSelectedTagIds.length === 0
    )
      return;
    try {
      await assignSkillTags([review.skill_id], manualSelectedTagIds);
      await skipAiTagReview(review.skill_id);
      toast.success(t("central.reviewChanged"));
    } catch (err) {
      toast.error(
        t("central.metadataError", { error: formatBackendError(err, t) }),
      );
    }
  }

  async function handleSkipReview(review: SkillAiTagReview) {
    if (!skipAiTagReview) return;
    try {
      await skipAiTagReview(review.skill_id);
      toast.success(t("central.reviewSkipped"));
    } catch (err) {
      toast.error(
        t("central.metadataError", { error: formatBackendError(err, t) }),
      );
    }
  }

  async function handleBulkSuggestTags() {
    if (!bulkSuggestSkillTags || selectedSkillIds.length === 0) return;
    try {
      const result = await bulkSuggestSkillTags(selectedSkillIds);
      const succeeded = result.filter(
        (item) => item.succeeded !== false && !item.error,
      ).length;
      const failed = result.length - succeeded;
      const review = result.reduce(
        (count, item) => count + (item.low_confidence_count ?? 0),
        0,
      );
      toast.success(t("central.aiTagsFinished", { succeeded, failed, review }));
    } catch (err) {
      toast.error(
        t("central.metadataError", { error: formatBackendError(err, t) }),
      );
    }
  }

  async function handleCancelAiTagJob() {
    if (!cancelAiTagJob) return;
    try {
      await cancelAiTagJob();
      toast.info(t("central.aiTagCancelRequested"));
    } catch (err) {
      toast.error(
        t("central.metadataError", { error: formatBackendError(err, t) }),
      );
    }
  }

  async function handleRefresh() {
    try {
      await refreshCounts();
      await loadCentralSkills();
    } catch (err) {
      toast.error(t("central.refreshError", { error: String(err) }));
    }
  }

  // ── 卡上标签增删（central 方案C） ──
  async function handleAddSkillTag(skillId: string, tagId: string) {
    if (!assignSkillTags) return;
    try {
      await assignSkillTags([skillId], [tagId]);
    } catch (err) {
      toast.error(
        t("central.metadataError", { error: formatBackendError(err, t) }),
      );
    }
  }

  async function handleCreateSkillTag(skillId: string, name: string) {
    if (!createTag || !assignSkillTags) return;
    try {
      const tag = await createTag(name);
      await assignSkillTags([skillId], [tag.id]);
    } catch (err) {
      toast.error(
        t("central.metadataError", { error: formatBackendError(err, t) }),
      );
    }
  }

  async function handleRemoveSkillTag(skillId: string, tagId: string) {
    if (!unassignSkillTags) return;
    try {
      await unassignSkillTags(skillId, [tagId]);
    } catch (err) {
      toast.error(
        t("central.metadataError", { error: formatBackendError(err, t) }),
      );
    }
  }

  return {
    ...deleteWorkflow,
    ...updateWorkflow,
    ...importWorkflow,
    handleApplyManualTags,
    handleApplyManualTagsToReview,
    handleAcceptReview,
    handleAddSkillTag,
    handleCreateSkillTag,
    handleRemoveSkillTag,
    handleBatchInstallCentralSkills,
    handleBatchUninstallCentralSkills,
    handleBulkSuggestTags,
    handleCancelAiTagJob,
    handleCreateManualTag,
    handleInstall,
    handleInstallClick,
    handleOpenDrawer,
    handleRefresh,
    handleSelectCurrentFilter,
    handleSelectUncategorized,
    handleSkipReview,
    handleToggleManualTag,
    handleTogglePlatform,
    handleToggleSelection,
    setDetailButtonRef,
  };
}
