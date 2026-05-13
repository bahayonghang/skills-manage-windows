import type { Dispatch, SetStateAction } from "react";
import type { TFunction } from "i18next";
import { toast } from "sonner";

import { usePlatformStore } from "@/stores/platformStore";
import { useCentralSkillsStore } from "@/stores/centralSkillsStore";
import type {
  BatchDeleteCentralSkillPreviewResult,
  BatchDeleteCentralSkillRequest,
  CentralSkillUpdateState,
} from "@/types";

type StateSetter<T> = Dispatch<SetStateAction<T>>;

export interface CentralSkillsUpdateWorkflowSetters {
  setIsUpdateConfirmDialogOpen: StateSetter<boolean>;
  setIsRemoteMissingDialogOpen: StateSetter<boolean>;
  setIsRemoteMissingPreviewLoading: StateSetter<boolean>;
  setIsResolvingRemoteMissing: StateSetter<boolean>;
  setPendingUpdateStates: StateSetter<CentralSkillUpdateState[]>;
  setQueuedRemoteMissingStates: StateSetter<CentralSkillUpdateState[]>;
  setRemoteMissingError: StateSetter<string | null>;
  setRemoteMissingPreview: StateSetter<BatchDeleteCentralSkillPreviewResult | null>;
  setRemoteMissingStates: StateSetter<CentralSkillUpdateState[]>;
  setSelectedSkillIds: StateSetter<string[]>;
}

export interface CentralSkillsUpdateWorkflowDeps {
  t: TFunction;
  state: {
    queuedRemoteMissingStates: CentralSkillUpdateState[];
  };
  setters: CentralSkillsUpdateWorkflowSetters;
}

export function useCentralSkillsUpdateWorkflow({
  t,
  state,
  setters,
}: CentralSkillsUpdateWorkflowDeps) {
  const cancelCentralUpdates = useCentralSkillsStore((store) => store.cancelCentralUpdates);
  const checkSkillUpdates = useCentralSkillsStore((store) => store.checkSkillUpdates);
  const deleteCentralSkills = useCentralSkillsStore((store) => store.deleteCentralSkills);
  const keepRemoteMissingSkills = useCentralSkillsStore((store) => store.keepRemoteMissingSkills);
  const loadBatchDeletePreview = useCentralSkillsStore((store) => store.loadBatchDeletePreview);
  const updateStatuses = useCentralSkillsStore((store) => store.updateStatuses);
  const updateSkills = useCentralSkillsStore((store) => store.updateSkills);
  const refreshCounts = usePlatformStore((store) => store.refreshCounts);

  const {
    setIsUpdateConfirmDialogOpen,
    setIsRemoteMissingDialogOpen,
    setIsRemoteMissingPreviewLoading,
    setIsResolvingRemoteMissing,
    setPendingUpdateStates,
    setQueuedRemoteMissingStates,
    setRemoteMissingError,
    setRemoteMissingPreview,
    setRemoteMissingStates,
    setSelectedSkillIds,
  } = setters;

  async function openRemoteMissingDialog(remoteMissing: CentralSkillUpdateState[]) {
    if (remoteMissing.length === 0) {
      return;
    }
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

  function openUpdateConfirmDialog(states: CentralSkillUpdateState[]) {
    const updatableStates = states.filter((item) => item.status === "update_available");
    if (updatableStates.length === 0) {
      return;
    }
    setPendingUpdateStates(updatableStates);
    setIsUpdateConfirmDialogOpen(true);
  }

  function handleUpdateConfirmDialogOpenChange(open: boolean) {
    setIsUpdateConfirmDialogOpen(open);
    if (open) {
      return;
    }
    setPendingUpdateStates([]);
    const queued = state.queuedRemoteMissingStates;
    if (queued.length > 0) {
      setQueuedRemoteMissingStates([]);
      void openRemoteMissingDialog(queued);
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
      const availableStates = states.filter(
        (state) => state.status === "update_available"
      );
      toast.success(
        t("central.updateCheckFinished", {
          available,
          unsupported,
          remoteMissing: remoteMissing.length,
          failed,
        })
      );
      if (remoteMissing.length > 0) {
        if (availableStates.length > 0) {
          setQueuedRemoteMissingStates(remoteMissing);
        } else {
          await openRemoteMissingDialog(remoteMissing);
        }
      }
      openUpdateConfirmDialog(availableStates);
    } catch (err) {
      toast.error(t("central.updateCheckError", { error: String(err) }));
    }
  }

  async function handleUpdateSkills(skillIds: string[]) {
    if (skillIds.length === 0) return;
    const states = skillIds
      .map((skillId) => updateStatuses[skillId])
      .filter((state): state is CentralSkillUpdateState => state?.status === "update_available");
    openUpdateConfirmDialog(states);
  }

  async function handleConfirmUpdateSkills(skillIds: string[]) {
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
      handleUpdateConfirmDialogOpenChange(false);
    } catch (err) {
      toast.error(t("central.updateError", { error: String(err) }));
      throw err;
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

      const deleteResult =
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
    handleConfirmUpdateSkills,
    handleUpdateConfirmDialogOpenChange,
    handleRemoteMissingDialogOpenChange,
    handleResolveRemoteMissing,
    handleUpdateSkills,
  };
}
