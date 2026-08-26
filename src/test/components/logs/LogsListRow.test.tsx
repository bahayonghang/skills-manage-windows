import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { LogsListRow } from "@/components/logs/LogsListRow";
import type { OperationLogEntry } from "@/types";

function entry(overrides: Partial<OperationLogEntry> = {}): OperationLogEntry {
  return {
    id: "65e8ecdf-c398-4ed7-a4a1-87f193f0dd25",
    createdAt: "2026-08-27T00:00:00Z",
    level: "error",
    targetKind: "ssh",
    targetId: "ssh-private.internal",
    targetLabel: "private.internal",
    category: "import",
    action: "import_github_repo_skills",
    status: "failed",
    summary: "sk_live_secret123",
    errorSummary: "private.internal",
    subjectLabel: "C:/Users/private",
    detailsJson: JSON.stringify({
      errorCode: "github_import.access_denied",
      retryable: false,
    }),
    ...overrides,
  };
}

describe("LogsListRow", () => {
  it("renders reviewed localized diagnostics without raw stored prose or identities", () => {
    render(<LogsListRow entry={entry()} onOpen={() => undefined} />);

    expect(screen.getByText("SSH")).toBeInTheDocument();
    expect(screen.getByText("操作执行失败。")).toBeInTheDocument();
    expect(
      screen.getByText(
        "GitHub 拒绝匿名访问该仓库。如果仓库需要认证，请配置 GitHub 令牌。",
      ),
    ).toBeInTheDocument();
    expect(document.body).not.toHaveTextContent("private.internal");
    expect(document.body).not.toHaveTextContent("sk_live_secret123");
    expect(document.body).not.toHaveTextContent("Users/private");
  });

  it.each([
    ["started", "进行中", "lucide-clock-3"],
    ["interrupted", "已中断", "lucide-circle-pause"],
  ])("shows %s with text and a distinct icon", (status, label, iconClass) => {
    const onOpen = vi.fn();
    const { container } = render(
      <LogsListRow entry={entry({ status })} onOpen={onOpen} />,
    );

    expect(screen.getByText(label)).toBeInTheDocument();
    expect(container.querySelector(`.${iconClass}`)).not.toBeNull();
    fireEvent.click(container.querySelector("button")!);
    expect(onOpen).toHaveBeenCalled();
  });
});
