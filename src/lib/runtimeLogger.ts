import type { FrontendRuntimeLogPayload } from "@/types";
import { invokeRaw, registerIpcFailureRecorder } from "@/lib/tauri";

type RuntimeLogLevel = NonNullable<FrontendRuntimeLogPayload["level"]>;

type RuntimeLogEventDetail =
  FrontendRuntimeLogPayload | string | null | undefined;

type TauriWindow = Window & {
  __TAURI__?: unknown;
  __TAURI_INTERNALS__?: unknown;
};

const RUNTIME_EVENT_NAME = "frontend.runtime";
const MAX_DETAIL_DEPTH = 4;
const MAX_ARRAY_ITEMS = 20;
const MAX_STRING_LENGTH = 1_000;
const SENSITIVE_KEY_PATTERN =
  /password|passphrase|token|pat|api[-_]?key|apikey|secret|private[-_]?key|credential/i;

let isInstalled = false;
let isRecording = false;
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

function truncateString(value: string): string {
  if (value.length <= MAX_STRING_LENGTH) return value;
  return `${value.slice(0, MAX_STRING_LENGTH)}…`;
}

function redactValue(value: unknown, depth = 0): unknown {
  if (value == null) return value;
  if (typeof value === "string") return truncateString(value);
  if (typeof value === "number" || typeof value === "boolean") return value;
  if (value instanceof Error) {
    return {
      name: value.name,
      message: truncateString(value.message),
      stack: value.stack ? truncateString(value.stack) : undefined,
    };
  }
  if (depth >= MAX_DETAIL_DEPTH) return "[MaxDepth]";
  if (Array.isArray(value)) {
    return value
      .slice(0, MAX_ARRAY_ITEMS)
      .map((item) => redactValue(item, depth + 1));
  }
  if (typeof value === "object") {
    const redacted: Record<string, unknown> = {};
    for (const [key, nestedValue] of Object.entries(
      value as Record<string, unknown>,
    )) {
      redacted[key] = SENSITIVE_KEY_PATTERN.test(key)
        ? "[REDACTED]"
        : redactValue(nestedValue, depth + 1);
    }
    return redacted;
  }
  return String(value);
}

function stringifyError(error: unknown): string {
  if (error instanceof Error) return error.message;
  return String(error);
}

function errorToDetails(error: unknown): unknown {
  if (error instanceof Error) {
    return {
      name: error.name,
      message: error.message,
      stack: error.stack,
    };
  }
  return { value: String(error) };
}

function normalizePayload(
  payload: FrontendRuntimeLogPayload,
): FrontendRuntimeLogPayload {
  return {
    level: normalizeLevel(payload.level),
    source: payload.source?.trim() || "frontend.runtime",
    message: payload.message?.trim() || "Frontend runtime event",
    details: redactValue(payload.details),
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
  if (!hasTauriRuntime() || isRecording) return;

  isRecording = true;
  try {
    await invokeRaw<void>("record_frontend_runtime_log", {
      payload: normalizePayload(payload),
    });
  } catch {
    // Runtime logging is diagnostic only. It must never break the user flow.
  } finally {
    isRecording = false;
  }
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
  void recordFrontendRuntimeLog({
    level: "error",
    source: "ipc.failure",
    message: `IPC command failed: ${command}`,
    details: {
      command,
      args: redactValue(args),
      error: errorToDetails(error),
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
      message: event.message || "Window error",
      details: {
        filename: event.filename,
        lineno: event.lineno,
        colno: event.colno,
        error: errorToDetails(event.error),
      },
    });
  };

  const handleUnhandledRejection = (event: PromiseRejectionEvent) => {
    void recordFrontendRuntimeLog({
      level: "error",
      source: "window.unhandledrejection",
      message: `Unhandled promise rejection: ${stringifyError(event.reason)}`,
      details: errorToDetails(event.reason),
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
  isRecording = false;
  registerIpcFailureRecorder(null);
}
