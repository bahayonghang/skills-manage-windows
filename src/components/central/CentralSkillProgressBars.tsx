import { AlertTriangle, Check, Download, Eye, Wand2, X } from "lucide-react";
import type { TFunction } from "i18next";

import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import type { AiTagJob, CentralSkillUpdateJob } from "@/types";

export function AiTagProgressBar({
  aiTagJob,
  t,
  onCancel,
  onViewReviews,
}: {
  aiTagJob: AiTagJob;
  t: TFunction;
  onCancel: () => void;
  onViewReviews: () => void;
}) {
  if (aiTagJob.status === "idle") return null;

  return (
    <div className="sticky bottom-0 z-10 border-t border-border bg-background/95 px-6 py-3 shadow-lg backdrop-blur">
      <div className="mb-2 flex flex-wrap items-center justify-between gap-3 text-xs">
        <div className="flex items-center gap-2 font-medium text-foreground">
          <Wand2 className="size-3.5" />
          <span>{t("central.aiTagProgressTitle")}</span>
          {aiTagJob.currentSkillName && aiTagJob.status === "running" && (
            <span className="text-muted-foreground">
              {t("central.aiTagCurrent", { name: aiTagJob.currentSkillName })}
            </span>
          )}
        </div>
        <div className="flex flex-wrap items-center gap-3 text-muted-foreground">
          <span>{t("central.aiTagCompleted", { completed: aiTagJob.completed, total: aiTagJob.total })}</span>
          <span>{t("central.aiTagSucceeded", { count: aiTagJob.succeeded })}</span>
          <span>{t("central.aiTagFailed", { count: aiTagJob.failed })}</span>
          <span>{t("central.aiTagLowConfidence", { count: aiTagJob.lowConfidenceCount })}</span>
          {aiTagJob.status === "completed" && (
            <button
              type="button"
              className="inline-flex items-center gap-1 text-foreground underline-offset-4 hover:underline"
              onClick={onViewReviews}
            >
              <Eye className="size-3.5" />
              {t("central.viewFailuresAndReviews")}
            </button>
          )}
          {aiTagJob.status === "running" && (
            <Button variant="outline" size="sm" onClick={onCancel}>
              <X className="size-3.5" />
              {t("central.cancelAiTagJob")}
            </Button>
          )}
        </div>
      </div>
      <div className="h-2 overflow-hidden rounded-full bg-muted">
        <div
          className="h-full w-full origin-left rounded-full bg-primary transition-transform duration-300 ease-out"
          style={{
            transform: `scaleX(${aiTagJob.total > 0 ? aiTagJob.completed / aiTagJob.total : 0})`,
          }}
        />
      </div>
      {aiTagJob.error && (
        <p className="mt-2 text-xs text-destructive">{aiTagJob.error}</p>
      )}
    </div>
  );
}

export function CentralUpdateProgressBar({
  isDismissible,
  t,
  updateJob,
  updateProgressKey,
  updateProgressRatio,
  onCancel,
  onDismiss,
}: {
  isDismissible: boolean;
  t: TFunction;
  updateJob: CentralSkillUpdateJob;
  updateProgressKey: string;
  updateProgressRatio: number;
  onCancel?: () => void;
  onDismiss: (key: string) => void;
}) {
  return (
    <div
      className={cn(
        "sticky bottom-0 z-10 border-t bg-background/95 px-6 py-3 shadow-lg backdrop-blur",
        updateJob.status === "failed" ? "border-destructive/30" : "border-border"
      )}
    >
      <div className="mb-2 flex flex-wrap items-center justify-between gap-3 text-xs">
        <div className="flex min-w-0 items-center gap-2 font-medium text-foreground">
          <span
            className={cn(
              "grid size-6 shrink-0 place-items-center rounded-full ring-1",
              updateJob.status === "failed"
                ? "bg-destructive/10 text-destructive ring-destructive/20"
                : updateJob.status === "completed"
                  ? "bg-emerald-500/10 text-emerald-600 ring-emerald-500/20 dark:text-emerald-300"
                  : "bg-primary/10 text-primary ring-primary/20"
            )}
          >
            {updateJob.status === "failed" ? (
              <AlertTriangle className="size-3.5" />
            ) : updateJob.status === "completed" ? (
              <Check className="size-3.5" />
            ) : (
              <Download className="size-3.5" />
            )}
          </span>
          <div className="min-w-0">
            <div className="flex flex-wrap items-center gap-x-2 gap-y-1">
              <span>{t("central.updateProgressTitle")}</span>
              {updateJob.status === "completed" && (
                <span className="rounded-full bg-emerald-500/10 px-2 py-0.5 text-[11px] text-emerald-700 dark:text-emerald-300">
                  {t("central.updateProgressFinished")}
                </span>
              )}
              {updateJob.status === "failed" && (
                <span className="rounded-full bg-destructive/10 px-2 py-0.5 text-[11px] text-destructive">
                  {t("central.updateProgressFailedStatus")}
                </span>
              )}
              {updateJob.status === "cancelled" && (
                <span className="rounded-full bg-amber-500/10 px-2 py-0.5 text-[11px] text-amber-700 dark:text-amber-300">
                  {t("central.updateProgressCancelledStatus")}
                </span>
              )}
            </div>
            {updateJob.currentSkillName && updateJob.status === "running" && (
              <div className="mt-0.5 truncate text-muted-foreground">
                {t("central.updateCurrent", { name: updateJob.currentSkillName })}
              </div>
            )}
          </div>
        </div>
        <div className="flex flex-wrap items-center gap-2 text-muted-foreground">
          <span className="rounded-full bg-muted/70 px-2 py-1">
            {t("central.updateCompleted", { completed: updateJob.completed, total: updateJob.total })}
          </span>
          <span className="rounded-full bg-emerald-500/10 px-2 py-1 text-emerald-700 dark:text-emerald-300">
            {t("central.updateSucceeded", { count: updateJob.succeeded })}
          </span>
          <span className="rounded-full bg-amber-500/10 px-2 py-1 text-amber-700 dark:text-amber-300">
            {t("central.updateSkipped", { count: updateJob.skipped })}
          </span>
          <span
            className={cn(
              "rounded-full px-2 py-1",
              updateJob.failed > 0
                ? "bg-destructive/10 text-destructive"
                : "bg-muted/70 text-muted-foreground"
            )}
          >
            {t("central.updateFailed", { count: updateJob.failed })}
          </span>
          {onCancel && (updateJob.status === "running" || updateJob.status === "cancelling") && (
            <Button
              variant="outline"
              size="sm"
              onClick={onCancel}
              disabled={updateJob.status === "cancelling"}
            >
              <X className="size-3.5" />
              {updateJob.status === "cancelling"
                ? t("central.cancellingUpdateJob")
                : t("central.cancelUpdateJob")}
            </Button>
          )}
          {isDismissible && (
            <Button
              variant="ghost"
              size="icon"
              className="size-7 rounded-full text-muted-foreground hover:text-foreground"
              aria-label={t("central.closeUpdateProgress")}
              title={t("central.closeUpdateProgress")}
              onClick={() => onDismiss(updateProgressKey)}
            >
              <X className="size-3.5" />
            </Button>
          )}
        </div>
      </div>
      <div className="h-2 overflow-hidden rounded-full bg-muted">
        <div
          className={cn(
            "h-full w-full origin-left rounded-full transition-transform duration-300 ease-out",
            updateJob.status === "failed" ? "bg-destructive" : "bg-primary"
          )}
          style={{
            transform: `scaleX(${updateProgressRatio})`,
          }}
        />
      </div>
      {updateJob.error && (
        <p
          className={cn(
            "mt-2 text-xs",
            updateJob.failed > 0 || updateJob.status === "failed"
              ? "text-destructive"
              : "text-amber-700 dark:text-amber-300"
          )}
        >
          {updateJob.error}
        </p>
      )}
    </div>
  );
}
