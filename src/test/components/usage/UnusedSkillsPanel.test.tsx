import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, within } from "@testing-library/react";
import { MemoryRouter, Route, Routes, useLocation } from "react-router-dom";

import { UnusedSkillsPanel } from "@/components/usage/UnusedSkillsPanel";
import type { UnusedSkillsReport } from "@/types/usage";

const wrap = (ui: React.ReactNode) => <MemoryRouter>{ui}</MemoryRouter>;

const DAY_MS = 86_400_000;
const now = Date.now();

function buildReport(): UnusedSkillsReport {
  return {
    central: [
      {
        skillId: "legacy-cleanup",
        name: "legacy-cleanup",
        matchStatus: "matched",
        origin: "central",
        agents: ["claude-code"],
        installedPath: "C:/central/legacy-cleanup",
        callCount: 0,
        lastUsedMs: null,
        staticTokenEstimate: 960,
        staticByteCount: 3_840,
        status: "never_used",
      },
      {
        skillId: "trellis-check",
        name: "trellis-check",
        matchStatus: "matched",
        origin: "central",
        agents: ["claude-code", "codex"],
        installedPath: "C:/central/trellis-check",
        callCount: 17,
        lastUsedMs: now - 45 * DAY_MS,
        staticTokenEstimate: null,
        staticByteCount: null,
        status: "stale",
      },
    ],
    platforms: [
      {
        skillId: null,
        name: "prompt-helper",
        matchStatus: "ambiguous",
        origin: "platform",
        agents: ["codex"],
        installedPath: "C:/agents/codex/prompt-helper",
        callCount: 3,
        lastUsedMs: now - 120 * DAY_MS,
        staticTokenEstimate: null,
        staticByteCount: null,
        status: "stale",
      },
      {
        skillId: null,
        name: "local-notes",
        matchStatus: "unmatched",
        origin: "platform",
        agents: ["claude-code", "zed"],
        installedPath: "C:/agents/claude/local-notes",
        callCount: 0,
        lastUsedMs: null,
        staticTokenEstimate: null,
        staticByteCount: null,
        status: "never_used",
      },
    ],
  };
}

describe("UnusedSkillsPanel", () => {
  it("groups central entries and sections platform entries per agent", () => {
    render(
      wrap(<UnusedSkillsPanel report={buildReport()} onSelect={vi.fn()} />),
    );

    expect(screen.getByText(/中央技能库|Central library/)).toBeInTheDocument();
    expect(screen.getByText(/平台 · codex|Platform · codex/)).toBeInTheDocument();
    // local-notes 装在两个平台 → 两个 agent 小节各出现一次
    expect(
      screen.getAllByTestId("unused-row-platform-local-notes"),
    ).toHaveLength(2);
    expect(
      screen.getByTestId("unused-row-central-legacy-cleanup"),
    ).toBeInTheDocument();
    // 默认 90 天阈值：45 天未用的 trellis-check 不满足
    expect(
      screen.queryByTestId("unused-row-central-trellis-check"),
    ).not.toBeInTheDocument();
  });

  it("keeps match status readable as text with semantic dots", () => {
    render(
      wrap(<UnusedSkillsPanel report={buildReport()} onSelect={vi.fn()} />),
    );

    expect(document.body.textContent).toMatch(/已匹配|Matched/);
    expect(document.body.textContent).toMatch(/无法唯一映射|Not uniquely mapped/);
    expect(document.body.textContent).toMatch(/未映射|Unmapped/);
    expect(
      screen
        .getByTestId("unused-row-central-legacy-cleanup")
        .querySelector('[class~="bg-success"]'),
    ).not.toBeNull();
    expect(
      screen
        .getByTestId("unused-row-platform-prompt-helper")
        .querySelector('[class~="bg-warning"]'),
    ).not.toBeNull();
    expect(
      screen
        .getAllByTestId("unused-row-platform-local-notes")[0]
        .querySelector('[class~="bg-muted-foreground/40"]'),
    ).not.toBeNull();
  });

  it("reclassifies staleness locally when the threshold chip changes", () => {
    render(
      wrap(<UnusedSkillsPanel report={buildReport()} onSelect={vi.fn()} />),
    );

    const thresholdGroup = screen.getByRole("group", {
      name: /长期未用阈值|Staleness threshold/,
    });
    // 30 天：trellis-check（45 天未用）进入列表
    fireEvent.click(
      within(thresholdGroup).getByRole("button", { name: /30 天|30d/ }),
    );
    expect(
      screen.getByTestId("unused-row-central-trellis-check"),
    ).toBeInTheDocument();
    expect(document.body.textContent).toMatch(/长期未用|Stale/);

    // 回到 90 天：再次排除
    fireEvent.click(
      within(thresholdGroup).getByRole("button", { name: /90 天|90d/ }),
    );
    expect(
      screen.queryByTestId("unused-row-central-trellis-check"),
    ).not.toBeInTheDocument();
  });

  it("filters never_used and stale entries locally without new requests", () => {
    render(
      wrap(<UnusedSkillsPanel report={buildReport()} onSelect={vi.fn()} />),
    );

    const filterGroup = screen.getByRole("group", {
      name: /按未使用状态筛选|Filter by unused state/,
    });
    fireEvent.click(
      within(filterGroup).getByRole("button", { name: /从未使用|Never used/ }),
    );
    expect(
      screen.getByTestId("unused-row-central-legacy-cleanup"),
    ).toBeInTheDocument();
    expect(
      screen.queryByTestId("unused-row-platform-prompt-helper"),
    ).not.toBeInTheDocument();
    expect(document.body.textContent).toMatch(/2\s*\/\s*3/);

    fireEvent.click(
      within(filterGroup).getByRole("button", { name: /长期未用|Stale/ }),
    );
    expect(
      screen.queryByTestId("unused-row-central-legacy-cleanup"),
    ).not.toBeInTheDocument();
    expect(
      screen.getByTestId("unused-row-platform-prompt-helper"),
    ).toBeInTheDocument();
  });

  it("sorts by size with missing estimates last, and alphabetically", () => {
    render(
      wrap(<UnusedSkillsPanel report={buildReport()} onSelect={vi.fn()} />),
    );
    const sortGroup = screen.getByRole("group", {
      name: /未使用技能排序|Sort unused skills/,
    });
    const thresholdGroup = screen.getByRole("group", {
      name: /长期未用阈值|Staleness threshold/,
    });
    // 30 天阈值让 trellis-check（无估算）与 legacy-cleanup（3840B）同区对比
    fireEvent.click(
      within(thresholdGroup).getByRole("button", { name: /30 天|30d/ }),
    );

    fireEvent.click(
      within(sortGroup).getByRole("button", { name: /按体积|largest/ }),
    );
    const bySize = screen.getAllByTestId(/^unused-row-central-/);
    expect(bySize[0]).toHaveAttribute(
      "data-testid",
      "unused-row-central-legacy-cleanup",
    );
    expect(bySize[1]).toHaveAttribute(
      "data-testid",
      "unused-row-central-trellis-check",
    );

    fireEvent.click(
      within(sortGroup).getByRole("button", { name: /按名称|name/ }),
    );
    const byName = screen.getAllByTestId(/^unused-row-central-/);
    expect(byName[0]).toHaveAttribute(
      "data-testid",
      "unused-row-central-legacy-cleanup",
    );
    expect(byName[1]).toHaveAttribute(
      "data-testid",
      "unused-row-central-trellis-check",
    );
  });

  it("renders unavailable static estimates as a dash, never as zero", () => {
    render(
      wrap(<UnusedSkillsPanel report={buildReport()} onSelect={vi.fn()} />),
    );
    const thresholdGroup = screen.getByRole("group", {
      name: /长期未用阈值|Staleness threshold/,
    });
    fireEvent.click(
      within(thresholdGroup).getByRole("button", { name: /30 天|30d/ }),
    );

    const row = screen.getByTestId("unused-row-central-trellis-check");
    const estimateCell = row.querySelector("[title]");
    expect(estimateCell?.getAttribute("title")).toMatch(
      /不可用|unavailable/i,
    );
    expect(estimateCell?.textContent).toBe("—");
    expect(estimateCell?.textContent).not.toBe("0");
  });

  it("selects rows by skill name and gates open-skill on a non-null skillId", () => {
    function Probe() {
      return <span data-testid="location">{useLocation().pathname}</span>;
    }
    const onSelect = vi.fn();
    render(
      <MemoryRouter initialEntries={["/usage"]}>
        <Routes>
          <Route
            path="/usage"
            element={
              <UnusedSkillsPanel report={buildReport()} onSelect={onSelect} />
            }
          />
          <Route path="/skill/:id" element={<Probe />} />
        </Routes>
      </MemoryRouter>,
    );

    const row = screen.getByTestId("unused-row-central-legacy-cleanup");
    const selectButton = row.querySelector("button")!;
    fireEvent.click(selectButton);
    expect(onSelect).toHaveBeenCalledWith("legacy-cleanup", selectButton);

    // 只有 skillId 非空的条目有打开按钮（4 行中仅 legacy-cleanup / trellis-check）
    const openButtons = screen.getAllByRole("button", {
      name: /打开技能|Open skill/,
    });
    expect(openButtons).toHaveLength(1);
    fireEvent.click(openButtons[0]);
    expect(screen.getByTestId("location")).toHaveTextContent(
      "/skill/legacy-cleanup",
    );
  });

  it("shows a stable skeleton while the first unused request is in flight", () => {
    render(
      wrap(
        <UnusedSkillsPanel report={null} loading onSelect={vi.fn()} />,
      ),
    );
    expect(
      screen.getByText(/正在扫描已安装技能|Scanning installed skills/),
    ).toBeInTheDocument();
  });

  it("shows a distinct empty state when nothing qualifies at the threshold", () => {
    const report: UnusedSkillsReport = {
      central: [
        {
          skillId: "fresh",
          name: "fresh",
          matchStatus: "matched",
          origin: "central",
          agents: ["codex"],
          installedPath: null,
          callCount: 2,
          lastUsedMs: now - 5 * DAY_MS,
          staticTokenEstimate: 100,
          staticByteCount: 400,
          status: "stale",
        },
      ],
      platforms: [],
    };
    render(wrap(<UnusedSkillsPanel report={report} onSelect={vi.fn()} />));

    expect(
      screen.getByText(/当前阈值下没有未使用技能|No unused skills at this threshold/),
    ).toBeInTheDocument();
  });
});
