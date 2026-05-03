import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import * as S from "./centralSkillsViewTestSupport";

const {
  mockLoadCentralSkills,
  mockAssignSkillTags,
  mockRescan,
  renderCentralSkillsView,
} = S;

describe("CentralSkillsView", () => {
  beforeEach(S.resetCentralSkillsViewTestState);
  afterEach(S.cleanupCentralSkillsViewTestState);

  it("restores all skills when search is cleared", async () => {
    renderCentralSkillsView();
    const searchInput = screen.getByPlaceholderText(/搜索中央技能库/i);
    fireEvent.change(searchInput, { target: { value: "frontend" } });
    fireEvent.change(searchInput, { target: { value: "" } });

    await waitFor(() => {
      expect(screen.getByText("frontend-design")).toBeInTheDocument();
      expect(screen.getByText("code-reviewer")).toBeInTheDocument();
    });
  });


  it("calls loadCentralSkills on mount", () => {
    renderCentralSkillsView();
    expect(mockLoadCentralSkills).toHaveBeenCalledTimes(1);
  });


  it("calls rescan then loadCentralSkills when refresh button is clicked", async () => {
    renderCentralSkillsView();
    const refreshBtn = screen.getByRole("button", {
      name: /刷新中央技能库/i,
    });
    fireEvent.click(refreshBtn);

    await waitFor(() => {
      // rescan is called once (only on refresh, not on mount)
      expect(mockRescan).toHaveBeenCalledTimes(1);
      // loadCentralSkills is called twice: once on mount, once on refresh
      expect(mockLoadCentralSkills).toHaveBeenCalledTimes(2);
    });
  });


  it("opens install dialog when 'Install to...' is clicked", async () => {
    renderCentralSkillsView();
    const installBtn = screen.getAllByRole("button", {
      name: /将 .* 安装到平台/i,
    })[0];
    fireEvent.click(installBtn);

    // Dialog should open (skill name should appear in dialog title)
    await waitFor(() => {
      expect(screen.getByRole("dialog")).toBeInTheDocument();
    });
  });


  it("shows localized categorize tabs", () => {
    renderCentralSkillsView();

    expect(screen.getByRole("tab", { name: "手动" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "AI" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "复核" })).toBeInTheDocument();
    expect(screen.queryByRole("tab", { name: "Manual" })).not.toBeInTheDocument();
    expect(screen.queryByRole("tab", { name: "Review" })).not.toBeInTheDocument();
  });


  it("resizes the categorize sidebar from its left edge", () => {
    renderCentralSkillsView();

    const sidebar = screen.getByTestId("central-categorize-sidebar");
    expect(sidebar).toHaveStyle({ width: "392px" });

    fireEvent.keyDown(
      within(sidebar).getByRole("separator", { name: "调整批量分类栏宽度" }),
      { key: "ArrowLeft" }
    );

    expect(sidebar).toHaveStyle({ width: "408px" });
  });


  it("disables the categorize primary action with a visible reason before selecting skills", () => {
    renderCentralSkillsView();

    const action = screen.getByTestId("categorize-primary-action");
    expect(action).toHaveTextContent("先选择技能");
    expect(action).toBeDisabled();
    expect(screen.getByTestId("categorize-action-reason")).toHaveTextContent(
      "先选择技能后才能应用分类。"
    );
  });


  it("updates categorize summary and action text after selecting current results", () => {
    renderCentralSkillsView();

    fireEvent.click(screen.getByRole("button", { name: "选择当前结果" }));

    expect(screen.getByText("已选 2 个技能")).toBeInTheDocument();
    expect(screen.getByText("待添加 0 个分类")).toBeInTheDocument();
    const action = screen.getByTestId("categorize-primary-action");
    expect(action).toHaveTextContent("选择要添加的分类");
    expect(action).toBeDisabled();
  });


  it("assigns selected skills to manual tags from the categorize panel", async () => {
    mockAssignSkillTags.mockResolvedValue(undefined);
    renderCentralSkillsView();

    fireEvent.click(screen.getByRole("button", { name: "选择当前结果" }));
    fireEvent.click(screen.getByRole("button", { name: "前端与视觉设计" }));
    const uncategorizedButtons = screen.getAllByRole("button", { name: "未分类" });
    fireEvent.click(uncategorizedButtons[uncategorizedButtons.length - 1]);
    fireEvent.click(screen.getByRole("button", { name: /应用到 2 个技能/i }));

    await waitFor(() => {
      expect(mockAssignSkillTags).toHaveBeenCalledWith(
        ["code-reviewer", "frontend-design"],
        ["frontend-visual-design", "uncategorized"]
      );
    });
  });


  it("shows AI config hint when AI tagging is unavailable", () => {
    renderCentralSkillsView({
      centralOverrides: { aiTaggingAvailable: false },
    });

    fireEvent.click(screen.getByRole("tab", { name: "AI" }));
    expect(screen.getByText(/配置 AI API Key 后可批量自动标注/i)).toBeInTheDocument();
    expect(screen.getByText(/AI 只处理已选技能/i)).toBeInTheDocument();
    expect(screen.getByTestId("categorize-primary-action")).toBeDisabled();
    expect(screen.getByTestId("categorize-action-reason")).toHaveTextContent(
      "配置 AI API Key 后才能批量标注。"
    );
  });
});
