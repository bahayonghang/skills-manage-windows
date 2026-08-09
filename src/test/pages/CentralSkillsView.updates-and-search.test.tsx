import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import type { AgentWithStatus, CentralSkillUpdateState, SkillWithLinks, TargetSummary } from "@/types";
import * as S from "./centralSkillsViewTestSupport";

const {
  CentralSkillsView,
  toast,
  tauriBridge,
  mockAgents,
  mockSkills,
  mockRepositories,
  mockBatchInstallSkills,
  mockBatchUninstallSkillsFromAgent,
  mockLoadBatchDeletePreview,
  mockLoadCentralSkills,
  mockDeleteCentralSkills,
  mockCheckSkillUpdates,
  mockCheckRepositorySync,
  mockUpdateSkills,
  mockRefreshUpdateInventory,
  mockOpenUpdateCenterDialog,
  mockEmptyUpdateInventory,
  mockRescan,
  mockUseCentralSkillsStore,
  mockUsePlatformStore,
  mockUseSkillDetailStore,
  localTarget,
  renderCentralSkillsView,
  useTargetStore,
  settingsStore,
} = S;

describe("CentralSkillsView updates + search（V2 markup）", () => {
  beforeEach(() => {
    S.resetCentralSkillsViewTestState();
    window.localStorage.clear();
  });
  afterEach(S.cleanupCentralSkillsViewTestState);

  it("defaults remote batch install to symlink when the target supports symlinks", async () => {
    const remoteTarget: TargetSummary = {
      id: "ssh-demo",
      kind: "ssh",
      label: "Demo",
      remoteOs: "Linux",
      symlinkEnabled: true,
      isActive: true,
    };
    useTargetStore.setState({
      targets: [localTarget, remoteTarget],
      activeTarget: remoteTarget,
    });
    mockBatchInstallSkills.mockResolvedValueOnce({
      succeeded: [
        { skill_id: "code-reviewer", agent_id: "codex", target_path: "/home/test/.agents/skills/code-reviewer" },
        { skill_id: "code-reviewer", agent_id: "claude-code", target_path: "/home/test/.claude/skills/code-reviewer" },
      ],
      failed: [],
    });
    renderCentralSkillsView();

    fireEvent.click(screen.getAllByRole("checkbox")[0]);
    fireEvent.click(screen.getByTestId("bulk-bar-batch-install"));

    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: /将 1 个技能安装到 2 个平台/i }));

    await waitFor(() => {
      expect(mockBatchInstallSkills).toHaveBeenCalledWith(
        ["code-reviewer"],
        ["codex", "claude-code"],
        "symlink",
        null
      );
    });
  });


  it("reports seven skills installed in three platforms instead of twenty-one installs", async () => {
    const sevenSkills: SkillWithLinks[] = Array.from({ length: 7 }, (_, index) => {
      const baseSkill = mockSkills[index % mockSkills.length]!;
      const repository = index % 2 === 0 ? mockRepositories[1]! : mockRepositories[0]!;

      return {
        ...baseSkill,
        id: `batch-skill-${index + 1}`,
        name: `batch-skill-${index + 1}`,
        file_path: `~/.skillsmanage/skills/batch-skill-${index + 1}/SKILL.md`,
        canonical_path: `~/.skillsmanage/skills/batch-skill-${index + 1}`,
        repository,
      };
    });
    const extraAgent: AgentWithStatus = {
      id: "windsurf",
      display_name: "Windsurf",
      category: "coding",
      global_skills_dir: "/Users/test/.windsurf/skills/",
      is_detected: true,
      is_builtin: true,
      is_enabled: true,
    };
    const expectedSkillIds = sevenSkills.map((skill) => skill.id);
    const expectedAgentIds = ["codex", "claude-code", "windsurf"];
    mockBatchInstallSkills.mockResolvedValueOnce({
      succeeded: sevenSkills.flatMap((skill) =>
        expectedAgentIds.map((agentId) => ({
          skill_id: skill.id,
          agent_id: agentId,
          target_path: `/Users/test/${agentId}/${skill.id}`,
        }))
      ),
      failed: [],
    });
    renderCentralSkillsView({
      centralOverrides: { skills: sevenSkills },
      platformOverrides: { agents: [...mockAgents, extraAgent] },
    });

    screen.getAllByRole("checkbox").forEach((checkbox) => {
      fireEvent.click(checkbox);
    });
    fireEvent.click(screen.getByTestId("bulk-bar-batch-install"));

    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: /将 7 个技能安装到 3 个平台/i }));

    await waitFor(() => {
      expect(mockBatchInstallSkills).toHaveBeenCalledWith(
        expectedSkillIds,
        expectedAgentIds,
        "symlink",
        null
      );
    });
    expect(toast.success).toHaveBeenCalledWith("已将 7 个技能安装到 3 个平台");
    expect(toast.success).not.toHaveBeenCalledWith(expect.stringContaining("21"));
  });


  it("uses skill and platform counts for partial batch install failures", async () => {
    mockBatchInstallSkills.mockResolvedValueOnce({
      succeeded: [
        { skill_id: "code-reviewer", agent_id: "codex", target_path: "/Users/test/.agents/skills/code-reviewer" },
        { skill_id: "frontend-design", agent_id: "codex", target_path: "/Users/test/.agents/skills/frontend-design" },
        { skill_id: "frontend-design", agent_id: "claude-code", target_path: "/Users/test/.claude/skills/frontend-design" },
      ],
      skipped: [],
      failed: [
        { skill_id: "code-reviewer", agent_id: "claude-code", error: "already exists" },
      ],
    });
    renderCentralSkillsView();

    const selectionCheckboxes = screen.getAllByRole("checkbox");
    fireEvent.click(selectionCheckboxes[0]);
    fireEvent.click(selectionCheckboxes[1]);
    fireEvent.click(screen.getByTestId("bulk-bar-batch-install"));

    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: /将 2 个技能安装到 2 个平台/i }));

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith(
        "批量安装完成：成功 3，跳过 0，失败 1"
      );
    });
    expect(screen.getByText("已请求将 2 个技能安装到 2 个平台：成功 3，跳过 0，失败 1")).toBeInTheDocument();
    expect(screen.queryByText(/4 install/i)).not.toBeInTheDocument();
  });


  it("batch installs selected central skills to a project target", async () => {
    mockBatchInstallSkills.mockResolvedValueOnce({
      succeeded: [
        { skill_id: "code-reviewer", agent_id: "codex", target_path: "D:\\work\\demo\\.agents\\skills\\code-reviewer" },
        { skill_id: "code-reviewer", agent_id: "claude-code", target_path: "D:\\work\\demo\\.claude\\skills\\code-reviewer" },
      ],
      failed: [],
    });
    renderCentralSkillsView();

    fireEvent.click(screen.getAllByRole("checkbox")[0]);
    fireEvent.click(screen.getByTestId("bulk-bar-batch-install"));

    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByText("项目目录"));
    fireEvent.change(within(dialog).getByRole("textbox"), {
      target: { value: "D:\\work\\demo" },
    });
    fireEvent.click(within(dialog).getByRole("button", { name: /将 1 个技能安装到 2 个平台/i }));

    await waitFor(() => {
      expect(mockBatchInstallSkills).toHaveBeenCalledWith(
        ["code-reviewer"],
        ["codex", "claude-code"],
        "copy",
        "D:\\work\\demo"
      );
    });
  });

  it("batch uninstalls selected central skills from installed removable platforms only", async () => {
    mockBatchUninstallSkillsFromAgent
      .mockResolvedValueOnce({
        succeeded: [
          { skill_id: "code-reviewer" },
          { skill_id: "frontend-design" },
        ],
        failed: [],
      })
      .mockResolvedValueOnce({
        succeeded: [{ skill_id: "frontend-design" }],
        failed: [],
      });
    renderCentralSkillsView();

    screen.getAllByLabelText("选择技能").forEach((checkbox) => {
      fireEvent.click(checkbox);
    });
    fireEvent.click(screen.getByTestId("bulk-bar-batch-uninstall"));

    const dialog = await screen.findByRole("dialog");
    expect(
      within(dialog).getByText("此操作只取消平台安装，不会删除中央技能库中的技能、仓库、标签或技能文件。"),
    ).toBeInTheDocument();
    fireEvent.click(
      within(dialog).getByTestId("confirm-batch-uninstall-central-skills"),
    );

    await waitFor(() => {
      expect(mockBatchUninstallSkillsFromAgent).toHaveBeenCalledTimes(2);
    });
    expect(mockBatchUninstallSkillsFromAgent).toHaveBeenCalledWith("codex", [
      { skill_id: "code-reviewer" },
      { skill_id: "frontend-design" },
    ]);
    expect(mockBatchUninstallSkillsFromAgent).toHaveBeenCalledWith(
      "claude-code",
      [{ skill_id: "frontend-design" }],
    );
    expect(mockRescan).toHaveBeenCalled();
    expect(toast.success).toHaveBeenCalledWith("已卸载 3 个平台安装，跳过 0 个技能");
    await waitFor(() => {
      expect(screen.queryByTestId("central-bulk-action-bar")).not.toBeInTheDocument();
    });
  });

  it("excludes shared-root platform links from central batch uninstall requests", async () => {
    const skills: SkillWithLinks[] = [
      {
        ...mockSkills[0]!,
        id: "shared-and-codex",
        name: "shared-and-codex",
        linked_agents: ["central", "cursor", "codex"],
        shared_root_agents: ["cursor"],
      },
      {
        ...mockSkills[1]!,
        id: "shared-only",
        name: "shared-only",
        linked_agents: ["cursor"],
        shared_root_agents: ["cursor"],
      },
    ];
    mockBatchUninstallSkillsFromAgent.mockResolvedValueOnce({
      succeeded: [{ skill_id: "shared-and-codex" }],
      failed: [],
    });
    renderCentralSkillsView({ centralOverrides: { skills } });

    screen.getAllByLabelText("选择技能").forEach((checkbox) => {
      fireEvent.click(checkbox);
    });
    fireEvent.click(screen.getByTestId("bulk-bar-batch-uninstall"));

    const dialog = await screen.findByRole("dialog");
    expect(
      within(dialog).getByText("共享中央目录平台不会独立卸载"),
    ).toBeInTheDocument();
    expect(
      within(dialog).getByText("只存在共享中央目录平台: 1"),
    ).toBeInTheDocument();

    fireEvent.click(
      within(dialog).getByTestId("confirm-batch-uninstall-central-skills"),
    );

    await waitFor(() => {
      expect(mockBatchUninstallSkillsFromAgent).toHaveBeenCalledTimes(1);
    });
    expect(mockBatchUninstallSkillsFromAgent).toHaveBeenCalledWith("codex", [
      { skill_id: "shared-and-codex" },
    ]);
    expect(mockBatchUninstallSkillsFromAgent).not.toHaveBeenCalledWith(
      "cursor",
      expect.anything(),
    );
  });

  it("does not call backend uninstall when selected central skills have no removable installs", async () => {
    const skills: SkillWithLinks[] = [
      {
        ...mockSkills[0]!,
        id: "local-only",
        name: "local-only",
        linked_agents: [],
        shared_root_agents: [],
      },
      {
        ...mockSkills[1]!,
        id: "shared-only",
        name: "shared-only",
        linked_agents: ["cursor"],
        shared_root_agents: ["cursor"],
      },
    ];
    renderCentralSkillsView({ centralOverrides: { skills } });

    screen.getAllByLabelText("选择技能").forEach((checkbox) => {
      fireEvent.click(checkbox);
    });
    fireEvent.click(screen.getByTestId("bulk-bar-batch-uninstall"));

    const dialog = await screen.findByRole("dialog");
    expect(
      within(dialog).getByTestId("central-batch-uninstall-noop"),
    ).toHaveTextContent("已选技能没有可独立卸载的平台安装。");
    expect(
      within(dialog).getByTestId("confirm-batch-uninstall-central-skills"),
    ).toBeDisabled();
    expect(mockBatchUninstallSkillsFromAgent).not.toHaveBeenCalled();
  });

  it("keeps failed central batch uninstall skills selected after partial failure", async () => {
    mockBatchUninstallSkillsFromAgent
      .mockResolvedValueOnce({
        succeeded: [{ skill_id: "code-reviewer" }],
        failed: [
          {
            skill_id: "frontend-design",
            error: "permission denied",
          },
        ],
      })
      .mockResolvedValueOnce({
        succeeded: [{ skill_id: "frontend-design" }],
        failed: [],
      });
    renderCentralSkillsView();

    screen.getAllByLabelText("选择技能").forEach((checkbox) => {
      fireEvent.click(checkbox);
    });
    fireEvent.click(screen.getByTestId("bulk-bar-batch-uninstall"));

    const dialog = await screen.findByRole("dialog");
    fireEvent.click(
      within(dialog).getByTestId("confirm-batch-uninstall-central-skills"),
    );

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith(
        "已卸载 2 个，1 个失败；失败项已保留选中，可重试。",
      );
    });
    expect(within(dialog).getByText(/frontend-design \/ codex/)).toBeInTheDocument();
    expect(screen.getByTestId("central-selection-summary")).toHaveTextContent(
      "已选 1",
    );
  });

  it("opens single-card platform uninstall for only the clicked central skill", async () => {
    mockBatchUninstallSkillsFromAgent
      .mockResolvedValueOnce({
        succeeded: [{ skill_id: "frontend-design" }],
        failed: [],
      })
      .mockResolvedValueOnce({
        succeeded: [{ skill_id: "frontend-design" }],
        failed: [],
    });
    renderCentralSkillsView();

    screen.getAllByLabelText("选择技能").forEach((checkbox) => {
      fireEvent.click(checkbox);
    });
    fireEvent.click(screen.getByTestId("uninstall-platforms-skill-frontend-design"));

    const dialog = await screen.findByRole("dialog");
    expect(
      within(dialog).getByText("此操作只取消平台安装，不会删除中央技能库中的技能、仓库、标签或技能文件。"),
    ).toBeInTheDocument();
    fireEvent.click(
      within(dialog).getByTestId("confirm-batch-uninstall-central-skills"),
    );

    await waitFor(() => {
      expect(mockBatchUninstallSkillsFromAgent).toHaveBeenCalledTimes(2);
    });
    expect(mockBatchUninstallSkillsFromAgent).toHaveBeenCalledWith(
      "claude-code",
      [{ skill_id: "frontend-design" }],
    );
    expect(mockBatchUninstallSkillsFromAgent).toHaveBeenCalledWith("codex", [
      { skill_id: "frontend-design" },
    ]);
    expect(mockBatchUninstallSkillsFromAgent).not.toHaveBeenCalledWith(
      "codex",
      expect.arrayContaining([{ skill_id: "code-reviewer" }]),
    );
  });

  it("shows no-op dialog from single-card uninstall when the skill has no removable installs", async () => {
    const skills: SkillWithLinks[] = [
      {
        ...mockSkills[0]!,
        id: "shared-only",
        name: "shared-only",
        linked_agents: ["cursor"],
        shared_root_agents: ["cursor"],
      },
    ];
    renderCentralSkillsView({ centralOverrides: { skills } });

    fireEvent.click(screen.getByTestId("uninstall-platforms-skill-shared-only"));

    const dialog = await screen.findByRole("dialog");
    expect(
      within(dialog).getByTestId("central-batch-uninstall-noop"),
    ).toHaveTextContent("已选技能没有可独立卸载的平台安装。");
    expect(
      within(dialog).getByText("共享中央目录平台不会独立卸载"),
    ).toBeInTheDocument();
    expect(
      within(dialog).getByTestId("confirm-batch-uninstall-central-skills"),
    ).toBeDisabled();
    expect(mockBatchUninstallSkillsFromAgent).not.toHaveBeenCalled();
  });


  it("previews and batch deletes selected central skills with selected platform copies", async () => {
    mockLoadBatchDeletePreview.mockResolvedValueOnce({
      previews: [
        {
          skill_id: "frontend-design",
          skill_name: "frontend-design",
          central_path: "~/.skillsmanage/skills/frontend-design",
          copy_installations: [
            {
              skill_id: "frontend-design",
              agent_id: "cursor",
              installed_path: "/Users/test/.cursor/skills/frontend-design",
              link_type: "copy",
              symlink_target: undefined,
              installed_at: "2026-04-11T00:00:00Z",
            },
          ],
          auto_removed_agent_ids: ["claude-code"],
        },
        {
          skill_id: "code-reviewer",
          skill_name: "code-reviewer",
          central_path: "~/.skillsmanage/skills/code-reviewer",
          copy_installations: [],
          auto_removed_agent_ids: ["codex"],
        },
      ],
      failed: [],
    });
    mockDeleteCentralSkills.mockResolvedValueOnce({
      succeeded: [
        {
          skill_id: "frontend-design",
          removed_central_path: "~/.skillsmanage/skills/frontend-design",
          removed_agent_ids: ["cursor"],
          retained_agent_ids: [],
        },
        {
          skill_id: "code-reviewer",
          removed_central_path: "~/.skillsmanage/skills/code-reviewer",
          removed_agent_ids: [],
          retained_agent_ids: [],
        },
      ],
      failed: [],
    });
    renderCentralSkillsView();

    const selectionCheckboxes = screen.getAllByRole("checkbox");
    fireEvent.click(selectionCheckboxes[0]);
    fireEvent.click(selectionCheckboxes[1]);
    fireEvent.click(screen.getByTestId("bulk-bar-batch-delete"));

    await waitFor(() => {
      expect(mockLoadBatchDeletePreview).toHaveBeenCalledWith([
        "code-reviewer",
        "frontend-design",
      ]);
    });

    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByRole("checkbox", { name: /Cursor/i }));
    fireEvent.click(within(dialog).getByTestId("confirm-batch-delete-central-skills"));

    await waitFor(() => {
      expect(mockDeleteCentralSkills).toHaveBeenCalledWith([
        {
          skill_id: "frontend-design",
          remove_agent_ids: ["cursor"],
        },
        {
          skill_id: "code-reviewer",
          remove_agent_ids: [],
        },
      ]);
    });
    expect(mockRescan).toHaveBeenCalled();
  });


  it("shows auto-cleaned linked installs instead of an empty copy state", async () => {
    mockLoadBatchDeletePreview.mockResolvedValueOnce({
      previews: [
        {
          skill_id: "frontend-design",
          skill_name: "frontend-design",
          central_path: "~/.skillsmanage/skills/frontend-design",
          copy_installations: [],
          auto_removed_agent_ids: ["claude-code"],
        },
      ],
      failed: [],
    });
    mockDeleteCentralSkills.mockResolvedValueOnce({
      succeeded: [
        {
          skill_id: "frontend-design",
          removed_central_path: "~/.skillsmanage/skills/frontend-design",
          removed_agent_ids: ["claude-code"],
          retained_agent_ids: [],
        },
      ],
      failed: [],
    });
    renderCentralSkillsView();

    fireEvent.click(screen.getAllByRole("checkbox")[0]);
    fireEvent.click(screen.getByTestId("bulk-bar-batch-delete"));

    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText("已安装的平台链接会自动移除")).toBeInTheDocument();
    expect(within(dialog).getByText("Claude Code")).toBeInTheDocument();
    expect(
      within(dialog).queryByText("已选技能没有已安装的平台链接或独立平台副本。")
    ).not.toBeInTheDocument();

    fireEvent.click(within(dialog).getByTestId("confirm-batch-delete-central-skills"));

    await waitFor(() => {
      expect(mockDeleteCentralSkills).toHaveBeenCalledWith([
        {
          skill_id: "frontend-design",
          remove_agent_ids: [],
        },
      ]);
    });
  });


  it("groups shared Universal linked installs and hides Central from cleanup preview", async () => {
    mockLoadBatchDeletePreview.mockResolvedValueOnce({
      previews: [
        {
          skill_id: "frontend-design",
          skill_name: "frontend-design",
          central_path: "~/.skillsmanage/skills/frontend-design",
          copy_installations: [],
          auto_removed_agent_ids: ["cursor", "codex", "central"],
        },
      ],
      failed: [],
    });
    renderCentralSkillsView();

    fireEvent.click(screen.getAllByRole("checkbox")[0]);
    fireEvent.click(screen.getByTestId("bulk-bar-batch-delete"));

    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText(/Universal/)).toBeInTheDocument();
    expect(within(dialog).getByText("Codex, Cursor")).toBeInTheDocument();
    expect(within(dialog).queryByText("Central Skills")).not.toBeInTheDocument();
  });


  it("opens mode selection before running the default regular check", async () => {
    renderCentralSkillsView();

    expect(screen.getByTestId("central-update-check-mode-select")).toHaveValue("regular");

    fireEvent.click(screen.getByTestId("central-check-updates"));

    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText("选择更新检查模式")).toBeInTheDocument();
    expect(mockRefreshUpdateInventory).not.toHaveBeenCalled();

    fireEvent.click(within(dialog).getByTestId("confirm-update-check-mode"));

    await waitFor(() => {
      expect(mockRefreshUpdateInventory).toHaveBeenCalledWith({
        kind: "skills",
        mode: "regular",
        skillIds: ["code-reviewer", "frontend-design"],
      });
    });
    expect(mockCheckSkillUpdates).not.toHaveBeenCalled();
    expect(mockCheckRepositorySync).not.toHaveBeenCalled();
  });

  it("surfaces Update Center refresh failures from the mode dialog", async () => {
    mockRefreshUpdateInventory
      .mockRejectedValueOnce("network unavailable")
      .mockImplementationOnce(() => new Promise(() => {}));
    renderCentralSkillsView();

    fireEvent.click(screen.getByTestId("central-check-updates"));
    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByTestId("confirm-update-check-mode"));

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith(
        "检查更新失败: network unavailable",
      );
    });
    expect(within(dialog).getByRole("alert")).toHaveTextContent(
      "检查更新失败: network unavailable",
    );
    expect(within(dialog).getByTestId("confirm-update-check-mode")).not.toBeDisabled();
    expect(mockOpenUpdateCenterDialog).not.toHaveBeenCalled();

    fireEvent.click(within(dialog).getByTestId("confirm-update-check-mode"));

    await waitFor(() => {
      expect(within(dialog).queryByRole("alert")).not.toBeInTheDocument();
    });
  });

  it("localizes coded archive redirect failures without exposing backend details", async () => {
    const seed = "ghp_secret https://example.invalid/private C:\\private\\SKILL.md";
    mockRefreshUpdateInventory.mockRejectedValueOnce(
      `github_import.archive_redirect_rejected:${seed}`,
    );
    renderCentralSkillsView();

    fireEvent.click(screen.getByTestId("central-check-updates"));
    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByTestId("confirm-update-check-mode"));

    const expected =
      "检查更新失败: GitHub 返回了不安全或异常的仓库压缩包跳转，已停止检查更新。请重试。";
    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith(expected);
    });
    expect(within(dialog).getByRole("alert")).toHaveTextContent(expected);
    expect(within(dialog).getByRole("alert")).not.toHaveTextContent(seed);
    expect(mockOpenUpdateCenterDialog).not.toHaveBeenCalled();
  });

  it("检查成功后先自动重取列表再打开 Update Center，更新状态可见生效", async () => {
    const { centralState } = renderCentralSkillsView();
    // 模拟重取把检查后的更新状态写回 store（真实 store 的行为）；
    // 组件后续重渲染时 mock selector 会读到新值。
    mockLoadCentralSkills.mockImplementation(async () => {
      centralState.updateStatuses = {
        "frontend-design": {
          skill_id: "frontend-design",
          source_type: "github",
          status: "update_available",
        },
      };
    });

    fireEvent.click(screen.getByTestId("central-check-updates"));
    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByTestId("confirm-update-check-mode"));

    await waitFor(() => {
      expect(mockOpenUpdateCenterDialog).toHaveBeenCalledWith("updatable", {
        skillIds: ["code-reviewer", "frontend-design"],
        mode: "regular",
      });
    });

    // 自动重取（throwOnError 路径）必须先于打开 Update Center。
    const autoRefreshCallIndex = mockLoadCentralSkills.mock.calls.findIndex(
      (args) => (args[0] as { throwOnError?: boolean } | undefined)?.throwOnError === true,
    );
    expect(autoRefreshCallIndex).toBeGreaterThanOrEqual(0);
    const autoRefreshOrder = mockLoadCentralSkills.mock.invocationCallOrder[autoRefreshCallIndex]!;
    const openOrder = mockOpenUpdateCenterDialog.mock.invocationCallOrder[0]!;
    expect(autoRefreshOrder).toBeLessThan(openOrder);

    // 列表 updateStatuses 可见更新：出现可更新 chip（断言 UI 结果而非仅调用）。
    expect(await screen.findByTestId("central-update-count-chip")).toHaveTextContent("+1");
  });

  it("检查后列表重取失败仍按原参数打开 Update Center，只报 refreshError", async () => {
    renderCentralSkillsView();
    // 挂载那次无参调用已消费默认实现；reject 只用于 throwOnError 路径的下一次调用。
    mockLoadCentralSkills.mockRejectedValueOnce(new Error("list read failed"));

    fireEvent.click(screen.getByTestId("central-check-updates"));
    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByTestId("confirm-update-check-mode"));

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith("刷新失败: Error: list read failed");
    });
    expect(mockOpenUpdateCenterDialog).toHaveBeenCalledWith("updatable", {
      skillIds: ["code-reviewer", "frontend-design"],
      mode: "regular",
    });
    expect(toast.error).not.toHaveBeenCalledWith(
      expect.stringContaining("检查更新失败"),
    );
    // 检查本身成功：无内联错误，弹窗按成功路径关闭。
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("手动刷新列表重取失败时报 refreshError toast，计数刷新仍并行执行", async () => {
    renderCentralSkillsView();
    mockLoadCentralSkills.mockRejectedValueOnce(new Error("disk gone"));

    fireEvent.click(screen.getByTestId("central-refresh-skills"));

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith("刷新失败: Error: disk gone");
    });
    expect(mockRescan).toHaveBeenCalled();
  });

  it("计数刷新失败不阻断列表重取，且同样给出失败反馈", async () => {
    renderCentralSkillsView();
    mockRescan.mockRejectedValueOnce(new Error("counts failed"));

    fireEvent.click(screen.getByTestId("central-refresh-skills"));

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith("刷新失败: Error: counts failed");
    });
    expect(mockLoadCentralSkills).toHaveBeenCalledWith({ throwOnError: true });
  });

  it("刷新中按钮禁用，重复点击不触发第二次请求", async () => {
    renderCentralSkillsView({ centralOverrides: { isRefreshingList: true } });

    const button = screen.getByTestId("central-refresh-skills");
    expect(button).toBeDisabled();
    fireEvent.click(button);

    // 只有挂载时那一次无参加载，没有新的列表/计数请求。
    await waitFor(() => {
      expect(mockLoadCentralSkills).toHaveBeenCalledTimes(1);
    });
    expect(mockRescan).not.toHaveBeenCalled();
  });

  it("shows all active repositories while the confirmed check is running", async () => {
    S.setMockUpdateCenterProgress({
      operationId: "refresh-1",
      phase: "checking",
      total: 4,
      completed: 1,
      activeRepositories: [
        { key: "openai/skills/main", name: "openai/skills" },
        { key: "anthropics/skills/main", name: "anthropics/skills" },
      ],
    });
    let resolveRefresh: ((inventory: typeof mockEmptyUpdateInventory) => void) | undefined;
    mockRefreshUpdateInventory.mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          resolveRefresh = resolve;
        }),
    );
    renderCentralSkillsView();

    fireEvent.click(screen.getByTestId("central-check-updates"));
    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByTestId("confirm-update-check-mode"));

    await waitFor(() => {
      expect(within(dialog).getByTestId("update-check-progress-view")).toBeInTheDocument();
    });
    expect(within(dialog).queryByTestId("update-check-mode-regular")).not.toBeInTheDocument();
    expect(within(dialog).getByTitle("openai/skills")).toBeInTheDocument();
    expect(within(dialog).getByTitle("anthropics/skills")).toBeInTheDocument();

    resolveRefresh?.(mockEmptyUpdateInventory);
    await waitFor(() => expect(mockOpenUpdateCenterDialog).toHaveBeenCalledOnce());
  });

  it("persists the visible update mode selector preference", async () => {
    renderCentralSkillsView();

    fireEvent.change(screen.getByTestId("central-update-check-mode-select"), {
      target: { value: "sync" },
    });

    await waitFor(() => {
      expect(settingsStore.getState().centralUpdateCheckMode).toBe("sync");
    });
  });

  it("runs a regular check through Update Center with selected skill ids only", async () => {
    const inventory = {
      ...mockEmptyUpdateInventory,
      updatable: [
        {
          state: {
            skill_id: "code-reviewer",
            source_type: "github",
            source_url: "https://github.com/openai/skills",
            ref: "main",
            source_path: "skills/frontend-design",
            last_remote_hash: "old",
            latest_remote_hash: "new",
            last_checked_at: "2026-05-30T00:00:00Z",
            last_updated_at: null,
            status: "update_available",
            error: null,
          },
          repositoryId: "github-openai-skills-main",
        },
      ],
    };
    mockRefreshUpdateInventory.mockResolvedValueOnce(inventory);
    renderCentralSkillsView();

    fireEvent.click(screen.getAllByLabelText("选择技能")[0]);
    fireEvent.click(screen.getByRole("button", { name: "检查所选（1）" }));
    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByTestId("confirm-update-check-mode"));

    await waitFor(() => {
      expect(mockRefreshUpdateInventory).toHaveBeenCalledWith({
        kind: "skills",
        mode: "regular",
        skillIds: ["code-reviewer"],
      });
    });
    expect(mockOpenUpdateCenterDialog).toHaveBeenCalledWith("updatable", {
      skillIds: ["code-reviewer"],
      mode: "regular",
    });
    expect(mockCheckSkillUpdates).not.toHaveBeenCalled();
    expect(mockCheckRepositorySync).not.toHaveBeenCalled();
  });
  it("renders browser fixture skill card on the localhost validation surface without Tauri", async () => {
    const isTauriSpy = vi.spyOn(tauriBridge, "isTauriRuntime").mockReturnValue(false);
    mockUseCentralSkillsStore.mockRestore();
    mockUsePlatformStore.mockRestore();
    mockUseSkillDetailStore.mockRestore();

    render(
      <MemoryRouter>
        <CentralSkillsView />
      </MemoryRouter>
    );

    expect(await screen.findByRole("button", { name: /查看 fixture-central-skill 的详情/i })).toBeInTheDocument();

    isTauriSpy.mockRestore();
  });


  it("keeps the check-updates button enabled on SSH targets and runs regular check", async () => {
    const remoteTarget: TargetSummary = {
      id: "ssh-demo",
      kind: "ssh",
      label: "Demo",
      remoteOs: "Linux",
      symlinkEnabled: true,
      isActive: true,
    };
    useTargetStore.setState({
      targets: [localTarget, remoteTarget],
      activeTarget: remoteTarget,
    });
    renderCentralSkillsView();

    const checkButton = screen.getByTestId("central-check-updates");
    expect(checkButton).not.toBeDisabled();

    fireEvent.click(checkButton);
    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByTestId("confirm-update-check-mode"));

    await waitFor(() => {
      expect(mockRefreshUpdateInventory).toHaveBeenCalledWith({
        kind: "skills",
        mode: "regular",
        skillIds: ["code-reviewer", "frontend-design"],
      });
    });
  });

  it("runs incremental and removal mode for all repositories when no single repo is scoped", async () => {
    settingsStore.setState({ centralUpdateCheckMode: "sync", centralUpdateCheckModeLoaded: true });
    const inventory = {
      ...mockEmptyUpdateInventory,
      remoteAdded: [
        {
          repositoryId: "github-openai-skills-main",
          sourcePath: "skills/new-skill",
          skillId: "new-skill",
          skillName: "New Skill",
          conflictExistingSkillId: null,
        },
      ],
    };
    mockRefreshUpdateInventory.mockResolvedValueOnce(inventory);
    renderCentralSkillsView();

    fireEvent.click(screen.getByRole("button", { name: "检查全部仓库（1 个）" }));
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText(/当前范围：检查全部仓库（1 个）/)).toBeInTheDocument();
    expect(within(dialog).getByTestId("update-check-mode-regular")).toBeInTheDocument();
    expect(within(dialog).getByTestId("update-check-mode-sync")).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    fireEvent.click(within(dialog).getByTestId("confirm-update-check-mode"));

    await waitFor(() => {
      expect(mockRefreshUpdateInventory).toHaveBeenCalledWith({ kind: "all", mode: "sync" });
    });
    expect(mockOpenUpdateCenterDialog).toHaveBeenCalledWith("added", { mode: "sync" });
    expect(mockCheckRepositorySync).not.toHaveBeenCalled();
  });

  it("shows all repository scope for selected skills when incremental mode would check all", async () => {
    settingsStore.setState({ centralUpdateCheckMode: "sync", centralUpdateCheckModeLoaded: true });
    renderCentralSkillsView();

    fireEvent.click(screen.getAllByLabelText("选择技能")[0]);
    fireEvent.click(screen.getAllByLabelText("选择技能")[1]);

    const checkAllRepositories = await screen.findByRole("button", {
      name: "检查全部仓库（1 个）",
    });
    fireEvent.click(checkAllRepositories);

    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText(/当前范围：检查全部仓库（1 个）/)).toBeInTheDocument();
    fireEvent.click(within(dialog).getByTestId("confirm-update-check-mode"));

    await waitFor(() => {
      expect(mockRefreshUpdateInventory).toHaveBeenCalledWith({ kind: "all", mode: "sync" });
    });
  });

  it("routes single-card update actions through the same confirmation dialog", async () => {
    const updateState: CentralSkillUpdateState = {
      skill_id: "frontend-design",
      source_type: "github",
      source_url: "https://github.com/openai/skills",
      ref: "main",
      source_path: "skills/frontend-design",
      last_remote_hash: "fnv1a64:old",
      latest_remote_hash: "fnv1a64:new",
      last_checked_at: "2026-04-29T01:23:45Z",
      last_updated_at: null,
      status: "update_available",
      error: null,
    };
    mockUpdateSkills.mockResolvedValueOnce({
      succeeded: ["frontend-design"],
      failed: [],
      skipped: [],
      states: [{ ...updateState, status: "up_to_date" }],
    });
    renderCentralSkillsView({
      centralOverrides: {
        updateStatuses: {
          "frontend-design": updateState,
        },
      },
    });

    fireEvent.click(screen.getByRole("button", { name: /从来源更新 frontend-design/i }));

    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText("确认更新技能")).toBeInTheDocument();
    fireEvent.click(within(dialog).getByTestId("confirm-central-skill-updates"));

    await waitFor(() => {
      expect(mockUpdateSkills).toHaveBeenCalledWith(["frontend-design"]);
    });
  });

  it("routes the update-all action through the confirmation dialog", async () => {
    const updateState: CentralSkillUpdateState = {
      skill_id: "frontend-design",
      source_type: "github",
      source_url: "https://github.com/openai/skills",
      ref: "main",
      source_path: "skills/frontend-design",
      last_remote_hash: "fnv1a64:old",
      latest_remote_hash: "fnv1a64:new",
      last_checked_at: "2026-04-29T01:23:45Z",
      last_updated_at: null,
      status: "update_available",
      error: null,
    };
    mockUpdateSkills.mockResolvedValueOnce({
      succeeded: ["frontend-design"],
      failed: [],
      skipped: [],
      states: [{ ...updateState, status: "up_to_date" }],
    });
    renderCentralSkillsView({
      centralOverrides: {
        updateStatuses: {
          "frontend-design": updateState,
        },
      },
    });

    fireEvent.click(screen.getByRole("button", { name: "更新可用 (1)" }));

    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText("确认更新技能")).toBeInTheDocument();
    expect(within(dialog).getByText("frontend-design")).toBeInTheDocument();

    fireEvent.click(within(dialog).getByTestId("confirm-central-skill-updates"));

    await waitFor(() => {
      expect(mockUpdateSkills).toHaveBeenCalledWith(["frontend-design"]);
    });
  });

  it("uses regular mode for selected current repository results without repository sync", async () => {
    window.history.replaceState(null, "", "/");
    window.localStorage.setItem("central.sidebarPinned", "true");
    const githubRepo = mockRepositories[1]!;
    const localRepo = mockRepositories[0]!;
    const skills: SkillWithLinks[] = [
      { ...mockSkills[0]!, id: "github-one", name: "github-one", repository: githubRepo },
      { ...mockSkills[0]!, id: "github-two", name: "github-two", repository: githubRepo },
      { ...mockSkills[1]!, id: "local-one", name: "local-one", repository: localRepo },
      { ...mockSkills[1]!, id: "local-two", name: "local-two", repository: localRepo },
    ];

    renderCentralSkillsView({
      centralOverrides: { skills },
    });

    fireEvent.click(screen.getByTestId(`repo-${githubRepo.id}`));

    await waitFor(() => {
      expect(screen.getByRole("button", { name: "检查 openai/skills（2）" })).toBeInTheDocument();
    });

    const checkboxes = screen.getAllByLabelText("选择技能");
    fireEvent.click(checkboxes[0]);
    fireEvent.click(checkboxes[1]);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: "检查所选（2）" })).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: "检查所选（2）" }));
    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByTestId("confirm-update-check-mode"));

    await waitFor(() => {
      expect(mockRefreshUpdateInventory).toHaveBeenCalledWith({
        kind: "skills",
        mode: "regular",
        skillIds: ["github-one", "github-two"],
      });
    });
    expect(mockCheckSkillUpdates).not.toHaveBeenCalled();
    expect(mockCheckRepositorySync).not.toHaveBeenCalled();
  });

  it("uses repository refresh for incremental mode on a single repository filter", async () => {
    settingsStore.setState({ centralUpdateCheckMode: "sync", centralUpdateCheckModeLoaded: true });
    window.history.replaceState(null, "", "/");
    window.localStorage.setItem("central.sidebarPinned", "true");
    const githubRepo = mockRepositories[1]!;
    const localRepo = mockRepositories[0]!;
    const skills: SkillWithLinks[] = [
      { ...mockSkills[0]!, id: "github-one", name: "github-one", repository: githubRepo },
      { ...mockSkills[0]!, id: "github-two", name: "github-two", repository: githubRepo },
      { ...mockSkills[1]!, id: "local-one", name: "local-one", repository: localRepo },
    ];

    renderCentralSkillsView({
      centralOverrides: { skills },
    });

    fireEvent.click(screen.getByTestId(`repo-${githubRepo.id}`));
    const checkCurrentResults = await screen.findByRole("button", {
      name: "检查 openai/skills（1 个仓库）",
    });
    fireEvent.click(checkCurrentResults);

    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText(/当前范围：检查 openai\/skills（1 个仓库）/)).toBeInTheDocument();
    fireEvent.click(within(dialog).getByTestId("confirm-update-check-mode"));

    await waitFor(() => {
      expect(mockRefreshUpdateInventory).toHaveBeenCalledWith({
        kind: "repositories",
        mode: "sync",
        repositoryIds: [githubRepo.id],
      });
    });
    expect(mockOpenUpdateCenterDialog).toHaveBeenCalledWith("updatable", {
      repositoryIds: [githubRepo.id],
      skillIds: ["github-one", "github-two"],
      mode: "sync",
    });
  });

  it("disables incremental mode when no syncable GitHub repository exists", async () => {
    settingsStore.setState({ centralUpdateCheckMode: "sync", centralUpdateCheckModeLoaded: true });
    const localRepo = mockRepositories[0]!;
    const skills: SkillWithLinks[] = [
      { ...mockSkills[1]!, id: "local-one", name: "local-one", repository: localRepo },
    ];

    renderCentralSkillsView({
      centralOverrides: { skills, repositories: [localRepo] },
    });

    fireEvent.click(screen.getByRole("button", { name: "检查全部（1）" }));
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByTestId("update-check-mode-sync")).toBeDisabled();
    expect(within(dialog).getByText("当前没有可同步的 GitHub 仓库。")).toBeInTheDocument();
  });
});
