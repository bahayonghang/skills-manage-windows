import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { MemoryRouter, Route, Routes, useLocation } from "react-router-dom";

import { UnusedSkillsPanel } from "@/components/usage/UnusedSkillsPanel";
import {
  centralTargets,
  platformTargets,
  platformUnlinkDisabledReason,
} from "@/components/usage/unusedUnlinkTargets";
import type { UnusedSkillEntry, UnusedSkillsReport } from "@/types/usage";

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
          onUnlinkAgents={vi.fn()}
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
          onUnlinkAgents={vi.fn()}
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
          onUnlinkAgents={vi.fn()}
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
          onUnlinkAgents={vi.fn()}
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
          onUnlinkAgents={vi.fn()}
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
          onUnlinkAgents={vi.fn()}
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
                onUnlinkAgents={vi.fn()}
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
          onUnlinkAgents={vi.fn()}
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
          onUnlinkAgents={vi.fn()}
        />,
      ),
    );

    expect(
      screen.getByText(/当前阈值下没有未使用技能|No unused skills at this threshold/),
    ).toBeInTheDocument();
  });

  it("renders one unlink trigger at the row right with a 40px hit area", () => {
    render(
      wrap(
        <UnusedSkillsPanel
          report={buildReport()}
          onSelect={vi.fn()}
          onUnlinkAgents={vi.fn()}
        />,
      ),
    );

    const row = screen.getByTestId("unused-row-central-legacy-cleanup");
    const trigger = within(row).getByTestId(
      "unused-unlink-trigger-central-legacy-cleanup",
    );
    expect(trigger).toHaveClass("after:size-10");
    // 行内不再出现任何内联 unlink 按钮（打开按钮之后只有 unlink 入口）
    expect(
      within(row).queryByTestId(/^unlink-chip-/),
    ).not.toBeInTheDocument();
    expect(
      within(row).queryByTestId(/^unlink-action-/),
    ).not.toBeInTheDocument();
    // Central 条目第二排改为弱化 agent 文本行，不含操作
    expect(row.textContent).toMatch(/claude-code/);
    expect(
      within(row).queryAllByRole("button", { name: /仅从|Remove only from/i }),
    ).toHaveLength(0);
  });

  it("disables the trigger with a title when every install is locked", () => {
    const report = buildReport();
    report.central.push({
      skillId: "locked-central",
      name: "locked-central",
      matchStatus: "matched",
      origin: "central",
      agents: [
        {
          agentId: "codex",
          linkType: "native",
          installedPath: "C:/central/locked-central",
          hasPendingRecovery: false,
        },
      ],
      installs: [],
      installedPath: "C:/central/locked-central",
      callCount: 0,
      lastUsedMs: null,
      staticTokenEstimate: null,
      staticByteCount: null,
      status: "never_used",
    });
    render(
      wrap(
        <UnusedSkillsPanel
          report={report}
          onSelect={vi.fn()}
          onUnlinkAgents={vi.fn()}
        />,
      ),
    );

    const trigger = screen.getByTestId(
      "unused-unlink-trigger-central-locked-central",
    );
    expect(trigger).toBeDisabled();
    expect(trigger).toHaveAttribute(
      "title",
      expect.stringMatching(/不可移除|locked/i),
    );
  });

  it("opens the dialog and normalizes Central targets from entry.agents", () => {
    render(
      wrap(
        <UnusedSkillsPanel
          report={buildReport()}
          onSelect={vi.fn()}
          onUnlinkAgents={vi.fn()}
        />,
      ),
    );
    fireEvent.click(
      screen.getByTestId("unused-unlink-trigger-central-legacy-cleanup"),
    );

    const dialog = screen.getByTestId("unused-unlink-dialog");
    expect(within(dialog).getByText(/legacy-cleanup/)).toBeInTheDocument();
    // Central 条目弹窗列出全部 agents，含中央库副本保留说明
    expect(
      within(dialog).getByTestId("unused-unlink-option-claude-code"),
    ).toBeInTheDocument();
    expect(within(dialog).getAllByLabelText("claude-code")).toHaveLength(2);
    expect(dialog.textContent).toMatch(/Central 库副本|Central library copy/);
  });

  it("lists every cross-agent platform install in the dialog with disabled reasons", () => {
    render(
      wrap(
        <UnusedSkillsPanel
          report={buildReport()}
          onSelect={vi.fn()}
          onUnlinkAgents={vi.fn()}
        />,
      ),
    );
    // 两个 agent 小节都指向同一散件，任一行触发都列出全部跨 Agent installs
    fireEvent.click(
      screen.getAllByTestId("unused-unlink-trigger-platform-local-notes")[0],
    );

    const dialog = screen.getByTestId("unused-unlink-dialog");
    expect(
      within(dialog).getByTestId("unused-unlink-option-claude-code"),
    ).toBeInTheDocument();
    expect(
      within(dialog).getByTestId("unused-unlink-option-zed"),
    ).toBeInTheDocument();
    expect(dialog.textContent).toMatch(/已列出安装此技能的全部 Agent|across all agents/i);
  });

  it("shows disabled reasons and keeps disabled targets out of select-all", async () => {
    const report = buildReport();
    // 给 legacy-cleanup 追加 shared-root + pending recovery 的 agent；保留一个可卸载项
    // 保证行右触发器可用，验证禁用项不计入全选
    report.central[0].agents[0].hasPendingRecovery = true;
    report.central[0].agents.push({
      agentId: "codex",
      linkType: "native",
      installedPath: "C:/central/legacy-cleanup",
      hasPendingRecovery: false,
    });
    report.central[0].agents.push({
      agentId: "zed",
      linkType: "junction",
      installedPath: "C:/agents/zed/legacy-cleanup",
      hasPendingRecovery: false,
    });
    const onUnlinkAgents = vi.fn(async () => []);
    render(
      wrap(
        <UnusedSkillsPanel
          report={report}
          onSelect={vi.fn()}
          onUnlinkAgents={onUnlinkAgents}
        />,
      ),
    );
    fireEvent.click(
      screen.getByTestId("unused-unlink-trigger-central-legacy-cleanup"),
    );

    const recoveryRow = screen.getByTestId(
      "unused-unlink-option-disabled-claude-code",
    );
    expect(recoveryRow).toHaveAttribute(
      "title",
      expect.stringMatching(/待恢复|pending Central recovery/i),
    );
    expect(
      within(recoveryRow).getByRole("checkbox"),
    ).toHaveAttribute("aria-disabled", "true");
    const sharedRootRow = screen.getByTestId(
      "unused-unlink-option-disabled-codex",
    );
    expect(sharedRootRow).toHaveAttribute(
      "title",
      expect.stringMatching(/共用技能目录|shares the Central directory/i),
    );
    expect(
      within(sharedRootRow).getByRole("checkbox"),
    ).toHaveAttribute("aria-disabled", "true");

    // 全选只勾选可卸载项：zed 被选中、禁用项不受影响
    fireEvent.click(
      within(screen.getByTestId("unused-unlink-select-all")).getByRole(
        "checkbox",
      ),
    );
    expect(
      within(recoveryRow).getByRole("checkbox"),
    ).not.toBeChecked();
    expect(
      within(sharedRootRow).getByRole("checkbox"),
    ).not.toBeChecked();
    const confirm = screen.getByTestId("unused-unlink-confirm");
    expect(confirm).toBeEnabled();
    fireEvent.click(confirm);
    await waitFor(() =>
      expect(onUnlinkAgents).toHaveBeenCalledWith([
        { skillId: "legacy-cleanup", agentId: "zed", rowId: null },
      ]),
    );
  });

  it("selects writable targets and calls the store batch action once on confirm", async () => {
    const onUnlinkAgents = vi.fn(async () => []);
    render(
      wrap(
        <UnusedSkillsPanel
          report={buildReport()}
          onSelect={vi.fn()}
          onUnlinkAgents={onUnlinkAgents}
        />,
      ),
    );
    fireEvent.click(
      screen.getByTestId("unused-unlink-trigger-central-legacy-cleanup"),
    );

    // 默认不勾选：确认按钮禁用
    expect(
      screen.getByRole("button", { name: /移除 \(0\)|Unlink \(0\)/ }),
    ).toBeDisabled();
    fireEvent.click(screen.getByRole("checkbox", { name: /claude-code/ }));
    // 选中计数随勾选更新
    expect(
      screen.getByRole("button", { name: /移除 \(1\)|Unlink \(1\)/ }),
    ).toBeEnabled();

    fireEvent.click(
      screen.getByRole("button", { name: /移除 \(1\)|Unlink \(1\)/ }),
    );
    await waitFor(() =>
      expect(onUnlinkAgents).toHaveBeenCalledWith([
        { skillId: "legacy-cleanup", agentId: "claude-code", rowId: null },
      ]),
    );
  });

  it("calls the store batch action with the full selected set after select-all", async () => {
    const onUnlinkAgents = vi.fn(async () => []);
    render(
      wrap(
        <UnusedSkillsPanel
          report={buildReport()}
          onSelect={vi.fn()}
          onUnlinkAgents={onUnlinkAgents}
        />,
      ),
    );
    // 30 天阈值下 trellis-check 可见，弹窗列出两个可卸载 agent
    fireEvent.click(
      within(
        screen.getByRole("group", {
          name: /长期未用阈值|Staleness threshold/,
        }),
      ).getByRole("button", { name: /30 天|30d/ }),
    );
    fireEvent.click(
      screen.getByTestId("unused-unlink-trigger-central-trellis-check"),
    );

    fireEvent.click(
      within(screen.getByTestId("unused-unlink-select-all")).getByRole(
        "checkbox",
      ),
    );
    const confirm = screen.getByRole("button", { name: /移除 \(2\)|Unlink \(2\)/ });
    fireEvent.click(confirm);
    await waitFor(() =>
      expect(onUnlinkAgents).toHaveBeenCalledWith([
        { skillId: "trellis-check", agentId: "claude-code", rowId: null },
        { skillId: "trellis-check", agentId: "codex", rowId: null },
      ]),
    );
  });

  it("keeps the dialog open on partial failure, shows the failed row, and resets selection for retry", async () => {
    const onUnlinkAgents = vi
      .fn()
      .mockResolvedValueOnce([
        {
          skillId: "trellis-check",
          agentId: "claude-code",
          rowId: null,
          ok: false,
          error: "installation.pending_central_recovery: blocked",
        },
        {
          skillId: "trellis-check",
          agentId: "codex",
          rowId: null,
          ok: true,
          error: null,
        },
      ])
      .mockResolvedValueOnce([
        {
          skillId: "trellis-check",
          agentId: "claude-code",
          rowId: null,
          ok: true,
          error: null,
        },
      ]);
    render(
      wrap(
        <UnusedSkillsPanel
          report={buildReport()}
          onSelect={vi.fn()}
          onUnlinkAgents={onUnlinkAgents}
        />,
      ),
    );
    fireEvent.click(
      within(
        screen.getByRole("group", {
          name: /长期未用阈值|Staleness threshold/,
        }),
      ).getByRole("button", { name: /30 天|30d/ }),
    );
    fireEvent.click(
      screen.getByTestId("unused-unlink-trigger-central-trellis-check"),
    );
    fireEvent.click(
      within(screen.getByTestId("unused-unlink-select-all")).getByRole(
        "checkbox",
      ),
    );
    fireEvent.click(
      screen.getByRole("button", { name: /移除 \(2\)|Unlink \(2\)/ }),
    );

    // 部分失败：弹窗保留、失败行呈现原因，勾选重置为仅失败项（同一次提交后的同一帧）
    await waitFor(() => {
      expect(
        screen.getByTestId("unused-unlink-error-claude-code"),
      ).toHaveTextContent(/blocked/);
      expect(
        screen.getByRole("button", { name: /移除 \(1\)|Unlink \(1\)/ }),
      ).toBeEnabled();
    });
    expect(screen.getByTestId("unused-unlink-dialog")).toBeInTheDocument();

    // 直接重试只提交失败项
    fireEvent.click(
      screen.getByRole("button", { name: /移除 \(1\)|Unlink \(1\)/ }),
    );
    await waitFor(() =>
      expect(onUnlinkAgents).toHaveBeenCalledTimes(2),
    );
    expect(onUnlinkAgents).toHaveBeenLastCalledWith([
      { skillId: "trellis-check", agentId: "claude-code", rowId: null },
    ]);
  });

  it("closes the dialog after a fully successful batch", async () => {
    const onUnlinkAgents = vi.fn().mockResolvedValueOnce([
      {
        skillId: "legacy-cleanup",
        agentId: "claude-code",
        rowId: null,
        ok: true,
        error: null,
      },
    ]);
    render(
      wrap(
        <UnusedSkillsPanel
          report={buildReport()}
          onSelect={vi.fn()}
          onUnlinkAgents={onUnlinkAgents}
        />,
      ),
    );
    fireEvent.click(
      screen.getByTestId("unused-unlink-trigger-central-legacy-cleanup"),
    );
    fireEvent.click(screen.getByRole("checkbox", { name: /claude-code/ }));
    fireEvent.click(
      screen.getByRole("button", { name: /移除 \(1\)|Unlink \(1\)/ }),
    );

    await waitFor(() =>
      expect(screen.queryByTestId("unused-unlink-dialog")).not.toBeInTheDocument(),
    );
  });
});

describe("unusedUnlinkTargets", () => {
  function centralEntry(overrides: Partial<UnusedSkillEntry> = {}): UnusedSkillEntry {
    return {
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
        {
          agentId: "codex",
          linkType: "native",
          installedPath: "C:/central/legacy-cleanup",
          hasPendingRecovery: true,
        },
        {
          agentId: "central",
          linkType: "native",
          installedPath: "C:/central/legacy-cleanup",
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
      ...overrides,
    };
  }

  it("centralTargets normalizes agents with shared-root and recovery reasons", () => {
    const targets = centralTargets(centralEntry());
    expect(targets).toEqual([
      {
        skillId: "legacy-cleanup",
        agentId: "claude-code",
        rowId: null,
        disabledReason: null,
      },
      {
        skillId: "legacy-cleanup",
        agentId: "codex",
        rowId: null,
        disabledReason: "disabledPendingRecovery",
      },
      {
        skillId: "legacy-cleanup",
        agentId: "central",
        rowId: null,
        disabledReason: "disabledSharedRoot",
      },
    ]);
  });

  it("platformTargets lists every cross-agent install with full disabled-reason mapping", () => {
    const entry = buildReport().platforms[1]; // local-notes: claude-code + zed（均可卸载）
    const targets = platformTargets(entry);
    expect(targets).toHaveLength(2);
    expect(targets[0]).toEqual({
      skillId: "obs-claude-local-notes",
      agentId: "claude-code",
      rowId: "obs-claude-local-notes",
      disabledReason: null,
    });
    expect(targets[1]).toEqual({
      skillId: "obs-zed-local-notes",
      agentId: "zed",
      rowId: "obs-zed-local-notes",
      disabledReason: null,
    });

    expect(platformUnlinkDisabledReason(entry.installs[0])).toBeNull();
    expect(
      platformUnlinkDisabledReason({ ...entry.installs[0], isReadOnly: true }),
    ).toBe("disabledReadOnly");
    expect(
      platformUnlinkDisabledReason({
        ...entry.installs[0],
        sourceKind: "plugin",
      }),
    ).toBe("disabledSourceKind");
    expect(
      platformUnlinkDisabledReason({ ...entry.installs[0], rowId: null }),
    ).toBe("disabledNoRow");
    expect(
      platformUnlinkDisabledReason({
        ...entry.installs[0],
        hasPendingRecovery: true,
      }),
    ).toBe("disabledPendingRecovery");
  });
});
