import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { fireEvent, screen, waitFor } from "@testing-library/react";
import * as S from "./centralSkillsViewTestSupport";

const {
  mockTags,
  mockCancelAiTagJob,
  renderCentralSkillsView,
} = S;

describe("CentralSkillsView", () => {
  beforeEach(S.resetCentralSkillsViewTestState);
  afterEach(S.cleanupCentralSkillsViewTestState);

  it("shows review replacement copy and disables replacement until manual tags are selected", () => {
    renderCentralSkillsView({
      centralOverrides: {
        aiTagReviews: [
          {
            skill_id: "code-reviewer",
            skill_name: "code-reviewer",
            tag: mockTags[1],
            confidence: 0.42,
            reason: "不确定",
            suggested_at: "2026-04-24T00:00:00Z",
            updated_at: "2026-04-24T00:00:00Z",
          },
        ],
      },
    });

    fireEvent.click(screen.getByRole("tab", { name: "复核" }));
    expect(screen.getByText("要替换标签，先在手动中选择分类。")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "用已选标签替换" })).toBeDisabled();

    fireEvent.click(screen.getByRole("tab", { name: "手动" }));
    fireEvent.click(screen.getByRole("button", { name: "前端与视觉设计" }));
    fireEvent.click(screen.getByRole("tab", { name: "复核" }));
    expect(screen.getByRole("button", { name: "用已选标签替换" })).not.toBeDisabled();
  });


  it("shows AI tagging progress and review queue entry", () => {
    renderCentralSkillsView({
      centralOverrides: {
        aiTagReviews: [
          {
            skill_id: "code-reviewer",
            skill_name: "code-reviewer",
            tag: mockTags[1],
            confidence: 0.42,
            reason: "不确定",
            suggested_at: "2026-04-24T00:00:00Z",
            updated_at: "2026-04-24T00:00:00Z",
          },
        ],
        aiTagJob: {
          jobId: "job-1",
          status: "completed",
          total: 2,
          completed: 2,
          succeeded: 1,
          failed: 1,
          lowConfidenceCount: 1,
          items: {
            "frontend-design": "succeeded",
            "code-reviewer": "failed",
          },
        },
      },
    });

    expect(screen.getByText("AI 标注进度")).toBeInTheDocument();
    expect(screen.getByText("成功 1")).toBeInTheDocument();
    expect(screen.getByText("失败 1")).toBeInTheDocument();
    expect(screen.getAllByText("AI 待复核").length).toBeGreaterThanOrEqual(1);
  });


  it("allows dismissing completed Central update progress", () => {
    renderCentralSkillsView({
      centralOverrides: {
        updateJob: {
          phase: "checking",
          status: "completed",
          total: 91,
          completed: 91,
          succeeded: 87,
          failed: 0,
          skipped: 4,
          items: {},
        },
      },
    });

    expect(screen.getByText("中央技能更新进度")).toBeInTheDocument();
    expect(screen.getByText("已完成")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "关闭更新进度" }));

    expect(screen.queryByText("中央技能更新进度")).not.toBeInTheDocument();
  });


  it("shows a cancel button while AI tagging is running", async () => {
    mockCancelAiTagJob.mockResolvedValue(undefined);
    renderCentralSkillsView({
      centralOverrides: {
        aiTagJob: {
          jobId: "job-1",
          status: "running",
          total: 2,
          completed: 1,
          succeeded: 1,
          failed: 0,
          lowConfidenceCount: 0,
          currentSkillName: "code-reviewer",
          items: {
            "frontend-design": "succeeded",
            "code-reviewer": "running",
          },
        },
        isSuggestingTags: true,
      },
    });

    fireEvent.click(screen.getByRole("button", { name: /中断 AI Tag/i }));

    await waitFor(() => {
      expect(mockCancelAiTagJob).toHaveBeenCalled();
    });
  });


  it("opens the skill detail drawer without navigating away", async () => {
    renderCentralSkillsView();

    fireEvent.click(screen.getByRole("button", { name: /查看 frontend-design 的详情/i }));

    await waitFor(() => {
      expect(screen.getByTestId("skill-detail-drawer")).toBeInTheDocument();
    });
    expect(screen.getByText("drawer-skill:frontend-design")).toBeInTheDocument();
  });


  it("preserves search and scroll state when closing the drawer and restores focus", async () => {
    renderCentralSkillsView();

    const searchInput = screen.getByPlaceholderText(/搜索中央技能库/i);
    fireEvent.change(searchInput, { target: { value: "frontend" } });

    const scroller = searchInput.closest(".flex.flex-col.h-full")?.querySelector(".flex-1.overflow-auto.p-6");
    expect(scroller).not.toBeNull();
    if (!scroller) return;
    (scroller as HTMLDivElement).scrollTop = 240;

    const trigger = screen.getByRole("button", { name: /查看 frontend-design 的详情/i });
    fireEvent.click(trigger);

    await waitFor(() => {
      expect(screen.getByTestId("skill-detail-drawer")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /close drawer/i }));

    await waitFor(() => {
      expect(screen.queryByTestId("skill-detail-drawer")).not.toBeInTheDocument();
    });

    expect(searchInput).toHaveValue("frontend");
    expect((scroller as HTMLDivElement).scrollTop).toBe(240);
    expect(trigger).toHaveFocus();
  });
});
