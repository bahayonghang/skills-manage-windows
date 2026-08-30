import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { OperationLogDetailDialog } from "@/components/logs/OperationLogDetailDialog";
import type { OperationLogEntry } from "@/types";

vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

const OPERATION_ID = "65e8ecdf-c398-4ed7-a4a1-87f193f0dd25";

function makeEntry(
  overrides: Partial<OperationLogEntry> = {},
): OperationLogEntry {
  return {
    id: OPERATION_ID,
    createdAt: "2026-08-17T11:49:15.213736600+00:00",
    level: "error",
    targetKind: "local",
    targetId: "local",
    targetLabel: "Local",
    category: "update_center",
    action: "update_center.apply",
    status: "failed",
    summary: "Skill update decisions failed",
    errorSummary: "The update could not be applied.",
    ...overrides,
  };
}

describe("OperationLogDetailDialog", () => {
  it("uses a centered, viewport-safe dialog with structured details collapsed", async () => {
    render(
      <OperationLogDetailDialog
        open
        entry={makeEntry({ detailsJson: '{"totalSkills":12}' })}
        onOpenChange={() => undefined}
      />,
    );

    const dialog = await screen.findByTestId("operation-log-detail-dialog");
    expect(dialog).toHaveClass("top-1/2", "left-1/2", "sm:max-w-[35rem]");
    expect(dialog).not.toHaveClass("right-0", "h-full");
    expect(
      within(dialog).getByText("安全结构化详情").closest("details"),
    ).not.toHaveAttribute("open");
  });

  it("shows only localized reviewed failure content and omits raw identifiers", async () => {
    render(
      <OperationLogDetailDialog
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

    const dialog = await screen.findByTestId("operation-log-detail-dialog");
    expect(dialog).toHaveTextContent(
      "GitHub 拒绝匿名访问该仓库。如果仓库需要认证，请配置 GitHub 令牌。",
    );
    expect(dialog).not.toHaveTextContent("github:emilkowalski-skill-main");
    expect(dialog).not.toHaveTextContent("token=secret");
    expect(dialog).not.toHaveTextContent("example.invalid");
    expect(dialog).not.toHaveTextContent("Users/private");
  });

  it("does not render legal-looking hosts, tokens, or backend prose", async () => {
    render(
      <OperationLogDetailDialog
        open
        entry={makeEntry({
          summary: "private.internal",
          errorSummary: "sk_live_secret123",
          targetLabel: "private.internal",
          subjectLabel: "sk_live_secret123",
          detailsJson: JSON.stringify({
            errorCode: "private.internal",
            errorCategory: "private.internal",
            phase: "sk_live_secret123",
            failureItems: [
              {
                errorCode: "private.internal",
                identifier: "sk_live_secret123",
              },
            ],
          }),
        })}
        onOpenChange={() => undefined}
      />,
    );

    const dialog = await screen.findByTestId("operation-log-detail-dialog");
    expect(dialog).not.toHaveTextContent("private.internal");
    expect(dialog).not.toHaveTextContent("sk_live_secret123");
    expect(dialog).toHaveTextContent("操作执行失败。");
  });

  it("fails closed for invalid JSON instead of rendering the raw value", async () => {
    render(
      <OperationLogDetailDialog
        open
        entry={makeEntry({ detailsJson: "raw-secret-token=abc" })}
        onOpenChange={() => undefined}
      />,
    );

    const details = screen.getByText("安全结构化详情").closest("details")!;
    fireEvent.click(within(details).getByText("安全结构化详情"));
    expect(details).toHaveTextContent("存储的详情无法安全解析，因此未予显示。");
    expect(details).not.toHaveTextContent("raw-secret-token=abc");
  });

  it("links valid correlation IDs to runtime evidence but is honest for legacy IDs", async () => {
    const inspectRuntime = vi.fn();
    const { rerender } = render(
      <OperationLogDetailDialog
        open
        entry={makeEntry()}
        onOpenChange={() => undefined}
        onInspectRuntime={inspectRuntime}
      />,
    );

    fireEvent.click(await screen.findByTestId("logs-detail-view-runtime"));
    expect(inspectRuntime).toHaveBeenCalledWith(OPERATION_ID);

    rerender(
      <OperationLogDetailDialog
        open
        entry={makeEntry({ id: "legacy-log-1" })}
        onOpenChange={() => undefined}
        onInspectRuntime={inspectRuntime}
      />,
    );
    expect(screen.getAllByText("旧版日志，无关联 ID").length).toBeGreaterThan(
      0,
    );
    expect(
      screen.queryByTestId("logs-detail-view-runtime"),
    ).not.toBeInTheDocument();
    expect(screen.queryByTestId("logs-detail-copy-id")).not.toBeInTheDocument();
  });

  it("closes on Escape through the shared accessible dialog primitive", async () => {
    const onOpenChange = vi.fn();
    render(
      <OperationLogDetailDialog
        open
        entry={makeEntry()}
        onOpenChange={onOpenChange}
      />,
    );

    await screen.findByTestId("operation-log-detail-dialog");
    fireEvent.keyDown(document, { key: "Escape" });
    await waitFor(() =>
      expect(onOpenChange).toHaveBeenCalledWith(false, expect.anything()),
    );
  });
});
