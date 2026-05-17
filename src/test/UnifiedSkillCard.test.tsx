import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { UnifiedSkillCard } from "@/components/skill/UnifiedSkillCard";

describe("UnifiedSkillCard", () => {
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
});
