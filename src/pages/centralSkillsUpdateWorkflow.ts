import type { Dispatch, SetStateAction } from "react";
import type { TFunction } from "i18next";
import { toast } from "sonner";

import type {
  BatchDeleteCentralSkillPreviewResult,
  BatchDeleteCentralSkillRequest,
  BatchDeleteCentralSkillResult,
  CentralSkillUpdateState,
} from "@/types";

type StateSetter<T> = Dispatch<SetStateAction<T>>;

export interface CentralSkillsUpdateWorkflowDeps {
  t: TFunction;

  cancelCentralUpdates: () => Promise<void>;
  checkSkillUpdates: (skillIds?: string[]) => Promise<CentralSkillUpdateState[]>;
  deleteCentralSkills: (
    requests: BatchDeleteCentralSkillRequest[]
  ) => Promise<BatchDeleteCentralSkillResult>;
  keepRemoteMissingSkills: (skillIds: string[]) => Promise<string[]>;
  loadBatchDeletePreview: (
    skillIds: string[]
  ) => Promise<BatchDeleteCentralSkillPreviewResult>;
  refreshCounts: () => Promise<void>;
  updateSkills: (skillIds: string[]) => Promise<{
    succeeded: unknown[];
    failed: unknown[];
    skipped: unknown[];
  }>;

  setIsRemoteMissingDialogOpen: StateSetter<boolean>;
  setIsRemoteMissingPreviewLoading: StateSetter<boolean>;
  setIsResolvingRemoteMissing: StateSetter<boolean>;
  setRemoteMissingError: StateSetter<string | null>;
  setRemoteMissingPreview: StateSetter<BatchDeleteCentralSkillPreviewResult | null>;
  setRemoteMissingStates: StateSetter<CentralSkillUpdateState[]>;
  setSelectedSkillIds: StateSetter<string[]>;
}

export function createCentralSkillsUpdateWorkflow(
  deps: CentralSkillsUpdateWorkflowDeps
) {
  const {
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
  } = deps;

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

  async function handleCancelCentralUpdates() {
    try {
      await cancelCentralUpdates();
      toast.info(t("central.updateCancelRequested"));
    } catch (err) {
      toast.error(t("central.updateError", { error: String(err) }));
    }
  }

  async function handleCheckUpdates(skillIds?: string[]) {
    try {
      const states = await checkSkillUpdates(skillIds);
      const available = states.filter(
        (state) => state.status === "update_available"
      ).length;
      const unsupported = states.filter(
        (state) => state.status === "unsupported"
      ).length;
      const remoteMissing = states.filter(
        (state) => state.status === "remote_missing"
      );
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
      const succeededDeleteIds = new Set(
        deleteResult.succeeded.map((item) => item.skill_id)
      );
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

  return {
    handleCancelCentralUpdates,
    handleCheckUpdates,
    handleRemoteMissingDialogOpenChange,
    handleResolveRemoteMissing,
    handleUpdateSkills,
  };
}
