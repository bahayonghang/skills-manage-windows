import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import * as S from "./centralSkillsViewTestSupport";

const {
  toast,
  mockRepositories,
  mockDeleteCentralPreview,
  mockDeletePreview,
  mockBatchInstallSkills,
  mockAgents,
  mockSkills,
  mockInstallSkill,
  mockLoadDeletePreview,
  mockLoadRepositoryDeletePreview,
  mockDeleteCentralSkill,
  mockDeleteSkillRepository,
  mockRescan,
  mockLoadCentralSkills,
  mockPreviewCentralStoreLocationChange,
  mockApplyCentralStoreLocationChange,
  mockRefreshInstallations,
  renderCentralSkillsView,
} = S;

describe("CentralSkillsView repositories + installs（V2 markup）", () => {
  beforeEach(() => {
    S.resetCentralSkillsViewTestState();
    window.localStorage.clear();
  });

  it("previews and applies a Local Central store location change", async () => {
    renderCentralSkillsView();

    fireEvent.click(screen.getByTestId("central-store-location-open"));
    expect(await screen.findByText("修改中央技能库位置")).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("新位置"), {
      target: { value: "D:\\SkillPort\\central-skills" },
    });
    fireEvent.click(screen.getByText("预览"));

    await waitFor(() => {
      expect(mockPreviewCentralStoreLocationChange).toHaveBeenCalledWith(
        "D:\\SkillPort\\central-skills"
      );
    });
    expect(await screen.findByText("迁移预览")).toBeInTheDocument();
    expect(screen.getByText("同名覆盖")).toBeInTheDocument();

    fireEvent.click(screen.getByText("迁移并切换"));

    await waitFor(() => {
      expect(mockApplyCentralStoreLocationChange).toHaveBeenCalledWith(
        "D:\\SkillPort\\central-skills"
      );
      expect(mockLoadCentralSkills).toHaveBeenCalled();
      expect(mockRescan).toHaveBeenCalled();
      expect(toast.success).toHaveBeenCalledWith(
        expect.stringContaining("中央技能库已切换")
      );
    });
  });

  it("disables Central store location changes for remote targets", () => {
    S.useTargetStore.setState({
      targets: [
        {
          id: "ssh-1",
          kind: "ssh",
          label: "Remote",
          host: "example.com",
          username: "alice",
          remoteHome: "/home/alice",
          remoteOs: "linux",
          isActive: true,
        },
      ],
      activeTarget: {
        id: "ssh-1",
        kind: "ssh",
        label: "Remote",
        host: "example.com",
        username: "alice",
        remoteHome: "/home/alice",
        remoteOs: "linux",
        isActive: true,
      },
    });

    renderCentralSkillsView();

    expect(screen.getByTestId("central-store-location-open")).toBeDisabled();
  });
  afterEach(S.cleanupCentralSkillsViewTestState);

  it("opens repository delete preview and deletes repository skills", async () => {
    mockLoadRepositoryDeletePreview.mockResolvedValue({
      repository: mockRepositories[1],
      delete_preview: {
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
      },
    });
    mockDeleteSkillRepository.mockResolvedValue({
      repository: mockRepositories[1],
      deleted_repository: true,
      delete_result: {
        succeeded: [
          {
            skill_id: "frontend-design",
            removed_central_path: "~/.skillsmanage/skills/frontend-design",
            removed_agent_ids: ["claude-code"],
            retained_agent_ids: [],
          },
        ],
        failed: [],
      },
    });
    window.localStorage.setItem("central.sidebarPinned", "true");
    renderCentralSkillsView();

    const sidebar = screen.getByTestId("central-sidebar-v2");
    fireEvent.click(within(sidebar).getByTestId("repo-github-openai-skills-main"));
    await waitFor(() => {
      expect(screen.queryByText("code-reviewer")).not.toBeInTheDocument();
    });

    fireEvent.click(within(sidebar).getByTestId("repo-delete-github-openai-skills-main"));

    expect(await screen.findByText("删除仓库：openai/skills")).toBeInTheDocument();
    expect(
      screen.getByText("frontend-design - ~/.skillsmanage/skills/frontend-design")
    ).toBeInTheDocument();
    fireEvent.click(screen.getByTestId("confirm-delete-skill-repository"));

    await waitFor(() => {
      expect(mockDeleteSkillRepository).toHaveBeenCalledWith(
        "github-openai-skills-main",
        [{ skill_id: "frontend-design", remove_agent_ids: [], force: false }]
      );
    });
    await waitFor(() => {
      expect(screen.getByText("code-reviewer")).toBeInTheDocument();
    });
  });


  it("confirms and deletes an empty repository without opening the skill delete dialog", async () => {
    const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(true);
    mockLoadRepositoryDeletePreview.mockResolvedValue({
      repository: { ...mockRepositories[1], skill_count: 0 },
      delete_preview: {
        previews: [],
        failed: [],
      },
    });
    mockDeleteSkillRepository.mockResolvedValue({
      repository: mockRepositories[1],
      deleted_repository: true,
      delete_result: {
        succeeded: [],
        failed: [],
      },
    });
    window.localStorage.setItem("central.sidebarPinned", "true");
    renderCentralSkillsView();

    const sidebar = screen.getByTestId("central-sidebar-v2");
    fireEvent.click(within(sidebar).getByTestId("repo-delete-github-openai-skills-main"));

    await waitFor(() => {
      expect(confirmSpy).toHaveBeenCalledWith("删除空仓库“openai/skills”？");
    });
    expect(mockDeleteSkillRepository).toHaveBeenCalledWith("github-openai-skills-main", []);
    expect(screen.queryByTestId("confirm-delete-skill-repository")).not.toBeInTheDocument();

    confirmSpy.mockRestore();
  });


  it("renders all central skills", () => {
    renderCentralSkillsView();
    expect(screen.getByText("frontend-design")).toBeInTheDocument();
    expect(screen.getByText("code-reviewer")).toBeInTheDocument();
  });

  it("treats repository facet selection as single-select", async () => {
    window.localStorage.setItem("central.sidebarPinned", "true");
    renderCentralSkillsView();

    const sidebar = screen.getByTestId("central-sidebar-v2");
    fireEvent.click(within(sidebar).getByTestId("repo-github-openai-skills-main"));
    await waitFor(() => {
      expect(screen.getByText("frontend-design")).toBeInTheDocument();
      expect(screen.queryByText("code-reviewer")).not.toBeInTheDocument();
    });

    fireEvent.click(within(sidebar).getByTestId("repo-local-unknown"));
    await waitFor(() => {
      expect(screen.queryByText("frontend-design")).not.toBeInTheDocument();
      expect(screen.getByText("code-reviewer")).toBeInTheDocument();
    });

    fireEvent.click(within(sidebar).getByTestId("repo-local-unknown"));
    await waitFor(() => {
      expect(screen.getByText("frontend-design")).toBeInTheDocument();
      expect(screen.getByText("code-reviewer")).toBeInTheDocument();
    });
  });


  it("sorts by installed platform count", async () => {
    renderCentralSkillsView();

    fireEvent.click(screen.getByTestId("central-toolbar-sort"));
    fireEvent.click(await screen.findByTestId("central-toolbar-sort-installedPlatformCount-desc"));

    await waitFor(() => {
      const detailButtons = screen.getAllByRole("button", {
        name: /查看 .* 的详情/i,
      });
      expect(detailButtons[0]).toHaveTextContent("frontend-design");
      expect(detailButtons[1]).toHaveTextContent("code-reviewer");
    });

    fireEvent.click(screen.getByTestId("central-toolbar-sort"));
    fireEvent.click(await screen.findByTestId("central-toolbar-sort-installedPlatformCount-asc"));

    await waitFor(() => {
      const detailButtons = screen.getAllByRole("button", {
        name: /查看 .* 的详情/i,
      });
      expect(detailButtons[0]).toHaveTextContent("code-reviewer");
      expect(detailButtons[1]).toHaveTextContent("frontend-design");
    });
  });

  it("sorts by modified time and reverses direction explicitly", async () => {
    renderCentralSkillsView();

    // V2：排序通过 toolbar 单 dropdown menu，6 项 (field × dir)
    fireEvent.click(screen.getByTestId("central-toolbar-sort"));
    fireEvent.click(await screen.findByTestId("central-toolbar-sort-updatedAt-asc"));

    await waitFor(() => {
      const detailButtons = screen.getAllByRole("button", {
        name: /查看 .* 的详情/i,
      });
      expect(detailButtons[0]).toHaveTextContent("frontend-design");
      expect(detailButtons[1]).toHaveTextContent("code-reviewer");
    });

    fireEvent.click(screen.getByTestId("central-toolbar-sort"));
    fireEvent.click(await screen.findByTestId("central-toolbar-sort-updatedAt-desc"));

    await waitFor(() => {
      const detailButtons = screen.getAllByRole("button", {
        name: /查看 .* 的详情/i,
      });
      expect(detailButtons[0]).toHaveTextContent("code-reviewer");
      expect(detailButtons[1]).toHaveTextContent("frontend-design");
    });
  });


  it("renders skill descriptions", () => {
    renderCentralSkillsView();
    expect(
      screen.getByText(/Build distinctive, production-grade frontend interfaces/)
    ).toBeInTheDocument();
  });


  it("shows Install to... button for each skill", () => {
    renderCentralSkillsView();
    const installButtons = screen.getAllByRole("button", {
      name: /将 .* 安装到平台/i,
    });
    expect(installButtons).toHaveLength(2);
  });

  it("opens the shared install dialog from the detail drawer header install button", async () => {
    mockInstallSkill.mockResolvedValueOnce({
      succeeded: ["codex", "claude-code"],
      skipped: [],
      failed: [],
    });
    renderCentralSkillsView();

    fireEvent.click(screen.getByRole("button", { name: /查看 frontend-design 的详情/i }));

    expect(await screen.findByTestId("skill-detail-drawer")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "drawer-install:frontend-design" }));

    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText("安装 frontend-design")).toBeInTheDocument();
    expect(within(dialog).getByText("选择此技能的安装位置。")).toBeInTheDocument();
    expect(within(dialog).getByRole("group", { name: "选择平台" })).toBeInTheDocument();

    fireEvent.click(within(dialog).getByRole("button", { name: /安装到 2 个平台/i }));

    await waitFor(() => {
      expect(mockInstallSkill).toHaveBeenCalledWith(
        "frontend-design",
        ["codex", "claude-code"],
        "symlink",
        null
      );
    });
    expect(mockBatchInstallSkills).not.toHaveBeenCalled();
    expect(mockRescan).toHaveBeenCalled();
  });

  it("refreshes the open detail drawer installation state after confirming install", async () => {
    mockInstallSkill.mockResolvedValueOnce({
      succeeded: ["codex", "claude-code"],
      skipped: [],
      failed: [],
    });
    renderCentralSkillsView({
      skillDetailOverrides: {
        detail: { ...mockDeletePreview, id: "frontend-design" },
      },
    });

    fireEvent.click(screen.getByRole("button", { name: /查看 frontend-design 的详情/i }));
    expect(await screen.findByTestId("skill-detail-drawer")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "drawer-install:frontend-design" }));

    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: /安装到 2 个平台/i }));

    await waitFor(() => {
      expect(mockRefreshInstallations).toHaveBeenCalledWith("frontend-design");
    });
  });


  it("shows delete button for each central skill", async () => {
    renderCentralSkillsView();

    // comfortable \u5bc6\u5ea6\u4e0b\u5220\u9664\u662f\u4e00\u7ea7\u56fe\u6807\uff0c\u76f4\u63a5\u53ef\u89c1
    expect(
      await screen.findByTestId("delete-central-skill-frontend-design")
    ).toBeInTheDocument();
    expect(
      await screen.findByTestId("delete-central-skill-code-reviewer")
    ).toBeInTheDocument();
  });


  it("opens delete dialog and deletes selected platform copies", async () => {
    mockLoadDeletePreview.mockResolvedValueOnce(mockDeleteCentralPreview);
    mockDeleteCentralSkill.mockResolvedValueOnce(undefined);
    renderCentralSkillsView();

    // V2\uff1acomfortable \u5bc6\u5ea6\u4e0b\u5220\u9664\u4ecd\u662f\u4e00\u7ea7\u56fe\u6807\uff0c\u4e0d\u9700\u8981\u70b9 \u22ef
    fireEvent.click(
      await screen.findByTestId("delete-central-skill-frontend-design")
    );

    await waitFor(() => {
      expect(mockLoadDeletePreview).toHaveBeenCalledWith("frontend-design");
    });
    expect(screen.getByText("/Users/test/.cursor/skills/frontend-design")).toBeInTheDocument();
    expect(screen.getByRole("dialog")).toHaveTextContent("Claude Code");

    fireEvent.click(screen.getByRole("checkbox", { name: /Cursor/i }));
    fireEvent.click(screen.getByRole("button", { name: /\u5220\u9664\u4e2d\u592e\u6280\u80fd/i }));

    await waitFor(() => {
      expect(mockDeleteCentralSkill).toHaveBeenCalledWith("frontend-design", ["cursor"], false);
    });
    expect(mockRescan).toHaveBeenCalled();
  });


  it("shows batch delete only after selecting central skills", () => {
    renderCentralSkillsView();

    expect(screen.queryByTestId("batch-delete-central-skills")).not.toBeInTheDocument();
    expect(screen.queryByTestId("batch-install-central-skills")).not.toBeInTheDocument();

    const selectionCheckboxes = screen.getAllByRole("checkbox");
    fireEvent.click(selectionCheckboxes[0]);

    expect(screen.getByTestId("bulk-bar-batch-delete")).toBeInTheDocument();
    expect(screen.getByTestId("bulk-bar-batch-install")).toBeInTheDocument();
  });


  it("batch installs selected central skills to global platform targets", async () => {
    mockBatchInstallSkills.mockResolvedValueOnce({
      succeeded: [
        { skill_id: "code-reviewer", agent_id: "codex", target_path: "/Users/test/.agents/skills/code-reviewer" },
        { skill_id: "code-reviewer", agent_id: "claude-code", target_path: "/Users/test/.claude/skills/code-reviewer" },
        { skill_id: "frontend-design", agent_id: "codex", target_path: "/Users/test/.agents/skills/frontend-design" },
        { skill_id: "frontend-design", agent_id: "claude-code", target_path: "/Users/test/.claude/skills/frontend-design" },
      ],
      skipped: [],
      failed: [],
    });
    renderCentralSkillsView();

    const selectionCheckboxes = screen.getAllByRole("checkbox");
    fireEvent.click(selectionCheckboxes[0]);
    fireEvent.click(selectionCheckboxes[1]);
    fireEvent.click(screen.getByTestId("bulk-bar-batch-install"));

    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: /将 2 个技能安装到 2 个平台/i }));

    await waitFor(() => {
      expect(mockBatchInstallSkills).toHaveBeenCalledWith(
        ["code-reviewer", "frontend-design"],
        ["codex", "claude-code"],
        "symlink",
        null
      );
    });
    expect(toast.success).toHaveBeenCalledWith("已将 2 个技能安装到 2 个平台");
    expect(mockRescan).toHaveBeenCalled();
  });

  it("treats skipped-only batch installs as non-failures", async () => {
    mockBatchInstallSkills.mockResolvedValueOnce({
      succeeded: [],
      skipped: [
        {
          skill_id: "frontend-design",
          agent_id: "claude-code",
          target_path: "/Users/test/.claude/skills/frontend-design",
          reason: "already_installed",
        },
      ],
      failed: [],
    });
    renderCentralSkillsView();

    const selectionCheckboxes = screen.getAllByRole("checkbox");
    fireEvent.click(selectionCheckboxes[0]);
    fireEvent.click(screen.getByTestId("bulk-bar-batch-install"));

    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByText("Universal (.agents/skills)"));
    fireEvent.click(within(dialog).getByRole("button", { name: /将 1 个技能安装到 1 个平台/i }));

    await waitFor(() => {
      expect(mockBatchInstallSkills).toHaveBeenCalledWith(
        ["code-reviewer"],
        ["claude-code"],
        "symlink",
        null
      );
    });
    expect(toast.error).not.toHaveBeenCalled();
    expect(toast.success).toHaveBeenCalledWith(
      "批量安装完成：成功 0，跳过 1 个已安装目标"
    );
    expect(mockRescan).toHaveBeenCalled();
  });

  it("reports mixed batch install success, skipped, and failed counts", async () => {
    mockBatchInstallSkills.mockResolvedValueOnce({
      succeeded: [
        { skill_id: "frontend-design", agent_id: "claude-code", target_path: "/Users/test/.claude/skills/frontend-design" },
      ],
      skipped: [
        {
          skill_id: "frontend-design",
          agent_id: "codex",
          target_path: "/Users/test/.agents/skills/frontend-design",
          reason: "already_installed",
        },
      ],
      failed: [
        {
          skill_id: "frontend-design",
          agent_id: "kiro",
          error: "A real directory already exists",
        },
      ],
    });
    renderCentralSkillsView({
      platformOverrides: {
        agents: [
          ...mockAgents,
          {
            id: "kiro",
            display_name: "Kiro",
            category: "coding",
            global_skills_dir: "/Users/test/.kiro/skills/",
            is_detected: true,
            is_builtin: true,
            is_enabled: true,
          },
        ],
      },
    });

    const selectionCheckboxes = screen.getAllByRole("checkbox");
    fireEvent.click(selectionCheckboxes[0]);
    fireEvent.click(screen.getByTestId("bulk-bar-batch-install"));

    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: /将 1 个技能安装到 3 个平台/i }));

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith(
        "批量安装完成：成功 1，跳过 1，失败 1"
      );
    });
    expect(dialog).toHaveTextContent("成功 1，跳过 1，失败 1");
  });

  it.skip("quick-filters installed skills and batch installs selected results", async () => {
    // V2: M2 拆掉了独立的 `installed-filter-*` 快速卡片，已安装筛选改走 ToolbarViewMenu 的
    // `central-toolbar-view-installed-*` 选项；"select results"/"install selected" 单击改由
    // BulkActionBar + 卡片 checkbox 提供，已无 1:1 对应入口，待后续重写为新交互链路。
    mockBatchInstallSkills.mockResolvedValueOnce({
      succeeded: [
        { skill_id: "frontend-design", agent_id: "kiro", target_path: "/Users/test/.kiro/skills/frontend-design" },
      ],
      skipped: [],
      failed: [],
    });
    renderCentralSkillsView({
      centralOverrides: {
        skills: [
          mockSkills[0]!,
          { ...mockSkills[1]!, linked_agents: [] },
        ],
      },
      platformOverrides: {
        agents: [
          ...mockAgents,
          {
            id: "kiro",
            display_name: "Kiro",
            category: "coding",
            global_skills_dir: "/Users/test/.kiro/skills/",
            is_detected: true,
            is_builtin: true,
            is_enabled: true,
          },
        ],
      },
    });

    fireEvent.click(screen.getByTestId("installed-filter-any"));

    await waitFor(() => {
      expect(screen.getByText("frontend-design")).toBeInTheDocument();
      expect(screen.queryByText("code-reviewer")).not.toBeInTheDocument();
    });

    fireEvent.click(screen.getByTestId("installed-filter-select-results"));
    fireEvent.click(screen.getByTestId("installed-filter-install-selected"));

    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByText("Universal (.agents/skills)"));
    fireEvent.click(within(dialog).getByText("Claude Code"));
    fireEvent.click(within(dialog).getByRole("button", { name: /将 1 个技能安装到 1 个平台/i }));

    await waitFor(() => {
      expect(mockBatchInstallSkills).toHaveBeenCalledWith(
        ["frontend-design"],
        ["kiro"],
        "symlink",
        null
      );
    });
  });
});
