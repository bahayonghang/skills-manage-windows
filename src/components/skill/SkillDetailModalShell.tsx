import { type ReactNode, type RefObject, useEffect } from "react";
import { Dialog as DialogPrimitive } from "@base-ui/react/dialog";
import { useTranslation } from "react-i18next";
import {
  Dialog,
  DialogClose,
  DialogOverlay,
  DialogPortal,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { XIcon } from "lucide-react";
import { cn } from "@/lib/utils";

export interface SkillDetailModalShellProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  returnFocusRef?: RefObject<HTMLElement | null>;
  titleId?: string;
  children?: ReactNode;

  /** 可选：自定义弹窗最大宽度，CSS 长度值。默认 "1200px" */
  maxWidth?: string;
  /** 可选：自定义弹窗最大高度，CSS 长度值。默认 "800px" */
  maxHeight?: string;

  /** 可选：头部右侧额外操作区域（如 Install 按钮） */
  headerActions?: ReactNode;
}

export function SkillDetailModalShell({
  open,
  onOpenChange,
  returnFocusRef,
  titleId,
  children,
  maxWidth,
  maxHeight,
  headerActions,
}: SkillDetailModalShellProps) {
  const { t } = useTranslation();

  useEffect(() => {
    if (open) {
      return;
    }
    const el = returnFocusRef?.current ?? document.body;
    el?.focus?.();
  }, [open, returnFocusRef]);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogPortal keepMounted={false}>
        <DialogOverlay
          data-testid="skill-detail-modal-overlay"
          className="bg-foreground/50"
        />
        <DialogPrimitive.Popup
          role="dialog"
          aria-modal="true"
          aria-labelledby={children ? titleId : undefined}
          data-testid="skill-detail-modal"
          className={cn(
            // 居中定位
            "fixed top-1/2 left-1/2 z-50 -translate-x-1/2 -translate-y-1/2",
            // 弹性布局
            "flex flex-col",
            // 尺寸约束
            "w-[min(90vw,var(--modal-max-w))] min-w-[360px]",
            "h-[min(85vh,var(--modal-max-h))] min-h-[400px]",
            // 视觉样式
            "rounded-xl bg-background shadow-2xl ring-1 ring-border outline-none",
            // 动画
            "will-change-transform",
            "data-[starting-style]:animate-in data-[starting-style]:fade-in-0 data-[starting-style]:zoom-in-95",
            "data-[ending-style]:animate-out data-[ending-style]:fade-out-0 data-[ending-style]:zoom-out-95",
            "animation-duration-150 data-[ending-style]:animation-duration-100",
            // 响应式宽度覆盖
            "lg:w-[min(70vw,var(--modal-max-w))]",
            "sm:max-lg:w-[85vw]",
            "max-sm:w-[95vw] max-sm:h-[90vh]"
          )}
          style={
            {
              "--modal-max-w": maxWidth ?? "1200px",
              "--modal-max-h": maxHeight ?? "800px",
            } as React.CSSProperties
          }
        >
          {/* 头部栏 */}
          <div className="flex h-10 shrink-0 items-center justify-end gap-2 border-b border-border px-3">
            {headerActions}
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
          {/* 内容区 */}
          <div className="min-h-0 flex-1 overflow-hidden">
            {children ?? null}
          </div>
        </DialogPrimitive.Popup>
      </DialogPortal>
    </Dialog>
  );
}
