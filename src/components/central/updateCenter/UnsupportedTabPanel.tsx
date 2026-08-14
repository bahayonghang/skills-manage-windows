import { useTranslation } from "react-i18next";
import { Trash2 } from "lucide-react";

import { Button } from "@/components/ui/button";
import type { UnsupportedSkill } from "@/types/skillUpdateInventory";

interface UnsupportedTabPanelProps {
  items: UnsupportedSkill[];
  onResetUnknownSource?: () => void;
  resetDisabled?: boolean;
}

export function UnsupportedTabPanel({
  items,
  onResetUnknownSource,
  resetDisabled = false,
}: UnsupportedTabPanelProps) {
  const { t } = useTranslation();

  return (
    <div className="space-y-3">
      <div className="flex items-start justify-between gap-3">
        <p className="text-xs text-muted-foreground">
          {t("central.updateCenter.unsupportedDescription")}
        </p>
        {onResetUnknownSource ? (
          <Button
            size="sm"
            variant="destructive"
            className="shrink-0"
            disabled={resetDisabled}
            onClick={onResetUnknownSource}
            data-testid="reset-unknown-source-skills"
          >
            <Trash2 className="size-3.5" />
            {t("central.updateCenter.resetUnknownSource")}
          </Button>
        ) : null}
      </div>
      <div className="space-y-2">
        {items.map((item) => (
          <div
            key={item.skillId}
            className="rounded-lg border border-border bg-muted/30 p-3"
          >
            <div className="break-all text-sm font-medium">{item.skillId}</div>
            <p className="mt-1 text-xs text-muted-foreground">
              {t(`central.updateCenter.unsupportedReasons.${item.reasonCode}`)}
            </p>
          </div>
        ))}
      </div>
    </div>
  );
}
