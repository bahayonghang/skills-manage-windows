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

  it("disables sync and submit while submitting", () => {
    const { onConfirm } = renderDialog({
      isSubmitting: true,
      syncDisabled: true,
      syncDisabledReason: "当前没有可同步的 GitHub 仓库。",
    });
    const dialog = screen.getByRole("dialog");

    expect(within(dialog).getByTestId("update-check-mode-sync")).toBeDisabled();
    expect(within(dialog).getByTestId("confirm-update-check-mode")).toBeDisabled();
    expect(within(dialog).getByText("当前没有可同步的 GitHub 仓库。")).toBeInTheDocument();
    expect(onConfirm).not.toHaveBeenCalled();
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
});
