import { describe, expect, it, vi } from "vitest";
import { render, screen, within } from "@testing-library/react";

import { OperationLogDetailDrawer } from "@/components/logs/OperationLogDetailDrawer";
import type { OperationLogEntry } from "@/types";

vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

const CREATED_AT = "2026-08-17T11:49:15.213736600+00:00";

function makeEntry(overrides: Partial<OperationLogEntry> = {}): OperationLogEntry {
  return {
    id: "log-apply",
    createdAt: CREATED_AT,
    level: "error",
    targetKind: "local",
    targetId: "local",
    targetLabel: "Local",
    category: "update_center",
    action: "update_center.apply",
    status: "failed",
    summary: "Skill update decisions failed",
    ...overrides,
  };
}

describe("OperationLogDetailDrawer", () => {
  it("shows local created-at text without the UTC offset and keeps the original ISO in title", async () => {
    render(
      <OperationLogDetailDrawer
        open
        entry={makeEntry()}
        onOpenChange={() => undefined}
      />,
    );

    const drawer = await screen.findByTestId("operation-log-detail-drawer");
    const createdAt = within(drawer).getByTitle(CREATED_AT);
    expect(createdAt).toHaveTextContent(/\d/);
    expect(createdAt.textContent).not.toContain("+00:00");
    expect(createdAt).toHaveAttribute("title", CREATED_AT);
  });

  it("renders the localized public sentence and identifier for apply failure items", async () => {
    render(
      <OperationLogDetailDrawer
        open
        entry={makeEntry({
          detailsJson: JSON.stringify({
            failureItems: [
              {
                step: "import_addition",
                identifier: "github:emilkowalski-skill-main",
                phase: "decision_apply",
                errorCode: "github_import.access_denied",
                errorCategory: "github_import.access_denied",
                error: "token=secret https://example.invalid C:/Users/private",
              },
            ],
          }),
        })}
        onOpenChange={() => undefined}
      />,
    );

    const failures = await screen.findByTestId("logs-detail-failures");
    expect(failures).toHaveTextContent(
      "GitHub 拒绝访问该仓库。请确认令牌具备读取权限。",
    );
    expect(failures).toHaveTextContent("github:emilkowalski-skill-main");
    expect(failures).not.toHaveTextContent("token=secret");
    expect(failures).not.toHaveTextContent("example.invalid");
    expect(failures).not.toHaveTextContent("Users/private");
  });
});
