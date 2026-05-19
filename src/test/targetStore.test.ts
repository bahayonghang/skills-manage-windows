import { beforeEach, describe, expect, it, vi } from "vitest";
import { TargetSummary } from "@/types";

vi.mock("@/lib/tauri", () => ({
  invoke: vi.fn(),
  isTauriRuntime: vi.fn(() => true),
}));

import { invoke } from "@/lib/tauri";
import { useTargetStore } from "@/stores/targetStore";

const localTarget: TargetSummary = {
  id: "local",
  kind: "local",
  label: "Local",
  isActive: true,
};

const sshTarget: TargetSummary = {
  id: "ssh-demo",
  kind: "ssh",
  label: "Lab",
  host: "lab.local",
  username: "alice",
  port: 22,
  remoteHome: "/home/alice",
  remoteOs: "Linux",
  cacheDbPath: "targets/ssh-demo/db.sqlite",
  isActive: false,
};

const wslTarget: TargetSummary = {
  id: "wsl-ubuntu",
  kind: "wsl",
  label: "Ubuntu",
  distribution: "Ubuntu-24.04",
  remoteHome: "/home/alice",
  remoteOs: "Linux",
  cacheDbPath: "targets/wsl-ubuntu/db.sqlite",
  symlinkEnabled: true,
  isActive: false,
};

describe("targetStore", () => {
  beforeEach(() => {
    useTargetStore.setState({
      targets: [localTarget],
      activeTarget: localTarget,
      wslDistributions: [],
      isLoading: false,
      isLoadingWslDistributions: false,
      isCreating: false,
      updatingTargetId: null,
      testingTargetId: null,
      updatingPasswordTargetId: null,
      switchingTargetId: null,
      deletingTargetId: null,
      error: null,
      wslDistributionError: null,
    });
    vi.clearAllMocks();
  });

  it("loads targets and resolves the active target", async () => {
    vi.mocked(invoke).mockResolvedValueOnce([
      { ...localTarget, isActive: false },
      { ...sshTarget, isActive: true },
    ]);

    await useTargetStore.getState().loadTargets();

    expect(invoke).toHaveBeenCalledWith("list_targets");
    expect(useTargetStore.getState().activeTarget.id).toBe("ssh-demo");
  });

  it("creates an ssh target through the backend command", async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce(sshTarget)
      .mockResolvedValueOnce([localTarget, sshTarget]);

    const result = await useTargetStore.getState().createSshTarget({
      label: "Lab",
      host: "lab.local",
      username: "alice",
      port: 22,
      keyPath: "C:/Users/alice/.ssh/id_ed25519",
    });

    expect(result).toEqual(sshTarget);
    expect(invoke).toHaveBeenCalledWith("create_ssh_target", {
      request: {
        label: "Lab",
        host: "lab.local",
        username: "alice",
        port: 22,
        keyPath: "C:/Users/alice/.ssh/id_ed25519",
      },
    });
  });

  it("passes password auth requests through without storing them in local state", async () => {
    const passwordTarget = { ...sshTarget, authMethod: "password" as const };
    vi.mocked(invoke)
      .mockResolvedValueOnce(passwordTarget)
      .mockResolvedValueOnce([localTarget, passwordTarget]);

    const result = await useTargetStore.getState().createSshTarget({
      label: "Lab",
      host: "lab.local",
      username: "alice",
      port: 22,
      authMethod: "password",
      password: "secret",
    });

    expect(result).toEqual(passwordTarget);
    expect(invoke).toHaveBeenCalledWith("create_ssh_target", {
      request: {
        label: "Lab",
        host: "lab.local",
        username: "alice",
        port: 22,
        authMethod: "password",
        password: "secret",
      },
    });
  });

  it("updates an ssh target through the backend command", async () => {
    const updatedTarget = {
      ...sshTarget,
      label: "New Lab",
      host: "new.lab.local",
      username: "bob",
      port: 2222,
      authMethod: "password" as const,
      credentialStatus: "stored" as const,
    };
    vi.mocked(invoke)
      .mockResolvedValueOnce(updatedTarget)
      .mockResolvedValueOnce([localTarget, updatedTarget]);

    const result = await useTargetStore.getState().updateSshTarget({
      id: "ssh-demo",
      label: "New Lab",
      host: "new.lab.local",
      username: "bob",
      port: 2222,
      authMethod: "password",
      password: "new-secret",
    });

    expect(result).toEqual(updatedTarget);
    expect(invoke).toHaveBeenCalledWith("update_ssh_target", {
      request: {
        id: "ssh-demo",
        label: "New Lab",
        host: "new.lab.local",
        username: "bob",
        port: 2222,
        authMethod: "password",
        password: "new-secret",
      },
    });
    expect(invoke).toHaveBeenCalledWith("list_targets");
  });

  it("loads WSL distributions through the backend command", async () => {
    vi.mocked(invoke).mockResolvedValueOnce([
      {
        name: "Ubuntu-24.04",
        isDefault: true,
        state: "Stopped",
        version: "2",
      },
    ]);

    await useTargetStore.getState().loadWslDistributions();

    expect(invoke).toHaveBeenCalledWith("list_wsl_distributions");
    expect(useTargetStore.getState().wslDistributions).toEqual([
      {
        name: "Ubuntu-24.04",
        isDefault: true,
        state: "Stopped",
        version: "2",
      },
    ]);
    expect(useTargetStore.getState().wslDistributionError).toBeNull();
  });

  it("stores WSL distribution discovery errors", async () => {
    vi.mocked(invoke).mockRejectedValueOnce("wsl.exe not found");

    await expect(useTargetStore.getState().loadWslDistributions()).rejects.toBe(
      "wsl.exe not found"
    );

    expect(useTargetStore.getState().wslDistributions).toEqual([]);
    expect(useTargetStore.getState().isLoadingWslDistributions).toBe(false);
    expect(useTargetStore.getState().wslDistributionError).toBe("wsl.exe not found");
  });

  it("creates a wsl target through the backend command", async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce(wslTarget)
      .mockResolvedValueOnce([localTarget, wslTarget]);

    const result = await useTargetStore.getState().createWslTarget({
      label: "Ubuntu",
      distribution: "Ubuntu-24.04",
    });

    expect(result).toEqual(wslTarget);
    expect(invoke).toHaveBeenCalledWith("create_wsl_target", {
      request: {
        label: "Ubuntu",
        distribution: "Ubuntu-24.04",
      },
    });
    expect(invoke).toHaveBeenCalledWith("list_targets");
  });

  it("updates a wsl target through the backend command", async () => {
    const updatedTarget = {
      ...wslTarget,
      label: "Work WSL",
      distribution: "Ubuntu",
    };
    vi.mocked(invoke)
      .mockResolvedValueOnce(updatedTarget)
      .mockResolvedValueOnce([localTarget, updatedTarget]);

    const result = await useTargetStore.getState().updateWslTarget({
      id: "wsl-ubuntu",
      label: "Work WSL",
      distribution: "Ubuntu",
    });

    expect(result).toEqual(updatedTarget);
    expect(invoke).toHaveBeenCalledWith("update_wsl_target", {
      request: {
        id: "wsl-ubuntu",
        label: "Work WSL",
        distribution: "Ubuntu",
      },
    });
    expect(invoke).toHaveBeenCalledWith("list_targets");
  });

  it("tests a wsl target through the backend command", async () => {
    vi.mocked(invoke).mockResolvedValueOnce({
      ok: true,
      remoteHome: "/home/alice",
      remoteOs: "Linux",
      message: "ok",
    });

    const result = await useTargetStore.getState().testWslTarget({
      distribution: "Ubuntu",
    });

    expect(result.ok).toBe(true);
    expect(invoke).toHaveBeenCalledWith("test_wsl_target", {
      request: {
        distribution: "Ubuntu",
      },
    });
  });

  it("switches active target and updates local state", async () => {
    useTargetStore.setState({ targets: [localTarget, sshTarget] });
    vi.mocked(invoke)
      .mockResolvedValueOnce({ ...sshTarget, isActive: true })
      .mockResolvedValueOnce([
        { ...localTarget, isActive: false },
        { ...sshTarget, isActive: true },
      ]);

    await useTargetStore.getState().switchTarget("ssh-demo");

    expect(invoke).toHaveBeenCalledWith("set_active_target", {
      targetId: "ssh-demo",
    });
    expect(useTargetStore.getState().activeTarget.id).toBe("ssh-demo");
  });

  it("updates a stored ssh target password through the backend command", async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce({
        ok: true,
        remoteHome: "/home/alice",
        remoteOs: "Linux",
        credentialStatus: "stored",
        message: "saved",
      })
      .mockResolvedValueOnce([
        { ...localTarget, isActive: false },
        { ...sshTarget, hasStoredPassword: true, credentialStatus: "stored", isActive: true },
      ]);

    const result = await useTargetStore
      .getState()
      .updateSshTargetPassword("ssh-demo", "secret");

    expect(result.ok).toBe(true);
    expect(result.credentialStatus).toBe("stored");
    expect(useTargetStore.getState().activeTarget.credentialStatus).toBe("stored");
    expect(invoke).toHaveBeenCalledWith("update_ssh_target_password", {
      targetId: "ssh-demo",
      password: "secret",
    });
    expect(invoke).toHaveBeenCalledWith("list_targets");
  });
});
