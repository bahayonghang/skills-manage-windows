import { useCallback } from "react";
import type { Dispatch, SetStateAction } from "react";
import type { TFunction } from "i18next";
import { toast } from "sonner";

import type { CentralStoreLocationChangeResult } from "@/types";

interface CentralStoreLocationControlsOptions {
  isRemoteTarget: boolean;
  setIsStoreLocationDialogOpen: Dispatch<SetStateAction<boolean>>;
  t: TFunction;
}

interface CentralStoreLocationAppliedOptions {
  loadCentralSkills: (options?: { throwOnError?: boolean }) => Promise<void>;
  refreshCounts: () => Promise<void>;
  t: TFunction;
}

export function createCentralStoreLocationControls({
  isRemoteTarget,
  setIsStoreLocationDialogOpen,
  t,
}: CentralStoreLocationControlsOptions) {
  return {
    disabled: isRemoteTarget,
    disabledReason: isRemoteTarget ? t("central.storeLocation.unsupportedTarget") : undefined,
    onOpen: () => {
      if (isRemoteTarget) {
        toast.info(t("central.storeLocation.unsupportedTarget"));
        return;
      }
      setIsStoreLocationDialogOpen(true);
    },
  };
}

export function useCentralStoreLocationApplied({
  loadCentralSkills,
  refreshCounts,
  t,
}: CentralStoreLocationAppliedOptions) {
  return useCallback(
    (result: CentralStoreLocationChangeResult) => {
      void Promise.all([loadCentralSkills(), refreshCounts()]);
      toast.success(
        t("central.storeLocation.success", {
          copied: result.copied,
          overwritten: result.overwritten,
          imported: result.targetOnlyImported,
          failed: result.symlinkRebuildFailed,
        })
      );
    },
    [loadCentralSkills, refreshCounts, t]
  );
}
