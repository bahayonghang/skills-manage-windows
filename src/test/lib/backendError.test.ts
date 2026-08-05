import { describe, expect, it } from "vitest";
import type { TFunction } from "i18next";

import {
  backendErrorStateValue,
  formatBackendError,
  parseBackendError,
} from "@/lib/backendError";
import { IpcInvokeError } from "@/lib/ipc";
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
      retryable: false,
    });
  });

  it("reads structured IPC errors without reparsing their message", () => {
    expect(
      parseBackendError(
        new IpcInvokeError({
          code: "github_import.rate_limited",
          message: "Try again later",
          retryable: true,
        }),
      ),
    ).toEqual({
      code: "github_import.rate_limited",
      message: "Try again later",
      details: null,
      retryable: true,
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

  it("preserves and localizes the archive redirect code without retaining details", async () => {
    const stateValue = backendErrorStateValue(
      "github_import.archive_redirect_rejected:Archive redirect rejected.\nghp_secret /private/path",
    );
    expect(stateValue).toBe(
      "github_import.archive_redirect_rejected:Archive redirect rejected.",
    );

    await i18n.changeLanguage("en");
    expect(formatBackendError(stateValue, i18n.t)).toBe(
      "GitHub returned an unsafe or unexpected repository archive redirect. Try checking for updates again.",
    );

    await i18n.changeLanguage("zh");
    expect(formatBackendError(stateValue, i18n.t)).toBe(
      "GitHub 返回了不安全或异常的仓库压缩包跳转，已停止检查更新。请重试。",
    );
  });

  it("localizes every GitHub network failure code in both languages", async () => {
    const codes = [
      "github_import.transport_failed",
      "github_import.rate_limited",
      "github_import.access_denied",
      "github_import.repo_not_found",
      "github_import.archive_unavailable",
      "github_import.response_invalid",
      "github_import.invalid_url",
      "github_import.budget_exceeded",
      "github_import.credential_unavailable",
      "central_updates.repository_check_failed",
    ];

    for (const language of ["en", "zh"] as const) {
      await i18n.changeLanguage(language);
      for (const code of codes) {
        const message = formatBackendError(
          `${code}:ghp_secret https://codeload.github.com/private/repo`,
          i18n.t,
        );
        expect(message, `${language} ${code}`).not.toBe("");
        expect(message, `${language} ${code}`).not.toContain(code);
        expect(message, `${language} ${code}`).not.toContain("ghp_secret");
        expect(message, `${language} ${code}`).not.toContain("codeload");
      }
    }
  });
});
