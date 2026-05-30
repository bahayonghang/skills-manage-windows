import { useCallback, useState } from "react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";

import {
  UpdateCheckModeDialog,
  type UpdateCheckMode,
} from "@/components/central/UpdateCheckModeDialog";
import type { CentralSkillsShellProps } from "@/components/central/CentralSkillsShell";
import type { CentralSkillsCheckButtonState } from "@/pages/centralSkillsCheckButton";
import {
  buildUpdateCheckRefreshContext,
  buildUpdateCheckScope,
  hasSyncableGitHubRepository,
  preferredUpdateCenterTab,
} from "@/pages/centralUpdateCheckMode";
import { useUpdateCenterStore } from "@/stores/updateCenterStore";
import type { SkillRepositoryWithStats } from "@/types";

interface UseCentralUpdateCheckModeControllerInput {
  checkButtonState: CentralSkillsCheckButtonState;
  repositories: readonly SkillRepositoryWithStats[];
  disabled: boolean;
}

interface UseCentralUpdateCheckModeControllerResult {
  checkButton: CentralSkillsShellProps["checkButton"];
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
  const refreshUpdateInventory = useUpdateCenterStore((state) => state.refresh);
  const openUpdateCenter = useUpdateCenterStore((state) => state.openDialog);
  const isUpdateCenterRefreshing = useUpdateCenterStore((state) => state.isRefreshing);
  const syncableRepositoryAvailable = hasSyncableGitHubRepository(repositories);

  const handleConfirm = useCallback(
    async (mode: UpdateCheckMode) => {
      setIsSubmitting(true);
      try {
        const scope = buildUpdateCheckScope(mode, checkButtonState);
        const inventory = await refreshUpdateInventory(scope);
        openUpdateCenter(
          preferredUpdateCenterTab(inventory),
          buildUpdateCheckRefreshContext(scope, checkButtonState),
        );
        setOpen(false);
      } finally {
        setIsSubmitting(false);
      }
    },
    [checkButtonState, openUpdateCenter, refreshUpdateInventory],
  );

  return {
    checkButton: {
      label: checkButtonState.label,
      disabled:
        disabled ||
        isUpdateCenterRefreshing ||
        isSubmitting ||
        checkButtonState.targetSkillIds.length === 0,
      onClick: () => setOpen(true),
    },
    dialog: (
      <UpdateCheckModeDialog
        open={open}
        onOpenChange={setOpen}
        scopeLabel={checkButtonState.label}
        isSubmitting={isSubmitting}
        syncDisabled={!syncableRepositoryAvailable}
        syncDisabledReason={t("central.updateCheckMode.sync.disabledNoRepository")}
        onConfirm={handleConfirm}
      />
    ),
  };
}
