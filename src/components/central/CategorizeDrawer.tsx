import { Dialog as DialogPrimitive } from "@base-ui/react/dialog";
import { XIcon } from "lucide-react";
import type { TFunction } from "i18next";

import { Button } from "@/components/ui/button";
import {
  CentralSkillCategorizePanel,
  type CentralSkillCategorizePanelProps,
} from "@/components/central/CentralSkillCategorizePanel";
import {
  Dialog,
  DialogClose,
  DialogOverlay,
  DialogPortal,
  DialogTitle,
} from "@/components/ui/dialog";

/**
 * CategorizeDrawer：右侧抽屉装载 CentralSkillCategorizePanel 内容。
 * - 仅在 open=true 时挂载（DialogPortal keepMounted=false）。
 * - 标题区由抽屉自己拼，PaneltClient 只关心三段内容。
 */
export interface CategorizeDrawerProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  t: TFunction;
  /** 透传给内部 Categorize 面板。 */
  panelProps: Omit<CentralSkillCategorizePanelProps, "t">;
}

export function CategorizeDrawer({
  open,
  onOpenChange,
  t,
  panelProps,
}: CategorizeDrawerProps) {
  const titleId = "central-categorize-drawer-title";

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogPortal keepMounted={false}>
        <DialogOverlay className="bg-overlay" />
        <DialogPrimitive.Popup
          data-testid="central-categorize-drawer"
          role="dialog"
          aria-modal="true"
          aria-labelledby={titleId}
          className="fixed inset-y-0 right-0 z-50 flex h-full w-screen flex-col bg-background shadow-2xl ring-1 ring-border outline-none sm:w-[min(440px,96vw)]"
        >
          <div className="flex shrink-0 items-start justify-between gap-3 border-b border-border p-4">
            <div className="min-w-0 space-y-1">
              <DialogTitle id={titleId}>
                {t("central.categorizePanelTitle")}
              </DialogTitle>
              <p className="text-sm text-muted-foreground">
                {t("central.categorizePanelDesc")}
              </p>
            </div>
            <DialogClose
              render={
                <Button
                  type="button"
                  variant="ghost"
                  size="icon-sm"
                  aria-label={t("common.close")}
                />
              }
            >
              <XIcon />
            </DialogClose>
          </div>
          <CentralSkillCategorizePanel {...panelProps} t={t} />
        </DialogPrimitive.Popup>
      </DialogPortal>
    </Dialog>
  );
}
