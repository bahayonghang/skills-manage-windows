import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";

import { SkillsCliGroupHeader } from "@/components/skillsCli/SkillsCliGroupHeader";
import type { SkillsCliBucket } from "@/pages/skillsCliViewModel";

const bucket: SkillsCliBucket = {
  id: "repo:owner/repo",
  labelKey: "skillsCli.buckets.named",
  labelValue: "owner/repo",
  skillCount: 2,
  managedLinkCount: 3,
  skills: [],
};

describe("SkillsCliGroupHeader", () => {
  it("is sticky, exposes expanded/controls, and remembers the stable bucket id", () => {
    const onToggle = vi.fn();
    render(
      <SkillsCliGroupHeader
        bucket={bucket}
        label="owner/repo"
        expanded
        panelId="skills-cli-group-panel-repo:owner/repo"
        onToggle={onToggle}
        updateCount={1}
      />,
    );
    const toggle = screen.getByRole("button", { name: /owner\/repo/ });
    expect(toggle).toHaveAttribute("aria-expanded", "true");
    expect(toggle).toHaveAttribute(
      "aria-controls",
      "skills-cli-group-panel-repo:owner/repo",
    );
    expect(
      screen.getByTestId("skills-cli-group-header-repo:owner/repo"),
    ).toHaveClass("sticky");
    expect(
      screen.getByTestId("skills-cli-update-badge-repo:owner/repo"),
    ).toHaveTextContent("1");
    fireEvent.click(toggle);
    expect(onToggle).toHaveBeenCalledTimes(1);
  });

  it("omits Select all and Update all when callbacks are not wired", () => {
    render(
      <SkillsCliGroupHeader
        bucket={bucket}
        label="owner/repo"
        expanded
        panelId="panel"
        onToggle={vi.fn()}
      />,
    );
    expect(screen.queryByRole("button", { name: "全选" })).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: "全部更新" }),
    ).not.toBeInTheDocument();
  });

  it("invokes wired Select all and Update all once", () => {
    const onSelectAll = vi.fn();
    const onUpdateAll = vi.fn();
    render(
      <SkillsCliGroupHeader
        bucket={bucket}
        label="owner/repo"
        expanded={false}
        panelId="panel"
        onToggle={vi.fn()}
        onSelectAll={onSelectAll}
        onUpdateAll={onUpdateAll}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "全选" }));
    fireEvent.click(screen.getByRole("button", { name: "全部更新" }));
    expect(onSelectAll).toHaveBeenCalledTimes(1);
    expect(onUpdateAll).toHaveBeenCalledTimes(1);
    const header = screen.getByTestId("skills-cli-group-header-repo:owner/repo");
    const title = screen.getByTestId("skills-cli-group-title-repo:owner/repo");
    const count = screen.getByTestId("skills-cli-group-count-repo:owner/repo");
    const actions = screen.getByTestId("skills-cli-group-actions-repo:owner/repo");
    expect(title.className).toContain("text-sm");
    expect(count.className).toContain("text-ui-meta");
    expect(
      title.compareDocumentPosition(count) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
    expect(
      count.compareDocumentPosition(actions) & Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
    expect(header.className).not.toMatch(/text-\[[0-9]/);
    expect(title.className).not.toMatch(/text-\[[0-9]/);
    expect(count.className).not.toMatch(/text-\[[0-9]/);
  });
});
