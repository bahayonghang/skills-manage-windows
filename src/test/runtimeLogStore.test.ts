import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@/lib/tauri", () => ({
  invoke: vi.fn(),
  isTauriRuntime: vi.fn(() => true),
}));

import { invoke } from "@/lib/tauri";
import { useRuntimeLogStore } from "@/stores/runtimeLogStore";
import type { RuntimeLogFile, RuntimeLogReadResult } from "@/types";

const mockFiles: RuntimeLogFile[] = [
  {
    fileName: "skillport-2026-06-03.log",
    date: "2026-06-03",
    sizeBytes: 256,
    modifiedAt: "2026-06-03T10:00:00Z",
  },
];

const mockReadResult: RuntimeLogReadResult = {
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
      message: "Boom",
      raw: "ERROR Boom token=[REDACTED]",
    },
  ],
};

function resetStore() {
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

describe("runtimeLogStore", () => {
  beforeEach(() => {
    resetStore();
    vi.clearAllMocks();
  });

  it("loads runtime log files", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(mockFiles);

    const files = await useRuntimeLogStore.getState().loadFiles();

    expect(files).toEqual(mockFiles);
    expect(useRuntimeLogStore.getState().selectedFileName).toBe(
      "skillport-2026-06-03.log"
    );
    expect(invoke).toHaveBeenCalledWith("list_runtime_log_files");
  });

  it("reads a selected runtime log with normalized filters", async () => {
    useRuntimeLogStore.setState({
      selectedFileName: "skillport-2026-06-03.log",
      filter: { query: " boom ", level: "error", source: " window " },
    });
    vi.mocked(invoke).mockResolvedValueOnce(mockReadResult);

    await useRuntimeLogStore.getState().readFile();

    expect(invoke).toHaveBeenCalledWith("read_runtime_log_file", {
      request: {
        fileName: "skillport-2026-06-03.log",
        query: "boom",
        level: "error",
        source: "window",
        limit: 200,
        offset: 0,
        tail: true,
      },
    });
    expect(useRuntimeLogStore.getState().readResult).toEqual(mockReadResult);
  });

  it("exports the selected runtime log", async () => {
    useRuntimeLogStore.setState({ selectedFileName: "skillport-2026-06-03.log" });
    vi.mocked(invoke).mockResolvedValueOnce("redacted runtime log");

    const payload = await useRuntimeLogStore.getState().exportLog();

    expect(payload).toBe("redacted runtime log");
    expect(invoke).toHaveBeenCalledWith("export_runtime_log_file", {
      fileName: "skillport-2026-06-03.log",
    });
  });

  it("clears the selected runtime log and reloads files", async () => {
    useRuntimeLogStore.setState({ selectedFileName: "skillport-2026-06-03.log" });
    vi.mocked(invoke)
      .mockResolvedValueOnce(1)
      .mockResolvedValueOnce([]);

    const deleted = await useRuntimeLogStore.getState().clearLogs();

    expect(deleted).toBe(1);
    expect(invoke).toHaveBeenNthCalledWith(1, "clear_runtime_logs", {
      request: { fileName: "skillport-2026-06-03.log" },
    });
    expect(invoke).toHaveBeenNthCalledWith(2, "list_runtime_log_files");
    expect(useRuntimeLogStore.getState().readResult).toBeNull();
  });
});
