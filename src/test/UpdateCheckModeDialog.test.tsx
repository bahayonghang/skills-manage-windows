import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, within } from "@testing-library/react";
import type { ComponentProps } from "react";

import { UpdateCheckModeDialog } from "@/components/central/UpdateCheckModeDialog";
import type { UpdateCheckMode } from "@/pages/centralUpdateCheckMode";

function renderDialog(overrides: Partial<ComponentProps<typeof UpdateCheckModeDialog>> = {}) {
  const onOpenChange = vi.fn();
  const onConfirm = vi.fn();
  render(
    <UpdateCheckModeDialog
      open
      onOpenChange={onOpenChange}
      regularScopeLabel="检查所选（2）"
      syncScopeLabel="检查全部仓库（1 个）"
      onConfirm={onConfirm}
      {...overrides}
    />,
  );
  return { onOpenChange, onConfirm };
}

describe("UpdateCheckModeDialog", () => {
  it("renders mode copy and defaults to regular mode", () => {
    renderDialog();

    const dialog = screen.getByRole("dialog");
    expect(within(dialog).getByText("选择更新检查模式")).toBeInTheDocument();
    expect(within(dialog).getByText(/当前范围：检查所选（2）/)).toBeInTheDocument();
    expect(within(dialog).getByTestId("update-check-mode-regular")).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(within(dialog).getByText("常规检查")).toBeInTheDocument();
    expect(within(dialog).getByText("增量和删减模式")).toBeInTheDocument();
  });

  it("confirms the selected sync mode", () => {
    const { onConfirm } = renderDialog();
    const dialog = screen.getByRole("dialog");

    fireEvent.click(within(dialog).getByTestId("update-check-mode-sync"));
    expect(within(dialog).getByText(/当前范围：检查全部仓库（1 个）/)).toBeInTheDocument();
    fireEvent.click(within(dialog).getByTestId("confirm-update-check-mode"));

    expect(onConfirm).toHaveBeenCalledWith("sync" satisfies UpdateCheckMode);
  });

  it("defaults to the saved sync preference while keeping both choices visible", () => {
    const { onConfirm } = renderDialog({ mode: "sync" });
    const dialog = screen.getByRole("dialog");

    expect(within(dialog).getByText("选择更新检查模式")).toBeInTheDocument();
    expect(within(dialog).getByText(/当前范围：检查全部仓库（1 个）/)).toBeInTheDocument();
    expect(within(dialog).getByTestId("update-check-mode-regular")).toBeInTheDocument();
    expect(within(dialog).getByTestId("update-check-mode-sync")).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    fireEvent.click(within(dialog).getByTestId("confirm-update-check-mode"));

    expect(onConfirm).toHaveBeenCalledWith("sync");
  });

  it("cancels without confirming", () => {
    const { onOpenChange, onConfirm } = renderDialog();

    fireEvent.click(screen.getByRole("button", { name: "取消" }));

    expect(onOpenChange).toHaveBeenCalledWith(false);
    expect(onConfirm).not.toHaveBeenCalled();
  });

  it("replaces mode choices with determinate repository progress", () => {
    renderDialog({
      isSubmitting: true,
      progress: {
        operationId: "refresh-1",
        phase: "checking",
        total: 5,
        completed: 1,
        activeRepositories: [
          { key: "openai/skills/main", name: "openai/skills" },
          { key: "anthropics/skills/main", name: "anthropics/skills" },
          { key: "vercel-labs/agent-skills/main", name: "vercel-labs/agent-skills" },
          {
            key: "example/a-very-long-repository-name/main",
            name: "example/a-very-long-repository-name",
          },
        ],
      },
    });
    const dialog = screen.getByRole("dialog");

    expect(within(dialog).queryByTestId("update-check-mode-sync")).not.toBeInTheDocument();
    expect(within(dialog).queryByTestId("confirm-update-check-mode")).not.toBeInTheDocument();
    const progressbar = within(dialog).getByRole("progressbar", {
      name: "更新检查进度：已完成 1 / 5 个仓库",
    });
    expect(progressbar).toHaveAttribute("aria-valuemin", "0");
    expect(progressbar).toHaveAttribute("aria-valuemax", "5");
    expect(progressbar).toHaveAttribute("aria-valuenow", "1");
    expect(within(dialog).getByText("已检查 1 / 5 个仓库")).toBeInTheDocument();
    for (const name of [
      "openai/skills",
      "anthropics/skills",
      "vercel-labs/agent-skills",
      "example/a-very-long-repository-name",
    ]) {
      expect(within(dialog).getByTitle(name)).toBeInTheDocument();
    }
  });

  it("shows an indeterminate preparing state before the repository total arrives", () => {
    renderDialog({ isSubmitting: true, progress: null });
    const progressbar = within(screen.getByRole("dialog")).getByRole("progressbar", {
      name: "正在准备更新检查",
    });

    expect(progressbar).not.toHaveAttribute("aria-valuenow");
    expect(progressbar).not.toHaveAttribute("aria-valuemax");
    expect(screen.getByText("正在准备仓库检查…")).toBeInTheDocument();
  });

  it("shows finalizing after all active repositories settle", () => {
    renderDialog({
      isSubmitting: true,
      progress: {
        operationId: "refresh-1",
        phase: "finalizing",
        total: 2,
        completed: 2,
        activeRepositories: [],
      },
    });

    expect(screen.getByText("正在整理检查结果…")).toBeInTheDocument();
    expect(screen.queryByText("正在检查的仓库")).not.toBeInTheDocument();
  });

  it("falls back to regular mode when saved sync preference is unavailable", () => {
    const { onConfirm } = renderDialog({
      mode: "sync",
      syncDisabled: true,
      syncDisabledReason: "当前没有可同步的 GitHub 仓库。",
    });
    const dialog = screen.getByRole("dialog");

    expect(within(dialog).getByTestId("update-check-mode-sync")).toBeDisabled();
    expect(within(dialog).getByTestId("update-check-mode-regular")).toHaveAttribute(
      "aria-pressed",
      "true",
    );

    fireEvent.click(within(dialog).getByTestId("confirm-update-check-mode"));

    expect(onConfirm).toHaveBeenCalledWith("regular");
  });

  it("shows an inline check failure", () => {
    renderDialog({ error: "检查更新失败: network unavailable" });
    const dialog = screen.getByRole("dialog");

    expect(within(dialog).getByRole("alert")).toHaveTextContent(
      "检查更新失败: network unavailable",
    );
  });
});
