import { render, screen } from "@testing-library/react";
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
    refreshContext: { repositoryIds: [], skillIds: [] },
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

  it("renders usageBadge when count > 0", () => {
    render(<UnifiedSkillCard name="review" usageBadge={12} />);
    expect(screen.getByTestId("usage-badge")).toBeInTheDocument();
    expect(screen.getByText("12×")).toBeInTheDocument();
  });

  it("hides usageBadge when count is 0 or undefined", () => {
    const { rerender } = render(<UnifiedSkillCard name="x" usageBadge={0} />);
    expect(screen.queryByTestId("usage-badge")).not.toBeInTheDocument();

    rerender(<UnifiedSkillCard name="x" />);
    expect(screen.queryByTestId("usage-badge")).not.toBeInTheDocument();
  });
});
