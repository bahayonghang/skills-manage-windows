import { useCallback, useMemo, useState } from "react";

import {
  createCentralBatchUninstallPreview,
  type CentralBatchUninstallApplyResult,
  type CentralBatchUninstallPreview,
} from "@/lib/centralBatchUninstall";
import type { SkillWithLinks } from "@/types";

export function useCentralBatchUninstallView({
  selectedSkillIds,
  skills,
  onConfirm,
}: {
  selectedSkillIds: string[];
  skills: SkillWithLinks[];
  onConfirm: (
    preview: CentralBatchUninstallPreview,
  ) => Promise<CentralBatchUninstallApplyResult>;
}) {
  const [open, setOpen] = useState(false);
  const [isUninstalling, setIsUninstalling] = useState(false);
  const preview = useMemo(
    () => createCentralBatchUninstallPreview(selectedSkillIds, skills),
    [selectedSkillIds, skills],
  );
  const confirm = useCallback(
    async (nextPreview: CentralBatchUninstallPreview) => {
      setIsUninstalling(true);
      try {
        return await onConfirm(nextPreview);
      } finally {
        setIsUninstalling(false);
      }
    },
    [onConfirm],
  );

  return {
    dialog: {
      open,
      onConfirm: confirm,
      onOpenChange: setOpen,
      preview,
      isUninstalling,
    },
    bulkBar: {
      isUninstalling,
      onBatchUninstall: () => setOpen(true),
    },
  };
}
