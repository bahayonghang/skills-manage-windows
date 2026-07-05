import { create } from "zustand";
import { invoke, isTauriRuntime } from "@/lib/ipc";
import type { ObsidianSkill, ObsidianVault } from "@/types";

const BROWSER_FIXTURE_VAULTS: ObsidianVault[] = [
  {
    id: "fixture-vault",
    name: "Fixture Vault",
    path: "/Users/fixture/Notes/Fixture Vault",
    skill_count: 1,
  },
];

const BROWSER_FIXTURE_SKILLS: Record<string, ObsidianSkill[]> = {
  "fixture-vault": [
    {
      id: "obsidian__fixture__fixture-skill",
      name: "fixture-skill",
      description: "Browser validation fixture for the Obsidian vault view.",
      file_path:
        "/Users/fixture/Notes/Fixture Vault/.skills/fixture-skill/SKILL.md",
      dir_path: "/Users/fixture/Notes/Fixture Vault/.skills/fixture-skill",
      platform_id: "obsidian",
      platform_name: "Obsidian",
      project_path: "/Users/fixture/Notes/Fixture Vault",
      project_name: "Fixture Vault",
      is_already_central: false,
    },
  ],
};

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
    if (!isTauriRuntime()) {
      set({ vaults: BROWSER_FIXTURE_VAULTS, isLoadingVaults: false });
      return;
    }

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

    if (!isTauriRuntime()) {
      set((state) => ({
        skillsByVault: {
          ...state.skillsByVault,
          [vaultId]: BROWSER_FIXTURE_SKILLS[vaultId] ?? [],
        },
        loadingSkillsByVault: {
          ...state.loadingSkillsByVault,
          [vaultId]: false,
        },
      }));
      return;
    }

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
