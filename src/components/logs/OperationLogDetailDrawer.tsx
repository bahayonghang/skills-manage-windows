import { Dialog as DialogPrimitive } from "@base-ui/react/dialog";
import { Copy, XIcon } from "lucide-react";
import { useId } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";

import {
  Dialog,
  DialogClose,
  DialogOverlay,
  DialogPortal,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { OperationLogEntry } from "@/types";
import { cn } from "@/lib/utils";

interface OperationLogDetailDrawerProps {
  open: boolean;
  entry: OperationLogEntry | null;
  onOpenChange: (open: boolean) => void;
}

function formatJson(detailsJson?: string | null): string | null {
  if (!detailsJson) return null;
  try {
    return JSON.stringify(JSON.parse(detailsJson), null, 2);
  } catch {
    return detailsJson;
  }
}

function DetailField({
  label,
  value,
}: {
  label: string;
  value?: string | number | null;
}) {
  if (value === undefined || value === null || value === "") return null;
  return (
    <div>
      <dt className="text-xs font-medium text-muted-foreground">{label}</dt>
      <dd className="mt-1 break-words text-sm text-foreground">{value}</dd>
    </div>
  );
}

export function OperationLogDetailDrawer({
  open,
  entry,
  onOpenChange,
}: OperationLogDetailDrawerProps) {
  const { t } = useTranslation();
  const titleId = useId();
  const formattedDetails = formatJson(entry?.detailsJson);

  async function handleCopy(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      toast.success(t("logs.copy.success"));
    } catch (err) {
      toast.error(t("logs.copy.failure", { error: String(err) }));
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogPortal keepMounted={false}>
        <DialogOverlay className="bg-foreground/30" />
        <DialogPrimitive.Popup
          role="dialog"
          aria-modal="true"
          aria-labelledby={entry ? titleId : undefined}
          data-testid="operation-log-detail-drawer"
          className={cn(
            "fixed inset-y-0 right-0 z-50 flex h-full w-screen flex-col bg-background shadow-2xl ring-1 ring-border outline-none",
            "md:w-[min(720px,88vw)]"
          )}
        >
          <div className="flex h-10 shrink-0 items-center justify-between border-b border-border px-3">
            <h2 id={titleId} className="truncate text-sm font-semibold">
              {t("logs.detailTitle")}
            </h2>
            <div className="flex items-center gap-1">
              {entry && (
                <Button
                  type="button"
                  variant="ghost"
                  size="icon-sm"
                  aria-label={t("logs.copy.id")}
                  title={t("logs.copy.id")}
                  onClick={() => handleCopy(entry.id)}
                  data-testid="logs-detail-copy-id"
                >
                  <Copy />
                </Button>
              )}
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
          </div>

          {entry ? (
            <div className="min-h-0 flex-1 overflow-y-auto p-5">
              <div className="rounded-lg border border-border bg-muted/20 p-4">
                <div className="text-xs uppercase tracking-wide text-muted-foreground">
                  {entry.action}
                </div>
                <div className="mt-2 text-lg font-semibold text-foreground">
                  {entry.summary}
                </div>
                {entry.errorSummary && (
                  <div className="mt-3 rounded-md border border-destructive/30 bg-destructive/10 p-3 text-sm text-destructive-text">
                    {entry.errorSummary}
                  </div>
                )}
              </div>

              <dl className="mt-5 grid gap-4 sm:grid-cols-2">
                <DetailField label={t("logs.fields.createdAt")} value={entry.createdAt} />
                <DetailField label={t("logs.fields.level")} value={entry.level} />
                <DetailField label={t("logs.fields.status")} value={entry.status} />
                <DetailField label={t("logs.fields.category")} value={entry.category} />
                <DetailField label={t("logs.fields.target")} value={entry.targetLabel ?? entry.targetId} />
                <DetailField label={t("logs.fields.targetKind")} value={entry.targetKind} />
                <DetailField label={t("logs.fields.subjectType")} value={entry.subjectType} />
                <DetailField label={t("logs.fields.subject")} value={entry.subjectLabel ?? entry.subjectId} />
                <DetailField label={t("logs.fields.duration")} value={entry.durationMs != null ? `${entry.durationMs} ms` : null} />
                <DetailField label={t("logs.fields.batchId")} value={entry.batchId} />
              </dl>

              {formattedDetails && (
                <div className="mt-5">
                  <div className="mb-2 flex items-center justify-between gap-3">
                    <div className="text-xs font-medium text-muted-foreground">
                      {t("logs.fields.details")}
                    </div>
                    <Button
                      type="button"
                      variant="ghost"
                      size="xs"
                      onClick={() => handleCopy(formattedDetails)}
                      data-testid="logs-detail-copy-json"
                    >
                      <Copy className="size-3.5" />
                      {t("logs.copy.json")}
                    </Button>
                  </div>
                  <pre className="max-h-[38vh] overflow-auto rounded-lg border border-border bg-muted/30 p-3 text-xs leading-relaxed">
                    {formattedDetails}
                  </pre>
                </div>
              )}
            </div>
          ) : (
            <div className="flex flex-1 items-center justify-center text-sm text-muted-foreground">
              {t("logs.noDetail")}
            </div>
          )}
        </DialogPrimitive.Popup>
      </DialogPortal>
    </Dialog>
  );
}
