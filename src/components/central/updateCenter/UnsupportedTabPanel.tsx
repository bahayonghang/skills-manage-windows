import { useTranslation } from "react-i18next";

import type { UnsupportedSkill } from "@/types/skillUpdateInventory";

interface UnsupportedTabPanelProps {
  items: UnsupportedSkill[];
}

export function UnsupportedTabPanel({ items }: UnsupportedTabPanelProps) {
  const { t } = useTranslation();

  return (
    <div className="space-y-3">
      <p className="text-xs text-muted-foreground">
        {t("central.updateCenter.unsupportedDescription")}
      </p>
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
