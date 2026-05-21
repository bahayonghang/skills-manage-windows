import { UploadCloud } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";

interface RemoteTargetSyncButtonProps {
  targetId: string;
  onOpenLocalRemoteSync: (targetId: string) => void;
}

export function RemoteTargetSyncButton({
  targetId,
  onOpenLocalRemoteSync,
}: RemoteTargetSyncButtonProps) {
  const { t } = useTranslation();

  return (
    <Button
      type="button"
      variant="outline"
      size="sm"
      onClick={() => onOpenLocalRemoteSync(targetId)}
    >
      <UploadCloud className="size-3.5" />
      {t("settings.localRemoteSync.open")}
    </Button>
  );
}
