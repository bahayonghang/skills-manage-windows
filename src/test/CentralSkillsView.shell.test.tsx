import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import {
  cleanupCentralSkillsViewTestState,
  renderCentralSkillsView,
  resetCentralSkillsViewTestState,
} from "./centralSkillsViewTestSupport";
import { centralSkillCardGridTemplateColumns } from "@/lib/centralSkillGrid";

describe("CentralSkillsView shell（V2 markup）", () => {
  beforeEach(() => {
    resetCentralSkillsViewTestState();
    window.localStorage.clear();
  });

  afterEach(cleanupCentralSkillsViewTestState);

  // ─── Header ────────────────────────────────────────────────────────

  it("渲染标题、路径副标题、GitHub 导入按钮、检查更新主 CTA、「⋯ 更多」按钮", () => {
    renderCentralSkillsView();
    expect(screen.getByText("中央技能库")).toBeInTheDocument();
    expect(screen.getByText("/Users/test/.skillsmanage/skills/")).toBeInTheDocument();
    expect(screen.getByTestId("central-github-import-open")).toHaveTextContent("从 GitHub 导入");
    expect(screen.getByTestId("central-check-updates")).toBeInTheDocument();
    expect(screen.getByTestId("central-toolbar-more")).toBeInTheDocument();
  });

  it("「⋯ 更多」menu 展开后只保留任务中心 / 平台管理 / 状态导入导出", async () => {
    renderCentralSkillsView();
    fireEvent.click(screen.getByTestId("central-toolbar-more"));
    const menu = await screen.findByTestId("central-toolbar-more-menu");
    expect(screen.getByTestId("central-toolbar-task-center")).toBeInTheDocument();
    expect(screen.getByTestId("central-portability-open")).toBeInTheDocument();
    expect(screen.getByText("管理平台")).toBeInTheDocument();
    expect(within(menu).queryByText("从 GitHub 导入")).not.toBeInTheDocument();
  });

  it("无可更新时不展示可更新 chip 与「更新 N 个」按钮", () => {
    renderCentralSkillsView();
    expect(screen.queryByTestId("central-update-count-chip")).not.toBeInTheDocument();
  });

  it("updateStatuses 含 update_available 时展示可更新 chip", () => {
    renderCentralSkillsView({
      centralOverrides: {
        updateStatuses: {
          "frontend-design": {
            skill_id: "frontend-design",
            source_type: "github",
            status: "update_available",
          },
        },
      },
    });
    expect(screen.getByTestId("central-update-count-chip")).toHaveTextContent("+1");
  });

  // ─── Search + toolbar ─────────────────────────────────────────────

  it("二级工具条含搜索框、排序 menu、视图 menu", () => {
    renderCentralSkillsView();
    expect(screen.getByTestId("central-toolbar-sort")).toBeInTheDocument();
    expect(screen.getByTestId("central-toolbar-view")).toBeInTheDocument();
    // 搜索框 input 存在
    const search = screen.getByRole("textbox");
    expect(search).toBeInTheDocument();
  });

  it("排序 menu 展开后含 8 个 (field × dir) 选项", async () => {
    renderCentralSkillsView();
    fireEvent.click(screen.getByTestId("central-toolbar-sort"));
    expect(
      await screen.findByTestId("central-toolbar-sort-name-asc")
    ).toBeInTheDocument();
    expect(screen.getByTestId("central-toolbar-sort-name-desc")).toBeInTheDocument();
    expect(screen.getByTestId("central-toolbar-sort-createdAt-asc")).toBeInTheDocument();
    expect(screen.getByTestId("central-toolbar-sort-createdAt-desc")).toBeInTheDocument();
    expect(screen.getByTestId("central-toolbar-sort-updatedAt-asc")).toBeInTheDocument();
    expect(screen.getByTestId("central-toolbar-sort-updatedAt-desc")).toBeInTheDocument();
    expect(screen.getByTestId("central-toolbar-sort-installedPlatformCount-asc")).toBeInTheDocument();
    expect(screen.getByTestId("central-toolbar-sort-installedPlatformCount-desc")).toBeInTheDocument();
  });

  it("选择摘要仅在已选时内联出现，支持全选当前结果和清空选择", async () => {
    renderCentralSkillsView();

    // 0 选中：搜索行不渲染内联选择摘要
    expect(screen.queryByTestId("central-selection-summary")).not.toBeInTheDocument();

    // 勾选一个卡片 → 摘要出现
    fireEvent.click(screen.getAllByLabelText("选择技能")[0]);
    expect(await screen.findByTestId("central-selection-summary")).toHaveTextContent("已选 1");
    expect(await screen.findByTestId("central-bulk-action-bar")).toBeInTheDocument();

    // 全选当前结果
    fireEvent.click(screen.getByTestId("central-select-current-results"));
    expect(screen.getByTestId("central-selection-summary")).toHaveTextContent("已选 2");

    // 清空 → 摘要与批量条均消失
    fireEvent.click(screen.getByTestId("central-clear-selection"));
    await waitFor(() => {
      expect(screen.queryByTestId("central-bulk-action-bar")).not.toBeInTheDocument();
    });
    expect(screen.queryByTestId("central-selection-summary")).not.toBeInTheDocument();
  });

  it("筛选变化后移除不可见的已选技能", async () => {
    window.localStorage.setItem("central.sidebarPinned", "true");
    renderCentralSkillsView();

    fireEvent.click(screen.getAllByLabelText("选择技能")[0]);
    fireEvent.click(await screen.findByTestId("central-select-current-results"));
    expect(screen.getByTestId("central-selection-summary")).toHaveTextContent("已选 2");

    const sidebar = screen.getByTestId("central-sidebar-v2");
    fireEvent.click(within(sidebar).getByTestId("repo-github-openai-skills-main"));

    await waitFor(() => {
      expect(screen.getByTestId("central-selection-summary")).toHaveTextContent("已选 1");
    });
  });

  it("视图 menu 展开后含 group / installed / quick filters 段", async () => {
    renderCentralSkillsView();
    fireEvent.click(screen.getByTestId("central-toolbar-view"));
    expect(
      await screen.findByTestId("central-toolbar-view-installed-all")
    ).toBeInTheDocument();
    expect(screen.getByTestId("central-toolbar-view-installed-any")).toBeInTheDocument();
    expect(screen.getByTestId("central-toolbar-view-uncategorized")).toBeInTheDocument();
  });

  it("分组卡片网格复用 Central 自适应列策略而不是硬限制两列", async () => {
    renderCentralSkillsView();
    fireEvent.click(screen.getByTestId("central-toolbar-view"));
    fireEvent.click(
      await screen.findByTestId("central-toolbar-view-group-repository"),
    );

    const groupBody = await screen.findByTestId(
      "group-body-repo:github-openai-skills-main",
    );
    expect(groupBody).not.toHaveClass("lg:grid-cols-2");
    expect(groupBody.style.gridTemplateColumns).toBe(
      centralSkillCardGridTemplateColumns(),
    );
  });

  // ─── Sidebar rail vs pinned ───────────────────────────────────────

  it("默认是 collapsed rail（unpinned）", () => {
    renderCentralSkillsView();
    const sidebar = screen.getByTestId("central-sidebar-v2");
    expect(sidebar).toHaveAttribute("data-pinned", "false");
    expect(sidebar).toHaveAttribute("data-expanded", "false");
    expect(screen.getByTestId("central-sidebar-rail")).toBeInTheDocument();
  });

  it("pin 后切换为 expanded 状态", () => {
    window.localStorage.setItem("central.sidebarPinned", "true");
    renderCentralSkillsView();
    const sidebar = screen.getByTestId("central-sidebar-v2");
    expect(sidebar).toHaveAttribute("data-pinned", "true");
    expect(sidebar).toHaveAttribute("data-expanded", "true");
  });

  // ─── 任务中心 + 进度顶线 ──────────────────────────────────────────

  it("默认无活跃任务时不渲染 1px 进度顶线、不渲染 chip", () => {
    renderCentralSkillsView();
    expect(screen.queryByTestId("central-progress-top-line")).not.toBeInTheDocument();
    expect(screen.queryByTestId("central-toolbar-task-center-chip")).not.toBeInTheDocument();
  });

  it("aiTagJob running 时渲染 1px 顶线 + 工具栏「任务进行中」chip", () => {
    renderCentralSkillsView({
      centralOverrides: {
        aiTagJob: {
          jobId: "ai-1",
          status: "running",
          total: 10,
          completed: 4,
          succeeded: 3,
          failed: 0,
          lowConfidenceCount: 1,
          items: {},
        },
      },
    });
    expect(screen.getByTestId("central-progress-top-line")).toBeInTheDocument();
    expect(screen.getByTestId("central-toolbar-task-center-chip")).toBeInTheDocument();
  });

  it("「⋯ 更多」menu 任务中心点击后打开抽屉并展示任务中心标题", async () => {
    renderCentralSkillsView({
      centralOverrides: {
        aiTagJob: {
          jobId: "ai-1",
          status: "running",
          total: 10,
          completed: 4,
          succeeded: 3,
          failed: 0,
          lowConfidenceCount: 1,
          items: {},
        },
      },
    });
    fireEvent.click(screen.getByTestId("central-toolbar-more"));
    const menuItem = await screen.findByTestId("central-toolbar-task-center");
    expect(within(menuItem).getByText("1")).toBeInTheDocument(); // active badge
    fireEvent.click(menuItem);
    expect(
      await screen.findByTestId("central-task-center-drawer")
    ).toBeInTheDocument();
    expect(screen.getByTestId("central-task-row-ai-tag")).toBeInTheDocument();
  });

  it("任务中心 chip 点击同样打开抽屉", async () => {
    renderCentralSkillsView({
      centralOverrides: {
        aiTagJob: {
          jobId: "ai-1",
          status: "running",
          total: 5,
          completed: 1,
          succeeded: 1,
          failed: 0,
          lowConfidenceCount: 0,
          items: {},
        },
      },
    });
    fireEvent.click(screen.getByTestId("central-toolbar-task-center-chip"));
    expect(
      await screen.findByTestId("central-task-center-drawer")
    ).toBeInTheDocument();
  });

  it("无任务时抽屉打开后展示 empty 状态", async () => {
    renderCentralSkillsView();
    fireEvent.click(screen.getByTestId("central-toolbar-more"));
    fireEvent.click(await screen.findByTestId("central-toolbar-task-center"));
    expect(
      await screen.findByTestId("central-task-center-empty")
    ).toBeInTheDocument();
  });

  // ─── BulkActionBar / CategorizeDrawer ────────────────────────────

  it("无选中时不渲染批量操作条", () => {
    renderCentralSkillsView();
    expect(screen.queryByTestId("central-bulk-action-bar")).not.toBeInTheDocument();
  });

  it("选中一张卡片后浮出底部批量条，含批装 / 批量卸载 / 打标签 / AI 建议 / 批删 / 取消选择", async () => {
    renderCentralSkillsView();
    const [firstCheckbox] = screen.getAllByLabelText("选择技能");
    fireEvent.click(firstCheckbox);
    expect(
      await screen.findByTestId("central-bulk-action-bar")
    ).toBeInTheDocument();
    expect(screen.getByTestId("bulk-bar-batch-install")).toBeInTheDocument();
    expect(screen.getByTestId("bulk-bar-batch-uninstall")).toHaveTextContent(
      "批量卸载",
    );
    expect(screen.getByTestId("bulk-bar-open-categorize")).toBeInTheDocument();
    expect(screen.getByTestId("bulk-bar-open-ai-suggest")).toBeInTheDocument();
    expect(screen.getByTestId("bulk-bar-batch-delete")).toBeInTheDocument();
    expect(screen.getByTestId("bulk-bar-clear-selection")).toBeInTheDocument();
  });

  it("首次选中后保留列表滚动位置并为底部批量条预留安全区", async () => {
    renderCentralSkillsView();
    const scrollContainer = screen.getByTestId("central-skill-list-scroll");
    const [firstCheckbox] = screen.getAllByLabelText("选择技能");
    scrollContainer.scrollTop = 240;

    fireEvent.click(firstCheckbox);

    expect(await screen.findByTestId("central-bulk-action-bar")).toBeInTheDocument();
    await waitFor(() => {
      expect(scrollContainer.scrollTop).toBe(240);
    });
    expect(scrollContainer).toHaveClass("pb-28");

    scrollContainer.scrollTop = 360;
    fireEvent.click(firstCheckbox);

    await waitFor(() => {
      expect(screen.queryByTestId("central-bulk-action-bar")).not.toBeInTheDocument();
    });
    await waitFor(() => {
      expect(scrollContainer.scrollTop).toBe(360);
    });
    expect(scrollContainer).not.toHaveClass("pb-28");
  });

  it("批量条「打标签」按钮触发 Categorize 抽屉", async () => {
    renderCentralSkillsView();
    fireEvent.click(screen.getAllByLabelText("选择技能")[0]);
    fireEvent.click(await screen.findByTestId("bulk-bar-open-categorize"));
    expect(
      await screen.findByTestId("central-categorize-drawer")
    ).toBeInTheDocument();
  });

  it("批量条「取消选择」清空选中并关闭批量条", async () => {
    renderCentralSkillsView();
    const scrollContainer = screen.getByTestId("central-skill-list-scroll");
    fireEvent.click(screen.getAllByLabelText("选择技能")[0]);
    expect(await screen.findByTestId("central-bulk-action-bar")).toBeInTheDocument();
    expect(scrollContainer).toHaveClass("pb-28");
    fireEvent.click(await screen.findByTestId("bulk-bar-clear-selection"));
    await waitFor(() => {
      expect(screen.queryByTestId("central-bulk-action-bar")).not.toBeInTheDocument();
    });
    expect(scrollContainer).not.toHaveClass("pb-28");
  });
});

