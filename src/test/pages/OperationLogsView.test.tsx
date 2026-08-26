import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";

vi.mock("@/lib/ipc", () => ({
  invoke: vi.fn(),
  isTauriRuntime: vi.fn(() => true),
}));

vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

import { invoke } from "@/lib/ipc";
import { toast } from "sonner";
import { OperationLogsView } from "@/pages/OperationLogsView";
import { useOperationLogStore } from "@/stores/operationLogStore";
import { useRuntimeLogStore } from "@/stores/runtimeLogStore";
import {
  OperationLogEntry,
  OperationLogPage,
  RuntimeLogReadResult,
} from "@/types";

const OPERATION_ID = "65e8ecdf-c398-4ed7-a4a1-87f193f0dd25";

const mockEntry: OperationLogEntry = {
  id: OPERATION_ID,
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
      operationId: OPERATION_ID,
      eventSource: "frontend",
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
    pendingOperations: [],
    isPendingOperationsLoading: false,
    retryingOperationId: null,
    pendingOperationsError: null,
  });
  useRuntimeLogStore.setState({
    files: [],
    selectedFileName: null,
    readResult: null,
    filter: {},
    isLoadingFiles: false,
    isLoadingLines: false,
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
    vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(
      () => undefined,
    );
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

  it("renders operation logs and opens the centered detail dialog", async () => {
    render(<OperationLogsView />);

    expect(await screen.findByText("操作日志")).toBeInTheDocument();
    expect(await screen.findByText("scan.all")).toBeInTheDocument();

    fireEvent.click(screen.getByText("scan.all").closest("button")!);

    expect(
      await screen.findByTestId("operation-log-detail-dialog"),
    ).toBeInTheDocument();
    expect(
      screen.getAllByText("操作已成功完成。").length,
    ).toBeGreaterThanOrEqual(1);
    expect(screen.getByText(/totalSkills/)).toBeInTheDocument();
  });

  it("shows current-target recovery and retries without blocking cached logs", async () => {
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === "list_operation_logs") return mockPage;
      if (command === "list_pending_fs_db_operations") {
        return [
          {
            operationId: "op-1",
            targetId: "local",
            targetKind: "local",
            operationKind: "central_update",
            skillId: "skill-a",
            phase: "copies_pending",
            updatedAt: "2026-07-27T00:00:00Z",
          },
        ];
      }
      if (command === "retry_fs_db_operation") return [];
      return null;
    });

    render(<OperationLogsView />);
    expect(await screen.findByText("Central 待恢复操作")).toBeInTheDocument();
    expect(screen.getByText("scan.all")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /skill-a/ }));
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("retry_fs_db_operation", {
        operationId: "op-1",
      }),
    );
    await waitFor(() =>
      expect(screen.queryByText("Central 待恢复操作")).not.toBeInTheDocument(),
    );
  });

  it("previews and confirms an eligible prepared-delete reconciliation", async () => {
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === "list_operation_logs") return mockPage;
      if (command === "list_pending_fs_db_operations") {
        return [
          {
            operationId: "op-reconcile",
            targetId: "local",
            targetKind: "local",
            operationKind: "central_delete",
            skillId: "skill-a",
            phase: "prepared",
            updatedAt: "2026-08-07T00:00:00Z",
          },
        ];
      }
      if (command === "preview_fs_db_operation_reconciliation") {
        return {
          operationId: "op-reconcile",
          skillId: "skill-a",
          eligible: true,
          duplicatePathCount: 9,
          missingUnownedPathCount: 13,
          blockerCodes: [],
        };
      }
      if (command === "reconcile_fs_db_operation") return [];
      return null;
    });

    render(<OperationLogsView />);
    fireEvent.click(await screen.findByTitle("核销 prepared 删除"));
    expect(
      await screen.findByText("核销 prepared 删除操作"),
    ).toBeInTheDocument();
    expect(await screen.findByText(/13 条已不归属/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "确认核销" }));

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("reconcile_fs_db_operation", {
        operationId: "op-reconcile",
      }),
    );
    await waitFor(() =>
      expect(
        screen.queryByText("核销 prepared 删除操作"),
      ).not.toBeInTheDocument(),
    );
  });

  it("shows recovery loading without hiding cached logs, then hides the empty band", async () => {
    let resolvePending!: (value: []) => void;
    vi.mocked(invoke).mockImplementation((command) => {
      if (command === "list_operation_logs") return Promise.resolve(mockPage);
      if (command === "list_pending_fs_db_operations") {
        return new Promise((resolve) => {
          resolvePending = resolve;
        });
      }
      return Promise.resolve(null);
    });

    render(<OperationLogsView />);
    expect(
      await screen.findByText("正在检查当前目标中断的变更..."),
    ).toBeInTheDocument();
    expect(screen.getByText("scan.all")).toBeInTheDocument();

    resolvePending([]);
    await waitFor(() =>
      expect(screen.queryByText("Central 待恢复操作")).not.toBeInTheDocument(),
    );
    expect(screen.getByText("scan.all")).toBeInTheDocument();
  });

  it("shows a fixed recovery error while cached logs remain usable", async () => {
    const seed = "Target is offline token=sk_live_secret C:/Users/private";
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === "list_operation_logs") return mockPage;
      if (command === "list_pending_fs_db_operations") {
        throw new Error(seed);
      }
      return null;
    });

    render(<OperationLogsView />);
    expect(
      await screen.findByText(/没有记录额外的公开错误原因/),
    ).toBeInTheDocument();
    expect(document.body).not.toHaveTextContent(seed);
    expect(document.body).not.toHaveTextContent("sk_live_secret");
    expect(screen.getByText("scan.all")).toBeInTheDocument();
  });

  it("does not expose an arbitrary operation refresh rejection", async () => {
    const seed = "private.internal token=sk_live_secret C:/Users/private";
    let rejectRefresh = false;
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === "list_operation_logs") {
        if (rejectRefresh) throw new Error(seed);
        return mockPage;
      }
      if (command === "list_pending_fs_db_operations") return [];
      return null;
    });

    render(<OperationLogsView />);
    await screen.findByText("scan.all");
    rejectRefresh = true;
    fireEvent.click(screen.getByRole("button", { name: "刷新" }));

    await waitFor(() => expect(toast.error).toHaveBeenCalled());
    const toastPayload = vi.mocked(toast.error).mock.calls.flat().join(" ");
    expect(toastPayload).not.toContain(seed);
    expect(toastPayload).not.toContain("sk_live_secret");
    expect(document.body).not.toHaveTextContent(seed);
    expect(document.body).not.toHaveTextContent("sk_live_secret");
  });

  it("keeps long pending identities inside the wrapping narrow-screen controls", async () => {
    const longSkillId =
      "skill-with-a-very-long-identity-that-must-not-overlap-neighbor-controls";
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === "list_operation_logs") return mockPage;
      if (command === "list_pending_fs_db_operations") {
        return [
          {
            operationId: "op-responsive",
            targetId: "local",
            targetKind: "local",
            operationKind: "central_update",
            skillId: longSkillId,
            phase: "copies_pending",
            updatedAt: "2026-07-27T00:00:00Z",
          },
        ];
      }
      return null;
    });
    Object.defineProperty(window, "innerWidth", {
      value: 320,
      configurable: true,
    });

    render(<OperationLogsView />);
    const band = await screen.findByLabelText("Central 待恢复操作");
    const identity = screen.getByText(longSkillId);
    expect(identity).toHaveClass("max-w-40", "truncate");
    expect(band.querySelector(".flex-wrap")).not.toBeNull();
    expect(band.querySelector(".min-w-0")).not.toBeNull();
    expect(screen.getByText("scan.all")).toBeInTheDocument();
  });

  it("reloads when filters change", async () => {
    render(<OperationLogsView />);

    await screen.findByText("scan.all");
    fireEvent.change(screen.getByLabelText("级别"), {
      target: { value: "error" },
    });

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

  it("copies the operation id from the detail dialog", async () => {
    render(<OperationLogsView />);
    await screen.findByText("scan.all");

    fireEvent.click(screen.getByText("scan.all").closest("button")!);
    fireEvent.click(await screen.findByTestId("logs-detail-copy-id"));

    await waitFor(() => {
      expect(navigator.clipboard.writeText).toHaveBeenCalledWith(OPERATION_ID);
    });
  });

  it("closes the detail dialog with Escape and restores focus to the row", async () => {
    render(<OperationLogsView />);
    const action = await screen.findByText("scan.all");
    const row = action.closest("button")!;

    fireEvent.click(row);
    await screen.findByTestId("operation-log-detail-dialog");
    fireEvent.keyDown(document, { key: "Escape" });

    await waitFor(() =>
      expect(
        screen.queryByTestId("operation-log-detail-dialog"),
      ).not.toBeInTheDocument(),
    );
    await waitFor(() => expect(document.activeElement).toBe(row));
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

    fireEvent.change(screen.getByLabelText("级别"), {
      target: { value: "error" },
    });

    await waitFor(() => {
      expect(invoke).toHaveBeenLastCalledWith("read_runtime_log_file", {
        request: {
          fileName: "skillport-2026-06-03.log",
          query: undefined,
          level: "error",
          source: undefined,
          operationId: undefined,
          eventSource: undefined,
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

  it("does not expose an arbitrary runtime load rejection", async () => {
    const seed = "private.internal token=sk_live_secret C:/Users/private";
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === "list_operation_logs") return mockPage;
      if (command === "list_pending_fs_db_operations") return [];
      if (command === "list_runtime_log_files") throw new Error(seed);
      return null;
    });

    render(<OperationLogsView />);
    await screen.findByText("scan.all");
    fireEvent.click(screen.getByRole("tab", { name: /运行日志/ }));

    await waitFor(() => expect(toast.error).toHaveBeenCalled());
    const toastPayload = vi.mocked(toast.error).mock.calls.flat().join(" ");
    expect(toastPayload).not.toContain(seed);
    expect(toastPayload).not.toContain("sk_live_secret");
    expect(document.body).not.toHaveTextContent(seed);
    expect(document.body).not.toHaveTextContent("sk_live_secret");
  });

  it("does not expose, copy, or link an invalid runtime operation ID", async () => {
    const invalidId = "private.internal-sk_live_secret";
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === "list_operation_logs") return mockPage;
      if (command === "list_pending_fs_db_operations") return [];
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
      if (command === "read_runtime_log_file") {
        return {
          ...mockRuntimeReadResult,
          lines: [
            {
              ...mockRuntimeReadResult.lines[0],
              operationId: invalidId,
              message: "Legacy runtime entry",
              raw: "2026-06-03T10:00:00Z WARN legacy runtime entry",
            },
          ],
        };
      }
      return null;
    });

    render(<OperationLogsView />);
    await screen.findByText("scan.all");
    fireEvent.click(screen.getByRole("tab", { name: /运行日志/ }));
    fireEvent.click(await screen.findByText("Legacy runtime entry"));

    expect(screen.getByText("旧版日志，无关联 ID")).toBeInTheDocument();
    expect(document.body).not.toHaveTextContent(invalidId);
    expect(
      screen.queryByTestId("runtime-log-copy-operation-id"),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByTestId("runtime-log-view-operation"),
    ).not.toBeInTheDocument();
  });

  it("traces one correlation ID across operation and frontend runtime views", async () => {
    render(<OperationLogsView />);

    await screen.findByText("scan.all");
    fireEvent.click(screen.getByText("scan.all").closest("button")!);
    fireEvent.click(await screen.findByTestId("logs-detail-view-runtime"));

    expect(await screen.findByText("运行时诊断")).toBeInTheDocument();
    expect((await screen.findAllByText("前端")).length).toBeGreaterThan(1);
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("read_runtime_log_file", {
        request: expect.objectContaining({ operationId: OPERATION_ID }),
      }),
    );

    fireEvent.click(screen.getByText("Render exploded").closest("button")!);
    fireEvent.click(await screen.findByTestId("runtime-log-copy-operation-id"));
    await waitFor(() =>
      expect(navigator.clipboard.writeText).toHaveBeenCalledWith(OPERATION_ID),
    );
    fireEvent.click(await screen.findByTestId("runtime-log-view-operation"));

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith("list_operation_logs", {
        filter: expect.objectContaining({ operationId: OPERATION_ID }),
      }),
    );
    expect(await screen.findByText("操作日志")).toBeInTheDocument();
  });
});
