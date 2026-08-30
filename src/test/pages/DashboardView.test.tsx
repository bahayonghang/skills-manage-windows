import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { MemoryRouter, useLocation } from "react-router-dom";

import i18n from "@/i18n";
import { DashboardView } from "@/pages/DashboardView";
import { useCentralSkillsStore } from "@/stores/centralSkillsStore";
import { useOperationLogStore } from "@/stores/operationLogStore";
import { usePlatformStore } from "@/stores/platformStore";
import { useSkillsCliStore } from "@/stores/skillsCliStore";
import { useTargetStore } from "@/stores/targetStore";
import type {
  AgentWithStatus,
  CentralTopTag,
  DailyOperationCount,
  OperationLogEntry,
  SkillRepositoryWithStats,
  TargetSummary,
} from "@/types";

vi.mock("@/stores/centralSkillsStore", () => ({
  useCentralSkillsStore: vi.fn(),
}));

vi.mock("@/stores/operationLogStore", () => ({
  useOperationLogStore: vi.fn(),
}));

vi.mock("@/stores/platformStore", () => ({
  usePlatformStore: vi.fn(),
}));

vi.mock("@/stores/targetStore", () => ({
  useTargetStore: vi.fn(),
}));

const mockUseCentralSkillsStore = vi.mocked(useCentralSkillsStore);
const mockUseOperationLogStore = vi.mocked(useOperationLogStore);
const mockUsePlatformStore = vi.mocked(usePlatformStore);
const mockUseTargetStore = vi.mocked(useTargetStore);

const mockSubscribeAiTagProgress = vi.fn();
const mockSubscribeUpdateProgress = vi.fn();
const mockLoadLogs = vi.fn();
const mockLoadDailyCounts = vi.fn();
const mockRefreshDashboardSummary = vi.fn();
const mockLoadTopTags = vi.fn();

const agents: AgentWithStatus[] = [
  {
    id: "central",
    display_name: "Central Skills",
    category: "central",
    global_skills_dir: "~/.skillsmanage/skills/",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
  {
    id: "codex",
    display_name: "Codex CLI",
    category: "coding",
    global_skills_dir: "~/.agents/skills/",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
  {
    id: "claude-code",
    display_name: "Claude Code",
    category: "coding",
    global_skills_dir: "~/.claude/skills/",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
];

const repositories: SkillRepositoryWithStats[] = [
  {
    id: "repo-known",
    name: "openai/skills",
    source_type: "github",
    owner: "openai",
    repo: "skills",
    branch: null,
    url: null,
    pinned: false,
    is_unknown: false,
    created_at: "2026-04-10T00:00:00Z",
    updated_at: "2026-04-10T00:00:00Z",
    skill_count: 2,
    unknown_skill_count: 0,
  },
];

function localDateKey(offsetDays: number): string {
  const date = new Date();
  date.setDate(date.getDate() - offsetDays);
  const month = `${date.getMonth() + 1}`.padStart(2, "0");
  const day = `${date.getDate()}`.padStart(2, "0");
  return `${date.getFullYear()}-${month}-${day}`;
}

/** 14 个本地日桶（升序、含今天），总计 5 次操作。 */
const dailyCounts: DailyOperationCount[] = Array.from(
  { length: 14 },
  (_, index) => ({
    date: localDateKey(13 - index),
    count: index === 13 ? 3 : index === 10 ? 2 : 0,
  }),
);

const topTags: CentralTopTag[] = [
  { id: "web", name: "Web", count: 3 },
  { id: "docs", name: "Docs", count: 2 },
];

const logs: OperationLogEntry[] = [
  {
    id: "log-1",
    createdAt: "2026-04-27T10:00:00Z",
    level: "info",
    targetKind: "local",
    targetId: "local",
    targetLabel: "Local",
    category: "scan",
    action: "scan.all",
    status: "succeeded",
    summary: "Scanned 3 skills",
  },
];

const localTarget: TargetSummary = {
  id: "local",
  kind: "local",
  label: "Local",
  isActive: true,
};

const wslTarget: TargetSummary = {
  id: "wsl-demo",
  kind: "wsl",
  label: "Ubuntu",
  distribution: "Ubuntu-24.04",
  remoteHome: "/home/alice",
  isActive: false,
};

let centralState: Record<string, unknown>;
let operationLogState: Record<string, unknown>;
let platformState: Record<string, unknown>;
let targetState: Record<string, unknown>;

function LocationProbe() {
  const location = useLocation();
  return (
    <div data-testid="location">
      {location.pathname}
      {location.search}
    </div>
  );
}

function renderDashboard() {
  return render(
    <MemoryRouter initialEntries={["/dashboard"]}>
      <DashboardView />
      <LocationProbe />
    </MemoryRouter>,
  );
}

function installStoreMocks() {
  mockUseCentralSkillsStore.mockImplementation((selector?: unknown) => {
    if (typeof selector === "function") return selector(centralState);
    return centralState;
  });
  mockUseOperationLogStore.mockImplementation((selector?: unknown) => {
    if (typeof selector === "function") return selector(operationLogState);
    return operationLogState;
  });
  mockUsePlatformStore.mockImplementation((selector?: unknown) => {
    if (typeof selector === "function") return selector(platformState);
    return platformState;
  });
  mockUseTargetStore.mockImplementation((selector?: unknown) => {
    if (typeof selector === "function") return selector(targetState);
    return targetState;
  });
}

describe("DashboardView", () => {
  beforeEach(async () => {
    vi.clearAllMocks();
    await i18n.changeLanguage("en");

    mockSubscribeAiTagProgress.mockResolvedValue(() => {});
    mockSubscribeUpdateProgress.mockResolvedValue(() => {});
    mockLoadLogs.mockResolvedValue({
      entries: logs,
      total: logs.length,
      limit: 5,
      offset: 0,
    });
    mockLoadDailyCounts.mockResolvedValue(undefined);
    mockRefreshDashboardSummary.mockResolvedValue(undefined);
    mockLoadTopTags.mockResolvedValue(undefined);

    centralState = {
      aiTagJob: { status: "idle", total: 0, completed: 0 },
      updateJob: { status: "idle", total: 0, completed: 0 },
      error: null,
      subscribeAiTagProgress: mockSubscribeAiTagProgress,
      subscribeUpdateProgress: mockSubscribeUpdateProgress,
    };
    operationLogState = {
      entries: logs,
      total: logs.length,
      isLoading: false,
      error: null,
      loadLogs: mockLoadLogs,
      dailyCounts,
      isDailyCountsLoading: false,
      dailyCountsError: null,
      loadDailyCounts: mockLoadDailyCounts,
    };
    platformState = {
      agents,
      skillsByAgent: {
        central: 3,
        codex: 2,
        "claude-code": 1,
      },
      dashboardCentralSummary: {
        centralSkillCount: 3,
        updatesAvailable: 1,
        aiReviewCount: 1,
        uncategorizedCount: 2,
        unassignedSourceCount: 0,
        readiness: {
          score: 65,
          categorizedRatio: 0.34,
          describedRatio: 0.82,
          sourcedRatio: 0.67,
          installHealthRatio: 0.5,
        },
        sourceRepositories: repositories,
      },
      categoryVisibility: {
        coding: true,
        lobster: true,
      },
      lastScanAt: "2026-04-27T10:00:00Z",
      scanState: "idle",
      scanGeneration: 1,
      isLoading: false,
      isRefreshing: false,
      topTags,
      isTopTagsLoading: false,
      topTagsError: null,
      refreshDashboardSummary: mockRefreshDashboardSummary,
      loadTopTags: mockLoadTopTags,
    };
    targetState = {
      activeTarget: localTarget,
      targets: [localTarget, wslTarget],
    };
    useSkillsCliStore.setState({
      skills: [],
      targets: [],
      loadAll: vi.fn(async () => {}),
      inventoryError: null,
    });
    installStoreMocks();
  });

  it("renders status header, work queue and panels from the summary", () => {
    renderDashboard();

    // Hero 营销块已删除：不再有 h1。
    expect(screen.queryByRole("heading", { level: 1 })).not.toBeInTheDocument();

    // 状态头：扫描状态 + 上次扫描 + 汇总句。
    expect(screen.getByText(/Idle|空闲/)).toBeInTheDocument();
    expect(screen.getByText(/Last scan:|上次扫描：/)).toBeInTheDocument();
    expect(
      screen.getByText(
        /3 central skills · 1 sources · 2 agents enabled|中央库 3 项技能 · 1 个来源 · 已启用 2 个平台/,
      ),
    ).toBeInTheDocument();

    // 工作队列：4 项平铺，计数来自 summary；0 值项依然可见。
    expect(screen.getByText(/Update available|可用更新/)).toBeInTheDocument();
    expect(
      screen.getByText(/AI review pending|AI 复核待处理/),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/Uncategorized skills|未分类技能/),
    ).toBeInTheDocument();
    const unassignedTile = screen.getByRole("button", {
      name: /Unknown source|未知来源/,
    });
    expect(unassignedTile).toHaveTextContent("0");

    // Readiness 卡（瘦身：无 mini stat 区）。
    expect(screen.getByText("65")).toBeInTheDocument();
    expect(screen.getByText(/Review gate|复核关口/)).toBeInTheDocument();

    // Activity：SVG 柱状图（14 桶，aria 概述含总数）。
    expect(
      screen.getByRole("img", {
        name: /5 operations over the last 14 days|最近 14 天共 5 次操作/,
      }),
    ).toBeInTheDocument();

    // TopTags 与日志。
    expect(screen.getByText("Web")).toBeInTheDocument();
    expect(screen.getByText("Docs")).toBeInTheDocument();
    expect(screen.getByText("Scanned 3 skills")).toBeInTheDocument();

    expect(screen.getByTestId("dashboard-scroll-region")).toHaveClass(
      "overflow-y-auto",
      "scrollbar-subtle",
    );
  });

  it("bootstraps summary, charts and recent logs on mount", async () => {
    renderDashboard();

    await waitFor(() => {
      expect(mockRefreshDashboardSummary).toHaveBeenCalledTimes(1);
      expect(mockLoadTopTags).toHaveBeenCalledWith(6);
      expect(mockLoadDailyCounts).toHaveBeenCalledWith(14);
      expect(mockLoadLogs).toHaveBeenCalledWith({ limit: 5, offset: 0 });
    });
    expect(mockSubscribeAiTagProgress).toHaveBeenCalledTimes(1);
    expect(mockSubscribeUpdateProgress).toHaveBeenCalledTimes(1);
  });

  it("navigates through status header actions", () => {
    renderDashboard();

    fireEvent.click(screen.getByTestId("dashboard-action-marketplace"));

    expect(screen.getByTestId("location")).toHaveTextContent("/marketplace");
  });

  it("navigates to the quick migrate settings action", () => {
    renderDashboard();

    const quickMigrate = screen.getByTestId("dashboard-action-quick-migrate");
    expect(quickMigrate).toHaveTextContent(/Quick migrate|快速迁移/);
    expect(quickMigrate).toHaveAttribute(
      "title",
      expect.stringMatching(/Open remote sync for Ubuntu\.|打开 Ubuntu 的远程同步。/),
    );

    fireEvent.click(quickMigrate);

    expect(screen.getByTestId("location")).toHaveTextContent(
      "/settings/connections?action=local-remote-sync&section=remote-targets",
    );
  });

  it("reloads dashboard data when scanGeneration changes", async () => {
    const { rerender } = renderDashboard();

    await waitFor(() => {
      expect(mockRefreshDashboardSummary).toHaveBeenCalledTimes(1);
    });

    platformState = { ...platformState, scanGeneration: 2 };
    rerender(
      <MemoryRouter initialEntries={["/dashboard"]}>
        <DashboardView />
        <LocationProbe />
      </MemoryRouter>,
    );

    await waitFor(() => {
      expect(mockRefreshDashboardSummary).toHaveBeenCalledTimes(2);
      expect(mockLoadTopTags).toHaveBeenCalledTimes(2);
      expect(mockLoadDailyCounts).toHaveBeenCalledTimes(2);
    });
  });

  it("shows chart error state with retry while the other chart stays healthy", async () => {
    operationLogState = {
      ...operationLogState,
      dailyCounts: [],
      dailyCountsError: "logs backend down",
    };

    renderDashboard();

    // 部分失败：Activity 错误占位，TopTags 正常渲染。
    expect(
      screen.getByText(/Couldn't load chart data|图表数据加载失败/),
    ).toBeInTheDocument();
    expect(screen.getByText("Web")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /Retry|重试/ }));

    await waitFor(() => {
      expect(mockLoadDailyCounts).toHaveBeenCalledTimes(2);
    });
  });

  it("renders empty and error states without crashing", () => {
    centralState = { ...centralState, error: "central offline" };
    operationLogState = {
      ...operationLogState,
      entries: [],
      total: 0,
      dailyCounts: [],
    };
    platformState = {
      ...platformState,
      topTags: [],
      dashboardCentralSummary: {
        centralSkillCount: 0,
        updatesAvailable: 0,
        aiReviewCount: 0,
        uncategorizedCount: 0,
        unassignedSourceCount: 0,
        readiness: {
          score: 0,
          categorizedRatio: 0,
          describedRatio: 0,
          sourcedRatio: 0,
          installHealthRatio: 0,
        },
        sourceRepositories: [],
      },
    };

    renderDashboard();

    expect(
      screen.getByText(/Some dashboard data could not be loaded|部分 Dashboard 数据加载失败/),
    ).toHaveAttribute("title", "central offline");
    expect(
      screen.getByText(/No recent operation logs|暂无最近操作日志/),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/No Central tags yet|还没有中央库标签/),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/No activity data yet|暂无活动数据/),
    ).toBeInTheDocument();
  });

  it("renders the Skills CLI census on Local without changing central summary", async () => {
    const summary = platformState.dashboardCentralSummary;
    const loadAll = vi.fn(async () => {});
    useSkillsCliStore.setState({ loadAll });
    renderDashboard();
    expect(
      await screen.findByTestId("dashboard-skills-cli-census"),
    ).toBeInTheDocument();
    expect(loadAll).toHaveBeenCalled();
    expect(platformState.dashboardCentralSummary).toBe(summary);
    expect(mockRefreshDashboardSummary).not.toHaveBeenCalledTimes(2);
  });

  it("does not load or render the Skills CLI census on a non-Local target", () => {
    const loadAll = vi.fn(async () => {});
    useSkillsCliStore.setState({ loadAll });
    targetState = { activeTarget: wslTarget, targets: [localTarget, wslTarget] };
    installStoreMocks();
    renderDashboard();
    expect(
      screen.queryByTestId("dashboard-skills-cli-census"),
    ).not.toBeInTheDocument();
    expect(loadAll).not.toHaveBeenCalled();
  });

  it("keeps dashboard central summary intact when the Skills CLI loader fails", async () => {
    const summary = platformState.dashboardCentralSummary;
    const loadAll = vi.fn(async () => {
      useSkillsCliStore.setState({
        inventoryError: "internal.unexpected:list failed",
      });
    });
    useSkillsCliStore.setState({ loadAll });
    renderDashboard();
    await waitFor(() => {
      expect(loadAll).toHaveBeenCalled();
    });
    expect(platformState.dashboardCentralSummary).toEqual(summary);
    expect(platformState.dashboardCentralSummary).toMatchObject({
      centralSkillCount: 3,
    });
  });
});
