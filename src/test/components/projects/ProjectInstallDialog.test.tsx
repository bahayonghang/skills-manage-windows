import { describe, it, expect, vi, beforeEach } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";

import { ProjectInstallDialog } from "@/components/projects/ProjectInstallDialog";
import { getProjectPlatformTargetGroups } from "@/lib/platformTargetGroups";
import type {
  AgentWithStatus,
  Project,
  ProjectSkill,
  SkillWithLinks,
} from "@/types";

const mockProject: Project = {
  id: "abc123",
  path: "//?/D:/Code/demo",
  name: "demo",
  pinned: false,
  addedAt: "2026-05-13T00:00:00Z",
  lastScannedAt: null,
  skillCount: 0,
};

const mockSkill: SkillWithLinks = {
  id: "brainstorming",
  name: "brainstorming",
  description: "Explore intent before coding",
  file_path: "/x/.skillsmanage/skills/brainstorming/SKILL.md",
  is_central: true,
  scanned_at: "2026-05-13T00:00:00Z",
  linked_agents: [],
  shared_root_agents: [],
};

const mockAgents: AgentWithStatus[] = [
  {
    id: "claude-code",
    display_name: "Claude Code",
    category: "coding",
    global_skills_dir: "~/.claude/skills/",
    project_skills_dir: "~/.claude/skills/",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
  {
    id: "codex",
    display_name: "Codex",
    category: "coding",
    global_skills_dir: "~/.agents/skills/",
    project_skills_dir: ".agents/skills",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
  {
    id: "antigravity",
    display_name: "Antigravity",
    category: "coding",
    global_skills_dir: "~/.gemini/antigravity/skills/",
    project_skills_dir: ".agents/skills",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
  {
    id: "antigravity-cli",
    display_name: "Antigravity CLI",
    category: "coding",
    global_skills_dir: "~/.gemini/antigravity-cli/skills/",
    project_skills_dir: ".agents/skills",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
  {
    id: "central",
    display_name: "Central",
    category: "central",
    global_skills_dir: "~/.skillsmanage/skills/",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
];

const mockOnConfirm = vi.fn();
const mockOnOpenChange = vi.fn();

function renderDialog(overrides: { existingSkills?: ProjectSkill[] } = {}) {
  const platformTargets = getProjectPlatformTargetGroups(mockAgents, {
    coding: true,
    lobster: true,
  });

  render(
    <ProjectInstallDialog
      open={true}
      onOpenChange={mockOnOpenChange}
      project={mockProject}
      centralSkills={[mockSkill]}
      platformTargets={platformTargets}
      existingSkills={overrides.existingSkills ?? []}
      isInstalling={false}
      onConfirm={mockOnConfirm}
    />
  );
}

describe("ProjectInstallDialog", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders project title and lists eligible (project-pattern) agents", () => {
    renderDialog();
    expect(
      screen.getByText(/把中央 skill 装到项目「demo」/)
    ).toBeInTheDocument();
    expect(screen.getByText("Universal")).toBeInTheDocument();
    expect(screen.getByRole("checkbox", { name: /Claude Code/ })).toBeInTheDocument();
    expect(screen.queryByRole("checkbox", { name: /Antigravity/ })).not.toBeInTheDocument();
    expect(screen.queryByRole("checkbox", { name: /Antigravity CLI/ })).not.toBeInTheDocument();
    expect(screen.queryByText("Codex")).not.toBeInTheDocument();
    // central agent should be filtered out
    expect(screen.queryByText("Central")).not.toBeInTheDocument();
  });

  it("disables confirm button until a skill is picked", () => {
    renderDialog();
    const confirm = screen.getByRole("button", { name: /安装到 2 个平台/ });
    expect(confirm).toBeDisabled();

    fireEvent.click(screen.getByText("brainstorming"));
    expect(confirm).not.toBeDisabled();
  });

  it("invokes onConfirm with selected skill, agents, and method", async () => {
    mockOnConfirm.mockResolvedValueOnce(undefined);
    renderDialog();

    fireEvent.click(screen.getByText("brainstorming"));
    fireEvent.click(screen.getByRole("button", { name: /安装到 2 个平台/ }));

    await waitFor(() => {
      expect(mockOnConfirm).toHaveBeenCalledWith(
        "brainstorming",
        expect.arrayContaining(["claude-code", "codex"]),
        "symlink"
      );
    });
    const [, agentIds] = mockOnConfirm.mock.calls[0];
    expect(agentIds).not.toContain("antigravity");
    expect(agentIds).not.toContain("antigravity-cli");
  });

  it("passes the Universal representative agent when only Universal is selected", async () => {
    mockOnConfirm.mockResolvedValueOnce(undefined);
    renderDialog();

    fireEvent.click(screen.getByRole("checkbox", { name: /Claude Code/ }));
    fireEvent.click(screen.getByText("brainstorming"));
    fireEvent.click(screen.getByRole("button", { name: /安装到 1 个平台/ }));

    await waitFor(() => {
      expect(mockOnConfirm).toHaveBeenCalledWith(
        "brainstorming",
        ["codex"],
        "symlink"
      );
    });
  });

  it("shows existing-install hint when skill already installed for an agent", () => {
    const existing: ProjectSkill = {
      projectId: mockProject.id,
      skillId: "brainstorming",
      name: "brainstorming",
      description: null,
      filePath: "D:/Code/demo/.claude/skills/brainstorming/SKILL.md",
      sourceOrigin: "central",
      agentId: "claude-code",
      agentDisplayName: "Claude Code",
      installedPath: "D:/Code/demo/.claude/skills/brainstorming",
      linkType: "copy",
      symlinkTarget: null,
    };
    renderDialog({ existingSkills: [existing] });
    fireEvent.click(screen.getByText("brainstorming"));
    expect(screen.getByText(/替换现有 copy/)).toBeInTheDocument();
  });

  it("renders project paths in native Windows display form", () => {
    renderDialog();

    expect(
      screen.getByText(/skill 会落在项目根 D:\\Code\\demo 下对应的平台子目录。/)
    ).toBeInTheDocument();
  });
});
