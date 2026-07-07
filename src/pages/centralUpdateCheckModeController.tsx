import { useCallback, useEffect, useState } from "react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";

import { UpdateCheckModeDialog } from "@/components/central/UpdateCheckModeDialog";
import { UpdateModePreferenceSelect } from "@/components/central/UpdateModePreferenceSelect";
import type { CentralSkillsShellProps } from "@/components/central/CentralSkillsShell";
import type { CentralSkillsCheckButtonState } from "@/pages/centralSkillsCheckButton";
import {
  buildUpdateCheckRefreshContext,
  buildUpdateCheckScope,
  hasSyncableGitHubRepository,
  preferredUpdateCenterTab,
  type UpdateCheckMode,
} from "@/pages/centralUpdateCheckMode";
import { useSettingsStore } from "@/stores/settingsStore";
import { useUpdateCenterStore } from "@/stores/updateCenterStore";
import type { SkillRepositoryWithStats } from "@/types";

interface UseCentralUpdateCheckModeControllerInput {
  checkButtonState: CentralSkillsCheckButtonState;
  repositories: readonly SkillRepositoryWithStats[];
  disabled: boolean;
}

interface UseCentralUpdateCheckModeControllerResult {
  checkButton: CentralSkillsShellProps["checkButton"];
  modeControl: ReactNode;
  dialog: ReactNode;
}

export function useCentralUpdateCheckModeController({
  checkButtonState,
  repositories,
  disabled,
}: UseCentralUpdateCheckModeControllerInput): UseCentralUpdateCheckModeControllerResult {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);
  const refreshUpdateInventory = useUpdateCenterStore((state) => state.refresh);
  const openUpdateCenter = useUpdateCenterStore((state) => state.openDialog);
  const isUpdateCenterRefreshing = useUpdateCenterStore((state) => state.isRefreshing);
  const modePreference = useSettingsStore((state) => state.centralUpdateCheckMode);
  const modeLoaded = useSettingsStore((state) => state.centralUpdateCheckModeLoaded);
  const isSavingModePreference = useSettingsStore(
    (state) => state.isLoadingCentralUpdateCheckMode,
  );
  const loadModePreference = useSettingsStore((state) => state.loadCentralUpdateCheckMode);
  const setModePreference = useSettingsStore((state) => state.setCentralUpdateCheckMode);
  const syncableRepositoryAvailable = hasSyncableGitHubRepository(repositories);
  const syncDisabledReason = t("central.updateCheckMode.sync.disabledNoRepository");
  const checkButtonLabel =
    modePreference === "sync" && syncableRepositoryAvailable
      ? checkButtonState.syncLabel
      : checkButtonState.regularLabel;

  useEffect(() => {
    if (!modeLoaded) {
      void loadModePreference();
    }
  }, [loadModePreference, modeLoaded]);

  const handleConfirm = useCallback(
    async (mode: UpdateCheckMode) => {
      setSubmitError(null);
      setIsSubmitting(true);
      try {
        const scope = buildUpdateCheckScope(mode, checkButtonState);
        const inventory = await refreshUpdateInventory(scope);
        openUpdateCenter(
          preferredUpdateCenterTab(inventory),
          buildUpdateCheckRefreshContext(scope, checkButtonState),
        );
        setOpen(false);
      } catch (err) {
        const message = t("central.updateCheckError", { error: String(err) });
        setSubmitError(message);
        toast.error(message);
      } finally {
        setIsSubmitting(false);
      }
    },
    [checkButtonState, openUpdateCenter, refreshUpdateInventory, t],
  );

  const handleClick = useCallback(() => {
    setSubmitError(null);
    setOpen(true);
  }, []);

  const handleOpenChange = useCallback((nextOpen: boolean) => {
    setOpen(nextOpen);
    if (!nextOpen) {
      setSubmitError(null);
    }
  }, []);

  const handleModePreferenceChange = useCallback(
    (mode: UpdateCheckMode) => {
      if (mode === "sync" && !syncableRepositoryAvailable) {
        return;
      }
      void setModePreference(mode);
    },
    [setModePreference, syncableRepositoryAvailable],
  );

  const modeControlDisabled =
    disabled || isUpdateCenterRefreshing || isSubmitting || isSavingModePreference;

  return {
    checkButton: {
      label: checkButtonLabel,
      disabled:
        disabled ||
        isUpdateCenterRefreshing ||
        isSubmitting ||
        checkButtonState.targetSkillIds.length === 0,
      onClick: handleClick,
    },
    modeControl: (
      <UpdateModePreferenceSelect
        mode={modePreference}
        disabled={modeControlDisabled}
        syncDisabled={!syncableRepositoryAvailable}
        syncDisabledReason={syncDisabledReason}
        onChange={handleModePreferenceChange}
      />
    ),
    dialog: (
      <UpdateCheckModeDialog
        open={open}
        onOpenChange={handleOpenChange}
        mode={modePreference}
        regularScopeLabel={checkButtonState.regularLabel}
        syncScopeLabel={checkButtonState.syncLabel}
        isSubmitting={isSubmitting}
        syncDisabled={!syncableRepositoryAvailable}
        syncDisabledReason={syncDisabledReason}
        error={submitError}
        onConfirm={handleConfirm}
      />
    ),
  };
}
