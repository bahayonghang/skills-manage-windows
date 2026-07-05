import { beforeEach, describe, expect, it, vi } from "vitest";
import * as tauriBridge from "@/lib/ipc";
import { useObsidianStore } from "../stores/obsidianStore";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";

const mockVaults = [
  {
    id: "vault-1",
    name: "Vault One",
    path: "/notes/vault-one",
    skill_count: 2,
  },
];

const mockSkills = [
  {
    id: "obsidian__vault1__demo",
    name: "demo",
    description: "Demo skill",
    file_path: "/notes/vault-one/.skills/demo/SKILL.md",
    dir_path: "/notes/vault-one/.skills/demo",
    platform_id: "obsidian",
    platform_name: "Obsidian",
    project_path: "/notes/vault-one",
    project_name: "Vault One",
    is_already_central: false,
  },
];

describe("obsidianStore", () => {
  beforeEach(() => {
    useObsidianStore.setState({
      vaults: [],
      skillsByVault: {},
      isLoadingVaults: false,
      loadingSkillsByVault: {},
      error: null,
    });
    vi.clearAllMocks();
  });

  it("loads vaults via Tauri invoke", async () => {
    vi.spyOn(tauriBridge, "isTauriRuntime").mockReturnValue(true);
    vi.mocked(invoke).mockResolvedValueOnce(mockVaults);

    await useObsidianStore.getState().loadVaults();

    expect(invoke).toHaveBeenCalledWith("get_obsidian_vaults");
    expect(useObsidianStore.getState().vaults).toEqual(mockVaults);
    expect(useObsidianStore.getState().isLoadingVaults).toBe(false);
  });

  it("loads vault skills via Tauri invoke", async () => {
    vi.spyOn(tauriBridge, "isTauriRuntime").mockReturnValue(true);
    vi.mocked(invoke).mockResolvedValueOnce(mockSkills);

    await useObsidianStore.getState().getVaultSkills("vault-1");

    expect(invoke).toHaveBeenCalledWith("get_obsidian_vault_skills", {
      vaultId: "vault-1",
    });
    expect(useObsidianStore.getState().skillsByVault["vault-1"]).toEqual(
      mockSkills,
    );
    expect(useObsidianStore.getState().loadingSkillsByVault["vault-1"]).toBe(
      false,
    );
  });

  it("uses browser fixtures when Tauri runtime is unavailable", async () => {
    vi.spyOn(tauriBridge, "isTauriRuntime").mockReturnValue(false);

    await useObsidianStore.getState().loadVaults();
    await useObsidianStore.getState().getVaultSkills("fixture-vault");

    expect(invoke).not.toHaveBeenCalled();
    expect(useObsidianStore.getState().vaults).toHaveLength(1);
    expect(
      useObsidianStore.getState().skillsByVault["fixture-vault"],
    ).toHaveLength(1);
  });

  it("resets all cached state on target change", () => {
    useObsidianStore.setState({
      vaults: mockVaults,
      skillsByVault: { "vault-1": mockSkills },
      isLoadingVaults: true,
      loadingSkillsByVault: { "vault-1": true },
      error: "boom",
    });

    useObsidianStore.getState().resetForTargetChange();

    expect(useObsidianStore.getState()).toMatchObject({
      vaults: [],
      skillsByVault: {},
      isLoadingVaults: false,
      loadingSkillsByVault: {},
      error: null,
    });
  });
});
