import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import * as S from "./centralSkillsViewTestSupport";
import { setFeatureFlag } from "../lib/featureFlags";

const {
  toast,
  mockRepositories,
  mockDeletePreview,
  mockBatchInstallSkills,
  mockAgents,
  mockSkills,
  mockLoadDeletePreview,
  mockLoadRepositoryDeletePreview,
  mockDeleteCentralSkill,
  mockDeleteSkillRepository,
  mockRescan,
  renderCentralSkillsView,
} = S;

describe("CentralSkillsView", () => {
  beforeEach(S.resetCentralSkillsViewTestState);
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
    renderCentralSkillsView();

    const sidebar = screen.getByTestId("central-filter-sidebar");
    fireEvent.click(within(sidebar).getByTestId("repository-filter-github-openai-skills-main"));
    await waitFor(() => {
      expect(screen.queryByText("code-reviewer")).not.toBeInTheDocument();
    });

    fireEvent.click(within(sidebar).getByTestId("repository-filter-github-openai-skills-main-delete"));

    expect(await screen.findByText("删除仓库：openai/skills")).toBeInTheDocument();
    expect(
      screen.getByText("frontend-design - ~/.skillsmanage/skills/frontend-design")
    ).toBeInTheDocument();
    fireEvent.click(screen.getByTestId("confirm-delete-skill-repository"));

    await waitFor(() => {
      expect(mockDeleteSkillRepository).toHaveBeenCalledWith(
        "github-openai-skills-main",
        [{ skill_id: "frontend-design", remove_agent_ids: [] }]
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
    renderCentralSkillsView();

    const sidebar = screen.getByTestId("central-filter-sidebar");
    fireEvent.click(within(sidebar).getByTestId("repository-filter-github-openai-skills-main-delete"));

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


  it("sorts by modified time and reverses direction explicitly", async () => {
    renderCentralSkillsView();

    fireEvent.click(screen.getByRole("button", { name: "修改时间" }));

    await waitFor(() => {
      const detailButtons = screen.getAllByRole("button", {
        name: /查看 .* 的详情/i,
      });
      expect(detailButtons[0]).toHaveTextContent("frontend-design");
      expect(detailButtons[1]).toHaveTextContent("code-reviewer");
    });

    fireEvent.click(screen.getByRole("button", { name: "倒排" }));

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


  it("shows delete button for each central skill", () => {
    renderCentralSkillsView();

    expect(screen.getByTestId("delete-central-skill-frontend-design")).toBeInTheDocument();
    expect(screen.getByTestId("delete-central-skill-code-reviewer")).toBeInTheDocument();
  });


  it("opens delete dialog and deletes selected platform copies", async () => {
    mockLoadDeletePreview.mockResolvedValueOnce(mockDeletePreview);
    mockDeleteCentralSkill.mockResolvedValueOnce(undefined);
    renderCentralSkillsView();

    fireEvent.click(screen.getByTestId("delete-central-skill-frontend-design"));

    await waitFor(() => {
      expect(mockLoadDeletePreview).toHaveBeenCalledWith("frontend-design");
    });
    expect(screen.getByText("/Users/test/.cursor/skills/frontend-design")).toBeInTheDocument();
    expect(screen.getByRole("dialog")).toHaveTextContent("Claude Code");

    fireEvent.click(screen.getByRole("checkbox", { name: /Cursor/i }));
    fireEvent.click(screen.getByRole("button", { name: /\u5220\u9664\u4e2d\u592e\u6280\u80fd/i }));

    await waitFor(() => {
      expect(mockDeleteCentralSkill).toHaveBeenCalledWith("frontend-design", ["cursor"]);
    });
    expect(mockRescan).toHaveBeenCalled();
  });


  it("shows batch delete only after selecting central skills", () => {
    renderCentralSkillsView();

    expect(screen.queryByTestId("batch-delete-central-skills")).not.toBeInTheDocument();
    expect(screen.queryByTestId("batch-install-central-skills")).not.toBeInTheDocument();

    const selectionCheckboxes = screen.getAllByRole("checkbox");
    fireEvent.click(selectionCheckboxes[0]);

    expect(screen.getByTestId("batch-delete-central-skills")).toBeInTheDocument();
    expect(screen.getByTestId("batch-install-central-skills")).toBeInTheDocument();
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
    fireEvent.click(screen.getByTestId("batch-install-central-skills"));

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
    fireEvent.click(screen.getByTestId("batch-install-central-skills"));

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
    fireEvent.click(screen.getByTestId("batch-install-central-skills"));

    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: /将 1 个技能安装到 3 个平台/i }));

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith(
        "批量安装完成：成功 1，跳过 1，失败 1"
      );
    });
    expect(dialog).toHaveTextContent("成功 1，跳过 1，失败 1");
  });

  it("switches from classic back to the new layout", async () => {
    setFeatureFlag("central.newLayout", true);
    renderCentralSkillsView();

    fireEvent.click(screen.getByTestId("central-v2-switch-classic"));
    expect(screen.getByTestId("central-switch-new-layout")).toBeInTheDocument();

    fireEvent.click(screen.getByTestId("central-switch-new-layout"));

    await waitFor(() => {
      expect(screen.getByTestId("central-v2-switch-classic")).toBeInTheDocument();
    });
  });

  it("quick-filters installed skills and batch installs selected results", async () => {
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
