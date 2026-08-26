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
    pendingOperations: [],
    isPendingOperationsLoading: false,
    retryingOperationId: null,
    previewingOperationId: null,
    reconcilingOperationId: null,
    reconciliationPreview: null,
    pendingOperationsError: null,
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
      operationId: " 123e4567-e89b-42d3-a456-426614174000 ",
      query: " scan ",
      level: "info",
      status: "",
    });

    expect(invoke).toHaveBeenCalledWith("list_operation_logs", {
      filter: {
        operationId: "123e4567-e89b-42d3-a456-426614174000",
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

  it("loads and retries pending Central operations", async () => {
    const pending = [
      {
        operationId: "op-1",
        targetId: "local",
        targetKind: "local" as const,
        operationKind: "central_update" as const,
        skillId: "skill-a",
        phase: "copies_pending" as const,
        updatedAt: "2026-07-27T00:00:00Z",
      },
    ];
    vi.mocked(invoke)
      .mockResolvedValueOnce(pending)
      .mockResolvedValueOnce([]);

    await useOperationLogStore.getState().loadPendingOperations();
    expect(invoke).toHaveBeenNthCalledWith(1, "list_pending_fs_db_operations");
    expect(useOperationLogStore.getState().pendingOperations).toEqual(pending);

    await useOperationLogStore.getState().retryPendingOperation("op-1");
    expect(invoke).toHaveBeenNthCalledWith(2, "retry_fs_db_operation", {
      operationId: "op-1",
    });
    expect(useOperationLogStore.getState().pendingOperations).toEqual([]);
  });

  it("previews and applies an eligible prepared-delete reconciliation", async () => {
    const pending = [
      {
        operationId: "op-reconcile",
        targetId: "local",
        targetKind: "local" as const,
        operationKind: "central_delete" as const,
        skillId: "skill-a",
        phase: "prepared" as const,
        updatedAt: "2026-08-07T00:00:00Z",
      },
    ];
    const preview = {
      operationId: "op-reconcile",
      skillId: "skill-a",
      eligible: true,
      duplicatePathCount: 9,
      missingUnownedPathCount: 13,
      blockerCodes: [],
    };
    useOperationLogStore.setState({ pendingOperations: pending });
    vi.mocked(invoke)
      .mockResolvedValueOnce(preview)
      .mockResolvedValueOnce([]);

    await useOperationLogStore
      .getState()
      .previewPendingOperationReconciliation("op-reconcile");
    expect(invoke).toHaveBeenNthCalledWith(
      1,
      "preview_fs_db_operation_reconciliation",
      { operationId: "op-reconcile" },
    );
    expect(useOperationLogStore.getState().reconciliationPreview).toEqual(
      preview,
    );

    await useOperationLogStore
      .getState()
      .reconcilePendingOperation("op-reconcile");
    expect(invoke).toHaveBeenNthCalledWith(2, "reconcile_fs_db_operation", {
      operationId: "op-reconcile",
    });
    expect(useOperationLogStore.getState().pendingOperations).toEqual([]);
    expect(useOperationLogStore.getState().reconciliationPreview).toBeNull();
  });

  it("preserves a blocked prepared-delete row and safe blocker preview", async () => {
    const pending = [
      {
        operationId: "op-blocked",
        targetId: "local",
        targetKind: "local" as const,
        operationKind: "central_delete" as const,
        skillId: "skill-a",
        phase: "prepared" as const,
        updatedAt: "2026-08-07T00:00:00Z",
      },
    ];
    const preview = {
      operationId: "op-blocked",
      skillId: "skill-a",
      eligible: false,
      duplicatePathCount: 1,
      missingUnownedPathCount: 0,
      blockerCodes: ["recovery.reconcile_artifact_remaining"],
    };
    useOperationLogStore.setState({ pendingOperations: pending });
    vi.mocked(invoke).mockResolvedValueOnce(preview);

    await useOperationLogStore
      .getState()
      .previewPendingOperationReconciliation("op-blocked");

    expect(useOperationLogStore.getState().pendingOperations).toEqual(pending);
    expect(useOperationLogStore.getState().reconciliationPreview).toEqual(
      preview,
    );
    expect(invoke).not.toHaveBeenCalledWith(
      "reconcile_fs_db_operation",
      expect.anything(),
    );
  });

  it("exposes loading and empty states for pending Central operations", async () => {
    let resolvePending!: (value: []) => void;
    vi.mocked(invoke).mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          resolvePending = resolve;
        }),
    );

    const loading = useOperationLogStore.getState().loadPendingOperations();
    expect(useOperationLogStore.getState().isPendingOperationsLoading).toBe(true);
    expect(useOperationLogStore.getState().pendingOperations).toEqual([]);

    resolvePending([]);
    await loading;
    expect(useOperationLogStore.getState().isPendingOperationsLoading).toBe(false);
    expect(useOperationLogStore.getState().pendingOperations).toEqual([]);
    expect(useOperationLogStore.getState().pendingOperationsError).toBeNull();
  });

  it("keeps pending state actionable when an offline retry fails", async () => {
    const pending = [
      {
        operationId: "op-offline",
        targetId: "ssh-offline",
        targetKind: "ssh" as const,
        operationKind: "central_update" as const,
        skillId: "skill-offline",
        phase: "copies_pending" as const,
        updatedAt: "2026-07-27T00:00:00Z",
      },
    ];
    useOperationLogStore.setState({ pendingOperations: pending });
    vi.mocked(invoke).mockRejectedValueOnce(new Error("Target is offline"));

    await expect(
      useOperationLogStore.getState().retryPendingOperation("op-offline"),
    ).rejects.toThrow("Target is offline");

    expect(useOperationLogStore.getState().pendingOperations).toEqual(pending);
    expect(useOperationLogStore.getState().retryingOperationId).toBeNull();
    expect(useOperationLogStore.getState().pendingOperationsError).toBe(
      "Error: Target is offline",
    );
  });

  it("clears the previous target and discards stale pending responses", async () => {
    const stale = [
      {
        operationId: "old-target-op",
        targetId: "ssh-old",
        targetKind: "ssh" as const,
        operationKind: "central_delete" as const,
        skillId: "skill-old",
        phase: "fs_staged" as const,
        updatedAt: "2026-07-27T00:00:00Z",
      },
    ];
    const fresh = [
      {
        operationId: "new-target-op",
        targetId: "wsl-new",
        targetKind: "wsl" as const,
        operationKind: "central_update" as const,
        skillId: "skill-new",
        phase: "copies_pending" as const,
        updatedAt: "2026-07-27T00:00:01Z",
      },
    ];
    let resolveFirst!: (value: typeof stale) => void;
    let resolveSecond!: (value: typeof fresh) => void;
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
    useOperationLogStore.setState({ pendingOperations: stale });

    const first = useOperationLogStore.getState().loadPendingOperations();
    expect(useOperationLogStore.getState().pendingOperations).toEqual([]);
    const second = useOperationLogStore.getState().loadPendingOperations();
    resolveSecond(fresh);
    await second;
    resolveFirst(stale);
    await first;

    expect(useOperationLogStore.getState().pendingOperations).toEqual(fresh);
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
