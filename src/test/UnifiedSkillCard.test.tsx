import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { UnifiedSkillCard } from "@/components/skill/UnifiedSkillCard";
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

describe("UnifiedSkillCard", () => {
  beforeEach(() => {
    resetUpdateCenterStore();
  });

  it("shows a cached AI summary before the original description", () => {
    render(
      <UnifiedSkillCard
        name="planner"
        description="Original English description"
        aiSummary="这是中文 AI 总结"
      />
    );

    expect(screen.getByText("AI 摘要")).toBeInTheDocument();
    expect(screen.getByText("这是中文 AI 总结")).toBeInTheDocument();
    expect(screen.queryByText("Original English description")).not.toBeInTheDocument();
  });

  it("falls back to the description when no AI summary exists", () => {
    render(
      <UnifiedSkillCard
        name="planner"
        description="Original English description"
      />
    );

    expect(screen.queryByText("AI 摘要")).not.toBeInTheDocument();
    expect(screen.getByText("Original English description")).toBeInTheDocument();
  });

  it("ignores blank AI summaries", () => {
    render(
      <UnifiedSkillCard
        name="planner"
        description="Original English description"
        aiSummary="   "
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
    render(<UnifiedSkillCard name="review" usageBadge={12} />);

    const badge = screen.getByTestId("usage-badge");
    expect(badge).toBeInTheDocument();
    expect(badge).toHaveAttribute("title", "最近 30 天调用 12 次");
    expect(screen.getByText("12 次")).toBeInTheDocument();
    expect(screen.getByText("近 30 天")).toBeInTheDocument();
  });

  it("hides usageBadge when count is 0 or undefined", () => {
    const { rerender } = render(<UnifiedSkillCard name="x" usageBadge={0} />);
    expect(screen.queryByTestId("usage-badge")).not.toBeInTheDocument();

    rerender(<UnifiedSkillCard name="x" />);
    expect(screen.queryByTestId("usage-badge")).not.toBeInTheDocument();
  });

  it("传入 statusChipLabel 渲染文字状态 chip", () => {
    render(
      <UnifiedSkillCard name="s" statusAccent="amber" statusChipLabel="可更新" />,
    );
    expect(screen.getByText("可更新")).toBeInTheDocument();
  });

  it("不传 statusChipLabel 时无状态 chip", () => {
    render(<UnifiedSkillCard name="s" />);
    expect(screen.queryByText("可更新")).not.toBeInTheDocument();
  });

  it("editableTags：渲染彩色标签，hover × 调用 onRemove", async () => {
    const onRemove = vi.fn();
    render(
      <UnifiedSkillCard
        name="s"
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
        name="s"
        footer={{ repoName: "anthropics/skills", repoColor: "#7c3aed" }}
        usageBadge={5}
      />,
    );
    expect(screen.getByText("anthropics/skills")).toBeInTheDocument();
  });

  it("不传 footer 时不渲染 footer 区域", () => {
    const { container } = render(<UnifiedSkillCard name="s" />);
    expect(container.querySelector("[data-testid='card-footer']")).toBeNull();
  });
});
