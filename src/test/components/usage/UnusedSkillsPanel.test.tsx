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
        agents: [
          {
            agentId: "claude-code",
            linkType: "symlink",
            installedPath: "C:/agents/claude/legacy-cleanup",
            hasPendingRecovery: false,
          },
        ],
        installs: [],
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
        agents: [
          {
            agentId: "claude-code",
            linkType: "symlink",
            installedPath: "C:/agents/claude/trellis-check",
            hasPendingRecovery: false,
          },
          {
            agentId: "codex",
            linkType: "junction",
            installedPath: "C:/agents/codex/trellis-check",
            hasPendingRecovery: false,
          },
        ],
        installs: [],
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
        agents: [],
        installs: [
          {
            agentId: "codex",
            rowId: "obs-codex-prompt-helper",
            skillId: "obs-codex-prompt-helper",
            linkType: "native",
            sourceKind: "user",
            isReadOnly: false,
            installedPath: "C:/agents/codex/prompt-helper",
            hasPendingRecovery: false,
          },
        ],
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
        agents: [],
        installs: [
          {
            agentId: "claude-code",
            rowId: "obs-claude-local-notes",
            skillId: "obs-claude-local-notes",
            linkType: "native",
            sourceKind: "user",
            isReadOnly: false,
            installedPath: "C:/agents/claude/local-notes",
            hasPendingRecovery: false,
          },
          {
            agentId: "zed",
            rowId: "obs-zed-local-notes",
            skillId: "obs-zed-local-notes",
            linkType: "native",
            sourceKind: "user",
            isReadOnly: false,
            installedPath: "C:/agents/zed/local-notes",
            hasPendingRecovery: false,
          },
        ],
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
      wrap(
        <UnusedSkillsPanel
          report={buildReport()}
          onSelect={vi.fn()}
          onUnlink={vi.fn()}
        />,
      ),
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
      wrap(
        <UnusedSkillsPanel
          report={buildReport()}
          onSelect={vi.fn()}
          onUnlink={vi.fn()}
        />,
      ),
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
      wrap(
        <UnusedSkillsPanel
          report={buildReport()}
          onSelect={vi.fn()}
          onUnlink={vi.fn()}
        />,
      ),
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
      wrap(
        <UnusedSkillsPanel
          report={buildReport()}
          onSelect={vi.fn()}
          onUnlink={vi.fn()}
        />,
      ),
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
      wrap(
        <UnusedSkillsPanel
          report={buildReport()}
          onSelect={vi.fn()}
          onUnlink={vi.fn()}
        />,
      ),
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
      wrap(
        <UnusedSkillsPanel
          report={buildReport()}
          onSelect={vi.fn()}
          onUnlink={vi.fn()}
        />,
      ),
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
              <UnusedSkillsPanel
                report={buildReport()}
                onSelect={onSelect}
                onUnlink={vi.fn()}
              />
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
        <UnusedSkillsPanel
          report={null}
          loading
          onSelect={vi.fn()}
          onUnlink={vi.fn()}
        />,
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
          agents: [
            {
              agentId: "codex",
              linkType: "junction",
              installedPath: "C:/agents/codex/fresh",
              hasPendingRecovery: false,
            },
          ],
          installs: [],
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
    render(
      wrap(
        <UnusedSkillsPanel
          report={report}
          onSelect={vi.fn()}
          onUnlink={vi.fn()}
        />,
      ),
    );

    expect(
      screen.getByText(/当前阈值下没有未使用技能|No unused skills at this threshold/),
    ).toBeInTheDocument();
  });

  it("requires a second click before unlinking a platform observation", () => {
    const onUnlink = vi.fn(async () => undefined);
    render(
      wrap(
        <UnusedSkillsPanel
          report={buildReport()}
          onSelect={vi.fn()}
          onUnlink={onUnlink}
        />,
      ),
    );

    fireEvent.click(
      screen.getByRole("button", {
        name: /仅从 codex 移除|Remove only from codex/i,
      }),
    );
    expect(onUnlink).not.toHaveBeenCalled();
    fireEvent.click(
      screen.getByRole("button", { name: /确认移除|Remove/i }),
    );

    expect(onUnlink).toHaveBeenCalledWith(
      "obs-codex-prompt-helper",
      "codex",
      "obs-codex-prompt-helper",
    );
  });

  it("prefers the writable observation when one agent has user and plugin copies", () => {
    const report = buildReport();
    report.platforms[0].installs.unshift({
      agentId: "codex",
      rowId: "obs-codex-plugin-prompt-helper",
      skillId: "obs-codex-prompt-helper",
      linkType: "native",
      sourceKind: "plugin",
      isReadOnly: true,
      installedPath: "C:/agents/codex/plugins/prompt-helper",
      hasPendingRecovery: false,
    });
    const onUnlink = vi.fn(async () => undefined);

    render(
      wrap(
        <UnusedSkillsPanel
          report={report}
          onSelect={vi.fn()}
          onUnlink={onUnlink}
        />,
      ),
    );

    expect(
      screen.getAllByTestId("unused-row-platform-prompt-helper"),
    ).toHaveLength(1);
    fireEvent.click(
      within(
        screen.getByTestId(
          "unlink-action-codex-obs-codex-prompt-helper",
        ),
      ).getByRole("button"),
    );
    fireEvent.click(
      screen.getByRole("button", { name: /确认移除|Remove/i }),
    );

    expect(onUnlink).toHaveBeenCalledWith(
      "obs-codex-prompt-helper",
      "codex",
      "obs-codex-prompt-helper",
    );
  });

  it("unlinks a Central installation by agent without deleting Central", () => {
    const onUnlink = vi.fn(async () => undefined);
    render(
      wrap(
        <UnusedSkillsPanel
          report={buildReport()}
          onSelect={vi.fn()}
          onUnlink={onUnlink}
        />,
      ),
    );

    fireEvent.click(
      within(screen.getByTestId("unlink-chip-claude-code")).getByRole("button", {
        name: /仅从 claude-code 移除|Remove only from claude-code/i,
      }),
    );
    fireEvent.click(
      screen.getByRole("button", { name: /确认移除|Remove/i }),
    );
    expect(onUnlink).toHaveBeenCalledWith(
      "legacy-cleanup",
      "claude-code",
    );
  });

  it("disables shared-root, pending-recovery, and read-only installs with reasons", () => {
    const report = buildReport();
    report.central[0].agents[0].hasPendingRecovery = true;
    report.central[0].agents.push({
      agentId: "codex",
      linkType: "native",
      installedPath: "C:/central/legacy-cleanup",
      hasPendingRecovery: false,
    });
    report.platforms[0].installs[0].isReadOnly = true;

    render(
      wrap(
        <UnusedSkillsPanel
          report={report}
          onSelect={vi.fn()}
          onUnlink={vi.fn()}
        />,
      ),
    );

    expect(
      screen.getByTestId("unlink-chip-disabled-claude-code"),
    ).toHaveAttribute(
      "title",
      expect.stringMatching(/待恢复|pending Central recovery/i),
    );
    expect(screen.getByTestId("unlink-chip-disabled-codex")).toHaveAttribute(
      "title",
      expect.stringMatching(/共用技能目录|shares the Central directory/i),
    );
    const readOnlyButton = within(
      screen.getByTestId(
        "unlink-action-codex-obs-codex-prompt-helper",
      ),
    ).getByRole("button");
    expect(readOnlyButton).toBeDisabled();
    expect(readOnlyButton).toHaveAttribute(
      "title",
      expect.stringMatching(/只读插件源|read-only plugin source/i),
    );
  });

  it("keeps state chips and agent chips untruncated and icon hit areas at 40px", () => {
    render(
      wrap(
        <UnusedSkillsPanel
          report={buildReport()}
          onSelect={vi.fn()}
          onUnlink={vi.fn()}
        />,
      ),
    );

    const row = screen.getByTestId("unused-row-central-legacy-cleanup");
    const stateChip = within(row).getByText(/从未使用|Never used/);
    expect(stateChip).toHaveClass("whitespace-nowrap");
    expect(stateChip).not.toHaveClass("truncate");
    expect(screen.getByTestId("unlink-chip-claude-code")).toHaveClass(
      "whitespace-nowrap",
    );
    expect(
      within(screen.getByTestId("unlink-chip-claude-code")).getByRole(
        "button",
        {
          name: /仅从 claude-code 移除|Remove only from claude-code/i,
        },
      ),
    ).toHaveClass("after:size-10");
  });
});
