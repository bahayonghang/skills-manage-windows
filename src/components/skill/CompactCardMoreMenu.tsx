import { Menu as MenuPrimitive } from "@base-ui/react/menu";
import { MoreHorizontal, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";

import { cn } from "@/lib/utils";

interface CompactCardMoreMenuProps {
  skillName: string;
  isLoading: boolean;
  onDeleteFromCentral: () => void;
}

export function CompactCardMoreMenu({
  skillName,
  isLoading,
  onDeleteFromCentral,
}: CompactCardMoreMenuProps) {
  const { t } = useTranslation();
  return (
    <MenuPrimitive.Root>
      <MenuPrimitive.Trigger
        render={
          <button
            type="button"
            disabled={isLoading}
            aria-label={t("common.skillCardMoreActions")}
            title={t("common.skillCardMoreActions")}
            data-testid={`skill-card-more-${skillName}`}
            className="inline-flex h-8 w-8 items-center justify-center rounded-md transition-colors text-muted-foreground hover:bg-muted/60 hover:text-foreground disabled:opacity-50 disabled:cursor-default"
          >
            <MoreHorizontal className="size-4" />
          </button>
        }
      />
      <MenuPrimitive.Portal>
        <MenuPrimitive.Positioner align="end" sideOffset={4} className="z-50 outline-none">
          <MenuPrimitive.Popup
            className={cn(
              "min-w-[180px] rounded-lg bg-popover p-1 text-sm text-popover-foreground shadow-md ring-1 ring-foreground/10 outline-none",
              "data-[starting-style]:animate-in data-[starting-style]:fade-in-0 data-[starting-style]:zoom-in-95",
              "data-[ending-style]:animate-out data-[ending-style]:fade-out-0 data-[ending-style]:zoom-out-95",
              "animation-duration-100"
            )}
          >
            <MenuPrimitive.Item
              onClick={onDeleteFromCentral}
              data-testid={`delete-central-skill-${skillName}`}
              className="flex cursor-pointer items-center gap-2 rounded-md px-2.5 py-1.5 text-destructive-text outline-none data-[highlighted]:bg-destructive/10"
            >
              <Trash2 className="size-3.5 shrink-0" />
              {t("central.deleteSkill")}
            </MenuPrimitive.Item>
          </MenuPrimitive.Popup>
        </MenuPrimitive.Positioner>
      </MenuPrimitive.Portal>
    </MenuPrimitive.Root>
  );
}
