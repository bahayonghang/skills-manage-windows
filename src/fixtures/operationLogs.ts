import { registerIpcFixtures } from "@/lib/ipc";
import type {
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

export function registerOperationLogFixtures(): void {
  registerIpcFixtures({
    list_operation_logs: ({ filter }) => fixturePage(filter),
    get_operation_log: ({ logId }) =>
      fixtureLogEntries.find((item) => item.id === logId) ?? null,
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
