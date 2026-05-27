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

function countByStatus(preview: LocalRemoteSyncPreview | null) {
  const counts = { add: 0, update: 0, skip: 0, error: 0 };
  for (const item of preview?.skills ?? []) {
    counts[item.status] += 1;
  }
  return counts;
}

function hasSyncableChanges(preview: LocalRemoteSyncPreview | null) {
  if (!preview) return false;
  if (preview.repo.status === "add" || preview.repo.status === "update") return true;
  return preview.skills.some((item) => item.status === "add" || item.status === "update");
}

function hasItemErrors(preview: LocalRemoteSyncPreview | null) {
  if (!preview) return false;
  return preview.repo.status === "error" || preview.skills.some((item) => item.status === "error");
}

function SyncFlowSteps() {
  const { t } = useTranslation();
  const steps = [
    t("settings.localRemoteSync.flow.target"),
    t("settings.localRemoteSync.flow.preview"),
    t("settings.localRemoteSync.flow.apply"),
  ];

  return (
    <div className="grid gap-2 sm:grid-cols-3">
      {steps.map((step, index) => (
        <div
          key={step}
          className="rounded-xl border border-border bg-muted/20 px-3 py-2 text-xs text-muted-foreground"
        >
          <span className="mr-2 inline-grid size-5 place-items-center rounded-full border border-primary/30 bg-primary/10 text-[0.68rem] font-semibold text-primary">
            {index + 1}
          </span>
          {step}
        </div>
      ))}
    </div>
  );
}

function SyncPreviewSummary({ preview }: { preview: LocalRemoteSyncPreview }) {
  const { t } = useTranslation();
  const counts = countByStatus(preview);
  const changed = changedSkillCount(preview);
  const errors = hasItemErrors(preview);

  return (
    <div
      className={`rounded-xl border p-3 text-sm ${
        errors
          ? "border-amber-500/30 bg-amber-500/10 text-amber-700 dark:text-amber-300"
          : "border-primary/25 bg-primary/10 text-primary"
      }`}
      role="status"
    >
      {t("settings.localRemoteSync.previewSummary", {
        repoStatus: t(`settings.localRemoteSync.status.${preview.repo.status}`),
        total: preview.skills.length,
        changed,
        add: counts.add,
        update: counts.update,
        skip: counts.skip,
        error: counts.error,
        files: preview.totalFileCount,
        bytes: formatBytes(preview.totalByteCount),
      })}
      {errors ? (
        <p className="mt-1 text-xs">
          {t("settings.localRemoteSync.errorItemsWarning")}
        </p>
      ) : null}
    </div>
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
  const canApply =
    Boolean(preview) && hasSyncableChanges(preview) && !isPreviewing && !isApplying;
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
          <SyncFlowSteps />
          <p className="rounded-xl border border-border bg-muted/20 p-3 text-xs leading-5 text-muted-foreground">
            {t("settings.localRemoteSync.boundary")}
          </p>

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
              <SyncPreviewSummary preview={preview} />
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
          {preview && !hasSyncableChanges(preview) ? (
            <p className="text-xs text-muted-foreground" role="status">
              {t("settings.localRemoteSync.nothingToSync")}
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
