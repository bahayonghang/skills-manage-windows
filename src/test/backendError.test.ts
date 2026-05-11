import { describe, expect, it } from "vitest";
import type { TFunction } from "i18next";

import { formatBackendError, parseBackendError } from "@/lib/backendError";

describe("backend error helpers", () => {
  it("parses coded backend errors with details", () => {
    const parsed = parseBackendError(
      "ai.rate_limit:Provider rate limit reached.\nHTTP 429: Too Many Requests"
    );

    expect(parsed).toEqual({
      code: "ai.rate_limit",
      message: "Provider rate limit reached.",
      details: "HTTP 429: Too Many Requests",
    });
  });

  it("formats known coded errors through i18n", () => {
    const t = ((key: string) =>
      key === "backendErrors.ai.missing_api_key"
        ? "请先配置 AI API Key"
        : "") as TFunction;

    expect(formatBackendError("ai.missing_api_key:Configure an AI API key.", t)).toBe(
      "请先配置 AI API Key"
    );
  });

  it("falls back to the plain message for uncoded errors", () => {
    expect(formatBackendError(new Error("DB failed"))).toBe("DB failed");
  });
});
