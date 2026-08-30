import type { OperationLogEntry } from "@/types";

export interface LogsKpi {
  total: number;
  succeeded: number;
  failed: number;
  partial: number;
  cancelled: number;
  failures: number;
  successRate: number | null;
  avgDurationMs: number | null;
}

export function aggregateLogsKpi(entries: OperationLogEntry[]): LogsKpi {
  const total = entries.length;
  let succeeded = 0;
  let failed = 0;
  let partial = 0;
  let cancelled = 0;
  let durationSum = 0;
  let durationCount = 0;

  for (const entry of entries) {
    switch (entry.status) {
      case "succeeded":
        succeeded += 1;
        break;
      case "failed":
        failed += 1;
        break;
      case "partial":
        partial += 1;
        break;
      case "cancelled":
        cancelled += 1;
        break;
      default:
        break;
    }
    if (
      typeof entry.durationMs === "number" &&
      Number.isFinite(entry.durationMs)
    ) {
      durationSum += entry.durationMs;
      durationCount += 1;
    }
  }

  return {
    total,
    succeeded,
    failed,
    partial,
    cancelled,
    failures: failed + partial,
    successRate: total > 0 ? succeeded / total : null,
    avgDurationMs: durationCount > 0 ? durationSum / durationCount : null,
  };
}

export function formatLogAbsoluteTime(
  iso: string,
  options?: { withSeconds?: boolean },
): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  return new Intl.DateTimeFormat(undefined, {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    ...(options?.withSeconds ? { second: "2-digit" } : {}),
  }).format(date);
}

export function formatLogIsoTime(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  return iso;
}

export function formatDurationMs(ms: number | null | undefined): string {
  if (ms === null || ms === undefined || !Number.isFinite(ms)) return "—";
  if (ms < 1000) return `${Math.round(ms)} ms`;
  const seconds = ms / 1000;
  if (seconds < 60) {
    return `${seconds >= 10 ? Math.round(seconds) : seconds.toFixed(1)} s`;
  }
  const minutes = Math.floor(seconds / 60);
  const remSec = Math.round(seconds % 60);
  return `${minutes}m ${remSec}s`;
}

export function formatPercent(value: number | null | undefined): string {
  if (value === null || value === undefined || !Number.isFinite(value)) {
    return "—";
  }
  const pct = value * 100;
  if (pct >= 99.95) return "100%";
  if (pct <= 0.05) return "0%";
  return `${pct.toFixed(1)}%`;
}

export function formatRelativeTime(
  value: string,
  locale?: string,
  now: number = Date.now(),
): string {
  const date = new Date(value);
  const time = date.getTime();
  if (Number.isNaN(time)) return value;
  const diffMs = time - now;
  const absDiff = Math.abs(diffMs);
  const rtf = new Intl.RelativeTimeFormat(locale, { numeric: "auto" });

  const second = 1000;
  const minute = 60 * second;
  const hour = 60 * minute;
  const day = 24 * hour;

  if (absDiff < minute) {
    return rtf.format(Math.round(diffMs / second), "second");
  }
  if (absDiff < hour) {
    return rtf.format(Math.round(diffMs / minute), "minute");
  }
  if (absDiff < day) {
    return rtf.format(Math.round(diffMs / hour), "hour");
  }
  return rtf.format(Math.round(diffMs / day), "day");
}

export type LogsGroupKey = "today" | "yesterday" | "earlier";

export interface LogsGroup {
  key: LogsGroupKey;
  entries: OperationLogEntry[];
}

function startOfLocalDay(value: Date): Date {
  const date = new Date(value);
  date.setHours(0, 0, 0, 0);
  return date;
}

export function groupLogsByDay(
  entries: OperationLogEntry[],
  now: Date = new Date(),
): LogsGroup[] {
  const todayStart = startOfLocalDay(now).getTime();
  const yesterdayStart = todayStart - 24 * 60 * 60 * 1000;

  const today: OperationLogEntry[] = [];
  const yesterday: OperationLogEntry[] = [];
  const earlier: OperationLogEntry[] = [];

  for (const entry of entries) {
    const date = new Date(entry.createdAt);
    const time = date.getTime();
    if (Number.isNaN(time)) {
      earlier.push(entry);
      continue;
    }
    if (time >= todayStart) {
      today.push(entry);
    } else if (time >= yesterdayStart) {
      yesterday.push(entry);
    } else {
      earlier.push(entry);
    }
  }

  const groups: LogsGroup[] = [];
  if (today.length > 0) groups.push({ key: "today", entries: today });
  if (yesterday.length > 0) groups.push({ key: "yesterday", entries: yesterday });
  if (earlier.length > 0) groups.push({ key: "earlier", entries: earlier });
  return groups;
}

export function dayBoundsIso(date: Date): {
  start: string;
  end: string;
} {
  const start = startOfLocalDay(date);
  const end = new Date(start);
  end.setDate(start.getDate() + 1);
  end.setMilliseconds(end.getMilliseconds() - 1);
  return {
    start: start.toISOString(),
    end: end.toISOString(),
  };
}
