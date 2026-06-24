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
  const [singleSkillId, setSingleSkillId] = useState<string | null>(null);
  const [isUninstalling, setIsUninstalling] = useState(false);
  const preview = useMemo(
    () =>
      createCentralBatchUninstallPreview(
        singleSkillId ? [singleSkillId] : selectedSkillIds,
        skills,
      ),
    [selectedSkillIds, singleSkillId, skills],
  );
  const handleOpenChange = useCallback((nextOpen: boolean) => {
    setOpen(nextOpen);
    if (!nextOpen) {
      setSingleSkillId(null);
    }
  }, []);
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
      onOpenChange: handleOpenChange,
      preview,
      isUninstalling,
    },
    openForSkill: (skillId: string) => {
      setSingleSkillId(skillId);
      setOpen(true);
    },
    bulkBar: {
      isUninstalling,
      onBatchUninstall: () => {
        setSingleSkillId(null);
        setOpen(true);
      },
    },
  };
}
