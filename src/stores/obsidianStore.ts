import { create } from "zustand";
import { invoke } from "@/lib/ipc";
import type { ObsidianSkill, ObsidianVault } from "@/types";

interface ObsidianState {
  vaults: ObsidianVault[];
  skillsByVault: Record<string, ObsidianSkill[]>;
  isLoadingVaults: boolean;
  loadingSkillsByVault: Record<string, boolean>;
  error: string | null;
  loadVaults: () => Promise<void>;
  getVaultSkills: (vaultId: string) => Promise<void>;
  openObsidianPath: (path: string) => Promise<void>;
  resetForTargetChange: () => void;
}

async function importObsidianSkillToCentral(skill: ObsidianSkill) {
  return invoke("import_obsidian_skill_to_central", {
    dirPath: skill.dir_path,
  });
}

async function importObsidianSkillToPlatform(
  skill: ObsidianSkill,
  agentId: string,
  method?: "symlink" | "copy",
) {
  return invoke("import_obsidian_skill_to_platform", {
    dirPath: skill.dir_path,
    agentId,
    method,
  });
}

export const useObsidianStore = create<ObsidianState>((set) => ({
  vaults: [],
  skillsByVault: {},
  isLoadingVaults: false,
  loadingSkillsByVault: {},
  error: null,

  loadVaults: async () => {
    set({ isLoadingVaults: true, error: null });

    try {
      const vaults = await invoke("get_obsidian_vaults");
      set({ vaults: vaults ?? [], isLoadingVaults: false });
    } catch (err) {
      set({ error: String(err), isLoadingVaults: false });
    }
  },

  getVaultSkills: async (vaultId: string) => {
    set((state) => ({
      loadingSkillsByVault: {
        ...state.loadingSkillsByVault,
        [vaultId]: true,
      },
      error: null,
    }));

    try {
      const skills = await invoke("get_obsidian_vault_skills", { vaultId });
      set((state) => ({
        skillsByVault: {
          ...state.skillsByVault,
          [vaultId]: skills ?? [],
        },
        loadingSkillsByVault: {
          ...state.loadingSkillsByVault,
          [vaultId]: false,
        },
      }));
    } catch (err) {
      set((state) => ({
        error: String(err),
        loadingSkillsByVault: {
          ...state.loadingSkillsByVault,
          [vaultId]: false,
        },
      }));
    }
  },

  openObsidianPath: async (path) => {
    await invoke("open_obsidian_path", { path });
  },

  resetForTargetChange: () => {
    set({
      vaults: [],
      skillsByVault: {},
      isLoadingVaults: false,
      loadingSkillsByVault: {},
      error: null,
    });
  },
}));

export { importObsidianSkillToCentral, importObsidianSkillToPlatform };
