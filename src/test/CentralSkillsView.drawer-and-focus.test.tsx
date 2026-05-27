import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { fireEvent, screen, waitFor } from "@testing-library/react";
import * as S from "./centralSkillsViewTestSupport";

const {
  mockTags,
  mockCancelAiTagJob,
  renderCentralSkillsView,
} = S;

describe("CentralSkillsView drawer + focus（V2 markup）", () => {
  beforeEach(() => {
    S.resetCentralSkillsViewTestState();
    window.localStorage.clear();
  });
  afterEach(S.cleanupCentralSkillsViewTestState);

  it("点击技能名打开 detail drawer 而不路由跳转", async () => {
    renderCentralSkillsView();
    fireEvent.click(
      screen.getByRole("button", { name: /查看 frontend-design 的详情/i })
    );

    await waitFor(() => {
      expect(screen.getByTestId("skill-detail-drawer")).toBeInTheDocument();
    });
    expect(screen.getByText("drawer-skill:frontend-design")).toBeInTheDocument();
  });

  it("关闭 detail drawer 后焦点回到触发按钮", async () => {
    renderCentralSkillsView();
    const trigger = screen.getByRole("button", { name: /查看 frontend-design 的详情/i });
    fireEvent.click(trigger);
    await waitFor(() => {
      expect(screen.getByTestId("skill-detail-drawer")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /close drawer/i }));
    await waitFor(() => {
      expect(screen.queryByTestId("skill-detail-drawer")).not.toBeInTheDocument();
    });
    expect(trigger).toHaveFocus();
  });

  it("任务中心抽屉内展示 AI 进度行 + cancel 按钮（running 时）", async () => {
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
          items: { "frontend-design": "succeeded", "code-reviewer": "running" },
        },
        isSuggestingTags: true,
      },
    });

    // 经由顶部「⋯ 更多」→ 任务中心 进入抽屉
    fireEvent.click(screen.getByTestId("central-toolbar-more"));
    fireEvent.click(await screen.findByTestId("central-toolbar-task-center"));
    expect(
      await screen.findByTestId("central-task-row-ai-tag")
    ).toBeInTheDocument();
    fireEvent.click(screen.getByTestId("task-row-ai-tag-cancel"));
    await waitFor(() => expect(mockCancelAiTagJob).toHaveBeenCalled());
  });

  it("任务中心抽屉内 AI 完成时展示「查看复核」入口，并能打开 Categorize 抽屉", async () => {
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

    fireEvent.click(screen.getByTestId("central-toolbar-more"));
    fireEvent.click(await screen.findByTestId("central-toolbar-task-center"));
    const viewReviews = await screen.findByTestId("task-row-ai-tag-view-reviews");
    fireEvent.click(viewReviews);

    expect(
      await screen.findByTestId("central-categorize-drawer")
    ).toBeInTheDocument();
  });

  it("Categorize 抽屉内复核 tab 在未选手动标签时禁用「替换」按钮", async () => {
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
    const checkboxes = screen.getAllByLabelText("选择技能");
    fireEvent.click(checkboxes[0]);
    const openBtn = await screen.findByTestId("bulk-bar-open-categorize");
    fireEvent.click(openBtn);
    const drawer = await screen.findByTestId("central-categorize-drawer");
    expect(drawer).toBeInTheDocument();
    const reviewTab = await screen.findByRole("tab", { name: /复核/ });
    fireEvent.click(reviewTab);

    await waitFor(() => {
      expect(
        screen.getByText("要替换标签，先在手动中选择分类。")
      ).toBeInTheDocument();
    });
    expect(
      screen.getByRole("button", { name: "用已选标签替换" })
    ).toBeDisabled();
  });
});
