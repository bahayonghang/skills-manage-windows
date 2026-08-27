import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";

import { SkillsCliToolbar } from "@/components/skillsCli/SkillsCliToolbar";
import type { SkillsCliInstallTarget } from "@/types";

const targets: SkillsCliInstallTarget[] = [
  {
    id: "cursor",
    displayName: "Cursor",
    iconName: null,
    cliAgent: "cursor",
    isEnabled: true,
    defaultSelected: true,
  },
  {
    id: "amp",
    displayName: "Amp",
    iconName: null,
    cliAgent: "amp",
    isEnabled: true,
    defaultSelected: true,
  },
];

const baseProps = {
  query: "",
  onQueryChange: vi.fn(),
  groupBy: "repo" as const,
  onGroupByChange: vi.fn(),
  platformFilter: null as string | null,
  onPlatformFilterChange: vi.fn(),
  unlinkedOnly: false,
  onUnlinkedOnlyChange: vi.fn(),
  selectMode: false,
  onSelectModeChange: vi.fn(),
  targets,
};

describe("SkillsCliToolbar", () => {
  it("exposes search, group, chip, unlinked, and select controls", () => {
    render(<SkillsCliToolbar {...baseProps} query="demo" />);
    expect(screen.getByLabelText("搜索技能")).toHaveValue("demo");
    expect(screen.getByRole("button", { name: "清除搜索" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "仓库" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(screen.getByRole("button", { name: "筛选 Cursor" })).toHaveAttribute(
      "aria-pressed",
      "false",
    );
    expect(screen.getByRole("button", { name: "仅未链接" })).toHaveAttribute(
      "aria-pressed",
      "false",
    );
    expect(screen.getByRole("button", { name: "选择" })).toHaveAttribute(
      "aria-pressed",
      "false",
    );
  });

  it("toggles a platform chip off on a second click", () => {
    const onPlatformFilterChange = vi.fn();
    const { rerender } = render(
      <SkillsCliToolbar
        {...baseProps}
        platformFilter={null}
        onPlatformFilterChange={onPlatformFilterChange}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "筛选 Cursor" }));
    expect(onPlatformFilterChange).toHaveBeenCalledWith("cursor");
    rerender(
      <SkillsCliToolbar
        {...baseProps}
        platformFilter="cursor"
        onPlatformFilterChange={onPlatformFilterChange}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "筛选 Cursor" }));
    expect(onPlatformFilterChange).toHaveBeenLastCalledWith(null);
  });

  it("disables Export all when unwired or busy and calls once with no payload", () => {
    const { rerender } = render(<SkillsCliToolbar {...baseProps} />);
    expect(screen.getByRole("button", { name: "导出全部" })).toBeDisabled();

    const onExportAll = vi.fn();
    rerender(
      <SkillsCliToolbar {...baseProps} onExportAll={onExportAll} isExporting />,
    );
    expect(screen.getByRole("button", { name: "导出全部" })).toBeDisabled();

    rerender(<SkillsCliToolbar {...baseProps} onExportAll={onExportAll} />);
    fireEvent.click(screen.getByRole("button", { name: "导出全部" }));
    expect(onExportAll).toHaveBeenCalledTimes(1);
    expect(onExportAll).toHaveBeenCalledWith();

    expect(screen.getByTestId("skills-cli-cleanup")).toBeDisabled();
    const onCleanupUnavailable = vi.fn();
    rerender(
      <SkillsCliToolbar
        {...baseProps}
        onCleanupUnavailable={onCleanupUnavailable}
        cleanupUnavailableCount={0}
      />,
    );
    expect(screen.getByTestId("skills-cli-cleanup")).toBeDisabled();
    fireEvent.click(screen.getByTestId("skills-cli-cleanup"));
    expect(onCleanupUnavailable).not.toHaveBeenCalled();

    rerender(
      <SkillsCliToolbar
        {...baseProps}
        onCleanupUnavailable={onCleanupUnavailable}
        cleanupUnavailableCount={2}
      />,
    );
    expect(screen.getByTestId("skills-cli-cleanup")).toBeEnabled();
    fireEvent.click(screen.getByTestId("skills-cli-cleanup"));
    expect(onCleanupUnavailable).toHaveBeenCalledTimes(1);
  });
});
