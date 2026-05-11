import type { TFunction } from "i18next";

const CODED_ERROR_PATTERN = /^([a-z][a-z0-9_]*(?:[._-][a-z0-9_]+)+):(.*)$/s;

export interface ParsedBackendError {
  code: string | null;
  message: string;
  details: string | null;
}

function stringifyError(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export function parseBackendError(error: unknown): ParsedBackendError {
  const raw = stringifyError(error);
  const match = raw.match(CODED_ERROR_PATTERN);
  if (!match) {
    return {
      code: null,
      message: raw,
      details: null,
    };
  }

  const [, code, rest] = match;
  const [messageLine = "", ...detailsLines] = rest.trimStart().split(/\r?\n/);
  return {
    code,
    message: messageLine.trim() || raw,
    details: detailsLines.join("\n").trim() || null,
  };
}

export function formatBackendError(error: unknown, t?: TFunction): string {
  const parsed = parseBackendError(error);
  if (!parsed.code || !t) {
    return parsed.message;
  }

  const key = `backendErrors.${parsed.code}`;
  const translated = t(key, { defaultValue: "" }) as string;
  return translated || parsed.message;
}
