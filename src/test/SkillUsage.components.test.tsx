import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { MemoryRouter, Routes, Route, useLocation } from "react-router-dom";

import { KpiStrip } from "../components/usage/KpiStrip";
import { ProviderHealthList } from "../components/usage/ProviderHealthList";
import { SkillBarChart } from "../components/usage/SkillBarChart";
import { ActivityHeatmap } from "../components/usage/ActivityHeatmap";
import { RecentCallsFeed } from "../components/usage/RecentCallsFeed";
import { useUsageStore } from "../stores/usageStore";
import { useTargetStore } from "../stores/targetStore";

const wrap = (ui: React.ReactNode) => <MemoryRouter>{ui}</MemoryRouter>;

beforeEach(() => {
  // reset zustand state for SkillBarChart's resolveSkillId selector
  useUsageStore.setState({
    overview: null,
    recent: [],
    providers: [],
    detail: null,
    scope: null,
    loading: false,
    refreshing: false,
    error: null,
    lastRefreshMs: null,
    resolveSkillId: vi.fn(async () => null),
  });
  useTargetStore.setState({
    targets: [{ id: "local", kind: "local", label: "Local", isActive: true }],
    activeTarget: { id: "local", kind: "local", label: "Local", isActive: true },
  });
});

describe("KpiStrip", () => {
  it("renders all 4 KPI cards with localized values", () => {
    render(
      wrap(
        <KpiStrip
          kpis={{
            totalCalls: 1234,
            uniqueSkills: 7,
            uniqueProjects: 3,
            uniqueSources: 2,
          }}
        />
      )
    );

    // toLocaleString → 1,234 (依 jsdom 默认 locale)
    expect(screen.getByText(/1,234|1234/)).toBeInTheDocument();
    expect(screen.getByText("7")).toBeInTheDocument();
    expect(screen.getByText("3")).toBeInTheDocument();
    expect(screen.getByText("2")).toBeInTheDocument();
  });
});

describe("ProviderHealthList", () => {
  it("renders one row per provider with available state", () => {
    render(
      wrap(
        <ProviderHealthList
          providers={[
            {
              providerId: "claude-code",
              displayName: "Claude Code",
              available: true,
              callCount: 42,
              scannedAtMs: Date.now() - 1000 * 60 * 3,
            },
            {
              providerId: "antigravity",
              displayName: "Antigravity",
              available: false,
              callCount: 0,
              scannedAtMs: 0,
            },
          ]}
        />
      )
    );

    expect(screen.getByTestId("provider-row-claude-code")).toBeInTheDocument();
    expect(screen.getByText("Claude Code")).toBeInTheDocument();
    expect(screen.getByText("42")).toBeInTheDocument();

    const antigravityRow = screen.getByTestId("provider-row-antigravity");
    expect(antigravityRow).toBeInTheDocument();
    // 未检测到的行不应当显示数字 callCount，应当出现 i18n notDetected 文案
    // (中文 "未检测到" 或英文 "not detected"，取决于 i18n init)
    expect(antigravityRow.textContent).toMatch(/未检测到|not detected/);
  });

  it("shows empty placeholder when providers list is empty", () => {
    render(wrap(<ProviderHealthList providers={[]} />));
    // i18n placeholder
    expect(document.body.textContent).toMatch(/暂无数据|No data yet/);
  });
});

describe("SkillBarChart", () => {
  const skills = [
    { skill: "git-commit", count: 10, projects: 2, sessions: 5, lastUsedMs: 3000 },
    { skill: "review", count: 25, projects: 1, sessions: 8, lastUsedMs: 2000 },
    { skill: "facts", count: 5, projects: 3, sessions: 2, lastUsedMs: 5000 },
  ];

  it("renders one row per skill, default sort by count desc", () => {
    render(wrap(<SkillBarChart skills={skills} />));
    const rows = screen.getAllByTestId(/^bar-row-/);
    // count desc: review(25), git-commit(10), facts(5)
    expect(rows[0]).toHaveAttribute("data-testid", "bar-row-review");
    expect(rows[1]).toHaveAttribute("data-testid", "bar-row-git-commit");
    expect(rows[2]).toHaveAttribute("data-testid", "bar-row-facts");
  });

  it("cycles through count → alpha → recent on sort button click", () => {
    render(wrap(<SkillBarChart skills={skills} />));
    const cycle = screen.getByTestId("sort-cycle");

    fireEvent.click(cycle); // → alpha (asc default)
    let rows = screen.getAllByTestId(/^bar-row-/);
    // alpha asc: facts, git-commit, review
    expect(rows[0]).toHaveAttribute("data-testid", "bar-row-facts");
    expect(rows[1]).toHaveAttribute("data-testid", "bar-row-git-commit");
    expect(rows[2]).toHaveAttribute("data-testid", "bar-row-review");

    fireEvent.click(cycle); // → recent (desc default)
    rows = screen.getAllByTestId(/^bar-row-/);
    // recent desc: facts(5000), git-commit(3000), review(2000)
    expect(rows[0]).toHaveAttribute("data-testid", "bar-row-facts");
    expect(rows[1]).toHaveAttribute("data-testid", "bar-row-git-commit");
    expect(rows[2]).toHaveAttribute("data-testid", "bar-row-review");
  });

  it("invokes onSkillClick callback when resolveSkillId returns null", async () => {
    const onSkillClick = vi.fn();
    render(wrap(<SkillBarChart skills={skills} onSkillClick={onSkillClick} />));

    fireEvent.click(screen.getByTestId("bar-row-review"));
    await waitFor(() => expect(onSkillClick).toHaveBeenCalledWith("review"));
  });

  it("navigates to /skill/:id when resolveSkillId returns a skill id", async () => {
    useUsageStore.setState({ resolveSkillId: vi.fn(async () => "abc123") });

    function Probe() {
      const loc = useLocation();
      return <div data-testid="loc">{loc.pathname}</div>;
    }

    render(
      <MemoryRouter initialEntries={["/usage"]}>
        <Routes>
          <Route path="/usage" element={<SkillBarChart skills={skills} />} />
          <Route path="/skill/:id" element={<Probe />} />
        </Routes>
      </MemoryRouter>
    );

    fireEvent.click(screen.getByTestId("bar-row-review"));
    await waitFor(() =>
      expect(screen.getByTestId("loc")).toHaveTextContent("/skill/abc123")
    );
  });

  it("shows empty state when skills list is empty", () => {
    render(wrap(<SkillBarChart skills={[]} />));
    expect(document.body.textContent).toMatch(/尚未发现|No skill invocations/);
  });
});

describe("ActivityHeatmap", () => {
  function buildDays(counts: number[]): Array<{ date: string; count: number }> {
    return counts.map((c, i) => {
      // 用 epoch + i 天保证 date 唯一，避免重复 key 警告
      const ms = Date.UTC(2024, 0, 1) + i * 86400000;
      return {
        date: new Date(ms).toISOString().slice(0, 10),
        count: c,
      };
    });
  }

  it("renders 112 cells for a full 16w x 7d window", () => {
    const days = buildDays(new Array(16 * 7).fill(0));
    render(<ActivityHeatmap days={days} />);
    const grid = screen.getByTestId("heatmap-grid");
    const cells = grid.querySelectorAll("[data-testid^='heatmap-cell-']");
    expect(grid).toHaveClass("auto-cols-fr");
    expect(cells.length).toBe(112);
    expect(cells[0]).toHaveClass("aspect-square");
  });

  it("assigns level 0 to zero-count days and level 4 to max-count days", () => {
    const days = buildDays([0, 1, 2, 3, 10]); // max=10
    render(<ActivityHeatmap days={days} />);

    const cellLevels = days.map((d) => {
      const cell = screen.getByTestId(`heatmap-cell-${d.date}`);
      return cell.getAttribute("data-level");
    });
    expect(cellLevels[0]).toBe("0");
    // 10 是 max 应当是 lvl 4
    expect(cellLevels[4]).toBe("4");
  });

  it("renders empty placeholder when days array is empty", () => {
    render(<ActivityHeatmap days={[]} />);
    expect(screen.queryByTestId("heatmap-grid")).not.toBeInTheDocument();
  });
});

describe("RecentCallsFeed", () => {
  const calls = [
    {
      skill: "review",
      timestampMs: Date.now() - 60_000,
      project: "/home/me/repo-a",
      sessionId: "s1",
      source: "Claude Code",
    },
    {
      skill: "git-commit",
      timestampMs: Date.now() - 3_600_000,
      project: "/home/me/repo-b",
      sessionId: "s2",
      source: "Codex CLI",
    },
  ];

  it("renders one row per call with project short name + source pill", () => {
    render(wrap(<RecentCallsFeed calls={calls} />));
    expect(screen.getByText("review")).toBeInTheDocument();
    expect(screen.getByText("git-commit")).toBeInTheDocument();
    expect(screen.getByText("repo-a")).toBeInTheDocument();
    expect(screen.getByText("repo-b")).toBeInTheDocument();
    expect(screen.getByText("Claude Code")).toBeInTheDocument();
    expect(screen.getByText("Codex CLI")).toBeInTheDocument();
  });

  it("respects the limit prop", () => {
    render(wrap(<RecentCallsFeed calls={calls} limit={1} />));
    expect(screen.getByText("review")).toBeInTheDocument();
    expect(screen.queryByText("git-commit")).not.toBeInTheDocument();
  });

  it("shows empty placeholder when no calls", () => {
    render(wrap(<RecentCallsFeed calls={[]} />));
    expect(document.body.textContent).toMatch(/尚未发现|No skill invocations/);
  });
});

describe("SkillUsageView ScopeBadge", () => {
  // ScopeBadge 是 SkillUsageView 内部的 helper；测试通过 store 设置 scope
  // 后渲染整个 view 验证 badge data-scope-kind 属性。
  it("renders Local badge when scope.isRemote=false", async () => {
    useUsageStore.setState({
      scope: { targetId: "local", label: "Local", isRemote: false, remoteReachable: false },
      lastRefreshMs: Date.now(),
      providers: [],
      recent: [],
      overview: {
        kpis: { totalCalls: 0, uniqueSkills: 0, uniqueProjects: 0, uniqueSources: 0 },
        topSkills: [],
        heatmap: [],
        lastScanMs: null,
      },
    });
    const { SkillUsageView } = await import("../pages/SkillUsageView");
    render(wrap(<SkillUsageView />));
    const badge = screen.getByTestId("scope-badge");
    expect(badge).toHaveAttribute("data-scope-kind", "local");
  });

  it("renders Remote badge with host label and warning banner when unreachable", async () => {
    useTargetStore.setState({
      targets: [
        { id: "local", kind: "local", label: "Local", isActive: false },
        { id: "ssh-prod", kind: "ssh", label: "alice@prod", isActive: true },
      ],
      activeTarget: {
        id: "ssh-prod",
        kind: "ssh",
        label: "alice@prod",
        isActive: true,
      },
    });
    useUsageStore.setState({
      scope: {
        targetId: "ssh-prod",
        label: "alice@prod",
        isRemote: true,
        remoteReachable: false,
      },
      lastRefreshMs: Date.now(),
      providers: [],
      recent: [],
      overview: {
        kpis: { totalCalls: 9, uniqueSkills: 2, uniqueProjects: 1, uniqueSources: 1 },
        topSkills: [
          {
            skill: "review",
            count: 9,
            projects: 1,
            sessions: 2,
            lastUsedMs: Date.now(),
          },
        ],
        heatmap: [],
        lastScanMs: null,
      },
    });
    const { SkillUsageView } = await import("../pages/SkillUsageView");
    render(wrap(<SkillUsageView />));
    const badge = screen.getByTestId("scope-badge");
    expect(badge).toHaveAttribute("data-scope-kind", "remote");
    expect(badge.textContent).toMatch(/alice@prod/);
    expect(screen.getByTestId("remote-unreachable-banner")).toBeInTheDocument();
    expect(screen.getByText("review")).toBeInTheDocument();
    expect(document.body.textContent).toMatch(/9/);
  });
});
