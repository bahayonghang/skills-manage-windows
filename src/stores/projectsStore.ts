import { create } from "zustand";

import { invoke, listen, isTauriRuntime, type UnlistenFn } from "@/lib/ipc";
import type {
  Project,
  ProjectScannedPayload,
  ProjectSkill,
} from "@/types";

interface ProjectsState {
  projects: Project[];
  currentProjectId: string | null;
  skillsByProject: Record<string, ProjectSkill[]>;
  scanningProjectIds: Set<string>;
  isLoading: boolean;
  error: string | null;

  loadProjects: () => Promise<void>;
  pickProjectFolder: () => Promise<string | null>;
  addProject: (path: string) => Promise<Project | null>;
  renameProject: (id: string, name: string) => Promise<void>;
  setPinned: (id: string, pinned: boolean) => Promise<void>;
  rescanProject: (id: string) => Promise<void>;
  getProjectSkills: (id: string) => Promise<void>;
  removeProject: (id: string, uninstallSkills: boolean) => Promise<void>;
  installSkillToProject: (
    projectId: string,
    skillId: string,
    agentId: string,
    method: "symlink" | "copy"
  ) => Promise<void>;
  uninstallSkillFromProject: (
    projectId: string,
    skillId: string,
    agentId: string
  ) => Promise<void>;
  setCurrentProjectId: (id: string | null) => void;
  clearError: () => void;
  ensureEventListener: () => Promise<void>;
}

let unlistenScanned: UnlistenFn | null = null;

async function refreshProjectSkills(
  projectId: string,
  set: (
    fn:
      | Partial<ProjectsState>
      | ((state: ProjectsState) => Partial<ProjectsState>)
  ) => void
): Promise<ProjectSkill[]> {
  const skills = await invoke<ProjectSkill[]>("get_project_skills", {
    id: projectId,
  });
  set((state) => ({
    skillsByProject: { ...state.skillsByProject, [projectId]: skills },
  }));
  return skills;
}

async function setupScannedListener(
  set: (
    fn:
      | Partial<ProjectsState>
      | ((state: ProjectsState) => Partial<ProjectsState>)
  ) => void
) {
  if (unlistenScanned || !isTauriRuntime()) {
    return;
  }
  unlistenScanned = await listen<ProjectScannedPayload>(
    "project:scanned",
    (event) => {
      const { projectId, skillCount } = event.payload;
      set((state) => {
        const nextScanning = new Set(state.scanningProjectIds);
        nextScanning.delete(projectId);
        return {
          projects: state.projects.map((p) =>
            p.id === projectId
              ? {
                  ...p,
                  skillCount,
                  lastScannedAt: new Date().toISOString(),
                }
              : p
          ),
          scanningProjectIds: nextScanning,
        };
      });
      void refreshProjectSkills(projectId, set).catch((err) => {
        set({ error: String(err) });
      });
    }
  );
}

export const useProjectsStore = create<ProjectsState>((set) => ({
  projects: [],
  currentProjectId: null,
  skillsByProject: {},
  scanningProjectIds: new Set<string>(),
  isLoading: false,
  error: null,

  ensureEventListener: async () => {
    await setupScannedListener(set);
  },

  loadProjects: async () => {
    set({ isLoading: true, error: null });
    if (!isTauriRuntime()) {
      set({ projects: [], isLoading: false });
      return;
    }
    try {
      await setupScannedListener(set);
      const list = await invoke<Project[]>("list_projects");
      set({ projects: list, isLoading: false });
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  pickProjectFolder: async () => {
    if (!isTauriRuntime()) {
      return null;
    }
    try {
      const picked = await invoke<string | null>("pick_project_folder");
      return picked ?? null;
    } catch (err) {
      set({ error: String(err) });
      return null;
    }
  },

  addProject: async (path: string) => {
    set({ error: null });
    if (!isTauriRuntime()) {
      return null;
    }
    try {
      await setupScannedListener(set);
      const project = await invoke<Project>("add_project", { path });
      set((state) => {
        const exists = state.projects.some((p) => p.id === project.id);
        const projects = exists
          ? state.projects.map((p) => (p.id === project.id ? project : p))
          : [...state.projects, project];
        const nextScanning = new Set(state.scanningProjectIds);
        nextScanning.add(project.id);
        return {
          projects,
          scanningProjectIds: nextScanning,
          currentProjectId: project.id,
        };
      });
      return project;
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },

  renameProject: async (id: string, name: string) => {
    if (!isTauriRuntime()) return;
    try {
      await invoke("rename_project", { id, name });
      set((state) => ({
        projects: state.projects.map((p) =>
          p.id === id ? { ...p, name } : p
        ),
      }));
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },

  setPinned: async (id: string, pinned: boolean) => {
    if (!isTauriRuntime()) return;
    set((state) => ({
      projects: state.projects.map((p) =>
        p.id === id ? { ...p, pinned } : p
      ),
    }));
    try {
      await invoke("set_project_pinned", { id, pinned });
    } catch (err) {
      set((state) => ({
        error: String(err),
        projects: state.projects.map((p) =>
          p.id === id ? { ...p, pinned: !pinned } : p
        ),
      }));
      throw err;
    }
  },

  rescanProject: async (id: string) => {
    if (!isTauriRuntime()) return;
    set((state) => {
      const next = new Set(state.scanningProjectIds);
      next.add(id);
      return { scanningProjectIds: next, error: null };
    });
    try {
      const skillCount = await invoke<number>("rescan_project", { id });
      set((state) => {
        const next = new Set(state.scanningProjectIds);
        next.delete(id);
        return {
          scanningProjectIds: next,
          projects: state.projects.map((p) =>
            p.id === id
              ? {
                  ...p,
                  skillCount,
                  lastScannedAt: new Date().toISOString(),
                }
              : p
          ),
        };
      });
    } catch (err) {
      set((state) => {
        const next = new Set(state.scanningProjectIds);
        next.delete(id);
        return { scanningProjectIds: next, error: String(err) };
      });
      throw err;
    }
  },

  getProjectSkills: async (id: string) => {
    if (!isTauriRuntime()) return;
    try {
      await refreshProjectSkills(id, set);
    } catch (err) {
      set({ error: String(err) });
    }
  },

  removeProject: async (id: string, uninstallSkills: boolean) => {
    if (!isTauriRuntime()) return;
    try {
      await invoke("remove_project", { id, uninstallSkills });
      set((state) => {
        const projects = state.projects.filter((p) => p.id !== id);
        const skillsByProject = { ...state.skillsByProject };
        delete skillsByProject[id];
        const currentProjectId =
          state.currentProjectId === id ? null : state.currentProjectId;
        const nextScanning = new Set(state.scanningProjectIds);
        nextScanning.delete(id);
        return {
          projects,
          skillsByProject,
          currentProjectId,
          scanningProjectIds: nextScanning,
        };
      });
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },

  setCurrentProjectId: (id) => set({ currentProjectId: id }),

  clearError: () => set({ error: null }),

  installSkillToProject: async (projectId, skillId, agentId, method) => {
    if (!isTauriRuntime()) return;
    try {
      await invoke("install_skill_to_project", {
        projectId,
        skillId,
        agentId,
        method,
      });
      const skills = await refreshProjectSkills(projectId, set);
      set((state) => ({
        projects: state.projects.map((p) =>
          p.id === projectId ? { ...p, skillCount: skills.length } : p
        ),
      }));
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },

  uninstallSkillFromProject: async (projectId, skillId, agentId) => {
    if (!isTauriRuntime()) return;
    try {
      await invoke("uninstall_skill_from_project", {
        projectId,
        skillId,
        agentId,
      });
      const skills = await refreshProjectSkills(projectId, set);
      set((state) => ({
        projects: state.projects.map((p) =>
          p.id === projectId ? { ...p, skillCount: skills.length } : p
        ),
      }));
    } catch (err) {
      set({ error: String(err) });
      throw err;
    }
  },
}));

export function _resetProjectsStoreForTests(): void {
  if (unlistenScanned) {
    void Promise.resolve(unlistenScanned()).catch(() => undefined);
    unlistenScanned = null;
  }
  useProjectsStore.setState({
    projects: [],
    currentProjectId: null,
    skillsByProject: {},
    scanningProjectIds: new Set<string>(),
    isLoading: false,
    error: null,
  });
}
