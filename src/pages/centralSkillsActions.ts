import type { Dispatch, RefObject, SetStateAction } from "react";
import type { TFunction } from "i18next";
import { toast } from "sonner";

import { createCentralSkillsDeleteWorkflow } from "@/pages/centralSkillsDeleteWorkflow";
import { createCentralSkillsImportWorkflow } from "@/pages/centralSkillsImportWorkflow";
import { createCentralSkillsUpdateWorkflow } from "@/pages/centralSkillsUpdateWorkflow";
import type {
  BatchDeleteCentralSkillPreviewResult,
  BatchDeleteCentralSkillRequest,
  BatchDeleteCentralSkillResult,
  BatchInstallResult,
  CentralBatchInstallResult,
  CentralSkillUpdateState,
  DeleteSkillRepositoryPreview,
  DeleteSkillRepositoryResult,
  GitHubRepoImportResult,
  GitHubRepoPreview,
  GitHubSkillImportSelection,
  ScannedSkill,
  SkillAiTagReview,
  SkillDetail,
  SkillRepositoryWithStats,
  SkillWithLinks,
} from "@/types";

type StateSetter<T> = Dispatch<SetStateAction<T>>;

export function useCentralSkillsActions({
  acceptAiTagReview,
  assignSkillTags,
  batchInstallSkills,
  bulkSuggestSkillTags,
  cancelAiTagJob,
  cancelCentralUpdates,
  checkSkillUpdates,
  createTag,
  deleteCentralSkill,
  deleteCentralSkills,
  deleteSkillRepository,
  detailButtonRefs,
  getSkillsByAgent,
  githubRepoUrl,
  importGitHubRepoSkills,
  deleteTargetSkill,
  installSkill,
  loadBatchDeletePreview,
  loadCentralSkills,
  loadDeletePreview,
  loadRepositoryDeletePreview,
  manualSelectedTagIds,
  manualTagQuery,
  previewGitHubRepoImport,
  refreshCounts,
  repositoryDeleteTarget,
  repositoryFilter,
  selectedSkillIds,
  skipAiTagReview,
  sortedSkills,
  skillsByAgent,
  t,
  togglePlatformLink,
  updateSkills,
  keepRemoteMissingSkills,
  setBatchDeletePreview,
  setBatchDeletePreviewError,
  setDeletePreview,
  setDeletePreviewError,
  setDeleteTargetSkill,
  setDrawerSkillId,
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
  setInstallTargetSkill,
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
}: {
  acceptAiTagReview?: (skillId: string, tagIds: string[]) => Promise<void>;
  assignSkillTags?: (skillIds: string[], tagIds: string[]) => Promise<void>;
  batchInstallSkills: (
    skillIds: string[],
    agentIds: string[],
    method: "symlink" | "copy",
    projectPath?: string | null
  ) => Promise<CentralBatchInstallResult>;
  bulkSuggestSkillTags?: (skillIds: string[]) => Promise<Array<{
    succeeded?: boolean;
    error?: string | null;
    low_confidence_count?: number | null;
  }>>;
  cancelAiTagJob?: () => Promise<void>;
  cancelCentralUpdates: () => Promise<void>;
  checkSkillUpdates: (skillIds?: string[]) => Promise<CentralSkillUpdateState[]>;
  createTag?: (name: string) => Promise<{ id: string }>;
  deleteCentralSkill: (skillId: string, removeAgentIds: string[]) => Promise<void>;
  deleteCentralSkills: (
    requests: BatchDeleteCentralSkillRequest[]
  ) => Promise<BatchDeleteCentralSkillResult>;
  deleteSkillRepository: (
    repositoryId: string,
    requests: BatchDeleteCentralSkillRequest[]
  ) => Promise<DeleteSkillRepositoryResult>;
  detailButtonRefs: RefObject<Record<string, HTMLButtonElement | null>>;
  getSkillsByAgent: (agentId: string) => Promise<void>;
  githubRepoUrl: string;
  importGitHubRepoSkills: (
    repoUrl: string,
    selections: GitHubSkillImportSelection[]
  ) => Promise<GitHubRepoImportResult>;
  deleteTargetSkill: SkillWithLinks | null;
  installSkill: (
    skillId: string,
    agentIds: string[],
    method: "symlink" | "copy",
    projectPath?: string | null
  ) => Promise<BatchInstallResult>;
  loadBatchDeletePreview: (skillIds: string[]) => Promise<BatchDeleteCentralSkillPreviewResult>;
  loadCentralSkills: () => Promise<void>;
  loadDeletePreview: (skillId: string) => Promise<SkillDetail>;
  loadRepositoryDeletePreview: (repositoryId: string) => Promise<DeleteSkillRepositoryPreview>;
  manualSelectedTagIds: string[];
  manualTagQuery: string;
  previewGitHubRepoImport: (repoUrl: string) => Promise<GitHubRepoPreview | null>;
  refreshCounts: () => Promise<void>;
  repositoryDeleteTarget: SkillRepositoryWithStats | null;
  repositoryFilter: string;
  selectedSkillIds: string[];
  skipAiTagReview?: (skillId: string) => Promise<void>;
  sortedSkills: SkillWithLinks[];
  skillsByAgent: Record<string, ScannedSkill[]>;
  t: TFunction;
  togglePlatformLink: (skillId: string, agentId: string) => Promise<void>;
  updateSkills: (skillIds: string[]) => Promise<{
    succeeded: unknown[];
    failed: unknown[];
    skipped: unknown[];
  }>;
  keepRemoteMissingSkills: (skillIds: string[]) => Promise<string[]>;
  setBatchDeletePreview: StateSetter<BatchDeleteCentralSkillPreviewResult | null>;
  setBatchDeletePreviewError: StateSetter<string | null>;
  setDeletePreview: StateSetter<SkillDetail | null>;
  setDeletePreviewError: StateSetter<string | null>;
  setDeleteTargetSkill: StateSetter<SkillWithLinks | null>;
  setDrawerSkillId: StateSetter<string | null>;
  setIsBatchDeleteDialogOpen: StateSetter<boolean>;
  setIsBatchDeletePreviewLoading: StateSetter<boolean>;
  setIsDeleteDialogOpen: StateSetter<boolean>;
  setIsDeletePreviewLoading: StateSetter<boolean>;
  setIsDialogOpen: StateSetter<boolean>;
  setIsDrawerOpen: StateSetter<boolean>;
  setIsRemoteMissingDialogOpen: StateSetter<boolean>;
  setIsRemoteMissingPreviewLoading: StateSetter<boolean>;
  setIsRepositoryDeleteDialogOpen: StateSetter<boolean>;
  setIsRepositoryDeletePreviewLoading: StateSetter<boolean>;
  setIsResolvingRemoteMissing: StateSetter<boolean>;
  setInstallTargetSkill: StateSetter<SkillWithLinks | null>;
  setManualSelectedTagIds: StateSetter<string[]>;
  setManualTagQuery: StateSetter<string>;
  setRemoteMissingError: StateSetter<string | null>;
  setRemoteMissingPreview: StateSetter<BatchDeleteCentralSkillPreviewResult | null>;
  setRemoteMissingStates: StateSetter<CentralSkillUpdateState[]>;
  setRepositoryDeletePreview: StateSetter<DeleteSkillRepositoryPreview | null>;
  setRepositoryDeletePreviewError: StateSetter<string | null>;
  setRepositoryDeleteTarget: StateSetter<SkillRepositoryWithStats | null>;
  setRepositoryFilter: StateSetter<string>;
  setSelectedSkillIds: StateSetter<string[]>;
}) {
  const deleteWorkflow = createCentralSkillsDeleteWorkflow({
    t,
    deleteTargetSkill,
    repositoryDeleteTarget,
    repositoryFilter,
    selectedSkillIds,
    loadDeletePreview,
    loadBatchDeletePreview,
    loadRepositoryDeletePreview,
    deleteCentralSkill,
    deleteCentralSkills,
    deleteSkillRepository,
    refreshCounts,
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
  });

  const updateWorkflow = createCentralSkillsUpdateWorkflow({
    t,
    cancelCentralUpdates,
    checkSkillUpdates,
    deleteCentralSkills,
    keepRemoteMissingSkills,
    loadBatchDeletePreview,
    refreshCounts,
    updateSkills,
    setIsRemoteMissingDialogOpen,
    setIsRemoteMissingPreviewLoading,
    setIsResolvingRemoteMissing,
    setRemoteMissingError,
    setRemoteMissingPreview,
    setRemoteMissingStates,
    setSelectedSkillIds,
  });

  const importWorkflow = createCentralSkillsImportWorkflow({
    t,
    githubRepoUrl,
    skillsByAgent,
    getSkillsByAgent,
    importGitHubRepoSkills,
    installSkill,
    loadCentralSkills,
    previewGitHubRepoImport,
    refreshCounts,
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
    projectPath?: string | null
  ) {
    try {
      const result = await installSkill(skillId, agentIds, method, projectPath);
      await refreshCounts();
      if (result.failed.length > 0) {
        const failedNames = result.failed
          .map((failure) => `${failure.agent_id}: ${failure.error}`)
          .join("; ");
        toast.error(t("central.installPartialFail", { platforms: failedNames }));
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
    projectPath?: string | null
  ) {
    const requestedSkillIds = [...selectedSkillIds];
    const requestedSkillCount = requestedSkillIds.length;
    const platformCount = agentIds.length;

    try {
      const result = await batchInstallSkills(requestedSkillIds, agentIds, method, projectPath);
      await refreshCounts();
      if (result.failed.length > 0) {
        toast.error(
          t("central.batchInstallPartialToast", {
            skillCount: requestedSkillCount,
            platformCount,
            failedCount: result.failed.length,
          })
        );
      } else {
        toast.success(
          t("central.batchInstallSuccess", {
            skillCount: requestedSkillCount,
            platformCount,
          })
        );
        setSelectedSkillIds([]);
      }
      return result;
    } catch (err) {
      toast.error(t("central.installError", { error: String(err) }));
      throw err;
    }
  }

  function handleToggleSelection(skillId: string) {
    setSelectedSkillIds((current) =>
      current.includes(skillId)
        ? current.filter((id) => id !== skillId)
        : [...current, skillId]
    );
  }

  function handleSelectCurrentFilter() {
    setSelectedSkillIds(sortedSkills.map((skill) => skill.id));
  }

  function handleSelectUncategorized() {
    setSelectedSkillIds(
      sortedSkills
        .filter((skill) => {
          const skillTags = skill.tags ?? [];
          return skillTags.length === 0 || skillTags.some((tag) => tag.id === "uncategorized");
        })
        .map((skill) => skill.id)
    );
  }

  function handleToggleManualTag(tagId: string) {
    setManualSelectedTagIds((current) =>
      current.includes(tagId)
        ? current.filter((id) => id !== tagId)
        : [...current, tagId]
    );
  }

  async function handleCreateManualTag() {
    const name = manualTagQuery.trim();
    if (!name || !createTag) return;
    try {
      const tag = await createTag(name);
      setManualSelectedTagIds((current) =>
        current.includes(tag.id) ? current : [...current, tag.id]
      );
      setManualTagQuery("");
      toast.success(t("central.tagCreated"));
    } catch (err) {
      toast.error(t("central.metadataError", { error: String(err) }));
    }
  }

  async function handleApplyManualTags() {
    if (!assignSkillTags || selectedSkillIds.length === 0 || manualSelectedTagIds.length === 0) return;
    try {
      await assignSkillTags(selectedSkillIds, manualSelectedTagIds);
      toast.success(t("central.tagsAssigned", { count: selectedSkillIds.length }));
    } catch (err) {
      toast.error(t("central.metadataError", { error: String(err) }));
    }
  }

  async function handleAcceptReview(review: SkillAiTagReview) {
    if (!acceptAiTagReview) return;
    try {
      await acceptAiTagReview(review.skill_id, [review.tag.id]);
      toast.success(t("central.reviewAccepted"));
    } catch (err) {
      toast.error(t("central.metadataError", { error: String(err) }));
    }
  }

  async function handleApplyManualTagsToReview(review: SkillAiTagReview) {
    if (!assignSkillTags || !skipAiTagReview || manualSelectedTagIds.length === 0) return;
    try {
      await assignSkillTags([review.skill_id], manualSelectedTagIds);
      await skipAiTagReview(review.skill_id);
      toast.success(t("central.reviewChanged"));
    } catch (err) {
      toast.error(t("central.metadataError", { error: String(err) }));
    }
  }

  async function handleSkipReview(review: SkillAiTagReview) {
    if (!skipAiTagReview) return;
    try {
      await skipAiTagReview(review.skill_id);
      toast.success(t("central.reviewSkipped"));
    } catch (err) {
      toast.error(t("central.metadataError", { error: String(err) }));
    }
  }

  async function handleBulkSuggestTags() {
    if (!bulkSuggestSkillTags || selectedSkillIds.length === 0) return;
    try {
      const result = await bulkSuggestSkillTags(selectedSkillIds);
      const succeeded = result.filter((item) => item.succeeded !== false && !item.error).length;
      const failed = result.length - succeeded;
      const review = result.reduce((count, item) => count + (item.low_confidence_count ?? 0), 0);
      toast.success(t("central.aiTagsFinished", { succeeded, failed, review }));
    } catch (err) {
      toast.error(t("central.metadataError", { error: String(err) }));
    }
  }

  async function handleCancelAiTagJob() {
    if (!cancelAiTagJob) return;
    try {
      await cancelAiTagJob();
      toast.info(t("central.aiTagCancelRequested"));
    } catch (err) {
      toast.error(t("central.metadataError", { error: String(err) }));
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

  return {
    ...deleteWorkflow,
    ...updateWorkflow,
    ...importWorkflow,
    handleApplyManualTags,
    handleApplyManualTagsToReview,
    handleAcceptReview,
    handleBatchInstallCentralSkills,
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
