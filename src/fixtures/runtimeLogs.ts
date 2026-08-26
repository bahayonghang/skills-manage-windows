import { registerIpcFixtures } from "@/lib/ipc";
import type {
  RuntimeLogFile,
  RuntimeLogLine,
  RuntimeLogReadRequest,
  RuntimeLogReadResult,
} from "@/types";

const DEFAULT_RUNTIME_LIMIT = 200;

const fixtureFiles: RuntimeLogFile[] = [
  {
    fileName: "skillport-2026-06-03.log",
    date: "2026-06-03",
    sizeBytes: 512,
    modifiedAt: "2026-06-03T10:00:00Z",
  },
];

const fixtureLines: RuntimeLogLine[] = [
  {
    lineNumber: 1,
    timestamp: "2026-06-03T10:00:00Z",
    level: "info",
    source: "skillport::startup",
    message: "SkillPort file logging initialized",
    raw: "2026-06-03T10:00:00Z INFO skillport::startup: SkillPort file logging initialized",
  },
  {
    lineNumber: 2,
    timestamp: "2026-06-03T10:02:00Z",
    level: "error",
    source: "window.error",
    operationId: "123e4567-e89b-42d3-a456-426614174000",
    eventSource: "frontend",
    message: "Example frontend runtime error",
    raw: "2026-06-03T10:02:00Z ERROR skillport::frontend: Example frontend runtime error source=window.error token=[REDACTED]",
  },
];

function fixtureRead(request: RuntimeLogReadRequest): RuntimeLogReadResult {
  const query = request.query?.toLowerCase();
  const source = request.source?.toLowerCase();
  const level = request.level?.toLowerCase();
  const operationId = request.operationId?.toLowerCase();
  const eventSource = request.eventSource?.toLowerCase();
  const matched = fixtureLines.filter((line) => {
    if (level && line.level?.toLowerCase() !== level) return false;
    if (source && !line.source.toLowerCase().includes(source)) return false;
    if (operationId && line.operationId?.toLowerCase() !== operationId) {
      return false;
    }
    if (eventSource && line.eventSource?.toLowerCase() !== eventSource) {
      return false;
    }
    if (query) {
      const haystack =
        `${line.source} ${line.message} ${line.raw}`.toLowerCase();
      if (!haystack.includes(query)) return false;
    }
    return true;
  });
  const limit = request.limit ?? DEFAULT_RUNTIME_LIMIT;
  const offset = request.tail
    ? Math.max(0, matched.length - limit)
    : Math.max(0, request.offset ?? 0);

  return {
    fileName: request.fileName,
    total: matched.length,
    limit,
    offset,
    lines: matched.slice(offset, offset + limit),
  };
}

export function registerRuntimeLogFixtures(): void {
  registerIpcFixtures({
    list_runtime_log_files: () => fixtureFiles,
    read_runtime_log_file: ({ request }) => fixtureRead(request),
    clear_runtime_logs: ({ request }) =>
      request.fileName || request.all ? 1 : 0,
    export_runtime_log_file: () =>
      fixtureLines.map((line) => line.raw).join("\n"),
  });
}
