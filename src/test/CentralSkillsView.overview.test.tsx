import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { fireEvent, screen, waitFor } from "@testing-library/react";
import * as S from "./centralSkillsViewTestSupport";

const {
  mockExportSkillportState,
  renderCentralSkillsView,
} = S;

describe("CentralSkillsView overview（V2 markup）", () => {
  beforeEach(() => {
    S.resetCentralSkillsViewTestState();
    window.localStorage.clear();
  });
  afterEach(S.cleanupCentralSkillsViewTestState);

  it("展示中央技能库标题", () => {
    renderCentralSkillsView();
    expect(screen.getByText("中央技能库")).toBeInTheDocument();
  });

  it("展示中央技能目录路径副标题", () => {
    renderCentralSkillsView();
    expect(screen.getByText("/Users/test/.skillsmanage/skills/")).toBeInTheDocument();
  });

  it("从「⋯ 更多」menu 打开 portability dialog", async () => {
    renderCentralSkillsView();

    fireEvent.click(screen.getByTestId("central-toolbar-more"));
    fireEvent.click(await screen.findByTestId("central-portability-open"));

    expect(await screen.findByTestId("central-portability-save-export")).toBeInTheDocument();
    await waitFor(() => expect(mockExportSkillportState).toHaveBeenCalled());
  });

  it("从「⋯ 更多」menu 暴露 GitHub 导入入口", async () => {
    renderCentralSkillsView();
    fireEvent.click(screen.getByTestId("central-toolbar-more"));
    expect(await screen.findByText("从 GitHub 导入")).toBeInTheDocument();
  });

  it("搜索框可输入并触发列表过滤（按名称）", async () => {
    renderCentralSkillsView();
    const searchInput = screen.getByRole("textbox");
    fireEvent.change(searchInput, { target: { value: "frontend" } });

    await waitFor(() => {
      expect(screen.getByText("frontend-design")).toBeInTheDocument();
      expect(screen.queryByText("code-reviewer")).not.toBeInTheDocument();
    });
  });

  it("展开 sidebar（pinned）后可见仓库 facet 列表", () => {
    window.localStorage.setItem("central.sidebarPinned", "true");
    renderCentralSkillsView();

    const sidebar = screen.getByTestId("central-sidebar-v2");
    expect(sidebar).toHaveAttribute("data-pinned", "true");
    expect(screen.getByText("openai/skills")).toBeInTheDocument();
  });

  it("repo facet hover 后展示删除入口；is_unknown 仓库不暴露删除", () => {
    window.localStorage.setItem("central.sidebarPinned", "true");
    renderCentralSkillsView();

    expect(
      screen.getByTestId("repo-delete-github-openai-skills-main")
    ).toBeInTheDocument();
    expect(
      screen.queryByTestId("repo-delete-local-unknown")
    ).not.toBeInTheDocument();
  });
});
