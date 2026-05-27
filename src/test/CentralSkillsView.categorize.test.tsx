import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { fireEvent, screen, waitFor } from "@testing-library/react";
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
});
