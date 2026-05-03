import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import * as S from "./centralSkillsViewTestSupport";

const {
  mockExportSkillportState,
  renderCentralSkillsView,
} = S;

describe("CentralSkillsView", () => {
  beforeEach(S.resetCentralSkillsViewTestState);
  afterEach(S.cleanupCentralSkillsViewTestState);

  it("shows page title in header", () => {
    renderCentralSkillsView();
    expect(screen.getByText("中央技能库")).toBeInTheDocument();
  });


  it("shows the central skills directory path", () => {
    renderCentralSkillsView();
    expect(screen.getByText("/Users/test/.skillsmanage/skills/")).toBeInTheDocument();
  });


  it("opens the Central state portability dialog", async () => {
    renderCentralSkillsView();

    fireEvent.click(screen.getByTestId("central-portability-open"));

    expect(await screen.findByTestId("central-portability-save-export")).toBeInTheDocument();
    await waitFor(() => expect(mockExportSkillportState).toHaveBeenCalled());
  });


  it("shows a refresh button", () => {
    renderCentralSkillsView();
    expect(
      screen.getByRole("button", { name: /刷新中央技能库/i })
    ).toBeInTheDocument();
  });


  it("shows the shared github import launcher", () => {
    renderCentralSkillsView();
    expect(screen.getByRole("button", { name: /从 GitHub 导入/i })).toBeInTheDocument();
  });


  it("shows a search input", () => {
    renderCentralSkillsView();
    expect(
      screen.getByPlaceholderText(/搜索中央技能库/i)
    ).toBeInTheDocument();
  });


  it("shows explicit sort field and direction controls", () => {
    renderCentralSkillsView();

    expect(screen.getByRole("group", { name: "排序字段" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "名称" })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("button", { name: "创建时间" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "修改时间" })).toBeInTheDocument();

    expect(screen.getByRole("group", { name: "排序方向" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "正排" })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("button", { name: "倒排" })).toBeInTheDocument();
  });


  it("shows repository and tag workspace controls", () => {
    renderCentralSkillsView();

    expect(screen.getByText("仓库来源")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /未归仓/i })).toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "分类筛选" })).toBeInTheDocument();
    expect(screen.getAllByText("openai/skills").length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText("前端与视觉设计").length).toBeGreaterThanOrEqual(1);
  });


  it("renders repositories and tags with distinct sidebar semantics", async () => {
    renderCentralSkillsView();

    const sidebar = screen.getByTestId("central-filter-sidebar");
    expect(sidebar).toHaveStyle({ width: "286px" });
    const scrollContainer = sidebar.firstElementChild as HTMLElement;
    expect(scrollContainer).toHaveClass("overflow-y-auto");
    fireEvent.keyDown(within(sidebar).getByRole("separator"), { key: "ArrowRight" });
    expect(sidebar).toHaveStyle({ width: "302px" });
    expect(
      within(sidebar).getByTestId("repository-filter-github-openai-skills-main")
    ).toHaveAttribute("data-source-kind", "github");
    expect(
      within(sidebar).getByTestId("repository-filter-local-unknown")
    ).toHaveAttribute("data-source-kind", "local");
    expect(
      within(sidebar).getByTestId("tag-filter-frontend-visual-design")
    ).toHaveAttribute("data-filter-kind", "tag");

    fireEvent.click(within(sidebar).getByTestId("repository-filter-github-openai-skills-main"));
    await waitFor(() => {
      expect(screen.getByText("frontend-design")).toBeInTheDocument();
      expect(screen.queryByText("code-reviewer")).not.toBeInTheDocument();
    });

    fireEvent.click(within(sidebar).getByTestId("repository-filter-all"));
    fireEvent.click(within(sidebar).getByTestId("tag-filter-frontend-visual-design"));
    await waitFor(() => {
      expect(screen.getByText("frontend-design")).toBeInTheDocument();
      expect(screen.queryByText("code-reviewer")).not.toBeInTheDocument();
    });
  });


  it("does not show a repository delete action for the system unknown repository", () => {
    renderCentralSkillsView();

    const sidebar = screen.getByTestId("central-filter-sidebar");
    expect(
      within(sidebar).queryByTestId("repository-filter-local-unknown-delete")
    ).not.toBeInTheDocument();
    expect(
      within(sidebar).getByTestId("repository-filter-github-openai-skills-main-delete")
    ).toBeInTheDocument();
  });
});
