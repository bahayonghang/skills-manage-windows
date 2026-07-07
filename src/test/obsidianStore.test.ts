import { beforeEach, describe, expect, it } from "vitest";

import { useObsidianStore } from "../stores/obsidianStore";
import { ipcInvokeCalls, mockIpcCommand } from "./ipcMock";

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
  });

  it("loads vaults via Tauri invoke", async () => {
    mockIpcCommand("get_obsidian_vaults", mockVaults);

    await useObsidianStore.getState().loadVaults();

    expect(ipcInvokeCalls("get_obsidian_vaults")).toHaveLength(1);
    expect(useObsidianStore.getState().vaults).toEqual(mockVaults);
    expect(useObsidianStore.getState().isLoadingVaults).toBe(false);
  });

  it("loads vault skills via Tauri invoke", async () => {
    mockIpcCommand("get_obsidian_vault_skills", mockSkills);

    await useObsidianStore.getState().getVaultSkills("vault-1");

    expect(ipcInvokeCalls("get_obsidian_vault_skills")).toEqual([
      {
        command: "get_obsidian_vault_skills",
        args: { vaultId: "vault-1" },
      },
    ]);
    expect(useObsidianStore.getState().skillsByVault["vault-1"]).toEqual(
      mockSkills,
    );
    expect(useObsidianStore.getState().loadingSkillsByVault["vault-1"]).toBe(
      false,
    );
  });

  it("surfaces load errors as store error state", async () => {
    mockIpcCommand("get_obsidian_vaults", () => {
      throw new Error("scan failed");
    });

    await useObsidianStore.getState().loadVaults();

    expect(useObsidianStore.getState().error).toContain("scan failed");
    expect(useObsidianStore.getState().isLoadingVaults).toBe(false);
  });

  it("opens obsidian paths through the store action", async () => {
    mockIpcCommand("open_obsidian_path", undefined);

    await useObsidianStore.getState().openObsidianPath("/notes/vault-one");

    expect(ipcInvokeCalls("open_obsidian_path")).toEqual([
      {
        command: "open_obsidian_path",
        args: { path: "/notes/vault-one" },
      },
    ]);
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
