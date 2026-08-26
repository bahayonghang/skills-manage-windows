import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";

import { SkillsCliBatchBar } from "@/components/skillsCli/SkillsCliBatchBar";
import type { SkillsCliLinkTargetSummary } from "@/pages/skillsCliBatchModel";

const summaries: SkillsCliLinkTargetSummary[] = [
  {
    agentId: "cursor",
    displayName: "Cursor",
    linkableCount: 2,
    managedCount: 1,
    directCopyCount: 1,
    blockedCount: 1,
  },
  {
    agentId: "amp",
    displayName: "Amp",
    linkableCount: 0,
    managedCount: 0,
    directCopyCount: 2,
    blockedCount: 3,
  },
];

const baseProps = {
  selectedCount: 3,
  summaries,
  unlinkEnabled: true,
  busy: false,
  runtimeBlocked: false,
  exporting: false,
  linkMenuOpen: false,
  onLinkMenuOpenChange: vi.fn(),
  onLink: vi.fn(),
  onUnlink: vi.fn(),
  onExportSelected: vi.fn(),
  onUninstall: vi.fn(),
  onClear: vi.fn(),
};

describe("SkillsCliBatchBar", () => {
  it("hides when nothing is selected and shows the count otherwise", () => {
    const { rerender } = render(
      <SkillsCliBatchBar {...baseProps} selectedCount={0} />,
    );
    expect(screen.queryByTestId("skills-cli-batch-bar")).not.toBeInTheDocument();
    rerender(<SkillsCliBatchBar {...baseProps} />);
    expect(screen.getByTestId("skills-cli-batch-bar")).toHaveTextContent(
      "已选 3 项",
    );
    expect(screen.getByTestId("skills-cli-batch-bar")).toHaveAttribute(
      "aria-busy",
      "false",
    );
  });

  it("opens a placement menu with linkable and blocked reasons and skips disabled targets", () => {
    const onLink = vi.fn();
    const onLinkMenuOpenChange = vi.fn();
    render(
      <SkillsCliBatchBar
        {...baseProps}
        onLink={onLink}
        linkMenuOpen
        onLinkMenuOpenChange={onLinkMenuOpenChange}
      />,
    );
    expect(screen.getByRole("menu")).toHaveTextContent("2 个可链接");
    expect(screen.getByRole("menu")).toHaveTextContent("1 个直接副本");
    expect(screen.getByRole("menu")).toHaveTextContent("1 个已阻止");
    fireEvent.click(screen.getByTestId("skills-cli-batch-link-cursor"));
    expect(onLink).toHaveBeenCalledWith("cursor");
    expect(screen.getByTestId("skills-cli-batch-link-amp")).toHaveAttribute(
      "data-disabled",
    );
  });

  it("disables conflicting actions while busy and keeps clear available", () => {
    const onClear = vi.fn();
    render(
      <SkillsCliBatchBar
        {...baseProps}
        busy
        onClear={onClear}
      />,
    );
    expect(screen.getByTestId("skills-cli-batch-bar")).toHaveAttribute(
      "aria-busy",
      "true",
    );
    expect(screen.getByRole("button", { name: "取消链接" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "导出所选" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "卸载" })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "清除选择" }));
    expect(onClear).toHaveBeenCalledTimes(1);
  });

  it("closes the open menu on Escape without invoking link", () => {
    const onLink = vi.fn();
    const onLinkMenuOpenChange = vi.fn();
    render(
      <SkillsCliBatchBar
        {...baseProps}
        linkMenuOpen
        onLink={onLink}
        onLinkMenuOpenChange={onLinkMenuOpenChange}
      />,
    );
    fireEvent.keyDown(screen.getByRole("menu"), { key: "Escape" });
    expect(onLinkMenuOpenChange.mock.calls[0]?.[0]).toBe(false);
    expect(onLink).not.toHaveBeenCalled();
  });
});
