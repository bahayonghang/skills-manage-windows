import {
  AlertTriangle,
  GitPullRequestArrow,
  LoaderCircle,
  RefreshCw,
} from "lucide-react";
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
import type { UpdateCheckMode } from "@/pages/centralUpdateCheckMode";
import type { SkillUpdateInventoryRefreshProgress } from "@/types/skillUpdateInventory";

interface UpdateCheckModeDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  regularScopeLabel: string;
  syncScopeLabel: string;
  mode?: UpdateCheckMode;
  isSubmitting?: boolean;
  progress?: SkillUpdateInventoryRefreshProgress | null;
  syncDisabled?: boolean;
  syncDisabledReason?: string;
  error?: string | null;
  onConfirm: (mode: UpdateCheckMode) => void;
}

export function UpdateCheckModeDialog({
  open,
  onOpenChange,
  regularScopeLabel,
  syncScopeLabel,
  mode: initialMode = "regular",
  isSubmitting = false,
  progress = null,
  syncDisabled = false,
  syncDisabledReason,
  error = null,
  onConfirm,
}: UpdateCheckModeDialogProps) {
  const { t } = useTranslation();
  const [mode, setMode] = useState<UpdateCheckMode>("regular");

  useEffect(() => {
    if (open) {
      setMode(initialMode === "sync" && syncDisabled ? "regular" : initialMode);
    }
  }, [initialMode, open, syncDisabled]);

  const effectiveMode = mode === "sync" && syncDisabled ? "regular" : mode;
  const scopeLabel = effectiveMode === "sync" ? syncScopeLabel : regularScopeLabel;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className="sm:max-w-2xl"
        data-testid="update-check-mode-dialog"
      >
        <DialogHeader>
          <DialogTitle>{t("central.updateCheckMode.title")}</DialogTitle>
          <DialogDescription>
            {isSubmitting
              ? t("central.updateCheckMode.progress.scope", { scope: scopeLabel })
              : t("central.updateCheckMode.description", { scope: scopeLabel })}
          </DialogDescription>
        </DialogHeader>

        <DialogBody className={cn(isSubmitting ? "min-h-56" : "space-y-3")}>
          {isSubmitting ? (
            <UpdateCheckProgressView progress={progress} />
          ) : (
            <>
              <ModeCard
                checked={effectiveMode === "regular"}
                icon={<RefreshCw className="size-4" aria-hidden="true" />}
                title={t("central.updateCheckMode.regular.title")}
                description={t("central.updateCheckMode.regular.description")}
                bullets={[
                  t("central.updateCheckMode.regular.scope"),
                  t("central.updateCheckMode.regular.risk"),
                ]}
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
                disabled={syncDisabled}
                disabledReason={syncDisabled ? syncDisabledReason : undefined}
                testId="update-check-mode-sync"
                onSelect={() => setMode("sync")}
              />
              <div className="flex items-start gap-2 rounded-lg border border-warning/30 bg-warning/10 px-3 py-2 text-xs text-warning-foreground">
                <AlertTriangle
                  className="mt-0.5 size-3.5 shrink-0"
                  aria-hidden="true"
                />
                <span>{t("central.updateCheckMode.note")}</span>
              </div>
              {error ? (
                <div
                  className="flex items-start gap-2 rounded-lg border border-destructive/30 bg-destructive/10 px-3 py-2 text-xs text-destructive-text"
                  data-testid="update-check-mode-error"
                  role="alert"
                >
                  <AlertTriangle
                    className="mt-0.5 size-3.5 shrink-0"
                    aria-hidden="true"
                  />
                  <span>{error}</span>
                </div>
              ) : null}
            </>
          )}
        </DialogBody>

        {!isSubmitting ? (
          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={() => onOpenChange(false)}
            >
              {t("common.cancel")}
            </Button>
            <Button
              type="button"
              size="sm"
              disabled={effectiveMode === "sync" && syncDisabled}
              data-testid="confirm-update-check-mode"
              onClick={() => onConfirm(effectiveMode)}
            >
              {t("central.updateCheckMode.confirm")}
            </Button>
          </DialogFooter>
        ) : null}
      </DialogContent>
    </Dialog>
  );
}

function UpdateCheckProgressView({
  progress,
}: {
  progress: SkillUpdateInventoryRefreshProgress | null;
}) {
  const { t } = useTranslation();
  const total = progress?.total ?? 0;
  const completed = progress?.completed ?? 0;
  const determinate = total > 0;
  const percent = determinate
    ? Math.min(100, Math.round((completed / total) * 100))
    : 0;
  const status =
    progress?.phase === "finalizing"
      ? t("central.updateCheckMode.progress.finalizing")
      : determinate
        ? t("central.updateCheckMode.progress.checking", { completed, total })
        : t("central.updateCheckMode.progress.preparing");
  const ariaLabel = determinate
    ? t("central.updateCheckMode.progress.aria", { completed, total })
    : t("central.updateCheckMode.progress.ariaPreparing");

  return (
    <div
      className="flex min-h-56 flex-col justify-center gap-5 py-3"
      data-testid="update-check-progress-view"
    >
      <div className="space-y-2">
        <div className="flex items-center justify-between gap-3 text-sm">
          <span className="font-medium" aria-live="polite">
            {status}
          </span>
          {determinate ? (
            <span className="shrink-0 text-xs tabular-nums text-muted-foreground">
              {percent}%
            </span>
          ) : null}
        </div>
        <div
          className="h-2 overflow-hidden rounded-full bg-muted"
          role="progressbar"
          aria-label={ariaLabel}
          aria-valuemin={0}
          aria-valuemax={determinate ? total : undefined}
          aria-valuenow={determinate ? completed : undefined}
        >
          <div
            className={cn(
              "h-full rounded-full bg-primary",
              determinate
                ? "transition-[width] duration-300 ease-out"
                : "w-1/3 animate-pulse",
            )}
            style={determinate ? { width: `${percent}%` } : undefined}
          />
        </div>
      </div>

      {progress?.phase === "checking" && progress.activeRepositories.length > 0 ? (
        <div className="space-y-2">
          <div className="text-xs font-medium text-muted-foreground">
            {t("central.updateCheckMode.progress.activeRepositories")}
          </div>
          <ul
            className="grid gap-1.5 sm:grid-cols-2"
            aria-label={t("central.updateCheckMode.progress.activeRepositories")}
          >
            {progress.activeRepositories.map((repository) => (
              <li
                key={repository.key}
                className="flex min-w-0 items-center gap-2 rounded-md bg-muted/60 px-2.5 py-2 text-xs"
                title={repository.name}
              >
                <LoaderCircle
                  className="size-3.5 shrink-0 animate-spin text-primary"
                  aria-hidden="true"
                />
                <span className="min-w-0 truncate">{repository.name}</span>
              </li>
            ))}
          </ul>
        </div>
      ) : null}
    </div>
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
            checked
              ? "border-primary bg-primary text-primary-foreground"
              : "border-border bg-muted",
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
              <span key={item} className="block">
                • {item}
              </span>
            ))}
          </span>
          {disabledReason ? (
            <span className="mt-2 block text-xs text-destructive-text">
              {disabledReason}
            </span>
          ) : null}
        </span>
      </div>
    </button>
  );
}
