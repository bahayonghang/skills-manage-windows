import { CheckCircle2, Loader2, UploadCloud } from "lucide-react";
import { useTranslation } from "react-i18next";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogBody,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import type {
  LocalRemoteSyncApplyResult,
  LocalRemoteSyncItemPreview,
  LocalRemoteSyncPreview,
} from "@/types";

interface LocalRemoteSyncDialogProps {
  open: boolean;
  targetLabel: string;
  preview: LocalRemoteSyncPreview | null;
  result: LocalRemoteSyncApplyResult | null;
  isPreviewing: boolean;
  isApplying: boolean;
  error: string | null;
  onOpenChange: (open: boolean) => void;
  onPreview: () => void;
  onApply: () => void;
}

function formatBytes(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function changedSkillCount(preview: LocalRemoteSyncPreview | null) {
  return (
    preview?.skills.filter((item) => item.status === "add" || item.status === "update")
      .length ?? 0
  );
}

function SyncItemCard({ item }: { item: LocalRemoteSyncItemPreview }) {
  const { t } = useTranslation();
  return (
    <article className="rounded-xl border border-border bg-background p-3">
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div className="min-w-0">
          <div className="truncate text-sm font-medium text-foreground">{item.label}</div>
          <div className="mt-1 truncate font-mono text-xs text-muted-foreground">
            {item.remotePath}
          </div>
        </div>
        <span className="rounded-full border border-border px-2 py-0.5 text-xs text-muted-foreground">
          {t(`settings.localRemoteSync.status.${item.status}`)}
        </span>
      </div>
      <div className="mt-2 text-xs text-muted-foreground">
        {t("settings.localRemoteSync.itemSummary", {
          files: item.fileCount,
          bytes: formatBytes(item.byteCount),
          status: t(`settings.localRemoteSync.status.${item.status}`),
        })}
      </div>
      {item.error ? (
        <p className="mt-2 text-xs text-destructive" role="alert">
          {item.error}
        </p>
      ) : null}
    </article>
  );
}

export function LocalRemoteSyncDialog({
  open,
  targetLabel,
  preview,
  result,
  isPreviewing,
  isApplying,
  error,
  onOpenChange,
  onPreview,
  onApply,
}: LocalRemoteSyncDialogProps) {
  const { t } = useTranslation();
  const canApply = Boolean(preview) && !isPreviewing && !isApplying;
  const changed = changedSkillCount(preview);

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-4xl">
        <DialogHeader>
          <DialogTitle>{t("settings.localRemoteSync.title")}</DialogTitle>
          <DialogClose />
        </DialogHeader>
        <DialogBody className="max-h-[75vh] space-y-4">
          <DialogDescription>
            {t("settings.localRemoteSync.desc", { target: targetLabel })}
          </DialogDescription>

          {!preview ? (
            <div className="rounded-xl border border-dashed border-border bg-muted/20 p-4 text-sm text-muted-foreground">
              {isPreviewing ? (
                <span className="inline-flex items-center gap-2">
                  <Loader2 className="size-4 animate-spin" />
                  {t("settings.localRemoteSync.previewing")}
                </span>
              ) : (
                t("settings.localRemoteSync.previewEmpty")
              )}
            </div>
          ) : (
            <div className="space-y-4">
              <section className="space-y-2">
                <h3 className="text-sm font-semibold text-foreground">
                  {t("settings.localRemoteSync.repoTitle")}
                </h3>
                <SyncItemCard item={preview.repo} />
              </section>

              <section className="space-y-2">
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <h3 className="text-sm font-semibold text-foreground">
                    {t("settings.localRemoteSync.skillsTitle", {
                      count: preview.skills.length,
                    })}
                  </h3>
                  <span className="text-xs text-muted-foreground">
                    {t("settings.localRemoteSync.skillsSummary", {
                      changed,
                      total: preview.skills.length,
                    })}
                  </span>
                </div>
                {preview.skills.length > 0 ? (
                  <div className="max-h-72 space-y-2 overflow-auto pr-1">
                    {preview.skills.map((item) => (
                      <SyncItemCard key={item.id} item={item} />
                    ))}
                  </div>
                ) : (
                  <div className="rounded-xl border border-border bg-muted/20 p-3 text-sm text-muted-foreground">
                    {t("settings.localRemoteSync.noSkills")}
                  </div>
                )}
              </section>
            </div>
          )}

          {result ? (
            <div
              className={`rounded-xl border p-3 text-sm ${
                result.failed.length > 0
                  ? "border-amber-500/30 bg-amber-500/10 text-amber-700 dark:text-amber-300"
                  : "border-primary/25 bg-primary/10 text-primary"
              }`}
              role="status"
            >
              <CheckCircle2 className="mr-1 inline size-4" />
              {result.failed.length > 0
                ? t("settings.localRemoteSync.applyPartial", {
                    failed: result.failed.length,
                  })
                : t("settings.localRemoteSync.applySuccess", {
                    skills: result.syncedSkills.length,
                  })}
              {result.failed.length > 0 ? (
                <ul className="mt-2 list-disc space-y-1 pl-5 text-xs">
                  {result.failed.map((failure) => (
                    <li key={`${failure.id}:${failure.targetPath}`}>
                      {failure.label}: {failure.error}
                    </li>
                  ))}
                </ul>
              ) : null}
            </div>
          ) : null}

          {error ? (
            <p className="text-xs text-destructive" role="alert">
              {error}
            </p>
          ) : null}
        </DialogBody>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)} disabled={isApplying}>
            {t("common.cancel")}
          </Button>
          <Button variant="outline" onClick={onPreview} disabled={isPreviewing || isApplying}>
            {isPreviewing ? <Loader2 className="size-3.5 animate-spin" /> : null}
            {t("settings.localRemoteSync.preview")}
          </Button>
          <Button
            onClick={onApply}
            disabled={!canApply}
            data-testid="apply-local-remote-sync"
          >
            {isApplying ? (
              <Loader2 className="size-3.5 animate-spin" />
            ) : (
              <UploadCloud className="size-3.5" />
            )}
            {t("settings.localRemoteSync.apply")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
