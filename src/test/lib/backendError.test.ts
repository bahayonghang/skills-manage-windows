import { describe, expect, it } from "vitest";
import type { TFunction } from "i18next";

import { formatBackendError, parseBackendError } from "@/lib/backendError";
import i18n from "@/i18n";

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

  it("localizes exclusive job errors without exposing the code envelope", async () => {
    await i18n.changeLanguage("en");
    expect(
      formatBackendError(
        "job.central_update_busy:A Central update job is already running.",
        i18n.t,
      ),
    ).toBe("A Central update is already running. Wait for it to finish or cancel it first.");

    await i18n.changeLanguage("zh");
    expect(
      formatBackendError(
        "job.portability_busy:A portability job is already running.",
        i18n.t,
      ),
    ).toBe("已有 SkillPort 状态导入或导出任务正在运行。");
  });
});
