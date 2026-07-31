import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { fireEvent, screen, waitFor } from "@testing-library/react";
import * as S from "./centralSkillsViewTestSupport";

const {
  mockLoadCentralSkills,
  mockAssignSkillTags,
  renderCentralSkillsView,
} = S;

describe("CentralSkillsView shared GitHub import + categorize handlers", () => {
  beforeEach(() => {
    S.resetCentralSkillsViewTestState();
    window.localStorage.clear();
  });
  afterEach(S.cleanupCentralSkillsViewTestState);

  it("shares the controlled optional branch field with the Central launcher", async () => {
    renderCentralSkillsView();
    await S.openGitHubImportViaLauncher(screen);

    const branchInput = await screen.findByLabelText("Branch (optional)");
    fireEvent.change(branchInput, { target: { value: "dev" } });
    expect(branchInput).toHaveValue("dev");
  });

  it("清空搜索后恢复完整列表", async () => {
    renderCentralSkillsView();
    const searchInput = screen.getByRole("textbox");
    fireEvent.change(searchInput, { target: { value: "frontend" } });
    fireEvent.change(searchInput, { target: { value: "" } });

    await waitFor(() => {
      expect(screen.getByText("frontend-design")).toBeInTheDocument();
      expect(screen.getByText("code-reviewer")).toBeInTheDocument();
    });
  });

  it("挂载时调用 loadCentralSkills 一次", () => {
    renderCentralSkillsView();
    expect(mockLoadCentralSkills).toHaveBeenCalledTimes(1);
  });

  it("hover 卡片显示安装按钮，点击打开 install dialog", async () => {
    renderCentralSkillsView();
    const installBtn = screen.getAllByRole("button", {
      name: /将 .* 安装到平台/i,
    })[0];
    fireEvent.click(installBtn);

    await waitFor(() => {
      expect(screen.getByRole("dialog")).toBeInTheDocument();
    });
  });

  it("Categorize 抽屉本地化展示 manual / AI / 复核 三个 tab", async () => {
    renderCentralSkillsView();
    fireEvent.click(screen.getAllByLabelText("选择技能")[0]);
    fireEvent.click(await screen.findByTestId("bulk-bar-open-categorize"));

    expect(await screen.findByRole("tab", { name: "手动" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "AI" })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: "复核" })).toBeInTheDocument();
    expect(screen.queryByRole("tab", { name: "Manual" })).not.toBeInTheDocument();
    expect(screen.queryByRole("tab", { name: "Review" })).not.toBeInTheDocument();
  });

  it("Categorize 抽屉内未选 tag 时 primary action 禁用", async () => {
    renderCentralSkillsView();
    fireEvent.click(screen.getAllByLabelText("选择技能")[0]);
    fireEvent.click(await screen.findByTestId("bulk-bar-open-categorize"));
    expect(await screen.findByTestId("categorize-primary-action")).toBeDisabled();
  });

  it("选中 1 个技能后 categorize 抽屉支持选 tag 并应用到所选技能", async () => {
    mockAssignSkillTags.mockResolvedValue(undefined);
    renderCentralSkillsView();
    fireEvent.click(screen.getAllByLabelText("选择技能")[0]);
    fireEvent.click(await screen.findByTestId("bulk-bar-open-categorize"));

    fireEvent.click(await screen.findByRole("button", { name: "前端与视觉设计" }));
    fireEvent.click(screen.getByTestId("categorize-primary-action"));

    await waitFor(() => {
      expect(mockAssignSkillTags).toHaveBeenCalled();
    });
  });

  it("aiTaggingAvailable=false 时 AI tab 展示配置提示并禁用 primary action", async () => {
    renderCentralSkillsView({
      centralOverrides: { aiTaggingAvailable: false },
    });
    fireEvent.click(screen.getAllByLabelText("选择技能")[0]);
    fireEvent.click(await screen.findByTestId("bulk-bar-open-categorize"));

    fireEvent.click(await screen.findByRole("tab", { name: "AI" }));
    await waitFor(() => {
      expect(screen.getByText(/配置 AI API Key 后可批量自动标注/i)).toBeInTheDocument();
    });
    expect(screen.getByTestId("categorize-primary-action")).toBeDisabled();
    expect(screen.getByTestId("categorize-action-reason")).toHaveTextContent(
      "配置 AI API Key 后才能批量标注。"
    );
  });
});
