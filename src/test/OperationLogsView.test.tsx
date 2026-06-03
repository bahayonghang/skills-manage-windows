import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";

vi.mock("@/lib/tauri", () => ({
  invoke: vi.fn(),
  isTauriRuntime: vi.fn(() => true),
}));

vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

import { invoke } from "@/lib/tauri";
import { OperationLogsView } from "@/pages/OperationLogsView";
import { useOperationLogStore } from "@/stores/operationLogStore";
import { OperationLogEntry, OperationLogPage, RuntimeLogReadResult } from "@/types";

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
  detailsJson: '{"totalSkills":12}',
  durationMs: 12,
};

const mockPage: OperationLogPage = {
  entries: [mockEntry],
  total: 1,
  limit: 100,
  offset: 0,
};

const mockRuntimeReadResult: RuntimeLogReadResult = {
  fileName: "skillport-2026-06-03.log",
  total: 1,
  limit: 200,
  offset: 0,
  lines: [
    {
      lineNumber: 1,
      timestamp: "2026-06-03T10:00:00Z",
      level: "error",
      source: "window.error",
      message: "Render exploded",
      raw: "2026-06-03T10:00:00Z ERROR skillport::frontend: Render exploded source=window.error token=[REDACTED]",
    },
  ],
};

const mockEntry2: OperationLogEntry = {
  ...mockEntry,
  id: "log-2",
  createdAt: "2026-04-27T11:00:00Z",
  action: "scan.partial",
  status: "partial",
  summary: "Second batch",
  detailsJson: null,
  errorSummary: "One target unavailable",
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

describe("OperationLogsView", () => {
  beforeEach(() => {
    resetStore();
    vi.clearAllMocks();
    window.localStorage.clear();
    Object.defineProperty(navigator, "clipboard", {
      value: { writeText: vi.fn().mockResolvedValue(undefined) },
      configurable: true,
    });
    vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(() => undefined);
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === "list_operation_logs") return mockPage;
      if (command === "get_operation_log") return mockEntry;
      if (command === "clear_operation_logs") return 1;
      if (command === "export_operation_logs") return '{"entries":[]}';
      if (command === "list_runtime_log_files") {
        return [
          {
            fileName: "skillport-2026-06-03.log",
            date: "2026-06-03",
            sizeBytes: 512,
            modifiedAt: "2026-06-03T10:00:00Z",
          },
        ];
      }
      if (command === "read_runtime_log_file") return mockRuntimeReadResult;
      if (command === "export_runtime_log_file") return "runtime log";
      if (command === "clear_runtime_logs") return 1;
      return null;
    });
  });

  it("renders operation logs and opens the detail drawer", async () => {
    render(<OperationLogsView />);

    expect(await screen.findByText("操作日志")).toBeInTheDocument();
    expect(await screen.findByText("scan.all")).toBeInTheDocument();

    fireEvent.click(screen.getByText("scan.all").closest("button")!);

    expect(await screen.findByTestId("operation-log-detail-drawer")).toBeInTheDocument();
    expect(screen.getAllByText("Scan completed").length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText(/totalSkills/)).toBeInTheDocument();
  });

  it("reloads when filters change", async () => {
    render(<OperationLogsView />);

    await screen.findByText("scan.all");
    fireEvent.change(screen.getByLabelText("级别"), { target: { value: "error" } });

    await waitFor(() => {
      expect(invoke).toHaveBeenLastCalledWith("list_operation_logs", {
        filter: { level: "error", limit: 100, offset: 0 },
      });
    });
  });

  it("requires confirmation before clearing logs", async () => {
    render(<OperationLogsView />);

    await screen.findByText("scan.all");
    fireEvent.click(screen.getByRole("button", { name: "清空日志" }));
    fireEvent.click(screen.getByRole("button", { name: "确认清空" }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("clear_operation_logs", {
        filter: { limit: 100, offset: 0 },
      });
    });
  });

  it("persists row density to localStorage when toggled", async () => {
    render(<OperationLogsView />);
    await screen.findByText("scan.all");

    fireEvent.click(screen.getByTestId("logs-density-toggle"));

    await waitFor(() => {
      expect(window.localStorage.getItem("skillport.logs.density")).toBe(
        "compact",
      );
    });
  });

  it("applies the failed quick chip and reloads with status=failed", async () => {
    render(<OperationLogsView />);
    await screen.findByText("scan.all");

    fireEvent.click(screen.getByTestId("logs-chip-failed"));

    await waitFor(() => {
      expect(invoke).toHaveBeenLastCalledWith("list_operation_logs", {
        filter: { status: "failed", limit: 100, offset: 0 },
      });
    });
  });

  it("loads more entries when the load more button is clicked", async () => {
    vi.mocked(invoke).mockImplementation(async (command, args) => {
      if (command === "list_operation_logs") {
        const offset =
          (args as { filter?: { offset?: number } } | undefined)?.filter
            ?.offset ?? 0;
        return offset === 0
          ? { entries: [mockEntry], total: 2, limit: 100, offset: 0 }
          : { entries: [mockEntry2], total: 2, limit: 100, offset: 1 };
      }
      if (command === "get_operation_log") return mockEntry;
      return null;
    });

    render(<OperationLogsView />);
    await screen.findByText("scan.all");
    fireEvent.click(await screen.findByText(/加载更多/));

    await waitFor(() => {
      expect(invoke).toHaveBeenLastCalledWith("list_operation_logs", {
        filter: { limit: 100, offset: 1 },
      });
    });
    expect(await screen.findByText("scan.partial")).toBeInTheDocument();
  });

  it("copies the log id from the detail drawer", async () => {
    render(<OperationLogsView />);
    await screen.findByText("scan.all");

    fireEvent.click(screen.getByText("scan.all").closest("button")!);
    fireEvent.click(await screen.findByTestId("logs-detail-copy-id"));

    await waitFor(() => {
      expect(navigator.clipboard.writeText).toHaveBeenCalledWith("log-1");
    });
  });

  it("focuses the search input when pressing /", async () => {
    render(<OperationLogsView />);
    await screen.findByText("scan.all");

    const searchInput = screen.getByPlaceholderText(/搜索摘要/);
    expect(document.activeElement).not.toBe(searchInput);

    fireEvent.keyDown(document.body, { key: "/" });

    await waitFor(() => {
      expect(document.activeElement).toBe(searchInput);
    });
  });

  it("renders runtime logs, filters, exports, and clears the selected file", async () => {
    render(<OperationLogsView />);

    fireEvent.click(screen.getByRole("tab", { name: /运行日志/ }));

    expect(await screen.findByText("运行时诊断")).toBeInTheDocument();
    expect(await screen.findByText("Render exploded")).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("级别"), { target: { value: "error" } });

    await waitFor(() => {
      expect(invoke).toHaveBeenLastCalledWith("read_runtime_log_file", {
        request: {
          fileName: "skillport-2026-06-03.log",
          query: undefined,
          level: "error",
          source: undefined,
          limit: 200,
          offset: 0,
          tail: true,
        },
      });
    });

    fireEvent.click(screen.getByRole("button", { name: /导出|Export/ }));
    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("export_runtime_log_file", {
        fileName: "skillport-2026-06-03.log",
      });
    });

    fireEvent.click(screen.getByRole("button", { name: "清理选中的运行日志" }));
    fireEvent.click(screen.getByRole("button", { name: "清理文件" }));

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("clear_runtime_logs", {
        request: { fileName: "skillport-2026-06-03.log" },
      });
    });
  });
});
