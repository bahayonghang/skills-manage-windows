// ─── Operation Log Types ─────────────────────────────────────────────────────

export type OperationLogLevel = "info" | "warn" | "error";
export type OperationLogStatus = "succeeded" | "failed" | "partial" | "cancelled";
export type OperationLogTargetKind = "local" | "ssh" | "wsl";

export interface OperationLogEntry {
  id: string;
  createdAt: string;
  level: OperationLogLevel | string;
  targetKind: OperationLogTargetKind | string;
  targetId: string;
  targetLabel?: string | null;
  category: string;
  action: string;
  status: OperationLogStatus | string;
  subjectType?: string | null;
  subjectId?: string | null;
  subjectLabel?: string | null;
  summary: string;
  errorSummary?: string | null;
  detailsJson?: string | null;
  durationMs?: number | null;
  batchId?: string | null;
}

export interface OperationLogFilter {
  query?: string;
  targetKind?: OperationLogTargetKind | "";
  targetId?: string;
  level?: OperationLogLevel | "";
  status?: OperationLogStatus | "";
  category?: string;
  action?: string;
  createdAfter?: string;
  createdBefore?: string;
  limit?: number;
  offset?: number;
}

export interface OperationLogPage {
  entries: OperationLogEntry[];
  total: number;
  limit: number;
  offset: number;
}
