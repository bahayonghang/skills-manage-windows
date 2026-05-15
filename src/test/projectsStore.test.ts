import { describe, it, expect, vi, beforeEach } from "vitest";
import type { Project, ProjectScannedPayload, ProjectSkill } from "../types";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

const listenMock = vi.fn();
vi.mock("@tauri-apps/api/event", () => ({
  listen: (...args: unknown[]) => listenMock(...args),
}));

import { invoke } from "@tauri-apps/api/core";
import {
  useProjectsStore,
  _resetProjectsStoreForTests,
} from "../stores/projectsStore";

const sampleProject: Project = {
  id: "abc123",
  path: "D:/Code/demo",
  name: "demo",
  pinned: false,
  addedAt: "2026-05-13T00:00:00Z",
  lastScannedAt: null,
  skillCount: 0,
};

const sampleSkill: ProjectSkill = {
  projectId: "abc123",
  skillId: "brainstorming",
  name: "brainstorming",
  description: "explore intent",
  filePath: "D:/Code/demo/.claude/skills/brainstorming/SKILL.md",
  sourceOrigin: "central",
  agentId: "claude-code",
  agentDisplayName: "Claude Code",
  installedPath: "D:/Code/demo/.claude/skills/brainstorming",
  linkType: "symlink",
  symlinkTarget: "C:/Users/x/.agents/skills/brainstorming",
};

describe("projectsStore", () => {
  let scannedHandler:
    | ((event: { payload: ProjectScannedPayload }) => void)
    | null = null;

  beforeEach(() => {
    _resetProjectsStoreForTests();
    vi.clearAllMocks();
    scannedHandler = null;
    listenMock.mockImplementation(async (_event: string, cb: typeof scannedHandler) => {
      scannedHandler = cb;
      return () => undefined;
    });
  });

  it("has correct initial state", () => {
    const state = useProjectsStore.getState();
    expect(state.projects).toEqual([]);
    expect(state.currentProjectId).toBeNull();
    expect(state.skillsByProject).toEqual({});
    expect(state.scanningProjectIds.size).toBe(0);
    expect(state.isLoading).toBe(false);
  });

  it("loadProjects fetches list_projects and sets store", async () => {
    vi.mocked(invoke).mockResolvedValueOnce([sampleProject]);

    await useProjectsStore.getState().loadProjects();

    expect(invoke).toHaveBeenCalledWith("list_projects");
    expect(useProjectsStore.getState().projects).toEqual([sampleProject]);
    expect(useProjectsStore.getState().isLoading).toBe(false);
  });

  it("addProject invokes add_project and marks project as scanning", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(sampleProject);

    const project = await useProjectsStore
      .getState()
      .addProject("D:/Code/demo");

    expect(invoke).toHaveBeenCalledWith("add_project", { path: "D:/Code/demo" });
    expect(project).toEqual(sampleProject);
    const state = useProjectsStore.getState();
    expect(state.projects).toContainEqual(sampleProject);
    expect(state.currentProjectId).toBe(sampleProject.id);
    expect(state.scanningProjectIds.has(sampleProject.id)).toBe(true);
  });

  it("project:scanned event clears scanning flag, updates count, and refreshes skills", async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce(sampleProject)
      .mockResolvedValueOnce([sampleSkill]);
    await useProjectsStore.getState().addProject("D:/Code/demo");

    expect(scannedHandler).not.toBeNull();
    scannedHandler!({ payload: { projectId: sampleProject.id, skillCount: 3 } });

    await vi.waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("get_project_skills", {
        id: sampleProject.id,
      });
    });

    const state = useProjectsStore.getState();
    expect(state.scanningProjectIds.has(sampleProject.id)).toBe(false);
    expect(state.projects[0].skillCount).toBe(3);
    expect(state.projects[0].lastScannedAt).not.toBeNull();
    expect(state.skillsByProject[sampleProject.id]).toEqual([sampleSkill]);
  });

  it("getProjectSkills populates skillsByProject", async () => {
    vi.mocked(invoke).mockResolvedValueOnce([sampleSkill]);

    await useProjectsStore.getState().getProjectSkills("abc123");

    expect(invoke).toHaveBeenCalledWith("get_project_skills", { id: "abc123" });
    expect(useProjectsStore.getState().skillsByProject["abc123"]).toEqual([
      sampleSkill,
    ]);
  });

  it("setPinned optimistically updates and persists via invoke", async () => {
    useProjectsStore.setState({ projects: [sampleProject] });
    vi.mocked(invoke).mockResolvedValueOnce(undefined);

    await useProjectsStore.getState().setPinned(sampleProject.id, true);

    expect(invoke).toHaveBeenCalledWith("set_project_pinned", {
      id: sampleProject.id,
      pinned: true,
    });
    expect(useProjectsStore.getState().projects[0].pinned).toBe(true);
  });

  it("renameProject persists and updates name", async () => {
    useProjectsStore.setState({ projects: [sampleProject] });
    vi.mocked(invoke).mockResolvedValueOnce(undefined);

    await useProjectsStore.getState().renameProject(sampleProject.id, "renamed");

    expect(invoke).toHaveBeenCalledWith("rename_project", {
      id: sampleProject.id,
      name: "renamed",
    });
    expect(useProjectsStore.getState().projects[0].name).toBe("renamed");
  });

  it("removeProject drops project and clears related state", async () => {
    useProjectsStore.setState({
      projects: [sampleProject],
      currentProjectId: sampleProject.id,
      skillsByProject: { [sampleProject.id]: [sampleSkill] },
    });
    vi.mocked(invoke).mockResolvedValueOnce(undefined);

    await useProjectsStore.getState().removeProject(sampleProject.id, false);

    expect(invoke).toHaveBeenCalledWith("remove_project", {
      id: sampleProject.id,
      uninstallSkills: false,
    });
    const state = useProjectsStore.getState();
    expect(state.projects).toEqual([]);
    expect(state.skillsByProject[sampleProject.id]).toBeUndefined();
    expect(state.currentProjectId).toBeNull();
  });

  it("setCurrentProjectId updates store", () => {
    useProjectsStore.getState().setCurrentProjectId("xyz");
    expect(useProjectsStore.getState().currentProjectId).toBe("xyz");
  });

  it("installSkillToProject invokes install + refresh", async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce(undefined) // install_skill_to_project
      .mockResolvedValueOnce([sampleSkill]); // get_project_skills

    await useProjectsStore.getState().installSkillToProject(
      "abc123",
      "brainstorming",
      "claude-code",
      "symlink"
    );

    expect(invoke).toHaveBeenNthCalledWith(1, "install_skill_to_project", {
      projectId: "abc123",
      skillId: "brainstorming",
      agentId: "claude-code",
      method: "symlink",
    });
    expect(invoke).toHaveBeenNthCalledWith(2, "get_project_skills", {
      id: "abc123",
    });
    expect(useProjectsStore.getState().skillsByProject["abc123"]).toEqual([
      sampleSkill,
    ]);
  });

  it("uninstallSkillFromProject invokes uninstall + refresh", async () => {
    useProjectsStore.setState({
      skillsByProject: { abc123: [sampleSkill] },
      projects: [{ ...sampleProject, skillCount: 1 }],
    });
    vi.mocked(invoke)
      .mockResolvedValueOnce(undefined) // uninstall_skill_from_project
      .mockResolvedValueOnce([]); // get_project_skills returns empty

    await useProjectsStore.getState().uninstallSkillFromProject(
      "abc123",
      "brainstorming",
      "claude-code"
    );

    expect(invoke).toHaveBeenNthCalledWith(1, "uninstall_skill_from_project", {
      projectId: "abc123",
      skillId: "brainstorming",
      agentId: "claude-code",
    });
    expect(useProjectsStore.getState().skillsByProject["abc123"]).toEqual([]);
    expect(useProjectsStore.getState().projects[0].skillCount).toBe(0);
  });
});
