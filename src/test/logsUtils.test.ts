import { describe, expect, it } from "vitest";

import {
  aggregateLogsKpi,
  dayBoundsIso,
  formatDurationMs,
  formatPercent,
  formatRelativeTime,
  groupLogsByDay,
} from "@/components/logs/logsUtils";
import type { OperationLogEntry } from "@/types";

function makeEntry(overrides: Partial<OperationLogEntry>): OperationLogEntry {
  return {
    id: "log",
    createdAt: "2026-04-27T10:00:00Z",
    level: "info",
    targetKind: "local",
    targetId: "local",
    targetLabel: "Local",
    category: "scan",
    action: "scan.all",
    status: "succeeded",
    summary: "Scan completed",
    ...overrides,
  };
}

describe("aggregateLogsKpi", () => {
  it("returns null rates for empty input", () => {
    const kpi = aggregateLogsKpi([]);
    expect(kpi.total).toBe(0);
    expect(kpi.successRate).toBeNull();
    expect(kpi.avgDurationMs).toBeNull();
    expect(kpi.failures).toBe(0);
  });

  it("counts statuses and averages durations across mixed entries", () => {
    const entries = [
      makeEntry({ id: "1", status: "succeeded", durationMs: 100 }),
      makeEntry({ id: "2", status: "succeeded", durationMs: 200 }),
      makeEntry({ id: "3", status: "failed", durationMs: 600 }),
      makeEntry({ id: "4", status: "partial", durationMs: undefined }),
      makeEntry({ id: "5", status: "cancelled" }),
    ];

    const kpi = aggregateLogsKpi(entries);
    expect(kpi.total).toBe(5);
    expect(kpi.succeeded).toBe(2);
    expect(kpi.failed).toBe(1);
    expect(kpi.partial).toBe(1);
    expect(kpi.cancelled).toBe(1);
    expect(kpi.failures).toBe(2);
    expect(kpi.successRate).toBeCloseTo(2 / 5);
    expect(kpi.avgDurationMs).toBeCloseTo(300);
  });

  it("ignores non-finite durationMs when averaging", () => {
    const entries = [
      makeEntry({ id: "1", status: "succeeded", durationMs: 100 }),
      makeEntry({ id: "2", status: "succeeded", durationMs: Number.NaN }),
    ];
    const kpi = aggregateLogsKpi(entries);
    expect(kpi.avgDurationMs).toBe(100);
  });
});

describe("formatDurationMs", () => {
  it("returns em dash for null / undefined / NaN", () => {
    expect(formatDurationMs(null)).toBe("—");
    expect(formatDurationMs(undefined)).toBe("—");
    expect(formatDurationMs(Number.NaN)).toBe("—");
  });

  it("formats sub-second values in milliseconds", () => {
    expect(formatDurationMs(0)).toBe("0 ms");
    expect(formatDurationMs(240)).toBe("240 ms");
    expect(formatDurationMs(999)).toBe("999 ms");
  });

  it("uses one decimal under 10s and integer above", () => {
    expect(formatDurationMs(1500)).toBe("1.5 s");
    expect(formatDurationMs(12000)).toBe("12 s");
  });

  it("formats minutes for values >= 60s", () => {
    expect(formatDurationMs(75000)).toBe("1m 15s");
  });
});

describe("formatPercent", () => {
  it("returns em dash on null / undefined", () => {
    expect(formatPercent(null)).toBe("—");
    expect(formatPercent(undefined)).toBe("—");
  });

  it("clamps boundary values to 0% / 100%", () => {
    expect(formatPercent(1)).toBe("100%");
    expect(formatPercent(0)).toBe("0%");
  });

  it("formats fractional values with one decimal", () => {
    expect(formatPercent(0.5)).toBe("50.0%");
    expect(formatPercent(0.123)).toBe("12.3%");
  });
});

describe("formatRelativeTime", () => {
  const now = new Date("2026-05-08T10:00:00Z").getTime();

  it("returns a minute-grained label for offsets within an hour", () => {
    const value = new Date(now - 5 * 60 * 1000).toISOString();
    const out = formatRelativeTime(value, "en", now);
    expect(out.toLowerCase()).toContain("minute");
  });

  it("returns a day-grained label for offsets >= 1 day", () => {
    const value = new Date(now - 24 * 60 * 60 * 1000).toISOString();
    const out = formatRelativeTime(value, "en", now);
    expect(out.toLowerCase()).toMatch(/yesterday|day/);
  });

  it("returns the original string when input is not a valid date", () => {
    expect(formatRelativeTime("not-a-date", "en", now)).toBe("not-a-date");
  });
});

describe("groupLogsByDay", () => {
  const now = new Date("2026-05-08T12:00:00");

  it("buckets entries into today / yesterday / earlier in that order", () => {
    const today = new Date(now);
    today.setHours(8, 0, 0, 0);
    const yesterday = new Date(now);
    yesterday.setDate(now.getDate() - 1);
    yesterday.setHours(20, 0, 0, 0);
    const earlier = new Date(now);
    earlier.setDate(now.getDate() - 5);
    earlier.setHours(10, 0, 0, 0);

    const entries = [
      makeEntry({ id: "a", createdAt: today.toISOString() }),
      makeEntry({ id: "b", createdAt: yesterday.toISOString() }),
      makeEntry({ id: "c", createdAt: earlier.toISOString() }),
    ];

    const groups = groupLogsByDay(entries, now);
    expect(groups.map((group) => group.key)).toEqual([
      "today",
      "yesterday",
      "earlier",
    ]);
    expect(groups[0].entries[0].id).toBe("a");
    expect(groups[1].entries[0].id).toBe("b");
    expect(groups[2].entries[0].id).toBe("c");
  });

  it("treats invalid createdAt as earlier", () => {
    const entries = [makeEntry({ id: "broken", createdAt: "not-a-date" })];
    const groups = groupLogsByDay(entries, now);
    expect(groups).toEqual([{ key: "earlier", entries: [entries[0]] }]);
  });

  it("returns an empty array for empty input", () => {
    expect(groupLogsByDay([], now)).toEqual([]);
  });
});

describe("dayBoundsIso", () => {
  it("spans exactly one day minus one millisecond", () => {
    const date = new Date("2026-05-08T10:30:00");
    const bounds = dayBoundsIso(date);
    const startMs = new Date(bounds.start).getTime();
    const endMs = new Date(bounds.end).getTime();
    expect(endMs - startMs).toBe(86_399_999);
  });
});
