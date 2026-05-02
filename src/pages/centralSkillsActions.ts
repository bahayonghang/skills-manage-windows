import type { Dispatch, RefObject, SetStateAction } from "react";
import type { TFunction } from "i18next";
import { toast } from "sonner";

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
  checkSkillUpdates: () => Promise<CentralSkillUpdateState[]>;
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
  function handleInstallClick(skill: SkillWithLinks) {
    setInstallTargetSkill(skill);
    setIsDialogOpen(true);
  }

  function handleDeleteDialogOpenChange(open: boolean) {
    setIsDeleteDialogOpen(open);
    if (!open) {
      setDeleteTargetSkill(null);
      setDeletePreview(null);
      setDeletePreviewError(null);
      setIsDeletePreviewLoading(false);
    }
  }

  function handleBatchDeleteDialogOpenChange(open: boolean) {
    setIsBatchDeleteDialogOpen(open);
    if (!open) {
      setBatchDeletePreview(null);
      setBatchDeletePreviewError(null);
      setIsBatchDeletePreviewLoading(false);
    }
  }

  function handleRepositoryDeleteDialogOpenChange(open: boolean) {
    setIsRepositoryDeleteDialogOpen(open);
    if (!open) {
      setRepositoryDeleteTarget(null);
      setRepositoryDeletePreview(null);
      setRepositoryDeletePreviewError(null);
      setIsRepositoryDeletePreviewLoading(false);
    }
  }

  function handleRemoteMissingDialogOpenChange(open: boolean) {
    setIsRemoteMissingDialogOpen(open);
    if (!open) {
      setRemoteMissingStates([]);
      setRemoteMissingPreview(null);
      setRemoteMissingError(null);
      setIsRemoteMissingPreviewLoading(false);
      setIsResolvingRemoteMissing(false);
    }
  }

  async function handleDeleteClick(skill: SkillWithLinks) {
    setDeleteTargetSkill(skill);
    setDeletePreview(null);
    setDeletePreviewError(null);
    setIsDeleteDialogOpen(true);
    setIsDeletePreviewLoading(true);
    try {
      const preview = await loadDeletePreview(skill.id);
      setDeletePreview(preview);
    } catch (err) {
      const message = String(err);
      setDeletePreviewError(message);
      toast.error(t("central.deletePreviewError", { error: message }));
    } finally {
      setIsDeletePreviewLoading(false);
    }
  }

  async function handleBatchDeleteClick() {
    if (selectedSkillIds.length === 0) return;
    setBatchDeletePreview(null);
    setBatchDeletePreviewError(null);
    setIsBatchDeleteDialogOpen(true);
    setIsBatchDeletePreviewLoading(true);
    try {
      const preview = await loadBatchDeletePreview(selectedSkillIds);
      setBatchDeletePreview(preview);
    } catch (err) {
      const message = String(err);
      setBatchDeletePreviewError(message);
      toast.error(t("central.batchDeletePreviewError", { error: message }));
    } finally {
      setIsBatchDeletePreviewLoading(false);
    }
  }

  async function handleRepositoryDeleteClick(repository: SkillRepositoryWithStats) {
    if (repository.is_unknown) return;

    setRepositoryDeleteTarget(repository);
    setRepositoryDeletePreview(null);
    setRepositoryDeletePreviewError(null);
    setIsRepositoryDeletePreviewLoading(true);
    try {
      const preview = await loadRepositoryDeletePreview(repository.id);
      const previewedSkillCount = preview.delete_preview.previews.length;
      if (previewedSkillCount === 0 && preview.delete_preview.failed.length === 0) {
        const confirmed = window.confirm(
          t("central.deleteRepositoryEmptyConfirm", { name: repository.name })
        );
        if (confirmed) {
          const result = await deleteSkillRepository(repository.id, []);
          await refreshCounts();
          if (repositoryFilter === repository.id) {
            setRepositoryFilter("all");
          }
          toast.success(
            t("central.deleteRepositorySuccess", {
              name: result.repository.name,
              count: result.delete_result.succeeded.length,
            })
          );
        }
        setRepositoryDeleteTarget(null);
        return;
      }

      setRepositoryDeletePreview(preview);
      setIsRepositoryDeleteDialogOpen(true);
    } catch (err) {
      const message = String(err);
      setRepositoryDeletePreviewError(message);
      toast.error(t("central.deleteRepositoryError", { error: message }));
    } finally {
      setIsRepositoryDeletePreviewLoading(false);
    }
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

  async function handleDeleteCentralSkill(skillId: string, removeAgentIds: string[]) {
    try {
      await deleteCentralSkill(skillId, removeAgentIds);
      await refreshCounts();
      toast.success(t("central.deleteSkillSuccess", { name: deleteTargetSkill?.name ?? skillId }));
      handleDeleteDialogOpenChange(false);
    } catch (err) {
      const message = String(err);
      setDeletePreviewError(message);
      toast.error(t("central.deleteSkillError", { error: message }));
      throw err;
    }
  }

  async function handleBatchDeleteCentralSkills(requests: BatchDeleteCentralSkillRequest[]) {
    try {
      const result = await deleteCentralSkills(requests);
      await refreshCounts();
      const succeededIds = new Set(result.succeeded.map((item) => item.skill_id));
      setSelectedSkillIds((current) => current.filter((skillId) => !succeededIds.has(skillId)));

      if (result.failed.length > 0) {
        toast.error(
          t("central.batchDeletePartialError", {
            succeeded: result.succeeded.length,
            failed: result.failed.length,
          })
        );
      } else {
        toast.success(t("central.batchDeleteSuccess", { count: result.succeeded.length }));
      }
      handleBatchDeleteDialogOpenChange(false);
      return result;
    } catch (err) {
      const message = String(err);
      setBatchDeletePreviewError(message);
      toast.error(t("central.deleteSkillError", { error: message }));
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

  async function handleCheckUpdates() {
    try {
      const states = await checkSkillUpdates();
      const available = states.filter((state) => state.status === "update_available").length;
      const unsupported = states.filter((state) => state.status === "unsupported").length;
      const remoteMissing = states.filter((state) => state.status === "remote_missing");
      const failed = states.filter((state) => state.status === "error").length;
      toast.success(
        t("central.updateCheckFinished", {
          available,
          unsupported,
          remoteMissing: remoteMissing.length,
          failed,
        })
      );
      if (remoteMissing.length > 0) {
        const missingSkillIds = remoteMissing.map((state) => state.skill_id);
        setRemoteMissingStates(remoteMissing);
        setRemoteMissingPreview(null);
        setRemoteMissingError(null);
        setIsRemoteMissingDialogOpen(true);
        setIsRemoteMissingPreviewLoading(true);
        try {
          const preview = await loadBatchDeletePreview(missingSkillIds);
          setRemoteMissingPreview(preview);
        } catch (err) {
          const message = String(err);
          setRemoteMissingError(message);
          toast.error(t("central.batchDeletePreviewError", { error: message }));
        } finally {
          setIsRemoteMissingPreviewLoading(false);
        }
      }
    } catch (err) {
      toast.error(t("central.updateCheckError", { error: String(err) }));
    }
  }

  async function handleUpdateSkills(skillIds: string[]) {
    if (skillIds.length === 0) return;
    try {
      const result = await updateSkills(skillIds);
      await refreshCounts();
      toast.success(
        t("central.updateFinished", {
          succeeded: result.succeeded.length,
          failed: result.failed.length,
          skipped: result.skipped.length,
        })
      );
    } catch (err) {
      toast.error(t("central.updateError", { error: String(err) }));
    }
  }

  async function handleResolveRemoteMissing(
    keepSkillIds: string[],
    deleteRequests: BatchDeleteCentralSkillRequest[]
  ) {
    setIsResolvingRemoteMissing(true);
    setRemoteMissingError(null);
    try {
      if (keepSkillIds.length > 0) {
        await keepRemoteMissingSkills(keepSkillIds);
      }

      const deleteResult: BatchDeleteCentralSkillResult =
        deleteRequests.length > 0
          ? await deleteCentralSkills(deleteRequests)
          : { succeeded: [], failed: [] };

      await refreshCounts();
      const succeededDeleteIds = new Set(deleteResult.succeeded.map((item) => item.skill_id));
      setSelectedSkillIds((current) =>
        current.filter((skillId) => !succeededDeleteIds.has(skillId))
      );

      if (deleteResult.failed.length > 0) {
        toast.error(
          t("central.remoteMissingResolvePartial", {
            kept: keepSkillIds.length,
            deleted: deleteResult.succeeded.length,
            failed: deleteResult.failed.length,
          })
        );
      } else {
        toast.success(
          t("central.remoteMissingResolveSuccess", {
            kept: keepSkillIds.length,
            deleted: deleteResult.succeeded.length,
          })
        );
      }
      handleRemoteMissingDialogOpenChange(false);
    } catch (err) {
      const message = String(err);
      setRemoteMissingError(message);
      toast.error(t("central.remoteMissingResolveError", { error: message }));
      throw err;
    } finally {
      setIsResolvingRemoteMissing(false);
    }
  }

  async function handleGitHubPreview() {
    try {
      return await previewGitHubRepoImport(githubRepoUrl);
    } catch {
      return null;
    }
  }

  async function handleGitHubImport(selections: GitHubSkillImportSelection[]) {
    try {
      const result = await importGitHubRepoSkills(githubRepoUrl, selections);
      await Promise.all([refreshCounts(), loadCentralSkills()]);
      toast.success(t("marketplace.githubImportCentralSuccess"));
      return result;
    } catch (err) {
      toast.error(t("marketplace.githubImportError", { error: String(err) }));
      throw err;
    }
  }

  async function handleDeleteSkillRepository(requests: BatchDeleteCentralSkillRequest[]) {
    if (!repositoryDeleteTarget) {
      throw new Error("Repository delete target is missing");
    }

    try {
      const result = await deleteSkillRepository(repositoryDeleteTarget.id, requests);
      await refreshCounts();
      const succeededIds = new Set(result.delete_result.succeeded.map((item) => item.skill_id));
      setSelectedSkillIds((current) => current.filter((skillId) => !succeededIds.has(skillId)));

      if (repositoryFilter === repositoryDeleteTarget.id && result.deleted_repository) {
        setRepositoryFilter("all");
      }

      if (result.delete_result.failed.length > 0) {
        toast.error(
          t("central.deleteRepositoryPartialError", {
            name: result.repository.name,
            succeeded: result.delete_result.succeeded.length,
            failed: result.delete_result.failed.length,
          })
        );
      } else {
        toast.success(
          t("central.deleteRepositorySuccess", {
            name: result.repository.name,
            count: result.delete_result.succeeded.length,
          })
        );
      }
      handleRepositoryDeleteDialogOpenChange(false);
      return result.delete_result;
    } catch (err) {
      const message = String(err);
      setRepositoryDeletePreviewError(message);
      toast.error(t("central.deleteRepositoryError", { error: message }));
      throw err;
    }
  }

  async function handleInstallImportedSkill(
    skillId: string,
    agentIds: string[],
    method: "symlink" | "copy",
    projectPath?: string | null
  ) {
    const result = await handleInstall(skillId, agentIds, method, projectPath);
    await Promise.all(agentIds.map((agentId) => getSkillsByAgent(agentId)));
    return result;
  }

  async function handleAfterImportSuccess() {
    const agentIds = Object.keys(skillsByAgent);
    if (agentIds.length === 0) return;
    await Promise.all(agentIds.map((agentId) => getSkillsByAgent(agentId)));
  }

  return {
    handleAfterImportSuccess,
    handleApplyManualTags,
    handleApplyManualTagsToReview,
    handleAcceptReview,
    handleBatchDeleteCentralSkills,
    handleBatchDeleteClick,
    handleBatchDeleteDialogOpenChange,
    handleBatchInstallCentralSkills,
    handleBulkSuggestTags,
    handleCancelAiTagJob,
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
  };
}
