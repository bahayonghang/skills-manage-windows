import type { ComponentType } from "react";
import { useTranslation } from "react-i18next";
import {
  AlertCircle,
  AlertTriangle,
  Ban,
  CheckCircle2,
  Info,
  XCircle,
} from "lucide-react";

import { OperationLogEntry } from "@/types";
import { cn } from "@/lib/utils";
import {
  formatDurationMs,
  formatLogAbsoluteTime,
  formatLogIsoTime,
  formatRelativeTime,
} from "@/components/logs/logsUtils";

export type LogsListDensity = "compact" | "comfortable";

interface LogsListRowProps {
  entry: OperationLogEntry;
  onOpen: (entry: OperationLogEntry) => void;
  density?: LogsListDensity;
  rowIndex?: number;
}

interface IconVisual {
  icon: ComponentType<{ className?: string }>;
  className: string;
}

function statusVisual(status: string): IconVisual {
  switch (status) {
    case "succeeded":
      return {
        icon: CheckCircle2,
        className: "bg-success/10 text-success-foreground",
      };
    case "failed":
      return { icon: XCircle, className: "bg-destructive/10 text-destructive-text" };
    case "partial":
      return {
        icon: AlertCircle,
        className: "bg-warning/10 text-warning-foreground",
      };
    case "cancelled":
      return { icon: Ban, className: "bg-muted text-muted-foreground" };
    default:
      return { icon: Info, className: "bg-muted text-muted-foreground" };
  }
}

function levelVisual(level: string): IconVisual {
  switch (level) {
    case "error":
      return { icon: XCircle, className: "text-destructive-text" };
    case "warn":
      return { icon: AlertTriangle, className: "text-warning-foreground" };
    default:
      return { icon: Info, className: "text-muted-foreground" };
  }
}

function rowAccentClass(entry: OperationLogEntry): string {
  if (entry.status === "failed" || entry.level === "error") {
    return "bg-destructive";
  }
  if (entry.status === "partial" || entry.level === "warn") {
    return "bg-warning";
  }
  return "bg-transparent";
}

export function LogsListRow({
  entry,
  onOpen,
  density = "comfortable",
  rowIndex,
}: LogsListRowProps) {
  const { t, i18n } = useTranslation();
  const subjectLabel = entry.subjectLabel ?? entry.subjectId ?? null;
  const hasError = Boolean(entry.errorSummary);
  const status = statusVisual(entry.status);
  const level = levelVisual(entry.level);
  const StatusIcon = status.icon;
  const LevelIcon = level.icon;
  const relative = formatRelativeTime(entry.createdAt, i18n.language);
  const absolute = formatLogAbsoluteTime(entry.createdAt);
  const iso = formatLogIsoTime(entry.createdAt);

  return (
    <button
      type="button"
      data-logs-row=""
      data-row-index={rowIndex}
      onClick={() => onOpen(entry)}
      className={cn(
        "group/log relative grid w-full grid-cols-[9rem_4.5rem_minmax(0,1fr)_minmax(0,1.6fr)_5rem_7rem] items-start gap-3 border-b border-border px-3 text-left text-sm transition-colors last:border-b-0 hover:bg-muted/40 focus-visible:bg-muted/40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/40 focus-visible:ring-inset",
        density === "compact" ? "py-1.5" : "py-2.5",
      )}
    >
      <span
        aria-hidden="true"
        className={cn("absolute inset-y-0 left-0 w-0.5", rowAccentClass(entry))}
      />
      <span className="flex flex-col" title={iso}>
        <span className="text-xs text-foreground">{relative}</span>
        <span className="font-mono text-ui-meta text-muted-foreground">
          {absolute}
        </span>
      </span>
      <span
        className={cn(
          "inline-flex items-center gap-1 text-xs font-medium",
          level.className,
        )}
      >
        <LevelIcon className="size-3.5" />
        <span>{entry.level}</span>
      </span>
      <span className="min-w-0">
        <span className="block truncate text-sm">
          {entry.targetLabel ?? entry.targetId}
        </span>
        <span className="text-xs text-muted-foreground">
          {entry.targetKind}
        </span>
      </span>
      <span className="min-w-0">
        <span className="block truncate font-mono text-xs">{entry.action}</span>
        <span className="block truncate text-xs text-muted-foreground">
          {entry.summary}
        </span>
        {subjectLabel && (
          <span className="block truncate text-ui-meta text-muted-foreground">
            {subjectLabel}
          </span>
        )}
        {hasError && (
          <span className="mt-1 line-clamp-2 block rounded-md border border-destructive/30 bg-destructive/10 px-2 py-1 text-xs text-destructive-text">
            <span className="font-medium">{t("logs.errorInline")}</span>
            <span className="ml-1">{entry.errorSummary}</span>
          </span>
        )}
      </span>
      <span className="text-xs tabular-nums text-muted-foreground">
        {formatDurationMs(entry.durationMs ?? null)}
      </span>
      <span
        className={cn(
          "inline-flex h-6 items-center gap-1 self-start rounded-full px-2 text-xs font-medium",
          status.className,
        )}
      >
        <StatusIcon className="size-3.5" />
        <span>
          {t(`logs.status.${entry.status}`, { defaultValue: entry.status })}
        </span>
      </span>
    </button>
  );
}
