import { registerIpcFixtures } from "@/lib/ipc";
import type {
  DailyOperationCount,
  OperationLogEntry,
  OperationLogFilter,
  OperationLogPage,
} from "@/types";

const DEFAULT_LIMIT = 100;

const fixtureLogEntries: OperationLogEntry[] = [
  {
    id: "fixture-log-scan",
    createdAt: "2026-04-27T10:00:00Z",
    level: "info",
    targetKind: "local",
    targetId: "local",
    targetLabel: "Local",
    category: "scan",
    action: "scan.all",
    status: "succeeded",
    subjectType: "scan_root",
    subjectId: "all",
    subjectLabel: "All scan directories",
    summary: "Scanned 12 skills across 5 agents",
    detailsJson: JSON.stringify({ totalSkills: 12, agentsScanned: 5 }, null, 2),
    durationMs: 240,
  },
  {
    id: "fixture-log-import",
    createdAt: "2026-04-27T09:30:00Z",
    level: "warn",
    targetKind: "ssh",
    targetId: "ssh-demo",
    targetLabel: "Remote Demo",
    category: "install",
    action: "central.batch_install",
    status: "partial",
    subjectType: "batch",
    subjectId: "central.batch_install",
    subjectLabel: "Central batch install",
    summary: "Installed 3 Central skill target(s), 1 failed",
    errorSummary: "One target platform was unavailable.",
    detailsJson: JSON.stringify({ succeeded: 3, failed: 1 }, null, 2),
    durationMs: 1080,
  },
];

function normalizeFilter(filter: OperationLogFilter): OperationLogFilter {
  const normalized: OperationLogFilter = {};
  for (const [key, value] of Object.entries(filter) as Array<
    [keyof OperationLogFilter, OperationLogFilter[keyof OperationLogFilter]]
  >) {
    if (typeof value === "string") {
      const trimmed = value.trim();
      if (trimmed) {
        normalized[key] = trimmed as never;
      }
      continue;
    }
    if (value !== undefined && value !== null) {
      normalized[key] = value as never;
    }
  }

  return {
    ...normalized,
    limit: normalized.limit ?? DEFAULT_LIMIT,
    offset: normalized.offset ?? 0,
  };
}

function matchesFixture(
  entry: OperationLogEntry,
  filter: OperationLogFilter,
): boolean {
  const query = filter.query?.toLowerCase();
  if (query) {
    const haystack = [
      entry.summary,
      entry.errorSummary,
      entry.subjectId,
      entry.subjectLabel,
      entry.action,
    ]
      .filter(Boolean)
      .join(" ")
      .toLowerCase();
    if (!haystack.includes(query)) return false;
  }
  if (filter.targetKind && entry.targetKind !== filter.targetKind) return false;
  if (filter.targetId && entry.targetId !== filter.targetId) return false;
  if (filter.level && entry.level !== filter.level) return false;
  if (filter.status && entry.status !== filter.status) return false;
  if (filter.category && entry.category !== filter.category) return false;
  if (filter.action && entry.action !== filter.action) return false;
  if (filter.createdAfter && entry.createdAt < filter.createdAfter)
    return false;
  if (filter.createdBefore && entry.createdAt > filter.createdBefore)
    return false;
  return true;
}

function fixturePage(filter: OperationLogFilter): OperationLogPage {
  const normalized = normalizeFilter(filter);
  const matched = fixtureLogEntries.filter((entry) =>
    matchesFixture(entry, normalized),
  );
  const offset = normalized.offset ?? 0;
  const limit = normalized.limit ?? DEFAULT_LIMIT;

  return {
    entries: matched.slice(offset, offset + limit),
    total: matched.length,
    limit,
    offset,
  };
}

// 合成演示数据：窗口语义与后端一致（本地今天向前 days-1 天，升序，零值填充）。
const FIXTURE_DAILY_PATTERN = [0, 2, 0, 0, 5, 1, 0, 0, 3, 0, 1, 0, 2, 4];

function formatLocalDateKey(date: Date): string {
  const year = date.getFullYear();
  const month = `${date.getMonth() + 1}`.padStart(2, "0");
  const day = `${date.getDate()}`.padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function fixtureDailyOperationCounts(days: number): DailyOperationCount[] {
  const safeDays = Math.max(1, Math.min(60, Math.trunc(days) || 14));
  const today = new Date();
  const buckets: DailyOperationCount[] = [];
  for (let index = 0; index < safeDays; index += 1) {
    const date = new Date(today);
    date.setDate(today.getDate() - (safeDays - 1 - index));
    buckets.push({
      date: formatLocalDateKey(date),
      count: FIXTURE_DAILY_PATTERN[index % FIXTURE_DAILY_PATTERN.length],
    });
  }
  return buckets;
}

export function registerOperationLogFixtures(): void {
  registerIpcFixtures({
    list_operation_logs: ({ filter }) => fixturePage(filter),
    get_operation_log: ({ logId }) =>
      fixtureLogEntries.find((item) => item.id === logId) ?? null,
    get_daily_operation_counts: ({ days }) =>
      fixtureDailyOperationCounts(days),
    clear_operation_logs: ({ filter }) =>
      fixtureLogEntries.filter((entry) =>
        matchesFixture(entry, normalizeFilter(filter)),
      ).length,
    export_operation_logs: ({ filter }) => {
      const normalized = normalizeFilter(filter);
      return JSON.stringify(
        {
          exportedAt: new Date().toISOString(),
          filter: normalized,
          total: fixturePage(normalized).total,
          entries: fixturePage(normalized).entries,
        },
        null,
        2,
      );
    },
  });
}
