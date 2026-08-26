import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  UnifiedSkillCard,
  type CentralSkillCardProps,
  type PlatformSkillCardProps,
  type SkillsCliSkillCardProps,
} from "@/components/skill/UnifiedSkillCard";
import { useUpdateCenterStore } from "@/stores/updateCenterStore";

function resetUpdateCenterStore() {
  useUpdateCenterStore.setState({
    inventory: null,
    isRefreshing: false,
    isApplying: false,
    lastRefreshedAt: null,
    isDialogOpen: false,
    activeTab: "updatable",
    refreshContext: { repositoryIds: [], skillIds: [], agentIds: [] },
    error: null,
  });
}

const noop = () => {};

/** central 场景的必填底座；用例按需覆盖字段。 */
const centralBaseProps = {
  variant: "central",
  name: "s",
  checkbox: { checked: false, onChange: noop },
  onDetail: noop,
  onInstallTo: noop,
  onUninstallFromPlatforms: noop,
  onUpdateCentral: noop,
  onDeleteFromCentral: noop,
} satisfies CentralSkillCardProps;

const platformBaseProps = {
  variant: "platform",
  name: "review",
  sourceType: "symlink",
  originKind: null,
  isReadOnly: false,
  onDetail: noop,
  uninstallFromLabel: "卸载 review",
} satisfies PlatformSkillCardProps;

describe("UnifiedSkillCard", () => {
  beforeEach(() => {
    resetUpdateCenterStore();
  });

  it("shows a cached AI summary before the original description", () => {
    render(
      <UnifiedSkillCard
        variant="marketplace"
        name="planner"
        description="Original English description"
        aiSummary="这是中文 AI 总结"
        onDetail={noop}
      />
    );

    expect(screen.getByText("AI 摘要")).toBeInTheDocument();
    expect(screen.getByText("这是中文 AI 总结")).toBeInTheDocument();
    expect(screen.queryByText("Original English description")).not.toBeInTheDocument();
  });

  it("falls back to the description when no AI summary exists", () => {
    render(
      <UnifiedSkillCard
        variant="marketplace"
        name="planner"
        description="Original English description"
        onDetail={noop}
      />
    );

    expect(screen.queryByText("AI 摘要")).not.toBeInTheDocument();
    expect(screen.getByText("Original English description")).toBeInTheDocument();
  });

  it("ignores blank AI summaries", () => {
    render(
      <UnifiedSkillCard
        variant="marketplace"
        name="planner"
        description="Original English description"
        aiSummary="   "
        onDetail={noop}
      />
    );

    expect(screen.queryByText("AI 摘要")).not.toBeInTheDocument();
    expect(screen.getByText("Original English description")).toBeInTheDocument();
  });

  it("does not enable card update action from inventory hasUpdate alone", () => {
    useUpdateCenterStore.setState({
      inventory: {
        updatable: [
          {
            repositoryId: "github:owner-repo-main",
            state: {
              skill_id: "planner",
              source_type: "github",
              status: "update_available",
            },
          },
        ],
        remoteAdded: [],
        remoteMissing: [],
        platformDuplicates: [],
        deletedPlatformCopies: [],
        orphans: [],
        failedRepositories: [],
        generatedAt: "2026-05-23T00:00:00.000Z",
      },
    });

    render(
      <UnifiedSkillCard
        {...centralBaseProps}
        name="planner"
        skillId="planner"
        onUpdateCentral={vi.fn()}
      />
    );

    expect(
      screen.getByRole("button", { name: "从来源更新 planner" }),
    ).toBeDisabled();
  });

  it("enables card update action only for explicit update_available status", () => {
    render(
      <UnifiedSkillCard
        {...centralBaseProps}
        name="planner"
        skillId="planner"
        onUpdateCentral={vi.fn()}
        updateStatus={{
          skill_id: "planner",
          source_type: "github",
          status: "update_available",
        }}
      />
    );

    expect(
      screen.getByRole("button", { name: "从来源更新 planner" }),
    ).toBeEnabled();
  });

  it("renders localized usageBadge text and tooltip when count > 0", () => {
    render(<UnifiedSkillCard {...centralBaseProps} name="review" usageBadge={12} />);

    const badge = screen.getByTestId("usage-badge");
    expect(badge).toBeInTheDocument();
    expect(badge).toHaveAttribute("title", "最近 30 天调用 12 次");
    expect(screen.getByText("12 次")).toBeInTheDocument();
    expect(screen.getByText("近 30 天")).toBeInTheDocument();
  });

  it("hides usageBadge when count is 0 or undefined", () => {
    const { rerender } = render(
      <UnifiedSkillCard {...centralBaseProps} name="x" usageBadge={0} />,
    );
    expect(screen.queryByTestId("usage-badge")).not.toBeInTheDocument();

    rerender(<UnifiedSkillCard {...centralBaseProps} name="x" />);
    expect(screen.queryByTestId("usage-badge")).not.toBeInTheDocument();
  });

  it("传入 statusChipLabel 渲染文字状态 chip", () => {
    render(
      <UnifiedSkillCard
        {...centralBaseProps}
        statusAccent="warning"
        statusChipLabel="可更新"
      />,
    );
    expect(screen.getByText("可更新")).toBeInTheDocument();
  });

  it("不传 statusChipLabel 时无状态 chip", () => {
    render(<UnifiedSkillCard {...centralBaseProps} />);
    expect(screen.queryByText("可更新")).not.toBeInTheDocument();
  });

  it("editableTags：渲染彩色标签，hover × 调用 onRemove", async () => {
    const onRemove = vi.fn();
    render(
      <UnifiedSkillCard
        {...centralBaseProps}
        editableTags={{
          tags: [{ id: "t1", name: "frontend" }],
          allTags: [
            { id: "t1", name: "frontend" },
            { id: "t2", name: "backend" },
          ],
          onAdd: vi.fn(),
          onCreate: vi.fn(),
          onRemove,
        }}
      />,
    );
    await userEvent.click(
      screen.getByLabelText(/移除标签 frontend|remove tag frontend/i),
    );
    expect(onRemove).toHaveBeenCalledWith("t1");
  });

  it("footer：显示 repo 名", () => {
    render(
      <UnifiedSkillCard
        {...centralBaseProps}
        footer={{ repoName: "anthropics/skills", repoColor: "#7c3aed" }}
        usageBadge={5}
      />,
    );
    expect(screen.getByText("anthropics/skills")).toBeInTheDocument();
  });

  it("不传 footer 时不渲染 footer 区域", () => {
    const { container } = render(<UnifiedSkillCard {...centralBaseProps} />);
    expect(container.querySelector("[data-testid='card-footer']")).toBeNull();
  });

  it("hides usage rank until lifetimeUsage is ready", () => {
    render(<UnifiedSkillCard {...platformBaseProps} />);
    expect(screen.queryByTestId("usage-rank")).not.toBeInTheDocument();
  });

  it("renders no-record usage rank when rank is null or below 1", () => {
    const { rerender } = render(
      <UnifiedSkillCard
        {...platformBaseProps}
        lifetimeUsage={{ rank: null, count: 0 }}
      />,
    );
    const rank = screen.getByTestId("usage-rank");
    expect(rank).toHaveClass("absolute", "bottom-3.5", "right-3.5");
    expect(rank).toHaveTextContent("无记录");
    expect(rank).toHaveAttribute("aria-label", "全部已记录历史中无调用记录");

    rerender(
      <UnifiedSkillCard
        {...platformBaseProps}
        lifetimeUsage={{ rank: 0, count: 0 }}
      />,
    );
    expect(screen.getByTestId("usage-rank")).toHaveTextContent("无记录");
    expect(screen.queryByText("#0")).not.toBeInTheDocument();
  });

  it("renders #N · count usage rank in the card corner", () => {
    render(
      <UnifiedSkillCard
        {...platformBaseProps}
        lifetimeUsage={{ rank: 2, count: 14 }}
      />,
    );
    const rank = screen.getByTestId("usage-rank");
    expect(rank).toHaveClass("absolute", "bottom-3.5", "right-3.5");
    expect(rank).toHaveTextContent("#2");
    expect(rank).toHaveTextContent("14");
    expect(rank).toHaveAttribute(
      "aria-label",
      "当前列表第 2 名，全部已记录历史 14 次",
    );
    expect(rank).toHaveAttribute(
      "title",
      "当前列表第 2 · 全部已记录历史 14 次",
    );
  });

  it("merges plugin origin into one warning chip and skips read-only plus standalone", () => {
    render(
      <UnifiedSkillCard
        {...platformBaseProps}
        sourceType="copy"
        originKind="plugin"
        isReadOnly
      />,
    );
    expect(screen.getByText("插件来源")).toBeInTheDocument();
    expect(screen.queryByText("只读")).not.toBeInTheDocument();
    expect(screen.queryByText("独立安装")).not.toBeInTheDocument();
  });

  it("renders a short Central Skills chip for symlink rows", () => {
    const { container } = render(<UnifiedSkillCard {...platformBaseProps} />);
    expect(screen.getByText("中央技能库")).toBeInTheDocument();
    expect(screen.queryByText("符号链接")).not.toBeInTheDocument();
    expect(container.querySelector(".bg-info")).not.toBeNull();
    expect(container.querySelector(".bg-warning")).toBeNull();
  });

  it("paints a warning origin bar for plugin rows", () => {
    const { container } = render(
      <UnifiedSkillCard
        {...platformBaseProps}
        sourceType="copy"
        originKind="plugin"
        isReadOnly
      />,
    );
    expect(container.querySelector(".bg-warning")).not.toBeNull();
    expect(container.querySelector(".bg-info")).toBeNull();
  });
});

const skillsCliBaseProps = {
  variant: "skillsCli",
  layout: "denseRow",
  name: "dense-skill",
  path: "/canonical/dense-skill",
  placements: [
    {
      agentId: "cursor",
      displayName: "Cursor",
      targetPath: "/cursor/dense-skill",
      state: "managed_link",
      managedLinkKind: "windows_junction",
      reasonCode: null,
    },
    {
      agentId: "codex",
      displayName: "Codex",
      targetPath: "/codex/dense-skill",
      state: "managed_link",
      managedLinkKind: "windows_junction",
      reasonCode: null,
    },
    {
      agentId: "amp",
      displayName: "Amp",
      targetPath: "/amp/dense-skill",
      state: "managed_link",
      managedLinkKind: "windows_junction",
      reasonCode: null,
    },
    {
      agentId: "claude-code",
      displayName: "Claude Code",
      targetPath: "/claude/dense-skill",
      state: "managed_link",
      managedLinkKind: "windows_junction",
      reasonCode: null,
    },
    {
      agentId: "opencode",
      displayName: "OpenCode",
      targetPath: "/opencode/dense-skill",
      state: "managed_link",
      managedLinkKind: "windows_junction",
      reasonCode: null,
    },
  ],
  onDetail: noop,
  onUninstall: noop,
} satisfies SkillsCliSkillCardProps;

describe("UnifiedSkillCard skillsCli dense-row", () => {
  it("uses the 76px dense-row target and rejects the 168px compact branch", () => {
    const { container } = render(<UnifiedSkillCard {...skillsCliBaseProps} />);
    const card = screen.getByTestId("skills-cli-dense-card-dense-skill");
    expect(card.className).toContain("min-h-[76px]");
    expect(card.className).toContain("h-auto");
    expect(card.className).not.toContain("min-h-[168px]");
    expect(container.querySelector(".min-h-\\[168px\\]")).toBeNull();
  });

  it("shows at most four placement icons and a +n overflow", () => {
    render(<UnifiedSkillCard {...skillsCliBaseProps} />);
    expect(screen.getByText("+1")).toBeInTheDocument();
    expect(screen.getByText("dense-skill")).toBeInTheDocument();
    expect(screen.getByText("/canonical/dense-skill")).toBeInTheDocument();
  });

  it("shows a localized status pill when there is no managed link", () => {
    render(
      <UnifiedSkillCard
        {...skillsCliBaseProps}
        placements={[
          {
            agentId: "cursor",
            displayName: "Cursor",
            targetPath: "/cursor/dense-skill",
            state: "direct_copy",
            managedLinkKind: null,
            reasonCode: null,
          },
        ]}
      />,
    );
    expect(screen.getByText("直接副本")).toBeInTheDocument();
    expect(screen.queryByText("+1")).not.toBeInTheDocument();
  });

  it("keeps hover actions visible under focus-within and stops inner clicks", async () => {
    const onDetail = vi.fn();
    const onUninstall = vi.fn();
    const onManageLinks = vi.fn();
    render(
      <UnifiedSkillCard
        {...skillsCliBaseProps}
        onDetail={onDetail}
        onUninstall={onUninstall}
        onManageLinks={onManageLinks}
      />,
    );
    const actions = screen.getByRole("button", { name: "卸载 dense-skill" })
      .parentElement;
    expect(actions?.className).toContain("group-hover/skill-card:opacity-100");
    expect(actions?.className).toContain(
      "group-focus-within/skill-card:opacity-100",
    );
    await userEvent.click(screen.getByRole("button", { name: "卸载 dense-skill" }));
    expect(onUninstall).toHaveBeenCalledTimes(1);
    expect(onDetail).not.toHaveBeenCalled();
    await userEvent.click(
      screen.getByRole("button", { name: "管理 dense-skill 的链接" }),
    );
    expect(onManageLinks).toHaveBeenCalledTimes(1);
    expect(onDetail).not.toHaveBeenCalled();
    await userEvent.click(
      screen.getByRole("button", { name: "查看 dense-skill 的详情" }),
    );
    expect(onDetail).toHaveBeenCalledTimes(1);
  });

  it("renders a keyboard-accessible checkbox in select mode", async () => {
    const onChange = vi.fn();
    render(
      <UnifiedSkillCard
        {...skillsCliBaseProps}
        checkbox={{ checked: false, onChange }}
      />,
    );
    const checkbox = screen.getByLabelText("选择技能");
    expect(checkbox.className).toContain("after:size-10");
    await userEvent.click(checkbox);
    expect(onChange).toHaveBeenCalled();
  });
});
