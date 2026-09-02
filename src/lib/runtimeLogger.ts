import type { FrontendRuntimeLogPayload } from "@/types";
import {
  IpcInvokeError,
  invokeRaw,
  isReviewedIpcCode,
  isSafeCorrelationId,
  registerIpcFailureRecorder,
  sanitizeIpcFailureArgs,
  type CommandArgs,
} from "@/lib/ipc";

type RuntimeLogLevel = NonNullable<FrontendRuntimeLogPayload["level"]>;

type RuntimeLogCommandPayload = CommandArgs<"record_frontend_runtime_log">["payload"];
type RuntimeLogDetails = Exclude<RuntimeLogCommandPayload["details"], undefined>;

type RuntimeLogEventDetail =
  FrontendRuntimeLogPayload | string | null | undefined;

type TauriWindow = Window & {
  __TAURI__?: unknown;
  __TAURI_INTERNALS__?: unknown;
};

const RUNTIME_EVENT_NAME = "frontend.runtime";
const MAX_DETAIL_DEPTH = 4;
const MAX_ARRAY_ITEMS = 20;
const MAX_OBJECT_FIELDS = 50;

let isInstalled = false;
let cleanupHandlers: Array<() => void> = [];

function hasTauriRuntime(): boolean {
  if (typeof window === "undefined") return false;
  const tauriWindow = window as TauriWindow;
  return Boolean(tauriWindow.__TAURI__ || tauriWindow.__TAURI_INTERNALS__);
}

function normalizeLevel(level: unknown): RuntimeLogLevel {
  if (level === "error" || level === "warn" || level === "debug") return level;
  return "info";
}

function normalizeSource(
  source: unknown,
): NonNullable<FrontendRuntimeLogPayload["source"]> {
  if (
    source === "ipc.failure" ||
    source === "window.error" ||
    source === "window.unhandledrejection"
  ) {
    return source;
  }
  return "frontend.runtime";
}

function messageForSource(source: string): string {
  switch (source) {
    case "ipc.failure":
      return "IPC command failed";
    case "window.error":
      return "A window error occurred";
    case "window.unhandledrejection":
      return "An unhandled promise rejection occurred";
    default:
      return "Frontend runtime event";
  }
}

function reviewedErrorName(error: unknown): string {
  const name =
    typeof error === "string"
      ? error
      : error instanceof Error
        ? error.name
        : undefined;
  return [
    "Error",
    "TypeError",
    "RangeError",
    "ReferenceError",
    "SyntaxError",
    "URIError",
    "EvalError",
    "AggregateError",
    "DOMException",
  ].includes(name ?? "")
    ? (name as string)
    : "Error";
}

function safeNumber(value: unknown): number | undefined {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0
    ? value
    : undefined;
}

function redactValue(value: unknown, depth = 0): RuntimeLogDetails | null {
  if (value == null) return null;
  if (typeof value === "string") return "[REDACTED]";
  if (typeof value === "number" || typeof value === "boolean") return value;
  if (value instanceof Error) {
    return { errorName: reviewedErrorName(value) };
  }
  if (depth >= MAX_DETAIL_DEPTH) return "[MaxDepth]";
  if (Array.isArray(value)) {
    return value
      .slice(0, MAX_ARRAY_ITEMS)
      .map((item) => redactValue(item, depth + 1));
  }
  if (typeof value === "object") {
    const redacted: { [key: string]: RuntimeLogDetails } = {};
    const entries = Object.entries(value as Record<string, unknown>).slice(
      0,
      MAX_OBJECT_FIELDS,
    );
    for (const [index, [, nestedValue]] of entries.entries()) {
      redacted[`field_${index}`] = redactValue(nestedValue, depth + 1);
    }
    return redacted;
  }
  return "[Unsupported]";
}

function normalizeIpcFailureDetails(details: unknown): RuntimeLogDetails | undefined {
  if (!details || typeof details !== "object") return undefined;
  const candidate = details as {
    command?: unknown;
    args?: unknown;
    error?: { code?: unknown; retryable?: unknown };
    correlationOrigin?: unknown;
  };
  const command =
    typeof candidate.command === "string" &&
    /^[a-z][a-z0-9_]*$/.test(candidate.command)
      ? candidate.command
      : "unknown";
  const candidateCode = candidate.error?.code;
  const code = isReviewedIpcCode(candidateCode)
    ? candidateCode
    : "internal.unexpected";
  return {
    command,
    args: redactValue(candidate.args),
    error: {
      code,
      retryable:
        typeof candidate.error?.retryable === "boolean"
          ? candidate.error.retryable
          : false,
    },
    correlationOrigin:
      candidate.correlationOrigin === "backend" ? "backend" : "frontend",
  };
}

function normalizeGlobalFailureDetails(
  details: unknown,
  includePosition: boolean,
): RuntimeLogDetails {
  if (!details || typeof details !== "object") {
    return { errorName: "Error" };
  }
  const candidate = details as Record<string, unknown>;
  const normalized: { [key: string]: RuntimeLogDetails } = {
    errorName:
      typeof candidate.errorName === "string"
        ? reviewedErrorName(candidate.errorName)
        : "Error",
  };
  if (includePosition) {
    const line = safeNumber(candidate.line);
    const column = safeNumber(candidate.column);
    if (line !== undefined) normalized.line = line;
    if (column !== undefined) normalized.column = column;
  }
  return normalized;
}

function normalizeDetails(
  source: string,
  details: unknown,
): RuntimeLogDetails | null | undefined {
  if (source === "ipc.failure") return normalizeIpcFailureDetails(details);
  if (source === "window.error") {
    return normalizeGlobalFailureDetails(details, true);
  }
  if (source === "window.unhandledrejection") {
    return normalizeGlobalFailureDetails(details, false);
  }
  return redactValue(details);
}

function normalizePayload(
  payload: FrontendRuntimeLogPayload,
): RuntimeLogCommandPayload {
  const source = normalizeSource(payload.source);
  return {
    level: normalizeLevel(payload.level),
    source,
    message: messageForSource(source),
    details: normalizeDetails(source, payload.details),
    operationId: isSafeCorrelationId(payload.operationId)
      ? payload.operationId
      : undefined,
  };
}

function payloadFromRuntimeEvent(
  detail: RuntimeLogEventDetail,
): FrontendRuntimeLogPayload {
  if (typeof detail === "string") {
    return {
      level: "info",
      source: RUNTIME_EVENT_NAME,
      message: detail,
    };
  }
  if (detail && typeof detail === "object") {
    return detail as FrontendRuntimeLogPayload;
  }
  return {
    level: "info",
    source: RUNTIME_EVENT_NAME,
    message: "Frontend runtime event",
  };
}

export async function recordFrontendRuntimeLog(
  payload: FrontendRuntimeLogPayload,
): Promise<void> {
  if (!hasTauriRuntime()) return;

  try {
    await invokeRaw("record_frontend_runtime_log", {
      payload: normalizePayload(payload),
    });
  } catch {
    // Runtime logging is diagnostic only. It must never break the user flow.
  }
}

function createFrontendCorrelationId(): string {
  const runtimeCrypto = globalThis.crypto;
  if (typeof runtimeCrypto?.randomUUID === "function") {
    const value = runtimeCrypto.randomUUID();
    if (isSafeCorrelationId(value)) return value;
  }

  const bytes = new Uint8Array(16);
  if (typeof runtimeCrypto?.getRandomValues === "function") {
    runtimeCrypto.getRandomValues(bytes);
  } else {
    for (let index = 0; index < bytes.length; index += 1) {
      bytes[index] = Math.floor(Math.random() * 256);
    }
  }
  bytes[6] = (bytes[6] & 0x0f) | 0x40;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;
  const hex = Array.from(bytes, (value) => value.toString(16).padStart(2, "0"));
  return `${hex.slice(0, 4).join("")}-${hex.slice(4, 6).join("")}-${hex.slice(6, 8).join("")}-${hex.slice(8, 10).join("")}-${hex.slice(10).join("")}`;
}

export function emitFrontendRuntimeLog(
  payload: FrontendRuntimeLogPayload,
): void {
  if (typeof window === "undefined") return;
  window.dispatchEvent(
    new CustomEvent(RUNTIME_EVENT_NAME, { detail: payload }),
  );
}

export function recordIpcFailure(
  command: string,
  args: unknown,
  error: unknown,
): void {
  const backendCorrelationId =
    error instanceof IpcInvokeError && isSafeCorrelationId(error.correlationId)
      ? error.correlationId
      : undefined;
  void recordFrontendRuntimeLog({
    level: "error",
    source: "ipc.failure",
    message: "IPC command failed",
    operationId: backendCorrelationId ?? createFrontendCorrelationId(),
    details: {
      command,
      args: sanitizeIpcFailureArgs(args),
      error: {
        code:
          error instanceof IpcInvokeError ? error.code : "internal.unexpected",
        retryable: error instanceof IpcInvokeError ? error.retryable : false,
      },
      correlationOrigin: backendCorrelationId ? "backend" : "frontend",
    },
  });
}

export function installRuntimeLogger(): void {
  if (isInstalled || typeof window === "undefined") return;
  isInstalled = true;

  const handleError = (event: ErrorEvent) => {
    void recordFrontendRuntimeLog({
      level: "error",
      source: "window.error",
      message: "A window error occurred",
      details: {
        errorName: reviewedErrorName(event.error),
        line: safeNumber(event.lineno),
        column: safeNumber(event.colno),
      },
    });
  };

  const handleUnhandledRejection = (event: PromiseRejectionEvent) => {
    void recordFrontendRuntimeLog({
      level: "error",
      source: "window.unhandledrejection",
      message: "An unhandled promise rejection occurred",
      details: { errorName: reviewedErrorName(event.reason) },
    });
  };

  const handleRuntimeEvent = (event: Event) => {
    const detail = event instanceof CustomEvent ? event.detail : undefined;
    void recordFrontendRuntimeLog(payloadFromRuntimeEvent(detail));
  };

  window.addEventListener("error", handleError);
  window.addEventListener("unhandledrejection", handleUnhandledRejection);
  window.addEventListener(RUNTIME_EVENT_NAME, handleRuntimeEvent);
  registerIpcFailureRecorder(recordIpcFailure);

  cleanupHandlers = [
    () => window.removeEventListener("error", handleError),
    () =>
      window.removeEventListener(
        "unhandledrejection",
        handleUnhandledRejection,
      ),
    () => window.removeEventListener(RUNTIME_EVENT_NAME, handleRuntimeEvent),
    () => registerIpcFailureRecorder(null),
  ];
}

export function __resetRuntimeLoggerForTest(): void {
  cleanupHandlers.forEach((cleanup) => cleanup());
  cleanupHandlers = [];
  isInstalled = false;
  registerIpcFailureRecorder(null);
}
