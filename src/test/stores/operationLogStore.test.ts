import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@/lib/ipc", () => ({
  invoke: vi.fn(),
  isTauriRuntime: vi.fn(() => true),
}));

import { invoke } from "@/lib/ipc";
import { useOperationLogStore } from "@/stores/operationLogStore";
import { OperationLogEntry, OperationLogPage } from "@/types";

const mockEntry: OperationLogEntry = {
  id: "log-1",
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
  summary: "Scan completed",
  detailsJson: "{}",
  durationMs: 10,
};

const mockPage: OperationLogPage = {
  entries: [mockEntry],
  total: 1,
  limit: 100,
  offset: 0,
};

function resetStore() {
  useOperationLogStore.setState({
    entries: [],
    total: 0,
    filter: { limit: 100, offset: 0 },
    selectedEntry: null,
    isLoading: false,
    isLoadingDetail: false,
    isClearing: false,
    isExporting: false,
    error: null,
    dailyCounts: [],
    isDailyCountsLoading: false,
    dailyCountsError: null,
  });
}

describe("operationLogStore", () => {
  beforeEach(() => {
    resetStore();
    vi.clearAllMocks();
  });

  it("loads operation logs with normalized filter", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(mockPage);

    await useOperationLogStore.getState().loadLogs({
      query: " scan ",
      level: "info",
      status: "",
    });

    expect(invoke).toHaveBeenCalledWith("list_operation_logs", {
      filter: {
        query: "scan",
        level: "info",
        limit: 100,
        offset: 0,
      },
    });
    expect(useOperationLogStore.getState().entries).toEqual([mockEntry]);
    expect(useOperationLogStore.getState().total).toBe(1);
  });

  it("loads a single operation log detail", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(mockEntry);

    await useOperationLogStore.getState().loadLogDetail("log-1");

    expect(invoke).toHaveBeenCalledWith("get_operation_log", { logId: "log-1" });
    expect(useOperationLogStore.getState().selectedEntry).toEqual(mockEntry);
  });

  it("clears logs with the current filter and reloads", async () => {
    useOperationLogStore.setState({ filter: { level: "error", limit: 100, offset: 0 } });
    vi.mocked(invoke)
      .mockResolvedValueOnce(2)
      .mockResolvedValueOnce({ ...mockPage, entries: [], total: 0 });

    const deleted = await useOperationLogStore.getState().clearLogs();

    expect(deleted).toBe(2);
    expect(invoke).toHaveBeenNthCalledWith(1, "clear_operation_logs", {
      filter: { level: "error", limit: 100, offset: 0 },
    });
    expect(invoke).toHaveBeenNthCalledWith(2, "list_operation_logs", {
      filter: { level: "error", limit: 100, offset: 0 },
    });
  });

  it("exports logs with the current filter", async () => {
    vi.mocked(invoke).mockResolvedValueOnce('{"entries":[]}');

    const payload = await useOperationLogStore.getState().exportLogs({ action: "scan.all" });

    expect(payload).toBe('{"entries":[]}');
    expect(invoke).toHaveBeenCalledWith("export_operation_logs", {
      filter: { action: "scan.all", limit: 100, offset: 0 },
    });
  });

  it("loads daily operation counts for the dashboard activity chart", async () => {
    const buckets = [
      { date: "2026-07-19", count: 2 },
      { date: "2026-07-20", count: 5 },
    ];
    vi.mocked(invoke).mockResolvedValueOnce(buckets);

    await useOperationLogStore.getState().loadDailyCounts(14);

    expect(invoke).toHaveBeenCalledWith("get_daily_operation_counts", {
      days: 14,
    });
    expect(useOperationLogStore.getState().dailyCounts).toEqual(buckets);
    expect(useOperationLogStore.getState().isDailyCountsLoading).toBe(false);
    expect(useOperationLogStore.getState().dailyCountsError).toBeNull();
  });

  it("stores dailyCountsError on failure and keeps previous buckets", async () => {
    const previous = [{ date: "2026-07-20", count: 1 }];
    useOperationLogStore.setState({ dailyCounts: previous });
    vi.mocked(invoke).mockRejectedValueOnce(new Error("backend down"));

    await useOperationLogStore.getState().loadDailyCounts(14);

    expect(useOperationLogStore.getState().dailyCounts).toEqual(previous);
    expect(useOperationLogStore.getState().isDailyCountsLoading).toBe(false);
    expect(useOperationLogStore.getState().dailyCountsError).toBe(
      "Error: backend down",
    );
  });

  it("discards stale loadDailyCounts responses (latest-wins)", async () => {
    let resolveFirst!: (value: { date: string; count: number }[]) => void;
    let resolveSecond!: (value: { date: string; count: number }[]) => void;
    vi.mocked(invoke)
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            resolveFirst = resolve;
          }),
      )
      .mockImplementationOnce(
        () =>
          new Promise((resolve) => {
            resolveSecond = resolve;
          }),
      );

    const first = useOperationLogStore.getState().loadDailyCounts(14);
    const second = useOperationLogStore.getState().loadDailyCounts(14);

    const staleBuckets = [{ date: "2026-07-01", count: 1 }];
    const freshBuckets = [{ date: "2026-07-02", count: 2 }];
    resolveSecond(freshBuckets);
    await second;
    resolveFirst(staleBuckets);
    await first;

    expect(useOperationLogStore.getState().dailyCounts).toEqual(freshBuckets);
  });
});
