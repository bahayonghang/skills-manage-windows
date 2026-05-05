import { describe, it, expect, beforeEach, afterEach, vi } from "vitest";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import type { AgentWithStatus, CentralSkillUpdateState, SkillWithLinks, TargetSummary } from "../types";
import * as S from "./centralSkillsViewTestSupport";

const {
  CentralSkillsView,
  toast,
  tauriBridge,
  mockAgents,
  mockSkills,
  mockRepositories,
  mockBatchInstallSkills,
  mockLoadBatchDeletePreview,
  mockDeleteCentralSkills,
  mockCheckSkillUpdates,
  mockKeepRemoteMissingSkills,
  mockRescan,
  mockUseCentralSkillsStore,
  mockUsePlatformStore,
  localTarget,
  renderCentralSkillsView,
  useTargetStore,
} = S;

describe("CentralSkillsView", () => {
  beforeEach(S.resetCentralSkillsViewTestState);
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
    fireEvent.click(screen.getByTestId("batch-install-central-skills"));

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
    fireEvent.click(screen.getByTestId("batch-install-central-skills"));

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
      failed: [
        { skill_id: "code-reviewer", agent_id: "claude-code", error: "already exists" },
      ],
    });
    renderCentralSkillsView();

    const selectionCheckboxes = screen.getAllByRole("checkbox");
    fireEvent.click(selectionCheckboxes[0]);
    fireEvent.click(selectionCheckboxes[1]);
    fireEvent.click(screen.getByTestId("batch-install-central-skills"));

    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: /将 2 个技能安装到 2 个平台/i }));

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith(
        "已请求将 2 个技能安装到 2 个平台，1 个目标失败"
      );
    });
    expect(screen.getByText("已请求将 2 个技能安装到 2 个平台，1 个目标失败")).toBeInTheDocument();
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
    fireEvent.click(screen.getByTestId("batch-install-central-skills"));

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
    fireEvent.click(screen.getByTestId("batch-delete-central-skills"));

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
    fireEvent.click(screen.getByTestId("batch-delete-central-skills"));

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
    fireEvent.click(screen.getByTestId("batch-delete-central-skills"));

    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText(/Universal/)).toBeInTheDocument();
    expect(within(dialog).getByText("Codex, Cursor")).toBeInTheDocument();
    expect(within(dialog).queryByText("Central Skills")).not.toBeInTheDocument();
  });


  it("opens remote-missing dialog after checking updates and keeps local skills by default", async () => {
    const remoteMissingState: CentralSkillUpdateState = {
      skill_id: "frontend-design",
      source_type: "github",
      source_url: "https://github.com/openai/skills",
      ref: "main",
      source_path: "skills/frontend-design",
      last_remote_hash: null,
      latest_remote_hash: null,
      last_checked_at: "2026-04-27T00:00:00Z",
      last_updated_at: null,
      status: "remote_missing",
      error: "Skill source path 'skills/frontend-design' no longer contains an importable skill.",
    };
    mockCheckSkillUpdates.mockResolvedValueOnce([remoteMissingState]);
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
    mockKeepRemoteMissingSkills.mockResolvedValueOnce(["frontend-design"]);
    renderCentralSkillsView();

    fireEvent.click(screen.getByRole("button", { name: /检查更新/i }));

    await waitFor(() => {
      expect(mockCheckSkillUpdates).toHaveBeenCalled();
      expect(mockLoadBatchDeletePreview).toHaveBeenCalledWith(["frontend-design"]);
    });
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText("远端已删除的技能")).toBeInTheDocument();
    expect(within(dialog).getByText("保留本地")).toBeInTheDocument();

    fireEvent.click(within(dialog).getByTestId("confirm-remote-missing-skills"));

    await waitFor(() => {
      expect(mockKeepRemoteMissingSkills).toHaveBeenCalledWith(["frontend-design"]);
    });
    expect(mockDeleteCentralSkills).not.toHaveBeenCalled();
    expect(mockRescan).toHaveBeenCalled();
    expect(toast.success).toHaveBeenCalledWith(
      "已处理远端缺失技能：保留 1 个，删除 0 个"
    );
  });


  it("deletes remote-missing local skills through the existing batch delete path", async () => {
    const remoteMissingState: CentralSkillUpdateState = {
      skill_id: "frontend-design",
      source_type: "github",
      source_url: "https://github.com/openai/skills",
      ref: "main",
      source_path: "skills/frontend-design",
      last_remote_hash: null,
      latest_remote_hash: null,
      last_checked_at: "2026-04-27T00:00:00Z",
      last_updated_at: null,
      status: "remote_missing",
      error: "Repository path 'skills/frontend-design' is no longer available.",
    };
    mockCheckSkillUpdates.mockResolvedValueOnce([remoteMissingState]);
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
      ],
      failed: [],
    });
    renderCentralSkillsView();

    fireEvent.click(screen.getByRole("button", { name: /检查更新/i }));

    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: /删除本地/i }));
    fireEvent.click(within(dialog).getByRole("checkbox", { name: /Cursor/i }));
    fireEvent.click(within(dialog).getByTestId("confirm-remote-missing-skills"));

    await waitFor(() => {
      expect(mockDeleteCentralSkills).toHaveBeenCalledWith([
        {
          skill_id: "frontend-design",
          remove_agent_ids: ["cursor"],
        },
      ]);
    });
    expect(mockKeepRemoteMissingSkills).not.toHaveBeenCalled();
    expect(mockRescan).toHaveBeenCalled();
  });


  it("renders browser fixture skill card on the localhost validation surface without Tauri", async () => {
    const isTauriSpy = vi.spyOn(tauriBridge, "isTauriRuntime").mockReturnValue(false);
    mockUseCentralSkillsStore.mockRestore();
    mockUsePlatformStore.mockRestore();

    render(
      <MemoryRouter>
        <CentralSkillsView />
      </MemoryRouter>
    );

    expect(await screen.findByRole("button", { name: /查看 fixture-central-skill 的详情/i })).toBeInTheDocument();

    isTauriSpy.mockRestore();
  });


  it("keeps the check-updates button enabled and triggers the backend on SSH targets", async () => {
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
    mockCheckSkillUpdates.mockResolvedValueOnce([]);
    renderCentralSkillsView();

    const checkButton = screen.getByRole("button", { name: /检查更新/i });
    expect(checkButton).not.toBeDisabled();

    fireEvent.click(checkButton);

    await waitFor(() => {
      expect(mockCheckSkillUpdates).toHaveBeenCalled();
    });
  });
});
