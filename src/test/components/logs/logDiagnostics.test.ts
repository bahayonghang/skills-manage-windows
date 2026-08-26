import { describe, expect, it } from "vitest";

import {
  buildOperationDiagnostic,
  formatLogUiError,
  safeCorrelationId,
} from "@/components/logs/logDiagnostics";
import i18n from "@/i18n";
import type { OperationLogEntry } from "@/types";

function entry(detailsJson?: string): OperationLogEntry {
  return {
    id: "65e8ecdf-c398-4ed7-a4a1-87f193f0dd25",
    createdAt: "2026-08-27T00:00:00Z",
    level: "error",
    targetKind: "ssh",
    targetId: "ssh-target",
    category: "imports",
    action: "import.apply",
    status: "failed",
    summary: "Import failed",
    detailsJson,
  };
}

describe("logDiagnostics", () => {
  it("accepts UUID correlation IDs and rejects legacy row IDs", () => {
    expect(safeCorrelationId(entry().id)).toBe(entry().id);
    expect(safeCorrelationId("log-1")).toBeNull();
  });

  it("projects arbitrary UI errors to a fixed fallback", () => {
    const t = i18n.getFixedT("zh");
    const seed = "private.internal token=sk_live_secret C:/Users/private";

    const unknown = formatLogUiError(new Error(seed), t);
    const reviewed = formatLogUiError(
      {
        code: "github_import.access_denied",
        message: seed,
        retryable: false,
      },
      t,
    );

    expect(unknown).toBe(
      "没有记录额外的公开错误原因，请结合诊断键和关联运行日志排查。",
    );
    expect(reviewed).toContain("GitHub 拒绝匿名访问该仓库");
    expect(`${unknown} ${reviewed}`).not.toContain(seed);
    expect(`${unknown} ${reviewed}`).not.toContain("sk_live_secret");
  });

  it("projects only allowlisted keys and safe token values", () => {
    const diagnostic = buildOperationDiagnostic(
      entry(
        JSON.stringify({
          errorCode: "github_import.access_denied",
          phase: "apply",
          retryable: true,
          path: "C:/Users/private/secret",
          error: "token=abc",
          url: "https://private.invalid",
        }),
      ),
    );

    expect(diagnostic.formattedDetails).toContain(
      "github_import.access_denied",
    );
    expect(diagnostic.formattedDetails).not.toContain("Users/private");
    expect(diagnostic.formattedDetails).not.toContain("token=abc");
    expect(diagnostic.formattedDetails).not.toContain("private.invalid");
  });

  it("marks malformed details invalid and caps failure rows", () => {
    expect(buildOperationDiagnostic(entry("not-json")).detailsState).toBe(
      "invalid",
    );
    const diagnostic = buildOperationDiagnostic(
      entry(
        JSON.stringify({
          failureItems: Array.from({ length: 80 }, (_, index) => ({
            errorCode: "github_import.access_denied",
            identifier: `skill-${index}`,
          })),
        }),
      ),
    );
    expect(diagnostic.failureRows).toHaveLength(50);
  });
});
