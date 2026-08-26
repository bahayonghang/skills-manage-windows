const CODE_PATTERN = /^[a-z][a-z0-9_]*(?:\.[a-z][a-z0-9_]*)+$/;
const LEGACY_CODED_ERROR_PATTERN =
  /^([a-z][a-z0-9_]*(?:[._-][a-z0-9_]+)+):(.*)$/s;
const INTERNAL_CODE = "internal.unexpected";
const INTERNAL_MESSAGE = "The operation failed. See runtime logs for details.";
const CORRELATION_ID_PATTERN =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;

export interface IpcErrorPayload {
  code: string;
  message: string;
  retryable: boolean;
  correlationId?: string;
}

export class IpcInvokeError extends Error implements IpcErrorPayload {
  readonly code: string;
  readonly retryable: boolean;
  readonly correlationId?: string;

  constructor(payload: IpcErrorPayload) {
    super(payload.message);
    this.name = "IpcInvokeError";
    this.code = payload.code;
    this.retryable = payload.retryable;
    this.correlationId = payload.correlationId;
  }

  override toString(): string {
    return this.message;
  }
}

export function isIpcErrorPayload(value: unknown): value is IpcErrorPayload {
  if (!value || typeof value !== "object") return false;
  const candidate = value as Partial<IpcErrorPayload>;
  return (
    typeof candidate.code === "string" &&
    CODE_PATTERN.test(candidate.code) &&
    typeof candidate.message === "string" &&
    typeof candidate.retryable === "boolean" &&
    (candidate.correlationId === undefined ||
      (typeof candidate.correlationId === "string" &&
        CORRELATION_ID_PATTERN.test(candidate.correlationId)))
  );
}

export function ipcFixtureError(
  code: string,
  message: string,
  retryable = false,
  correlationId?: string,
): IpcErrorPayload {
  const payload = { code, message, retryable, correlationId };
  if (!isIpcErrorPayload(payload)) {
    throw new Error(`Invalid fixture IPC error code: ${code}`);
  }
  return payload;
}

export function normalizeIpcRejection(error: unknown): Error {
  if (error instanceof IpcInvokeError) {
    return error;
  }
  if (error instanceof Error && error.name === "IpcFixtureMissingError") {
    return error;
  }
  if (isIpcErrorPayload(error)) {
    return new IpcInvokeError(error);
  }

  const raw = error instanceof Error ? error.message : error;
  if (typeof raw === "string") {
    const match = raw.match(LEGACY_CODED_ERROR_PATTERN);
    if (match) {
      return fromKnownCode(match[1].replace(/-/g, "_"), false);
    }
  }

  return unexpected();
}

/** Failure records retain shape and non-string scalars, never user strings. */
export function sanitizeIpcFailureArgs(value: unknown): unknown {
  if (Array.isArray(value)) {
    return value.map(sanitizeIpcFailureArgs);
  }
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value).map(([key, child]) => [
        key,
        sanitizeIpcFailureArgs(child),
      ]),
    );
  }
  return typeof value === "string" ? "[REDACTED]" : value;
}

function fromKnownCode(code: string, retryable: boolean): IpcInvokeError {
  const message = canonicalMessage(code);
  if (!message) return unexpected();
  return new IpcInvokeError({ code, message, retryable });
}

function unexpected(): IpcInvokeError {
  return new IpcInvokeError({
    code: INTERNAL_CODE,
    message: INTERNAL_MESSAGE,
    retryable: false,
  });
}

function canonicalMessage(code: string): string | null {
  switch (code) {
    case "operation.cancelled":
      return "The operation was cancelled.";
    case "portable_state.invalid_manifest_json":
      return "The SkillPort state file is not valid JSON.";
    case "portable_state.unsupported_export_kind":
      return "The SkillPort state export kind is not supported.";
    case "portable_state.unsupported_export_version":
      return "The SkillPort state export version is not supported.";
    case "github_import.rate_limited":
      return "GitHub rate limited the request. Try again later.";
    case "github_import.access_denied":
    case "github_import.configured_token_failed":
      return "GitHub denied access to the repository.";
    case "github_import.archive_redirect_rejected":
      return "GitHub repository archive redirect was rejected.";
    case "github_import.transport_failed":
      return "Could not reach GitHub. Check the network and try again.";
    case "github_import.repo_not_found":
      return "The GitHub repository was not found.";
    case "github_import.archive_unavailable":
      return "The GitHub repository archive is unavailable.";
    case "github_import.response_invalid":
      return "GitHub returned an unreadable response.";
    case "github_import.invalid_url":
      return "The GitHub request address is not allowed.";
    case "github_import.budget_exceeded":
      return "The GitHub repository exceeds the import resource limits.";
    case "github_import.credential_unavailable":
      return "The stored GitHub token could not be read. Save it again in Settings.";
    case "github_import.no_importable_skills":
      return "This GitHub repository does not contain an importable skill.";
    case "credential.ssh_password_unavailable":
      return "The SSH password is unavailable. Open Settings, save it, and retry.";
    case "runtime.desktop_required":
      return "This operation requires the Tauri desktop runtime.";
    case "resource.not_found":
      return "The requested resource was not found.";
    case "storage.unavailable":
      return "Storage is unavailable.";
    case "job.invalid_id":
      return "The job identifier is invalid.";
    case "job.id_mismatch":
      return "The cancellation request does not match the active job.";
    case "job.registry_unavailable":
      return "The job registry is unavailable.";
    case "job.central_update_busy":
      return "A Central update job is already running.";
    case "job.portability_busy":
      return "A portability job is already running.";
    case "startup.rebuild_unavailable":
      return "Database rebuild is not available.";
    case "central_operation.delete_restore_collision":
      return "Central recovery evidence conflicts with the current files. Review and resolve the pending operation in Operation Logs.";
    case "central_skills.delete_failed":
    case "central_skills.delete_preview_failed":
    case "central_skills.database_failed":
    case "central_skills.remote_failed":
    case "central_skills.budget_exceeded":
      return "This Central skill could not be deleted.";
    case "central_skills.force_delete_blocked":
      return "Force delete is not available for this Central skill.";
    default:
      if (code.startsWith("ai.")) return "The AI provider request failed.";
      if (code.startsWith("github_import.preview_")) {
        return "GitHub preview is unavailable. Preview the repository again.";
      }
      if (code.startsWith("local_archive.")) {
        return "The local archive import failed.";
      }
      return null;
  }
}
