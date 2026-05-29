import { useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";

import { Button } from "@/components/ui/button";
import { useUpdateCenterStore } from "@/stores/updateCenterStore";

interface PlatformCleanupScanButtonsProps {
  agentId?: string;
  platformName: string;
  isLoading: boolean;
  onBeforeScan: () => Promise<void>;
}

export function PlatformCleanupScanButtons({
  agentId,
  platformName,
  isLoading,
  onBeforeScan,
}: PlatformCleanupScanButtonsProps) {
  const { t } = useTranslation();
  const scanDuplicates = useUpdateCenterStore((state) => state.scanDuplicates);
  const scanDeletedPlatformCopies = useUpdateCenterStore(
    (state) => state.scanDeletedPlatformCopies,
  );
  const openUpdateCenter = useUpdateCenterStore((state) => state.openDialog);
  const [isDuplicateScanning, setIsDuplicateScanning] = useState(false);
  const [isDeletedScanning, setIsDeletedScanning] = useState(false);
  const isBusy = isLoading || isDuplicateScanning || isDeletedScanning;

  async function handleScanDuplicates() {
    if (!agentId) return;
    setIsDuplicateScanning(true);
    try {
      await onBeforeScan();
      await scanDuplicates([agentId]);
      const groups =
        useUpdateCenterStore.getState().inventory?.platformDuplicates ?? [];
      if (groups.length === 0) {
        toast.info(t("platform.duplicatesNone"));
        return;
      }

      const rowCount = groups.reduce(
        (sum, group) => sum + group.writablePaths.length,
        0,
      );
      openUpdateCenter("duplicates");
      toast.success(
        t("platform.duplicatesFound", {
          skillCount: groups.length,
          rowCount,
        }),
      );
    } catch (err) {
      toast.error(t("platform.duplicatesScanError", { error: String(err) }));
    } finally {
      setIsDuplicateScanning(false);
    }
  }

  async function handleScanDeletedPlatformCopies() {
    if (!agentId) return;
    setIsDeletedScanning(true);
    try {
      await onBeforeScan();
      await scanDeletedPlatformCopies([agentId]);
      const groups =
        useUpdateCenterStore.getState().inventory?.deletedPlatformCopies ?? [];
      if (groups.length === 0) {
        toast.info(t("platform.deletedNone"));
        return;
      }

      const rowCount = groups.reduce(
        (sum, group) => sum + group.writablePaths.length,
        0,
      );
      openUpdateCenter("deletedPlatformCopies");
      toast.success(
        t("platform.deletedFound", {
          skillCount: groups.length,
          rowCount,
        }),
      );
    } catch (err) {
      toast.error(t("platform.deletedScanError", { error: String(err) }));
    } finally {
      setIsDeletedScanning(false);
    }
  }

  return (
    <>
      <Button
        type="button"
        variant="outline"
        disabled={!agentId || isBusy}
        onClick={() => void handleScanDuplicates()}
        aria-label={t("platform.scanDuplicatesLabel", {
          platform: platformName,
        })}
      >
        {isDuplicateScanning
          ? t("platform.scanningDuplicates")
          : t("platform.scanDuplicates")}
      </Button>
      <Button
        type="button"
        variant="outline"
        disabled={!agentId || isBusy}
        onClick={() => void handleScanDeletedPlatformCopies()}
        aria-label={t("platform.scanDeletedLabel", { platform: platformName })}
      >
        {isDeletedScanning
          ? t("platform.scanningDeleted")
          : t("platform.scanDeleted")}
      </Button>
    </>
  );
}
