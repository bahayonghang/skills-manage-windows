import type { OperationLogEntry, SkillWithLinks } from "@/types";

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

export function buildTopTags(skills: SkillWithLinks[]) {
  const counts = new Map<string, { id: string; name: string; count: number }>();

  for (const skill of skills) {
    for (const tag of skill.tags ?? []) {
      if (tag.id === "uncategorized") continue;
      const existing = counts.get(tag.id);
      counts.set(tag.id, {
        id: tag.id,
        name: tag.name,
        count: (existing?.count ?? 0) + 1,
      });
    }
  }

  return [...counts.values()]
    .sort(
      (left, right) =>
        right.count - left.count || left.name.localeCompare(right.name)
    )
    .slice(0, TOP_TAG_LIMIT);
}

/**
 * 将一组 bucket count 数组转换为 SVG path 字符串供 sparkline 渲染。
 *
 * - 单一非零值或全零数组：返回空串，由调用方决定是否渲染占位
 * - 单一数据点：渲染为单 Move 命令，stroke-linecap=round 会变成一个点
 * - 多数据点：直线 polyline（L 命令），保持视觉简洁，避免曲线计算开销
 */
export function buildSparklinePath(
  values: number[],
  options: { width?: number; height?: number; padding?: number } = {},
): string {
  const { width = 220, height = 86, padding = 8 } = options;
  if (values.length === 0) return "";

  const max = values.reduce((acc, v) => Math.max(acc, v), 0);
  if (max === 0) return "";

  const innerW = width - padding * 2;
  const innerH = height - padding * 2;
  const stepX = values.length > 1 ? innerW / (values.length - 1) : 0;

  const points = values.map((value, index) => {
    const x = padding + stepX * index;
    const normalized = value / max;
    const y = padding + (1 - normalized) * innerH;
    return [x, y] as const;
  });

  if (points.length === 1) {
    return `M ${points[0][0].toFixed(2)} ${points[0][1].toFixed(2)}`;
  }

  let d = `M ${points[0][0].toFixed(2)} ${points[0][1].toFixed(2)}`;
  for (let i = 1; i < points.length; i++) {
    d += ` L ${points[i][0].toFixed(2)} ${points[i][1].toFixed(2)}`;
  }
  return d;
}
