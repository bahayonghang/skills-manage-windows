import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { ProjectsShell } from "../components/projects/ProjectsShell";
import { getPlatformTargetGroups } from "../lib/platformTargetGroups";
import type { AgentWithStatus, Project, ProjectSkill } from "../types";

const project: Project = {
  id: "project-1",
  path: "D:/Code/demo",
  name: "demo",
  pinned: false,
  addedAt: "2026-05-14T00:00:00Z",
  lastScannedAt: null,
  skillCount: 3,
};

const baseAgent = {
  category: "coding",
  global_skills_dir: "~/.agent/skills",
  is_detected: true,
  is_builtin: true,
  is_enabled: true,
} satisfies Omit<AgentWithStatus, "id" | "display_name">;

function agent(id: string, displayName: string): AgentWithStatus {
  return {
    ...baseAgent,
    id,
    display_name: displayName,
  };
}

function skill(
  skillId: string,
  agentId: string,
  agentDisplayName: string
): ProjectSkill {
  return {
    projectId: project.id,
    skillId,
    name: skillId,
    description: `${skillId} description`,
    filePath: `D:/Code/demo/${agentId}/${skillId}/SKILL.md`,
    sourceOrigin: "central",
    agentId,
    agentDisplayName,
    installedPath: `D:/Code/demo/${agentId}/${skillId}`,
    linkType: "symlink",
    symlinkTarget: `C:/Users/demo/.agents/skills/${skillId}`,
  };
}

function renderShell(overrides: Partial<Parameters<typeof ProjectsShell>[0]> = {}) {
  const platformTargets = getPlatformTargetGroups(
    [
      agent("codex", "Codex CLI"),
      agent("claude-code", "Claude Code"),
      agent("kiro", "Kiro"),
      agent("central", "Central Skills"),
    ],
    { coding: true, lobster: true }
  );

  const props = {
    projects: [project],
    currentProjectId: project.id,
    skills: [
      skill("universal-helper", "codex", "Codex CLI"),
      skill("claude-helper", "claude-code", "Claude Code"),
      skill("kiro-helper", "kiro", "Kiro"),
    ],
    platformTargets,
    isAddingProject: false,
    scanningProjectIds: new Set<string>(),
    uninstallingKeys: new Set<string>(),
    projectSearch: "",
    onProjectSearchChange: vi.fn(),
    onSelectProject: vi.fn(),
    onAddProject: vi.fn(),
    onRescanProject: vi.fn(),
    onOpenInstallDialog: vi.fn(),
    onUninstallSkill: vi.fn(),
    onTogglePin: vi.fn(),
    onRequestRename: vi.fn(),
    onRequestRemove: vi.fn(),
    ...overrides,
  };

  render(<ProjectsShell {...props} />);
  return props;
}

describe("ProjectsShell", () => {
  it("defaults to the first Sidebar platform target", () => {
    renderShell();

    expect(
      screen.getByRole("heading", { name: "Universal" })
    ).toBeInTheDocument();
    expect(screen.getByText("universal-helper")).toBeInTheDocument();
    expect(screen.queryByText("claude-helper")).not.toBeInTheDocument();
    expect(screen.queryByText("kiro-helper")).not.toBeInTheDocument();
  });

  it("renders the CLI sidebar with skill counts", () => {
    renderShell();

    const cliNav = screen.getByRole("navigation", { name: "项目 CLI 筛选" });
    expect(cliNav).toHaveTextContent("Universal");
    expect(cliNav).toHaveTextContent("Claude Code");
    expect(cliNav).toHaveTextContent("Kiro");
    expect(cliNav).not.toHaveTextContent("全部");

    expect(
      screen.getByRole("button", { name: /Universal1/ })
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /Claude Code1/ })
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /Kiro1/ })
    ).toBeInTheDocument();
  });

  it("filters the skill list to Universal when that CLI is selected", () => {
    renderShell();

    fireEvent.click(screen.getByRole("button", { name: /Claude Code1/ }));
    expect(screen.getByText("claude-helper")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /Universal1/ }));

    expect(screen.getByText("universal-helper")).toBeInTheDocument();
    expect(screen.queryByText("claude-helper")).not.toBeInTheDocument();
    expect(screen.queryByText("kiro-helper")).not.toBeInTheDocument();
  });

  it("keeps zero-count project-capable CLI entries and shows their empty state", () => {
    renderShell({
      skills: [skill("universal-helper", "codex", "Codex CLI")],
    });

    expect(
      screen.getByRole("button", { name: /Claude Code0/ })
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /Claude Code0/ }));

    expect(
      screen.getByText("该项目的 Claude Code 下还没有装入技能")
    ).toBeInTheDocument();
    expect(screen.queryByText("universal-helper")).not.toBeInTheDocument();
  });

  it("uses the grouped Universal label while uninstalling the raw project skill", async () => {
    const universalSkill = skill("universal-helper", "codex", "Codex CLI");
    const onUninstallSkill = vi.fn();
    renderShell({
      skills: [universalSkill],
      onUninstallSkill,
    });

    fireEvent.click(
      screen.getByRole("button", { name: "从 Universal 目录卸载" })
    );
    fireEvent.click(screen.getByRole("button", { name: "确认删除" }));

    await waitFor(() => {
      expect(onUninstallSkill).toHaveBeenCalledWith(universalSkill);
    });
  });
});
