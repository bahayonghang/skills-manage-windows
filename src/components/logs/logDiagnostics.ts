import type { TFunction } from "i18next";

import { formatBackendError, parseBackendError } from "@/lib/backendError";
import { isReviewedIpcCode, isSafeCorrelationId } from "@/lib/ipc/errors";
import type { OperationLogEntry } from "@/types";

export interface ApplyFailureRow {
  errorCode: string;
}

export interface OperationDiagnosticModel {
  correlationId: string | null;
  errorCode: string | null;
  errorCategory: string | null;
  phase: string | null;
  retryable: boolean | null;
  failureRows: ApplyFailureRow[];
  truncatedFailureCount: number;
  formattedDetails: string | null;
  detailsState: "available" | "empty" | "invalid";
}

const SAFE_PHASES = new Set([
  "apply",
  "archive_build",
  "central_write",
  "checking",
  "completed",
  "command",
  "copy_refresh",
  "database",
  "db_persist",
  "decision_apply",
  "durable_archive_build",
  "durable_stage",
  "filesystem",
  "local_or_remote_hash",
  "job",
  "network",
  "prepare",
  "recovery",
  "snapshot_download",
  "started",
  "startup",
  "target_open",
  "terminal",
]);
const SAFE_STATUS_VALUES = new Set([
  "started",
  "succeeded",
  "failed",
  "partial",
  "cancelled",
  "interrupted",
]);
const SAFE_TARGET_KINDS = new Set(["local", "ssh", "wsl"]);
const SAFE_EVENT_SOURCES = new Set(["backend", "frontend"]);
const SAFE_ORIGINS = new Set(["operation", "runtime"]);
const SAFE_COUNT_KEYS = new Set([
  "added",
  "cancelled",
  "created",
  "deleted",
  "exported",
  "failed",
  "failedCount",
  "imported",
  "installed",
  "linked",
  "partial",
  "partialCount",
  "removed",
  "requested",
  "requestedAgents",
  "requestedRepositories",
  "requestedSkills",
  "scanned",
  "skipped",
  "skippedCount",
  "succeeded",
  "succeededCount",
  "total",
  "totalSkills",
  "truncatedCount",
  "unchanged",
  "unlinked",
  "updated",
]);

function asRecord(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  return value as Record<string, unknown>;
}

function safeEnum(value: unknown, allowed: Set<string>): string | null {
  return typeof value === "string" && allowed.has(value) ? value : null;
}

export function safeCorrelationId(value: unknown): string | null {
  return isSafeCorrelationId(value) ? value : null;
}

export function formatLogUiError(error: unknown, t: TFunction): string {
  const parsed = parseBackendError(error);
  const fallback = t("logs.diagnostics.reasons.unavailable");
  if (!isReviewedIpcCode(parsed.code)) return fallback;

  return formatBackendError(
    {
      code: parsed.code,
      message: fallback,
      retryable: parsed.retryable,
    },
    t,
  );
}

function readFailureRows(record: Record<string, unknown>): {
  rows: ApplyFailureRow[];
  truncated: number;
} {
  const candidates = Array.isArray(record.failureItems)
    ? record.failureItems.flatMap((item) => {
        const row = asRecord(item);
        return row && isReviewedIpcCode(row.errorCode)
          ? [{ errorCode: row.errorCode }]
          : [];
      })
    : Array.isArray(record.failureCodes)
      ? record.failureCodes.flatMap((code) =>
          isReviewedIpcCode(code) ? [{ errorCode: code }] : [],
        )
      : [];
  return {
    rows: candidates.slice(0, 50),
    truncated: Math.max(0, candidates.length - 50),
  };
}

function projectSafeDetails(
  record: Record<string, unknown>,
  failureRows: ApplyFailureRow[],
  truncatedFailureCount: number,
): Record<string, unknown> {
  const projected: Record<string, unknown> = {};

  for (const [key, value] of Object.entries(record)) {
    if (SAFE_COUNT_KEYS.has(key)) {
      if (typeof value === "number" && Number.isFinite(value)) {
        projected[key] = value;
      }
      continue;
    }
    if (key === "retryable" && typeof value === "boolean") {
      projected.retryable = value;
      continue;
    }
    if (key === "errorCode" && isReviewedIpcCode(value)) {
      projected.errorCode = value;
      continue;
    }
    if (
      (key === "operationId" || key === "batchId") &&
      isSafeCorrelationId(value)
    ) {
      projected[key] = value;
      continue;
    }
    if ((key === "phase" || key === "step") && safeEnum(value, SAFE_PHASES)) {
      projected[key] = value;
      continue;
    }
    if (key === "status" && safeEnum(value, SAFE_STATUS_VALUES)) {
      projected.status = value;
      continue;
    }
    if (key === "targetKind" && safeEnum(value, SAFE_TARGET_KINDS)) {
      projected.targetKind = value;
      continue;
    }
    if (key === "eventSource" && safeEnum(value, SAFE_EVENT_SOURCES)) {
      projected.eventSource = value;
      continue;
    }
    if (key === "correlationOrigin" && safeEnum(value, SAFE_ORIGINS)) {
      projected.correlationOrigin = value;
    }
  }

  if (failureRows.length > 0) projected.failureItems = failureRows;
  if (truncatedFailureCount > 0) {
    projected.truncatedFailureCount = truncatedFailureCount;
  }
  return projected;
}

export function buildOperationDiagnostic(
  entry: OperationLogEntry,
): OperationDiagnosticModel {
  const correlationId = safeCorrelationId(entry.id);
  const empty: Omit<OperationDiagnosticModel, "detailsState"> = {
    correlationId,
    errorCode: null,
    errorCategory: null,
    phase: null,
    retryable: null,
    failureRows: [],
    truncatedFailureCount: 0,
    formattedDetails: null,
  };
  if (!entry.detailsJson) return { ...empty, detailsState: "empty" };

  let parsed: unknown;
  try {
    parsed = JSON.parse(entry.detailsJson);
  } catch {
    return { ...empty, detailsState: "invalid" };
  }

  const record = asRecord(parsed);
  if (!record) return { ...empty, detailsState: "empty" };

  const { rows: failureRows, truncated: truncatedFailureCount } =
    readFailureRows(record);
  const errorCode = isReviewedIpcCode(record.errorCode)
    ? record.errorCode
    : (failureRows[0]?.errorCode ?? null);
  const phase = safeEnum(record.phase, SAFE_PHASES);
  const projected = projectSafeDetails(
    record,
    failureRows,
    truncatedFailureCount,
  );
  const formattedDetails =
    Object.keys(projected).length > 0
      ? JSON.stringify(projected, null, 2)
      : null;

  return {
    correlationId,
    errorCode,
    errorCategory: errorCode?.split(".", 1)[0] ?? null,
    phase,
    retryable: typeof record.retryable === "boolean" ? record.retryable : null,
    failureRows,
    truncatedFailureCount,
    formattedDetails,
    detailsState: formattedDetails ? "available" : "empty",
  };
}
