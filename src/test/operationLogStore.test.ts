import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@/lib/tauri", () => ({
  invoke: vi.fn(),
  isTauriRuntime: vi.fn(() => true),
}));

import { invoke } from "@/lib/tauri";
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
});
