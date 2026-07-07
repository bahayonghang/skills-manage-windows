import { registerIpcFixtures } from "@/lib/ipc";
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

export function registerObsidianFixtures(): void {
  registerIpcFixtures({
    get_obsidian_vaults: () => BROWSER_FIXTURE_VAULTS,
    get_obsidian_vault_skills: ({ vaultId }) =>
      BROWSER_FIXTURE_SKILLS[vaultId] ?? [],
  });
}
