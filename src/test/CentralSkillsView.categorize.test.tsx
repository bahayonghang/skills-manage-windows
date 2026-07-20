import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import * as S from "./centralSkillsViewTestSupport";

const {
  renderCentralSkillsView,
} = S;

describe("CentralSkillsView categorize（V2 markup）", () => {
  beforeEach(() => {
    S.resetCentralSkillsViewTestState();
    window.localStorage.clear();
  });
  afterEach(S.cleanupCentralSkillsViewTestState);

  it("技能名作为按钮触发详情抽屉", () => {
    renderCentralSkillsView();
    const detailBtns = screen.getAllByRole("button", {
      name: /查看 frontend-design 的详情/i,
    });
    expect(detailBtns.length).toBeGreaterThanOrEqual(1);
  });

  it("无技能时展示初次访问空状态", () => {
    renderCentralSkillsView({
      centralOverrides: { skills: [] },
    });

    expect(screen.getByText(/欢迎使用 SkillPort/)).toBeInTheDocument();
    expect(
      screen.getAllByText(/skillsmanage\/skills/).length
    ).toBeGreaterThanOrEqual(1);
  });

  it("正在加载时展示 loading 状态", () => {
    renderCentralSkillsView({
      centralOverrides: { isLoading: true, skills: [] },
    });
    expect(screen.getByText("正在加载技能...")).toBeInTheDocument();
  });

  it("按描述关键字搜索可过滤列表", async () => {
    renderCentralSkillsView();
    const searchInput = screen.getByRole("textbox");
    fireEvent.change(searchInput, { target: { value: "actionable" } });

    await waitFor(() => {
      expect(screen.getByText("code-reviewer")).toBeInTheDocument();
      expect(screen.queryByText("frontend-design")).not.toBeInTheDocument();
    });
  });

  it("搜索无结果时展示空状态", async () => {
    renderCentralSkillsView();
    const searchInput = screen.getByRole("textbox");
    fireEvent.change(searchInput, { target: { value: "zzz-nonexistent" } });

    await waitFor(() => {
      expect(screen.getByText(/没有匹配的技能/)).toBeInTheDocument();
    });
  });

  it("打开 Categorize 抽屉后展示 manual / AI / 复核 三个 tab", async () => {
    renderCentralSkillsView();
    // 选一张卡片以激活批量条
    fireEvent.click(screen.getAllByLabelText("选择技能")[0]);
    // 打开抽屉
    fireEvent.click(await screen.findByTestId("bulk-bar-open-categorize"));

    expect(await screen.findByRole("tab", { name: "手动" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "AI" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "复核" })).toBeInTheDocument();
  });

  it("Categorize 抽屉内 primary action 默认禁用（未选 tag）", async () => {
    renderCentralSkillsView();
    fireEvent.click(screen.getAllByLabelText("选择技能")[0]);
    fireEvent.click(await screen.findByTestId("bulk-bar-open-categorize"));

    expect(await screen.findByTestId("categorize-primary-action")).toBeDisabled();
  });

  it("AI tab running 时展示当前任务、多个 running 项、进度和取消按钮", async () => {
    renderCentralSkillsView({
      centralOverrides: {
        aiTagJob: {
          jobId: "job-1",
          status: "running",
          total: 4,
          completed: 2,
          succeeded: 1,
          failed: 1,
          lowConfidenceCount: 1,
          currentSkillName: "code-reviewer",
          items: {
            "frontend-design": "running",
            "code-reviewer": "running",
            queued: "queued",
            done: "succeeded",
          },
        },
        isSuggestingTags: true,
      },
    });
    fireEvent.click(screen.getAllByLabelText("选择技能")[0]);
    fireEvent.click(await screen.findByTestId("bulk-bar-open-categorize"));
    fireEvent.click(await screen.findByRole("tab", { name: "AI" }));

    const cockpit = await screen.findByTestId("ai-tag-running-cockpit");
    expect(within(cockpit).getByText("frontend-design")).toBeInTheDocument();
    expect(within(cockpit).getByText("code-reviewer")).toBeInTheDocument();
    expect(
      within(cockpit).getByRole("progressbar", { name: "AI 标注进度 50%" })
    ).toHaveAttribute("aria-valuenow", "50");
    expect(within(cockpit).getByTestId("ai-tag-panel-cancel")).toBeInTheDocument();
  });

  it("AI tab 对 unsafe rate profile 显示风险提示", async () => {
    S.settingsStore.setState({
      aiSettings: {
        ...S.settingsStore.getState().aiSettings,
        tagConcurrency: "2",
        tagIntervalMs: "1000",
        tagStopOnRateLimit: false,
      },
      aiSettingsLoaded: true,
    });
    renderCentralSkillsView();
    fireEvent.click(screen.getAllByLabelText("选择技能")[0]);
    fireEvent.click(await screen.findByTestId("bulk-bar-open-categorize"));
    fireEvent.click(await screen.findByRole("tab", { name: "AI" }));

    const rateRisk = await screen.findByTestId("ai-tag-rate-risk");
    expect(rateRisk).toHaveTextContent(
      "当前配置存在限速风险"
    );
    expect(screen.getByText(/并发 2/)).toBeInTheDocument();
    expect(rateRisk).toHaveTextContent("1000ms");
  });

  it("AI tab cancelled job 将剩余项显示为 cancelled", async () => {
    renderCentralSkillsView({
      centralOverrides: {
        aiTagJob: {
          jobId: "job-1",
          status: "cancelled",
          total: 2,
          completed: 1,
          succeeded: 1,
          failed: 0,
          lowConfidenceCount: 0,
          items: {
            "frontend-design": "succeeded",
            "code-reviewer": "cancelled",
          },
        },
      },
    });
    fireEvent.click(screen.getAllByLabelText("选择技能")[0]);
    fireEvent.click(await screen.findByTestId("bulk-bar-open-categorize"));
    fireEvent.click(await screen.findByRole("tab", { name: "AI" }));

    const result = await screen.findByTestId("ai-tag-result-cockpit");
    expect(within(result).getByText("code-reviewer")).toBeInTheDocument();
    expect(within(result).getByText("已取消")).toBeInTheDocument();
  });

  it("复核 tab 标识 proposal 并沿用接受与跳过操作", async () => {
    renderCentralSkillsView({
      centralOverrides: {
        aiTagReviews: [
          {
            skill_id: "frontend-design",
            skill_name: "frontend-design",
            tag: {
              id: "security-audit",
              name: "安全审计",
              description: "Security auditing workflows.",
              is_builtin: false,
              created_at: "2026-07-20T00:00:00Z",
              updated_at: "2026-07-20T00:00:00Z",
            },
            confidence: 0.95,
            reason: "缺少现有分类",
            suggested_at: "2026-07-20T00:00:00Z",
            updated_at: "2026-07-20T00:00:00Z",
            is_proposal: true,
          },
        ],
      },
    });
    fireEvent.click(screen.getAllByLabelText("选择技能")[0]);
    fireEvent.click(await screen.findByTestId("bulk-bar-open-categorize"));
    const surface = await screen.findByRole(
      "dialog",
      { name: "批量分类" },
      { timeout: 5_000 },
    );
    const categorize = within(surface);
    fireEvent.click(await categorize.findByRole("tab", { name: /^复核/ }));

    expect(await categorize.findByText("AI 新建标签")).toBeInTheDocument();
    expect(categorize.getByText("安全审计")).toBeInTheDocument();
    expect(
      categorize.getByText("Security auditing workflows."),
    ).toBeInTheDocument();

    fireEvent.click(categorize.getByRole("button", { name: "接受" }));
    await waitFor(() =>
      expect(S.mockAcceptAiTagReview).toHaveBeenCalledWith(
        "frontend-design",
        ["security-audit"],
      ),
    );
    fireEvent.click(categorize.getByRole("button", { name: "跳过" }));
    await waitFor(() =>
      expect(S.mockSkipAiTagReview).toHaveBeenCalledWith("frontend-design"),
    );
  });
});
