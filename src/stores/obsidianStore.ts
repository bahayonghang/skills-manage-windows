import { create } from "zustand";
import { invoke, isTauriRuntime } from "@/lib/tauri";
import { DiscoveredSkill, ObsidianVault } from "@/types";

const BROWSER_FIXTURE_VAULTS: ObsidianVault[] = [
  {
    id: "fixture-vault",
    name: "Fixture Vault",
    path: "/Users/fixture/Notes/Fixture Vault",
    skill_count: 1,
  },
];

const BROWSER_FIXTURE_SKILLS: Record<string, DiscoveredSkill[]> = {
  "fixture-vault": [
    {
      id: "obsidian__fixture__fixture-skill",
      name: "fixture-skill",
      description: "Browser validation fixture for the Obsidian vault view.",
      file_path: "/Users/fixture/Notes/Fixture Vault/.skills/fixture-skill/SKILL.md",
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
  skillsByVault: Record<string, DiscoveredSkill[]>;
  isLoadingVaults: boolean;
  loadingSkillsByVault: Record<string, boolean>;
  error: string | null;
  loadVaults: () => Promise<void>;
  getVaultSkills: (vaultId: string) => Promise<void>;
  resetForTargetChange: () => void;
}

async function importToCentralFromDiscoveredOrSource(skill: DiscoveredSkill) {
  try {
    return await invoke("import_discovered_skill_to_central", {
      discoveredSkillId: skill.id,
    });
  } catch (err) {
    const message = String(err);
    if (!message.includes("not found")) {
      throw err;
    }
    return invoke("import_source_skill_to_central", {
      filePath: skill.file_path,
      dirPath: skill.dir_path,
    });
  }
}

async function importToPlatformFromDiscoveredOrSource(
  skill: DiscoveredSkill,
  agentId: string,
  method?: "symlink" | "copy"
) {
  try {
    return await invoke("import_discovered_skill_to_platform", {
      discoveredSkillId: skill.id,
      agentId,
      method,
    });
  } catch (err) {
    const message = String(err);
    if (!message.includes("not found")) {
      throw err;
    }
    return invoke("import_source_skill_to_platform", {
      filePath: skill.file_path,
      dirPath: skill.dir_path,
      agentId,
      method,
    });
  }
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
      const vaults = await invoke<ObsidianVault[]>("get_obsidian_vaults");
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
      const skills = await invoke<DiscoveredSkill[]>("get_obsidian_vault_skills", {
        vaultId,
      });
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

export { importToCentralFromDiscoveredOrSource, importToPlatformFromDiscoveredOrSource };
