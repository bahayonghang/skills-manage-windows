import type { ComponentType } from "react";
import {
  AlertCircle,
  AlertTriangle,
  Ban,
  CheckCircle2,
  Clock3,
  Copy,
  PauseCircle,
  Terminal,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";

import {
  buildOperationDiagnostic,
  safeCorrelationId,
  type OperationDiagnosticModel,
} from "@/components/logs/logDiagnostics";
import { formatLogAbsoluteTime } from "@/components/logs/logsUtils";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogTitle,
} from "@/components/ui/dialog";
import { formatBackendError } from "@/lib/backendError";
import { cn } from "@/lib/utils";
import type { OperationLogEntry } from "@/types";

interface OperationLogDetailDialogProps {
  open: boolean;
  entry: OperationLogEntry | null;
  onOpenChange: (open: boolean) => void;
  onInspectRuntime?: (operationId: string) => void;
}

interface StatusVisual {
  icon: ComponentType<{ className?: string }>;
  className: string;
}

function statusVisual(status: string): StatusVisual {
  switch (status) {
    case "succeeded":
      return {
        icon: CheckCircle2,
        className: "bg-success/10 text-success-foreground",
      };
    case "failed":
      return {
        icon: AlertCircle,
        className: "bg-destructive/10 text-destructive-text",
      };
    case "partial":
      return {
        icon: AlertTriangle,
        className: "bg-warning/10 text-warning-foreground",
      };
    case "cancelled":
      return { icon: Ban, className: "bg-muted text-muted-foreground" };
    case "interrupted":
      return {
        icon: PauseCircle,
        className: "bg-warning/10 text-warning-foreground",
      };
    default:
      return { icon: Clock3, className: "bg-info/10 text-info-foreground" };
  }
}

function DetailField({
  label,
  value,
  mono = false,
}: {
  label: string;
  value?: string | number | null;
  mono?: boolean;
}) {
  if (value === undefined || value === null || value === "") return null;
  return (
    <div className="min-w-0">
      <dt className="text-xs font-medium text-muted-foreground">{label}</dt>
      <dd
        className={cn(
          "mt-1 break-words text-sm text-foreground",
          mono && "break-all font-mono text-xs",
        )}
      >
        {value}
      </dd>
    </div>
  );
}

function nextActionKey(
  entry: OperationLogEntry,
  diagnostic: OperationDiagnosticModel,
): string {
  if (entry.status === "failed" && diagnostic.retryable) return "retryable";
  if (
    [
      "succeeded",
      "failed",
      "partial",
      "cancelled",
      "started",
      "interrupted",
    ].includes(entry.status)
  ) {
    return entry.status;
  }
  return "default";
}

function reasonKey(status: string): string {
  if (["succeeded", "started", "cancelled"].includes(status)) return status;
  return "unavailable";
}

function targetLabelKey(targetKind: string): string {
  return ["local", "ssh", "wsl"].includes(targetKind)
    ? `logs.targets.${targetKind}`
    : "logs.targets.unknown";
}

function statusLabelKey(status: string): string {
  return [
    "started",
    "succeeded",
    "failed",
    "partial",
    "cancelled",
    "interrupted",
  ].includes(status)
    ? `logs.status.${status}`
    : "logs.status.unknown";
}

export function OperationLogDetailDialog({
  open,
  entry,
  onOpenChange,
  onInspectRuntime,
}: OperationLogDetailDialogProps) {
  const { t } = useTranslation();
  const diagnostic = entry ? buildOperationDiagnostic(entry) : null;
  const visual = statusVisual(entry?.status ?? "started");
  const StatusIcon = visual.icon;
  const fallbackFailureMessage = t("backendErrors.central_updates.item_failed");
  const reviewedReason = diagnostic?.errorCode
    ? formatBackendError(
        {
          code: diagnostic.errorCode,
          message: t("logs.diagnostics.reasons.unavailable"),
          retryable: diagnostic.retryable ?? false,
        },
        t,
      )
    : null;

  async function handleCopy(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      toast.success(t("logs.copy.success"));
    } catch {
      toast.error(t("logs.copy.failure"));
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        data-testid="operation-log-detail-dialog"
        showCloseButton
        className="grid max-h-[calc(100dvh-2rem)] w-full grid-rows-[auto_minmax(0,1fr)] gap-0 overflow-hidden bg-background p-0 sm:max-w-[35rem]"
      >
        <header className="min-w-0 border-b border-border px-4 py-3 pr-12">
          <DialogTitle>{t("logs.detailTitle")}</DialogTitle>
          {entry ? (
            <div className="mt-1 flex min-w-0 items-center gap-1 text-xs text-muted-foreground">
              <span className="min-w-0 break-all font-mono">
                {diagnostic?.correlationId ?? t("logs.diagnostics.legacyId")}
              </span>
              {diagnostic?.correlationId ? (
                <Button
                  type="button"
                  variant="ghost"
                  size="icon-xs"
                  className="shrink-0"
                  aria-label={t("logs.copy.id")}
                  title={t("logs.copy.id")}
                  onClick={() =>
                    void handleCopy(diagnostic.correlationId as string)
                  }
                  data-testid="logs-detail-copy-id"
                >
                  <Copy />
                </Button>
              ) : null}
            </div>
          ) : null}
        </header>

        <DialogBody className="max-h-none min-h-0 overflow-y-auto px-4 py-4">
          {entry && diagnostic ? (
            <div className="space-y-4">
              <section className="rounded-lg border border-border bg-muted/20 p-3">
                <div className="flex flex-wrap items-center gap-2">
                  <span
                    className={cn(
                      "inline-flex h-6 items-center gap-1 rounded-full px-2 text-xs font-medium",
                      visual.className,
                    )}
                  >
                    <StatusIcon className="size-3.5" />
                    {t(statusLabelKey(entry.status))}
                  </span>
                  <span className="min-w-0 break-all font-mono text-xs text-muted-foreground">
                    {entry.action}
                  </span>
                </div>
                <h3 className="mt-2 text-base font-semibold text-foreground">
                  {t(`logs.diagnostics.statusSummaries.${entry.status}`, {
                    defaultValue: t("logs.diagnostics.statusSummaries.default"),
                  })}
                </h3>
              </section>

              <section className="grid gap-3 sm:grid-cols-2">
                <div className="rounded-lg border border-border p-3">
                  <h3 className="text-xs font-medium text-muted-foreground">
                    {t("logs.diagnostics.reason")}
                  </h3>
                  <p className="mt-1 text-sm text-foreground">
                    {reviewedReason ||
                      t(`logs.diagnostics.reasons.${reasonKey(entry.status)}`)}
                  </p>
                </div>
                <div className="rounded-lg border border-border p-3">
                  <h3 className="text-xs font-medium text-muted-foreground">
                    {t("logs.diagnostics.nextAction")}
                  </h3>
                  <p className="mt-1 text-sm text-foreground">
                    {t(
                      `logs.diagnostics.nextActions.${nextActionKey(entry, diagnostic)}`,
                    )}
                  </p>
                </div>
              </section>

              {diagnostic.correlationId && onInspectRuntime ? (
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  className="w-full justify-center"
                  onClick={() => onInspectRuntime(diagnostic.correlationId!)}
                  data-testid="logs-detail-view-runtime"
                >
                  <Terminal className="size-4" />
                  {t("logs.diagnostics.viewRuntime")}
                </Button>
              ) : null}

              <section>
                <h3 className="mb-2 text-xs font-semibold text-muted-foreground">
                  {t("logs.diagnostics.title")}
                </h3>
                <dl className="grid gap-3 rounded-lg border border-border p-3 sm:grid-cols-2">
                  <DetailField
                    label={t("logs.fields.operationId")}
                    value={
                      diagnostic.correlationId ?? t("logs.diagnostics.legacyId")
                    }
                    mono
                  />
                  <DetailField
                    label={t("logs.fields.errorCode")}
                    value={diagnostic.errorCode}
                    mono
                  />
                  <DetailField
                    label={t("logs.fields.errorCategory")}
                    value={diagnostic.errorCategory}
                    mono
                  />
                  <DetailField
                    label={t("logs.fields.phase")}
                    value={diagnostic.phase}
                    mono
                  />
                  <DetailField
                    label={t("logs.fields.retryable")}
                    value={
                      diagnostic.retryable === null
                        ? null
                        : t(
                            diagnostic.retryable
                              ? "logs.diagnostics.retryableYes"
                              : "logs.diagnostics.retryableNo",
                          )
                    }
                  />
                </dl>
              </section>

              <section>
                <h3 className="mb-2 text-xs font-semibold text-muted-foreground">
                  {t("logs.diagnostics.metadata")}
                </h3>
                <dl className="grid gap-3 rounded-lg border border-border p-3 sm:grid-cols-2">
                  <DetailField
                    label={t("logs.fields.createdAt")}
                    value={formatLogAbsoluteTime(entry.createdAt, {
                      withSeconds: true,
                    })}
                  />
                  <DetailField
                    label={t("logs.fields.duration")}
                    value={
                      entry.durationMs != null ? `${entry.durationMs} ms` : null
                    }
                  />
                  <DetailField
                    label={t("logs.fields.target")}
                    value={t(targetLabelKey(entry.targetKind))}
                  />
                  <DetailField
                    label={t("logs.fields.targetKind")}
                    value={entry.targetKind}
                    mono
                  />
                  <DetailField
                    label={t("logs.fields.batchId")}
                    value={safeCorrelationId(entry.batchId)}
                    mono
                  />
                </dl>
              </section>

              {diagnostic.failureRows.length > 0 ? (
                <section data-testid="logs-detail-failures">
                  <h3 className="mb-2 text-xs font-semibold text-muted-foreground">
                    {t("logs.fields.failures")}
                  </h3>
                  <ul className="space-y-2">
                    {diagnostic.failureRows.map((item, index) => (
                      <li
                        key={`${item.errorCode}-${index}`}
                        className="rounded-lg border border-border bg-muted/20 p-3 text-sm"
                      >
                        <div>
                          {formatBackendError(
                            {
                              code: item.errorCode,
                              message: fallbackFailureMessage,
                              retryable: false,
                            },
                            t,
                          )}
                        </div>
                      </li>
                    ))}
                  </ul>
                  {diagnostic.truncatedFailureCount > 0 ? (
                    <p className="mt-2 text-xs text-muted-foreground">
                      {t("logs.diagnostics.failuresTruncated", {
                        count: diagnostic.truncatedFailureCount,
                      })}
                    </p>
                  ) : null}
                </section>
              ) : null}

              <details className="rounded-lg border border-border">
                <summary className="cursor-pointer px-3 py-2 text-sm font-medium text-foreground">
                  {t("logs.diagnostics.structuredDetails")}
                </summary>
                <div className="border-t border-border p-3">
                  {diagnostic.detailsState === "available" &&
                  diagnostic.formattedDetails ? (
                    <>
                      <div className="mb-2 flex justify-end">
                        <Button
                          type="button"
                          variant="ghost"
                          size="xs"
                          onClick={() =>
                            void handleCopy(diagnostic.formattedDetails!)
                          }
                          data-testid="logs-detail-copy-json"
                        >
                          <Copy className="size-3.5" />
                          {t("logs.copy.json")}
                        </Button>
                      </div>
                      <pre className="rounded-lg bg-muted/30 p-3 font-mono text-xs leading-relaxed whitespace-pre-wrap break-all">
                        {diagnostic.formattedDetails}
                      </pre>
                    </>
                  ) : (
                    <p className="text-sm text-muted-foreground">
                      {t(
                        diagnostic.detailsState === "invalid"
                          ? "logs.diagnostics.detailsInvalid"
                          : "logs.diagnostics.detailsEmpty",
                      )}
                    </p>
                  )}
                </div>
              </details>
            </div>
          ) : (
            <div className="flex min-h-48 items-center justify-center text-sm text-muted-foreground">
              {t("logs.noDetail")}
            </div>
          )}
        </DialogBody>
      </DialogContent>
    </Dialog>
  );
}
