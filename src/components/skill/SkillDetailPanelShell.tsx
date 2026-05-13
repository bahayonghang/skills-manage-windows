import { type ReactNode, type RefObject, useEffect } from "react";
import { Dialog as DialogPrimitive } from "@base-ui/react/dialog";
import {
  Dialog,
  DialogClose,
  DialogOverlay,
  DialogPortal,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { XIcon } from "lucide-react";
import { cn } from "@/lib/utils";

export interface SkillDetailPanelShellProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  returnFocusRef?: RefObject<HTMLElement | null>;
  titleId?: string;
  children?: ReactNode;
}

export function SkillDetailPanelShell({
  open,
  onOpenChange,
  returnFocusRef,
  titleId,
  children,
}: SkillDetailPanelShellProps) {
  useEffect(() => {
    if (open) {
      return;
    }
    (returnFocusRef?.current ?? document.body)?.focus?.();
  }, [open, returnFocusRef]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogPortal keepMounted={false}>
        <DialogOverlay
          data-testid="skill-detail-drawer-overlay"
          className="bg-foreground/30"
        />
        <DialogPrimitive.Popup
          role="dialog"
          aria-modal="true"
          aria-labelledby={children ? titleId : undefined}
          data-testid="skill-detail-drawer"
          className={cn(
            "fixed inset-y-0 right-0 z-50 flex h-full w-screen flex-col bg-background shadow-2xl ring-1 ring-border outline-none",
            "md:w-[min(900px,90vw)]"
          )}
        >
          <div className="flex h-10 shrink-0 items-center justify-end border-b border-border px-2">
            <DialogClose
              render={
                <Button
                  type="button"
                  variant="ghost"
                  size="icon-sm"
                  aria-label="Close"
                />
              }
            >
              <XIcon />
            </DialogClose>
          </div>
          <div className="min-h-0 flex-1">{children ?? null}</div>
        </DialogPrimitive.Popup>
      </DialogPortal>
    </Dialog>
  );
}
