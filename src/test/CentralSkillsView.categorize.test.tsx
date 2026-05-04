import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { fireEvent, screen, waitFor } from "@testing-library/react";
import * as S from "./centralSkillsViewTestSupport";

const {
  mockTogglePlatformLink,
  renderCentralSkillsView,
} = S;

describe("CentralSkillsView", () => {
  beforeEach(S.resetCentralSkillsViewTestState);
  afterEach(S.cleanupCentralSkillsViewTestState);

  it("skill name is a clickable button for detail navigation", () => {
    renderCentralSkillsView();
    // The skill name itself is the detail link (no separate [详情] button).
    const detailBtns = screen.getAllByRole("button", {
      name: /查看 frontend-design 的详情/i,
    });
    expect(detailBtns.length).toBeGreaterThanOrEqual(1);
  });


  it("shows platform toggle icons for each non-central agent", () => {
    renderCentralSkillsView();
    const toggleButtons = screen.getAllByRole("button", {
      name: /切换 .* 的链接状态/i,
    });
    expect(toggleButtons.length).toBe(4);
  });


  it("renders Universal platform icon as toggleable", () => {
    renderCentralSkillsView();

    const codexButton = screen.getAllByRole("button", {
      name: /切换 frontend-design 在 Universal 的链接状态/i,
    })[0];
    expect(codexButton).not.toBeDisabled();

    fireEvent.click(codexButton);
    expect(mockTogglePlatformLink).toHaveBeenCalledWith("frontend-design", "codex");
  });


  it("shows first-visit empty state when no skills exist", () => {
    renderCentralSkillsView({
      centralOverrides: { skills: [] },
    });

    expect(
      screen.getByText(/欢迎使用 SkillPort/)
    ).toBeInTheDocument();
    // Should show guidance about creating a skill
    expect(
      screen.getAllByText(/skillsmanage\/skills/).length
    ).toBeGreaterThanOrEqual(1);
  });


  it("shows loading state", () => {
    renderCentralSkillsView({
      centralOverrides: { isLoading: true, skills: [] },
    });

    expect(screen.getByText("正在加载技能...")).toBeInTheDocument();
  });


  it("filters skills by name when searching", async () => {
    renderCentralSkillsView();
    const searchInput = screen.getByPlaceholderText(/搜索中央技能库/i);
    fireEvent.change(searchInput, { target: { value: "frontend" } });

    await waitFor(() => {
      expect(screen.getByText("frontend-design")).toBeInTheDocument();
      expect(screen.queryByText("code-reviewer")).not.toBeInTheDocument();
    });
  });


  it("filters skills by description when searching", async () => {
    renderCentralSkillsView();
    const searchInput = screen.getByPlaceholderText(/搜索中央技能库/i);
    fireEvent.change(searchInput, { target: { value: "actionable" } });

    await waitFor(() => {
      expect(screen.getByText("code-reviewer")).toBeInTheDocument();
      expect(screen.queryByText("frontend-design")).not.toBeInTheDocument();
    });
  });


  it("filters skills by repository shortcut", async () => {
    renderCentralSkillsView();

    fireEvent.click(screen.getByRole("button", { name: /未归仓/i }));

    await waitFor(() => {
      expect(screen.getByText("code-reviewer")).toBeInTheDocument();
      expect(screen.queryByText("frontend-design")).not.toBeInTheDocument();
    });
  });


  it("filters skills by tag", async () => {
    renderCentralSkillsView();

    fireEvent.click(screen.getByTestId("tag-search-trigger"));
    fireEvent.click(await screen.findByTestId("tag-search-item-frontend-visual-design"));

    await waitFor(() => {
      expect(screen.getByText("frontend-design")).toBeInTheDocument();
      expect(screen.queryByText("code-reviewer")).not.toBeInTheDocument();
    });
  });


  it("shows empty state when search has no results", async () => {
    renderCentralSkillsView();
    const searchInput = screen.getByPlaceholderText(/搜索中央技能库/i);
    fireEvent.change(searchInput, { target: { value: "zzz-nonexistent" } });

    await waitFor(() => {
      expect(screen.getByText(/没有匹配的技能/)).toBeInTheDocument();
    });
  });
});
