import type { Dispatch, SetStateAction } from "react";
import type { TFunction } from "i18next";
import { toast } from "sonner";

import type {
  BatchDeleteCentralSkillPreviewResult,
  BatchDeleteCentralSkillRequest,
  BatchDeleteCentralSkillResult,
  DeleteSkillRepositoryPreview,
  DeleteSkillRepositoryResult,
  SkillDetail,
  SkillRepositoryWithStats,
  SkillWithLinks,
} from "@/types";

type StateSetter<T> = Dispatch<SetStateAction<T>>;

export interface CentralSkillsDeleteWorkflowDeps {
  t: TFunction;

  deleteTargetSkill: SkillWithLinks | null;
  repositoryDeleteTarget: SkillRepositoryWithStats | null;
  repositoryFilter: string;
  selectedSkillIds: string[];

  loadDeletePreview: (skillId: string) => Promise<SkillDetail>;
  loadBatchDeletePreview: (
    skillIds: string[]
  ) => Promise<BatchDeleteCentralSkillPreviewResult>;
  loadRepositoryDeletePreview: (
    repositoryId: string
  ) => Promise<DeleteSkillRepositoryPreview>;
  deleteCentralSkill: (
    skillId: string,
    removeAgentIds: string[]
  ) => Promise<void>;
  deleteCentralSkills: (
    requests: BatchDeleteCentralSkillRequest[]
  ) => Promise<BatchDeleteCentralSkillResult>;
  deleteSkillRepository: (
    repositoryId: string,
    requests: BatchDeleteCentralSkillRequest[]
  ) => Promise<DeleteSkillRepositoryResult>;
  refreshCounts: () => Promise<void>;

  setBatchDeletePreview: StateSetter<BatchDeleteCentralSkillPreviewResult | null>;
  setBatchDeletePreviewError: StateSetter<string | null>;
  setDeletePreview: StateSetter<SkillDetail | null>;
  setDeletePreviewError: StateSetter<string | null>;
  setDeleteTargetSkill: StateSetter<SkillWithLinks | null>;
  setIsBatchDeleteDialogOpen: StateSetter<boolean>;
  setIsBatchDeletePreviewLoading: StateSetter<boolean>;
  setIsDeleteDialogOpen: StateSetter<boolean>;
  setIsDeletePreviewLoading: StateSetter<boolean>;
  setIsRepositoryDeleteDialogOpen: StateSetter<boolean>;
  setIsRepositoryDeletePreviewLoading: StateSetter<boolean>;
  setRepositoryDeletePreview: StateSetter<DeleteSkillRepositoryPreview | null>;
  setRepositoryDeletePreviewError: StateSetter<string | null>;
  setRepositoryDeleteTarget: StateSetter<SkillRepositoryWithStats | null>;
  setRepositoryFilter: StateSetter<string>;
  setSelectedSkillIds: StateSetter<string[]>;
}

export function createCentralSkillsDeleteWorkflow(
  deps: CentralSkillsDeleteWorkflowDeps
) {
  const {
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
  } = deps;

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

  async function handleRepositoryDeleteClick(
    repository: SkillRepositoryWithStats
  ) {
    if (repository.is_unknown) return;

    setRepositoryDeleteTarget(repository);
    setRepositoryDeletePreview(null);
    setRepositoryDeletePreviewError(null);
    setIsRepositoryDeletePreviewLoading(true);
    try {
      const preview = await loadRepositoryDeletePreview(repository.id);
      const previewedSkillCount = preview.delete_preview.previews.length;
      if (
        previewedSkillCount === 0 &&
        preview.delete_preview.failed.length === 0
      ) {
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

  async function handleDeleteCentralSkill(
    skillId: string,
    removeAgentIds: string[]
  ) {
    try {
      await deleteCentralSkill(skillId, removeAgentIds);
      await refreshCounts();
      toast.success(
        t("central.deleteSkillSuccess", {
          name: deleteTargetSkill?.name ?? skillId,
        })
      );
      handleDeleteDialogOpenChange(false);
    } catch (err) {
      const message = String(err);
      setDeletePreviewError(message);
      toast.error(t("central.deleteSkillError", { error: message }));
      throw err;
    }
  }

  async function handleBatchDeleteCentralSkills(
    requests: BatchDeleteCentralSkillRequest[]
  ) {
    try {
      const result = await deleteCentralSkills(requests);
      await refreshCounts();
      const succeededIds = new Set(
        result.succeeded.map((item) => item.skill_id)
      );
      setSelectedSkillIds((current) =>
        current.filter((skillId) => !succeededIds.has(skillId))
      );

      if (result.failed.length > 0) {
        toast.error(
          t("central.batchDeletePartialError", {
            succeeded: result.succeeded.length,
            failed: result.failed.length,
          })
        );
      } else {
        toast.success(
          t("central.batchDeleteSuccess", { count: result.succeeded.length })
        );
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

  async function handleDeleteSkillRepository(
    requests: BatchDeleteCentralSkillRequest[]
  ) {
    if (!repositoryDeleteTarget) {
      throw new Error("Repository delete target is missing");
    }

    try {
      const result = await deleteSkillRepository(
        repositoryDeleteTarget.id,
        requests
      );
      await refreshCounts();
      const succeededIds = new Set(
        result.delete_result.succeeded.map((item) => item.skill_id)
      );
      setSelectedSkillIds((current) =>
        current.filter((skillId) => !succeededIds.has(skillId))
      );

      if (
        repositoryFilter === repositoryDeleteTarget.id &&
        result.deleted_repository
      ) {
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

  return {
    handleBatchDeleteCentralSkills,
    handleBatchDeleteClick,
    handleBatchDeleteDialogOpenChange,
    handleDeleteCentralSkill,
    handleDeleteClick,
    handleDeleteDialogOpenChange,
    handleDeleteSkillRepository,
    handleRepositoryDeleteClick,
    handleRepositoryDeleteDialogOpenChange,
  };
}
