import type { MouseEvent, ReactNode } from "react";
import { useEffect, useRef, useState } from "react";
import { Loader2 } from "lucide-react";
import { cn } from "@/lib/utils";

interface InlineConfirmActionProps {
  idleAriaLabel: string;
  confirmLabel: string;
  onConfirm: () => void | Promise<void>;
  icon: ReactNode;
  disabled?: boolean;
  isLoading?: boolean;
  idleTitle?: string;
  className?: string;
}

export function InlineConfirmAction({
  idleAriaLabel,
  confirmLabel,
  onConfirm,
  icon,
  disabled = false,
  isLoading = false,
  idleTitle,
  className,
}: InlineConfirmActionProps) {
  const [isConfirming, setIsConfirming] = useState(false);
  const rootRef = useRef<HTMLDivElement | null>(null);
  const confirmButtonRef = useRef<HTMLButtonElement | null>(null);

  useEffect(() => {
    if (!isConfirming) return;

    function handlePointerDown(event: PointerEvent) {
      if (!rootRef.current?.contains(event.target as Node)) {
        setIsConfirming(false);
      }
    }

    document.addEventListener("pointerdown", handlePointerDown);
    return () => {
      document.removeEventListener("pointerdown", handlePointerDown);
    };
  }, [isConfirming]);

  useEffect(() => {
    if (disabled && !isLoading) {
      setIsConfirming(false);
    }
  }, [disabled, isLoading]);

  useEffect(() => {
    if (isConfirming) {
      confirmButtonRef.current?.focus();
    }
  }, [isConfirming]);

  function handleArmConfirm(event: MouseEvent<HTMLButtonElement>) {
    event.stopPropagation();
    if (disabled || isLoading) return;
    setIsConfirming(true);
  }

  async function handleConfirm(event: MouseEvent<HTMLButtonElement>) {
    event.stopPropagation();
    if (disabled || isLoading) return;

    try {
      await onConfirm();
    } finally {
      setIsConfirming(false);
    }
  }

  const showConfirmButton = isConfirming || isLoading;

  return (
    <div ref={rootRef} className="flex items-center shrink-0">
      {showConfirmButton ? (
        <button
          ref={confirmButtonRef}
          type="button"
          onClick={handleConfirm}
          disabled={disabled || isLoading}
          aria-label={confirmLabel}
          className={cn(
            "inline-flex h-8 items-center justify-center rounded-md px-2.5 text-xs font-medium transition-colors text-destructive bg-destructive/10 hover:bg-destructive/15 disabled:opacity-50 disabled:cursor-default",
            className
          )}
        >
          {isLoading ? (
            <span className="inline-flex items-center gap-1.5">
              <Loader2 className="size-3 animate-spin" />
              {confirmLabel}
            </span>
          ) : (
            confirmLabel
          )}
        </button>
      ) : (
        <button
          type="button"
          onClick={handleArmConfirm}
          disabled={disabled}
          title={idleTitle ?? idleAriaLabel}
          aria-label={idleAriaLabel}
          className={cn(
            "inline-flex h-8 w-8 items-center justify-center rounded-md transition-colors text-muted-foreground hover:text-destructive hover:bg-destructive/10 disabled:opacity-50 disabled:cursor-default",
            className
          )}
        >
          {icon}
        </button>
      )}
    </div>
  );
}
