import type { OperationLogEntry } from "@/types";

export const RECENT_LOG_LIMIT = 5;
export const ACTIVITY_DAY_COUNT = 14;
export const TOP_TAG_LIMIT = 6;

export const EMPTY_DASHBOARD_CENTRAL_SUMMARY = {
  centralSkillCount: 0,
  updatesAvailable: 0,
  aiReviewCount: 0,
  uncategorizedCount: 0,
  unassignedSourceCount: 0,
  readiness: {
    score: 0,
    categorizedRatio: 0,
    describedRatio: 0,
    sourcedRatio: 0,
    installHealthRatio: 0,
  },
  sourceRepositories: [],
};

export function formatDateTime(
  value: string | null | undefined,
  fallback: string
) {
  if (!value) return fallback;
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat(undefined, {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date);
}

function formatDateLabel(value: Date) {
  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "2-digit",
  }).format(value);
}

function startOfUtcDay(value: Date) {
  const date = new Date(value);
  date.setUTCHours(0, 0, 0, 0);
  return date;
}

function dayKey(value: Date) {
  return startOfUtcDay(value).toISOString().slice(0, 10);
}

export function heatCellClass(count: number, max: number) {
  if (count <= 0) return "bg-muted/45";
  const level = Math.ceil((count / Math.max(max, 1)) * 4);
  if (level >= 4) return "bg-primary";
  if (level === 3) return "bg-primary/70";
  if (level === 2) return "bg-primary/40";
  return "bg-primary/20";
}

export function buildActivitySummary(entries: OperationLogEntry[]) {
  const validDates = entries
    .map((entry) => new Date(entry.createdAt))
    .filter((date) => !Number.isNaN(date.getTime()));
  const endDate = startOfUtcDay(
    validDates.length > 0
      ? new Date(Math.max(...validDates.map((date) => date.getTime())))
      : new Date()
  );
  const startDate = new Date(endDate);
  startDate.setUTCDate(endDate.getUTCDate() - (ACTIVITY_DAY_COUNT - 1));

  const buckets = Array.from({ length: ACTIVITY_DAY_COUNT }, (_, index) => {
    const date = new Date(startDate);
    date.setUTCDate(startDate.getUTCDate() + index);
    return {
      key: dayKey(date),
      date,
      count: 0,
    };
  });
  const indexByKey = new Map(
    buckets.map((bucket, index) => [bucket.key, index])
  );

  for (const entry of entries) {
    const date = new Date(entry.createdAt);
    if (Number.isNaN(date.getTime())) continue;
    const index = indexByKey.get(dayKey(date));
    if (index === undefined) continue;
    buckets[index].count += 1;
  }

  const total = buckets.reduce((sum, bucket) => sum + bucket.count, 0);
  const max = buckets.reduce(
    (current, bucket) => Math.max(current, bucket.count),
    0
  );

  return {
    buckets,
    endLabel: formatDateLabel(endDate),
    max,
    startLabel: formatDateLabel(startDate),
    total,
  };
}
