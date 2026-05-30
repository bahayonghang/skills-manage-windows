import { AlertTriangle, GitPullRequestArrow, RefreshCw } from "lucide-react";
import type { ReactNode } from "react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { cn } from "@/lib/utils";

export type UpdateCheckMode = "regular" | "sync";

interface UpdateCheckModeDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  scopeLabel: string;
  isSubmitting?: boolean;
  syncDisabled?: boolean;
  syncDisabledReason?: string;
  onConfirm: (mode: UpdateCheckMode) => void;
}

export function UpdateCheckModeDialog({
  open,
  onOpenChange,
  scopeLabel,
  isSubmitting = false,
  syncDisabled = false,
  syncDisabledReason,
  onConfirm,
}: UpdateCheckModeDialogProps) {
  const { t } = useTranslation();
  const [mode, setMode] = useState<UpdateCheckMode>("regular");

  useEffect(() => {
    if (open) {
      setMode("regular");
    }
  }, [open]);

  const effectiveMode = mode === "sync" && syncDisabled ? "regular" : mode;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-2xl" data-testid="update-check-mode-dialog">
        <DialogHeader>
          <DialogTitle>{t("central.updateCheckMode.title")}</DialogTitle>
          <DialogDescription>
            {t("central.updateCheckMode.description", { scope: scopeLabel })}
          </DialogDescription>
        </DialogHeader>

        <DialogBody className="space-y-3">
          <ModeCard
            checked={effectiveMode === "regular"}
            icon={<RefreshCw className="size-4" aria-hidden="true" />}
            title={t("central.updateCheckMode.regular.title")}
            description={t("central.updateCheckMode.regular.description")}
            bullets={[
              t("central.updateCheckMode.regular.scope"),
              t("central.updateCheckMode.regular.risk"),
            ]}
            disabled={isSubmitting}
            testId="update-check-mode-regular"
            onSelect={() => setMode("regular")}
          />
          <ModeCard
            checked={effectiveMode === "sync"}
            icon={<GitPullRequestArrow className="size-4" aria-hidden="true" />}
            title={t("central.updateCheckMode.sync.title")}
            description={t("central.updateCheckMode.sync.description")}
            bullets={[
              t("central.updateCheckMode.sync.scope"),
              t("central.updateCheckMode.sync.risk"),
            ]}
            disabled={isSubmitting || syncDisabled}
            disabledReason={syncDisabled ? syncDisabledReason : undefined}
            testId="update-check-mode-sync"
            onSelect={() => setMode("sync")}
          />
          <div className="flex items-start gap-2 rounded-lg border border-amber-500/30 bg-amber-500/10 px-3 py-2 text-xs text-amber-900 dark:text-amber-100">
            <AlertTriangle className="mt-0.5 size-3.5 shrink-0" aria-hidden="true" />
            <span>{t("central.updateCheckMode.note")}</span>
          </div>
        </DialogBody>

        <DialogFooter>
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={isSubmitting}
            onClick={() => onOpenChange(false)}
          >
            {t("common.cancel")}
          </Button>
          <Button
            type="button"
            size="sm"
            disabled={isSubmitting || (effectiveMode === "sync" && syncDisabled)}
            data-testid="confirm-update-check-mode"
            onClick={() => onConfirm(effectiveMode)}
          >
            {isSubmitting
              ? t("central.updateCheckMode.confirming")
              : t("central.updateCheckMode.confirm")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

interface ModeCardProps {
  checked: boolean;
  icon: ReactNode;
  title: string;
  description: string;
  bullets: string[];
  disabled?: boolean;
  disabledReason?: string;
  testId: string;
  onSelect: () => void;
}

function ModeCard({
  checked,
  icon,
  title,
  description,
  bullets,
  disabled = false,
  disabledReason,
  testId,
  onSelect,
}: ModeCardProps) {
  return (
    <button
      type="button"
      className={cn(
        "w-full rounded-xl border p-4 text-left transition-colors",
        checked
          ? "border-primary bg-primary/10 text-foreground"
          : "border-border bg-card hover:bg-muted/60",
        disabled && "cursor-not-allowed opacity-60 hover:bg-card",
      )}
      disabled={disabled}
      aria-pressed={checked}
      data-testid={testId}
      onClick={onSelect}
    >
      <div className="flex items-start gap-3">
        <span
          className={cn(
            "mt-0.5 inline-flex size-8 shrink-0 items-center justify-center rounded-full border",
            checked ? "border-primary bg-primary text-primary-foreground" : "border-border bg-muted",
          )}
        >
          {icon}
        </span>
        <span className="min-w-0 flex-1">
          <span className="block text-sm font-semibold">{title}</span>
          <span className="mt-1 block text-xs leading-5 text-muted-foreground">
            {description}
          </span>
          <span className="mt-2 block space-y-1 text-xs text-muted-foreground">
            {bullets.map((item) => (
              <span key={item} className="block">• {item}</span>
            ))}
          </span>
          {disabledReason ? (
            <span className="mt-2 block text-xs text-destructive">{disabledReason}</span>
          ) : null}
        </span>
      </div>
    </button>
  );
}
