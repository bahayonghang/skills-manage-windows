import type { ReactNode } from "react";

import { ChevronRight } from "lucide-react";

import { CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { cn } from "@/lib/utils";
import type { OperationLogEntry } from "@/types";

const dashboardControlMotion =
  "transition-[scale,background-color,border-color,box-shadow,color] duration-150 ease-out active:scale-[0.96]";

function ratio(count: number, total: number) {
  if (total <= 0) return 0;
  return Math.min(1, Math.max(0, count / total));
}

function logStatusClass(status: string) {
  switch (status) {
    case "failed":
      return "border-destructive/30 bg-destructive/10 text-destructive-text";
    case "partial":
    case "cancelled":
      return "border-primary/30 bg-primary/10 text-primary-text";
    default:
      return "border-border bg-muted/40 text-muted-foreground";
  }
}

function logDotClass(status: string) {
  switch (status) {
    case "failed":
      return "bg-destructive";
    case "partial":
    case "cancelled":
      return "bg-primary";
    default:
      return "bg-muted-foreground";
  }
}

function formatTime(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat(undefined, {
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

export function PanelHeader({
  title,
  description,
  action,
}: {
  title: string;
  description?: string;
  action?: ReactNode;
}) {
  return (
    <CardHeader className="border-b border-border/70 px-4 py-3">
      <div className="flex flex-col gap-2 sm:flex-row sm:items-start sm:justify-between">
        <div className="min-w-0">
          <CardTitle className="truncate text-sm font-semibold text-balance">
            {title}
          </CardTitle>
          {description && (
            <CardDescription className="mt-1 max-w-full break-words text-pretty text-xs leading-5">
              {description}
            </CardDescription>
          )}
        </div>
        {action && <div className="shrink-0 self-start">{action}</div>}
      </div>
    </CardHeader>
  );
}

export function StatusTile({
  icon,
  label,
  value,
  detail,
  testId,
}: {
  icon: ReactNode;
  label: string;
  value: string;
  detail?: string;
  testId?: string;
}) {
  return (
    <div
      data-testid={testId}
      className="flex min-w-0 items-start gap-3 border-b border-border/70 bg-card px-3 py-3 last:border-b-0 sm:border-b-0 sm:border-r sm:last:border-r-0"
    >
      <span className="grid size-8 shrink-0 place-items-center rounded-md border border-border/80 bg-muted/40 text-primary-text">
        {icon}
      </span>
      <div className="min-w-0">
        <div className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
          {label}
        </div>
        <div className="mt-1 truncate text-sm font-semibold">{value}</div>
        {detail && (
          <div className="mt-1 truncate text-xs text-muted-foreground">
            {detail}
          </div>
        )}
      </div>
    </div>
  );
}

export function ProgressRow({
  label,
  count,
  total,
  tone = "default",
  testId,
}: {
  label: string;
  count: number;
  total: number;
  tone?: "default" | "primary" | "muted" | "destructive";
  testId?: string;
}) {
  const scale = ratio(count, total);
  const fillClass =
    tone === "destructive"
      ? "bg-destructive"
      : tone === "muted"
        ? "bg-muted-foreground/55"
        : "bg-primary";

  return (
    <div
      data-testid={testId}
      className="grid grid-cols-[minmax(7.5rem,0.9fr)_minmax(4.5rem,1fr)_2.5rem] items-center gap-3"
    >
      <span className="min-w-0 text-xs font-medium leading-4 text-muted-foreground">
        {label}
      </span>
      <span className="h-1.5 overflow-hidden rounded-full bg-muted">
        <span
          className={cn("block h-full origin-left rounded-full", fillClass)}
          style={{ transform: `scaleX(${scale})` }}
        />
      </span>
      <span className="text-right text-xs font-semibold tabular-nums">
        {count}
      </span>
    </div>
  );
}

export function QueueRow({
  label,
  count,
  description,
  onClick,
}: {
  label: string;
  count: number;
  description: string;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "focus-ring grid w-full grid-cols-[2.5rem_minmax(0,1fr)_auto] items-center gap-3 border-t border-border/70 px-4 py-3 text-left first:border-t-0 hover:bg-muted/25",
        dashboardControlMotion,
      )}
    >
      <span className="grid size-9 place-items-center rounded-xl border border-primary/25 bg-primary/10 text-sm font-semibold tabular-nums text-primary-text">
        {count}
      </span>
      <span className="min-w-0">
        <span className="block truncate text-sm font-medium">{label}</span>
        <span className="mt-1 block truncate text-xs text-muted-foreground">
          {description}
        </span>
      </span>
      <span className="inline-flex items-center gap-1 rounded-lg border border-border bg-background px-2 py-1 text-xs font-medium text-muted-foreground">
        <ChevronRight className="size-3" />
      </span>
    </button>
  );
}

export function LogRow({
  entry,
  statusLabel,
}: {
  entry: OperationLogEntry;
  statusLabel: string;
}) {
  return (
    <div className="grid grid-cols-[0.875rem_minmax(0,1fr)_auto] items-start gap-3 py-2">
      <span
        className={cn(
          "mt-1.5 size-2 rounded-full ring-4 ring-muted/35",
          logDotClass(entry.status),
        )}
      />
      <div className="min-w-0">
        <div className="truncate text-sm font-medium">{entry.summary}</div>
        <div className="mt-1 flex flex-wrap items-center gap-x-2 gap-y-1 text-xs text-muted-foreground">
          <span>{entry.action}</span>
          <span className="h-3 w-px bg-border" />
          <span>{entry.targetLabel ?? entry.targetKind}</span>
        </div>
      </div>
      <div className="flex shrink-0 items-center gap-2">
        <span
          className={cn(
            "rounded-md border px-2 py-0.5 text-xs font-medium",
            logStatusClass(entry.status),
          )}
        >
          {statusLabel}
        </span>
        <span className="hidden w-12 text-right text-xs text-muted-foreground sm:block">
          {formatTime(entry.createdAt)}
        </span>
      </div>
    </div>
  );
}
