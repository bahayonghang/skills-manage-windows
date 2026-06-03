// ─── Runtime Log Types ───────────────────────────────────────────────────────

export type RuntimeLogLevel = "error" | "warn" | "info" | "debug" | "trace";

export interface RuntimeLogFile {
  fileName: string;
  date: string;
  sizeBytes: number;
  modifiedAt?: string | null;
}

export interface RuntimeLogReadRequest {
  fileName: string;
  query?: string;
  level?: RuntimeLogLevel | "";
  source?: string;
  limit?: number;
  offset?: number;
  tail?: boolean;
}

export interface RuntimeLogLine {
  lineNumber: number;
  timestamp?: string | null;
  level?: RuntimeLogLevel | string | null;
  source: string;
  message: string;
  raw: string;
}

export interface RuntimeLogReadResult {
  fileName: string;
  total: number;
  limit: number;
  offset: number;
  lines: RuntimeLogLine[];
}

export interface RuntimeLogClearRequest {
  fileName?: string;
  olderThanDays?: number;
  all?: boolean;
}

export interface FrontendRuntimeLogPayload {
  level?: "error" | "warn" | "info" | "debug";
  source?: string;
  message?: string;
  details?: unknown;
}
