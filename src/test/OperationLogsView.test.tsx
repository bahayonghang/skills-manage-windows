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
  detailsJson: '{"totalSkills":12}',
  durationMs: 12,
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

describe("OperationLogsView", () => {
  beforeEach(() => {
    resetStore();
    vi.clearAllMocks();
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === "list_operation_logs") return mockPage;
      if (command === "get_operation_log") return mockEntry;
      if (command === "clear_operation_logs") return 1;
      if (command === "export_operation_logs") return '{"entries":[]}';
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
});
