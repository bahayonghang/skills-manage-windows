import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";

import { SkillsCliBatchBar, ICON_HIT } from "@/components/skillsCli/SkillsCliBatchBar";
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
  exporting: false,
  linkMenuOpen: false,
  onLinkMenuOpenChange: vi.fn(),
  unlinkMenuOpen: false,
  onUnlinkMenuOpenChange: vi.fn(),
  onLink: vi.fn(),
  onUnlink: vi.fn(),
  onUnlinkPlatform: vi.fn(),
  onUpdate: vi.fn(),
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
    expect(screen.getByTestId("skills-cli-batch-unlink")).toBeDisabled();
    expect(screen.getByTestId("skills-cli-batch-update")).toBeDisabled();
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

  it("exposes Update and a per-platform unlink menu without oversized min-width", () => {
    const onUpdate = vi.fn();
    const onUnlink = vi.fn();
    const onUnlinkPlatform = vi.fn();
    render(
      <SkillsCliBatchBar
        {...baseProps}
        unlinkMenuOpen
        onUpdate={onUpdate}
        onUnlink={onUnlink}
        onUnlinkPlatform={onUnlinkPlatform}
      />,
    );
    const bar = screen.getByTestId("skills-cli-batch-bar");
    expect(bar.className).toContain("flex-wrap");
    const update = screen.getByTestId("skills-cli-batch-update");
    expect(update.className).not.toMatch(/min-w-/);
    fireEvent.click(update);
    expect(onUpdate).toHaveBeenCalledTimes(1);
    fireEvent.click(screen.getByTestId("skills-cli-batch-unlink-cursor"));
    expect(onUnlinkPlatform).toHaveBeenCalledWith("cursor");
    fireEvent.click(screen.getByTestId("skills-cli-batch-unlink-all"));
    expect(onUnlink).toHaveBeenCalledTimes(1);
    expect(screen.getByTestId("skills-cli-batch-unlink-amp")).toHaveAttribute(
      "data-disabled",
    );
  });

  it("reuses ICON_HIT for the clear control with a focus-visible ring", () => {
    render(<SkillsCliBatchBar {...baseProps} />);
    expect(ICON_HIT).toContain("size-8");
    expect(ICON_HIT).toContain("after:size-10");
    const clear = screen.getByRole("button", { name: "清除选择" });
    expect(clear.className).toContain("after:size-10");
    expect(clear.className).toContain("focus-visible:ring-2");
    expect(clear).not.toHaveAttribute("tabIndex", "-1");
  });
});
