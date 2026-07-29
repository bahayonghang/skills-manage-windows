import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, within } from "@testing-library/react";
import { MemoryRouter, Route, Routes, useLocation } from "react-router-dom";

import { ActivityHeatmap } from "@/components/usage/ActivityHeatmap";
import { PlatformFilterBar } from "@/components/usage/PlatformFilterBar";
import { ProviderHealthList } from "@/components/usage/ProviderHealthList";
import { RecentCallsFeed } from "@/components/usage/RecentCallsFeed";
import { SkillUsageDetailPanel } from "@/components/usage/SkillUsageDetailPanel";
import { SkillUsageTable } from "@/components/usage/SkillUsageTable";
import { UsageMetricStrip } from "@/components/usage/UsageMetricStrip";
import { useTargetStore } from "@/stores/targetStore";
import { useUsageStore } from "@/stores/usageStore";
import type {
  DayCount,
  RecentSkillCall,
  SkillUsageDetail,
  SkillUsageSummary,
} from "@/types/usage";

const wrap = (ui: React.ReactNode) => <MemoryRouter>{ui}</MemoryRouter>;

const initialUsageActions = {
  refresh: useUsageStore.getState().refresh,
  subscribeTargetChanged: useUsageStore.getState().subscribeTargetChanged,
};

const skills: SkillUsageSummary[] = [
  {
    skill: "git-commit",
    count: 10,
    projects: 2,
    sessions: 5,
    lastUsedMs: 3_000,
    matchStatus: "matched",
    resolvedSkillId: "git-commit",
    staticTokenEstimate: 420,
    staticByteCount: 1_600,
  },
  {
    skill: "review",
    count: 25,
    projects: 1,
    sessions: 8,
    lastUsedMs: 2_000,
    matchStatus: "ambiguous",
    resolvedSkillId: null,
    staticTokenEstimate: null,
    staticByteCount: null,
  },
  {
    skill: "facts",
    count: 5,
    projects: 3,
    sessions: 2,
    lastUsedMs: 5_000,
    matchStatus: "unmatched",
    resolvedSkillId: null,
    staticTokenEstimate: null,
    staticByteCount: null,
  },
];

const recent: RecentSkillCall[] = [
  {
    skill: "git-commit",
    timestampMs: Date.now() - 60_000,
    project: "C:/Users/demo/repo-a",
    sessionId: "s1",
    source: "Codex CLI",
    matchStatus: "matched",
    resolvedSkillId: "git-commit",
  },
  {
    skill: "review",
    timestampMs: Date.now() - 3_600_000,
    project: "/home/demo/repo-b",
    sessionId: "s2",
    source: "Claude Code",
    matchStatus: "ambiguous",
    resolvedSkillId: null,
  },
];

beforeEach(() => {
  useUsageStore.setState({
    overview: null,
    recent: [],
    providers: [],
    detail: null,
    scope: null,
    selectedSource: null,
    selectedSkill: null,
    loading: false,
    refreshing: false,
    detailLoading: false,
    error: null,
    refreshError: null,
    usedCachedData: false,
    lastRefreshMs: null,
    ...initialUsageActions,
  });
  useTargetStore.setState({
    targets: [{ id: "local", kind: "local", label: "Local", isActive: true }],
    activeTarget: {
      id: "local",
      kind: "local",
      label: "Local",
      isActive: true,
    },
  });
});

describe("UsageMetricStrip", () => {
  it("renders a compact four-metric strip with an explicit all-history range", () => {
    render(
      wrap(
        <UsageMetricStrip
          kpis={{
            totalCalls: 1_234,
            uniqueSkills: 7,
            uniqueProjects: 3,
            uniqueSources: 2,
            uniqueSessions: 5,
          }}
        />,
      ),
    );

    expect(screen.getByText(/1,234|1234/)).toBeInTheDocument();
    expect(
      screen.getByRole("region", { name: /全部已记录|All recorded/ }),
    ).toBeInTheDocument();
    expect(document.querySelector("[class*='gradient']")).toBeNull();
  });
});

describe("platform and provider controls", () => {
  const providers = [
    {
      providerId: "claude-code",
      displayName: "Claude Code",
      available: true,
      callCount: 42,
      scannedAtMs: Date.now(),
    },
    {
      providerId: "antigravity",
      displayName: "Antigravity",
      available: false,
      callCount: 0,
      scannedAtMs: 0,
    },
  ];

  it("disables sources without calls and keeps provider zero distinct from unavailable", () => {
    const onSelect = vi.fn();
    render(
      wrap(
        <>
          <PlatformFilterBar
            providers={providers}
            selected={null}
            onSelect={onSelect}
          />
          <ProviderHealthList providers={providers} />
        </>,
      ),
    );

    expect(screen.getByTestId("platform-pill-claude-code")).toBeEnabled();
    expect(screen.getByTestId("platform-pill-antigravity")).toBeDisabled();
    expect(screen.getByTestId("provider-row-antigravity").textContent).toMatch(
      /未检测到|not detected/,
    );
  });
});

describe("SkillUsageTable", () => {
  it("shows stable decision columns and defaults to count descending", () => {
    render(
      wrap(
        <SkillUsageTable
          skills={skills}
          selectedSkill={null}
          onSelect={vi.fn()}
        />,
      ),
    );
    const rows = screen.getAllByTestId(/^usage-row-/);
    expect(rows[0]).toHaveAttribute("data-testid", "usage-row-review");
    expect(rows[1]).toHaveAttribute("data-testid", "usage-row-git-commit");
    expect(document.body.textContent).toMatch(
      /无法唯一映射|Not uniquely mapped/,
    );
    expect(document.body.textContent).toMatch(/420/);
  });

  it("uses explicit sort options and row selection for usage detail", () => {
    const onSelect = vi.fn();
    render(
      wrap(
        <SkillUsageTable
          skills={skills}
          selectedSkill="review"
          onSelect={onSelect}
        />,
      ),
    );

    fireEvent.click(screen.getByRole("button", { name: /按名称|name/i }));
    expect(screen.getAllByTestId(/^usage-row-/)[0]).toHaveAttribute(
      "data-testid",
      "usage-row-facts",
    );
    const reviewButton = screen
      .getByTestId("usage-row-review")
      .querySelector("button")!;
    fireEvent.click(reviewButton);
    expect(onSelect).toHaveBeenCalledWith("review", reviewButton);
    expect(reviewButton).toHaveAttribute("aria-pressed", "true");
  });

  it("exposes navigation only for uniquely matched rows", () => {
    function Probe() {
      return <span data-testid="location">{useLocation().pathname}</span>;
    }
    render(
      <MemoryRouter initialEntries={["/usage"]}>
        <Routes>
          <Route
            path="/usage"
            element={<SkillUsageTable skills={skills} onSelect={vi.fn()} />}
          />
          <Route path="/skill/:id" element={<Probe />} />
        </Routes>
      </MemoryRouter>,
    );

    expect(
      screen.getAllByRole("button", { name: /打开技能|Open skill/ }),
    ).toHaveLength(1);
    fireEvent.click(
      screen.getByRole("button", {
        name: /打开技能 git-commit|Open skill git-commit/,
      }),
    );
    expect(screen.getByTestId("location")).toHaveTextContent(
      "/skill/git-commit",
    );
  });

  it("filters installed and unlinked rows locally and updates the count", () => {
    render(wrap(<SkillUsageTable skills={skills} onSelect={vi.fn()} />));

    const filterGroup = screen.getByRole("group", {
      name: /按安装状态筛选|Filter by install state/,
    });
    const installed = within(filterGroup).getByRole("button", {
      name: /已安装|Installed/,
    });
    const unlinked = within(filterGroup).getByRole("button", {
      name: /未关联|Unlinked/,
    });
    const all = within(filterGroup).getByRole("button", {
      name: /^全部$|^All$/,
    });

    fireEvent.click(installed);
    expect(screen.getByTestId("usage-row-git-commit")).toBeInTheDocument();
    expect(screen.queryByTestId("usage-row-review")).not.toBeInTheDocument();
    expect(screen.queryByTestId("usage-row-facts")).not.toBeInTheDocument();
    expect(document.body.textContent).toMatch(/1\s*\/\s*3/);

    fireEvent.click(unlinked);
    expect(screen.queryByTestId("usage-row-git-commit")).not.toBeInTheDocument();
    expect(screen.getByTestId("usage-row-review")).toBeInTheDocument();
    expect(screen.getByTestId("usage-row-facts")).toBeInTheDocument();
    expect(document.body.textContent).toMatch(/2\s*\/\s*3/);

    fireEvent.click(all);
    expect(screen.getAllByTestId(/^usage-row-/)).toHaveLength(3);
  });

  it("shows a distinct empty state when the install filter has no matches", () => {
    render(
      wrap(<SkillUsageTable skills={[skills[2]]} onSelect={vi.fn()} />),
    );

    const filterGroup = screen.getByRole("group", {
      name: /按安装状态筛选|Filter by install state/,
    });
    fireEvent.click(
      within(filterGroup).getByRole("button", {
        name: /已安装|Installed/,
      }),
    );

    expect(
      screen.getByText(/没有符合筛选条件的技能|No skills match this filter/),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/切回.*全部|Switch back to.*All/i),
    ).toBeInTheDocument();
    expect(screen.queryByTestId("usage-row-facts")).not.toBeInTheDocument();
  });

  it("keeps match text alongside semantic status dots", () => {
    render(wrap(<SkillUsageTable skills={skills} onSelect={vi.fn()} />));

    expect(
      screen
        .getByTestId("usage-row-git-commit")
        .querySelector('[class~="bg-success"]'),
    ).not.toBeNull();
    expect(
      screen
        .getByTestId("usage-row-review")
        .querySelector('[class~="bg-warning"]'),
    ).not.toBeNull();
    expect(
      screen
        .getByTestId("usage-row-facts")
        .querySelector('[class~="bg-muted-foreground/40"]'),
    ).not.toBeNull();
    expect(document.body.textContent).toMatch(/已匹配|Matched/);
    expect(document.body.textContent).toMatch(/无法唯一映射|Not uniquely mapped/);
    expect(document.body.textContent).toMatch(/未映射|Unmapped/);
  });
});

describe("ActivityHeatmap", () => {
  function buildDays(counts: number[]): DayCount[] {
    return counts.map((count, index) => ({
      date: new Date(Date.UTC(2024, 0, 1) + index * 86_400_000)
        .toISOString()
        .slice(0, 10),
      count,
    }));
  }

  it("renders 112 focusable cells, quantile levels, months and a legend", () => {
    const days = buildDays([1, 2, 3, 4, 5, 100, ...new Array(106).fill(0)]);
    render(<ActivityHeatmap days={days} />);

    const cells = screen.getAllByRole("gridcell");
    expect(cells).toHaveLength(112);
    expect(cells.filter((cell) => cell.tabIndex === 0)).toHaveLength(1);
    cells[0].focus();
    fireEvent.keyDown(cells[0], { key: "ArrowRight" });
    expect(cells[7]).toHaveFocus();
    expect(document.body.textContent).toMatch(/少|Less/);
    expect(document.body.textContent).toMatch(/多|More/);
    expect(cells[5]).toHaveAttribute("data-level", "5");
  });
});

describe("detail and recent actions", () => {
  const detail: SkillUsageDetail = {
    skill: "git-commit",
    count: 10,
    sessions: 5,
    firstUsedMs: Date.now() - 10 * 86_400_000,
    lastUsedMs: Date.now() - 60_000,
    byProject: [
      {
        project: "C:/Users/demo/repo-a",
        count: 10,
        sessions: 5,
        lastUsedMs: Date.now() - 60_000,
      },
    ],
    weekly: [],
    matchStatus: "matched",
    resolvedSkillId: "git-commit",
    staticTokenEstimate: 420,
    staticByteCount: 1_600,
  };

  it("selects recent rows without exposing full project paths", () => {
    const onSelect = vi.fn();
    render(wrap(<RecentCallsFeed calls={recent} onSelect={onSelect} />));
    expect(screen.getByText("repo-a")).toBeInTheDocument();
    expect(document.body.textContent).not.toContain("C:/Users/demo");
    fireEvent.click(screen.getByText("review"));
    expect(onSelect).toHaveBeenCalledWith(
      "review",
      expect.any(HTMLButtonElement),
    );
  });

  it("renders project session counts, static estimate and close action", () => {
    const onClose = vi.fn();
    render(
      wrap(
        <SkillUsageDetailPanel
          detail={detail}
          loading={false}
          onClose={onClose}
        />,
      ),
    );
    expect(screen.getByTestId("usage-detail-panel")).toHaveTextContent(
      "repo-a",
    );
    expect(screen.getByTestId("usage-detail-panel")).toHaveTextContent("420");
    fireEvent.click(screen.getByRole("button", { name: /关闭|Close usage/ }));
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});

describe("SkillUsageView", () => {
  it("shows a reduced-motion-safe scanning skeleton before refresh starts", async () => {
    useUsageStore.setState({
      overview: null,
      refreshing: false,
      refresh: vi.fn(async () => null),
      subscribeTargetChanged: vi.fn(async () => () => undefined),
    });
    const { SkillUsageView } = await import("@/pages/SkillUsageView");
    render(wrap(<SkillUsageView />));

    const status = screen.getByRole("status");
    expect(status).toHaveTextContent(
      /正在扫描各平台会话日志|Scanning session logs across platforms/,
    );
    expect(status.querySelector("svg")).toHaveClass(
      "motion-reduce:animate-none",
    );
    expect(screen.queryByText(/调用次数|Calls/)).not.toBeInTheDocument();
  });

  it("exits the scanning skeleton when an uncached refresh fails", async () => {
    useUsageStore.setState({
      overview: null,
      refreshing: false,
      error: "scan failed",
      refresh: vi.fn(async () => null),
      subscribeTargetChanged: vi.fn(async () => () => undefined),
    });
    const { SkillUsageView } = await import("@/pages/SkillUsageView");
    render(wrap(<SkillUsageView />));

    expect(screen.getByRole("alert")).toHaveTextContent("scan failed");
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });

  it("shows remote cached state and keeps provider health secondary", async () => {
    useTargetStore.setState({
      targets: [
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
      overview: {
        kpis: {
          totalCalls: 40,
          uniqueSkills: 3,
          uniqueProjects: 2,
          uniqueSources: 2,
          uniqueSessions: 12,
        },
        topSkills: skills,
        heatmap: [],
        lastScanMs: Date.now(),
      },
      recent,
      providers: [],
      scope: {
        targetId: "ssh-prod",
        label: "alice@prod",
        isRemote: true,
        remoteReachable: false,
      },
      lastRefreshMs: Date.now(),
      usedCachedData: true,
      refreshError: "timeout",
    });
    const { SkillUsageView } = await import("@/pages/SkillUsageView");
    render(wrap(<SkillUsageView />));

    expect(screen.getByTestId("scope-badge")).toHaveAttribute(
      "data-scope-kind",
      "remote",
    );
    expect(screen.getByTestId("remote-unreachable-banner").textContent).toMatch(
      /上次|last successful/i,
    );
    expect(
      screen.getByText(/数据源状态|Provider health/).closest("details"),
    ).not.toHaveAttribute("open");
  });
});
