import { beforeEach, describe, expect, it, vi } from "vitest";
import { TargetSummary } from "@/types";

vi.mock("@/lib/ipc", () => ({
  invoke: vi.fn(),
  isTauriRuntime: vi.fn(() => true),
}));

import { invoke } from "@/lib/ipc";
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

const healthyQuarantineStatus = {
  version: 1 as const,
  incidents: [],
  activeTargetReset: false,
};

const SECRET_SENTINEL = "target-secret-A7f4Q9x2Lm8P";

function expectSecretAbsentFromStore(): void {
  expect(JSON.stringify(useTargetStore.getState())).not.toContain(SECRET_SENTINEL);
}

describe("targetStore", () => {
  beforeEach(() => {
    useTargetStore.setState({
      targets: [localTarget],
      activeTarget: localTarget,
      quarantineStatus: null,
      quarantineStatusError: null,
      wslDistributions: [],
      isLoading: false,
      isLoadingWslDistributions: false,
      isCreating: false,
      updatingTargetId: null,
      testingTargetId: null,
      updatingPasswordTargetId: null,
      switchingTargetId: null,
      deletingTargetId: null,
      requiresTargetReload: false,
      error: null,
      wslDistributionError: null,
    });
    vi.clearAllMocks();
  });

  it("loads targets and resolves the active target", async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce([
        { ...localTarget, isActive: false },
        { ...sshTarget, isActive: true },
      ])
      .mockResolvedValueOnce(healthyQuarantineStatus);

    await useTargetStore.getState().loadTargets();

    expect(invoke).toHaveBeenCalledWith("list_targets");
    expect(invoke).toHaveBeenCalledWith("get_target_config_quarantine_status");
    expect(useTargetStore.getState().activeTarget.id).toBe("ssh-demo");
  });

  it("creates an ssh target through the backend command", async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce(sshTarget)
      .mockResolvedValueOnce([localTarget, sshTarget])
      .mockResolvedValueOnce(healthyQuarantineStatus);

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
      .mockResolvedValueOnce([localTarget, passwordTarget])
      .mockResolvedValueOnce(healthyQuarantineStatus);

    const result = await useTargetStore.getState().createSshTarget({
      label: "Lab",
      host: "lab.local",
      username: "alice",
      port: 22,
      authMethod: "password",
      password: SECRET_SENTINEL,
    });

    expect(result).toEqual(passwordTarget);
    expect(invoke).toHaveBeenCalledWith("create_ssh_target", {
      request: {
        label: "Lab",
        host: "lab.local",
        username: "alice",
        port: 22,
        authMethod: "password",
        password: SECRET_SENTINEL,
      },
    });
    expectSecretAbsentFromStore();
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
      .mockResolvedValueOnce([localTarget, updatedTarget])
      .mockResolvedValueOnce(healthyQuarantineStatus);

    const result = await useTargetStore.getState().updateSshTarget({
      id: "ssh-demo",
      label: "New Lab",
      host: "new.lab.local",
      username: "bob",
      port: 2222,
      authMethod: "password",
      password: SECRET_SENTINEL,
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
        password: SECRET_SENTINEL,
      },
    });
    expect(invoke).toHaveBeenCalledWith("list_targets");
    expectSecretAbsentFromStore();
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
      .mockResolvedValueOnce([localTarget, wslTarget])
      .mockResolvedValueOnce(healthyQuarantineStatus);

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
      .mockResolvedValueOnce([localTarget, updatedTarget])
      .mockResolvedValueOnce(healthyQuarantineStatus);

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

  it("tests a password ssh target without retaining the submitted secret", async () => {
    vi.mocked(invoke).mockResolvedValueOnce({
      ok: true,
      remoteHome: "/home/alice",
      remoteOs: "Linux",
      credentialStatus: "stored",
      message: "ok",
    });

    const result = await useTargetStore.getState().testSshTarget({
      id: "ssh-demo",
      password: SECRET_SENTINEL,
    });

    expect(result.ok).toBe(true);
    expect(invoke).toHaveBeenCalledWith("test_ssh_target", {
      request: {
        id: "ssh-demo",
        password: SECRET_SENTINEL,
      },
    });
    expectSecretAbsentFromStore();
  });

  it("switches active target and updates local state", async () => {
    useTargetStore.setState({ targets: [localTarget, sshTarget] });
    vi.mocked(invoke)
      .mockResolvedValueOnce({ ...sshTarget, isActive: true })
      .mockResolvedValueOnce([
        { ...localTarget, isActive: false },
        { ...sshTarget, isActive: true },
      ])
      .mockResolvedValueOnce(healthyQuarantineStatus);

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
      ])
      .mockResolvedValueOnce(healthyQuarantineStatus);

    const result = await useTargetStore
      .getState()
      .updateSshTargetPassword("ssh-demo", SECRET_SENTINEL);

    expect(result.ok).toBe(true);
    expect(result.credentialStatus).toBe("stored");
    expect(useTargetStore.getState().activeTarget.credentialStatus).toBe("stored");
    expect(invoke).toHaveBeenCalledWith("update_ssh_target_password", {
      targetId: "ssh-demo",
      password: SECRET_SENTINEL,
    });
    expect(invoke).toHaveBeenCalledWith("list_targets");
    expectSecretAbsentFromStore();
  });

  it.each([
    {
      action: "create",
      run: () =>
        useTargetStore.getState().createSshTarget({
          label: "Lab",
          host: "lab.local",
          username: "alice",
          authMethod: "password",
          password: SECRET_SENTINEL,
        }),
      idleState: { isCreating: false },
    },
    {
      action: "update",
      run: () =>
        useTargetStore.getState().updateSshTarget({
          id: "ssh-demo",
          label: "Lab",
          host: "lab.local",
          username: "alice",
          authMethod: "password",
          password: SECRET_SENTINEL,
        }),
      idleState: { updatingTargetId: null },
    },
    {
      action: "test",
      run: () =>
        useTargetStore.getState().testSshTarget({
          id: "ssh-demo",
          password: SECRET_SENTINEL,
        }),
      idleState: { testingTargetId: null },
    },
    {
      action: "password update",
      run: () =>
        useTargetStore
          .getState()
          .updateSshTargetPassword("ssh-demo", SECRET_SENTINEL),
      idleState: { updatingPasswordTargetId: null },
    },
    {
      action: "delete",
      run: () => useTargetStore.getState().deleteTarget("ssh-demo"),
      idleState: { deletingTargetId: null },
    },
    {
      action: "switch",
      run: () => useTargetStore.getState().switchTarget("local"),
      idleState: { switchingTargetId: null },
    },
  ])(
    "preserves target state and rethrows when the $action command rejects",
    async ({ run, idleState }) => {
      const activeSsh = { ...sshTarget, isActive: true };
      const targets = [{ ...localTarget, isActive: false }, activeSsh];
      useTargetStore.setState({ targets, activeTarget: activeSsh });
      const failure = new Error("target command failed");
      vi.mocked(invoke).mockRejectedValueOnce(failure);

      await expect(run()).rejects.toBe(failure);

      expect(invoke).toHaveBeenCalledTimes(1);
      expect(useTargetStore.getState()).toMatchObject({
        targets,
        activeTarget: activeSsh,
        error: String(failure),
        ...idleState,
      });
      expectSecretAbsentFromStore();
    },
  );

  it("keeps a successful create and marks reload required when refresh fails", async () => {
    const passwordTarget = {
      ...sshTarget,
      authMethod: "password" as const,
      credentialStatus: "stored" as const,
    };
    const refreshFailure = new Error("target refresh failed");
    vi.mocked(invoke)
      .mockResolvedValueOnce(passwordTarget)
      .mockRejectedValueOnce(refreshFailure)
      .mockResolvedValueOnce(healthyQuarantineStatus);

    await expect(
      useTargetStore.getState().createSshTarget({
        label: "Lab",
        host: "lab.local",
        username: "alice",
        authMethod: "password",
        password: SECRET_SENTINEL,
      }),
    ).resolves.toEqual(passwordTarget);

    expect(useTargetStore.getState()).toMatchObject({
      targets: [localTarget, passwordTarget],
      activeTarget: localTarget,
      isCreating: false,
      requiresTargetReload: true,
      error: String(refreshFailure),
    });
    expect(invoke).toHaveBeenCalledWith("create_ssh_target", expect.anything());
    expectSecretAbsentFromStore();
  });

  it("keeps stale target rows after a successful delete whose refresh fails", async () => {
    const targets = [localTarget, sshTarget];
    const refreshFailure = new Error("target refresh failed");
    useTargetStore.setState({ targets, activeTarget: localTarget });
    vi.mocked(invoke)
      .mockResolvedValueOnce(undefined)
      .mockRejectedValueOnce(refreshFailure)
      .mockResolvedValueOnce(healthyQuarantineStatus);

    await expect(
      useTargetStore.getState().deleteTarget("ssh-demo"),
    ).resolves.toBeUndefined();

    expect(useTargetStore.getState()).toMatchObject({
      targets,
      activeTarget: localTarget,
      deletingTargetId: null,
      requiresTargetReload: true,
      error: String(refreshFailure),
    });
    expect(invoke).toHaveBeenCalledWith("delete_target", { targetId: "ssh-demo" });
  });

  it("keeps a successful switch when the authoritative refresh fails", async () => {
    const activated = { ...sshTarget, isActive: true };
    const refreshFailure = new Error("target refresh failed");
    useTargetStore.setState({ targets: [localTarget, sshTarget] });
    vi.mocked(invoke)
      .mockResolvedValueOnce(activated)
      .mockRejectedValueOnce(refreshFailure)
      .mockResolvedValueOnce(healthyQuarantineStatus);

    await expect(
      useTargetStore.getState().switchTarget("ssh-demo"),
    ).resolves.toEqual(activated);

    expect(useTargetStore.getState()).toMatchObject({
      targets: [
        { ...localTarget, isActive: false },
        { ...sshTarget, isActive: true },
      ],
      activeTarget: activated,
      switchingTargetId: null,
      requiresTargetReload: true,
      error: String(refreshFailure),
    });
    expect(invoke).toHaveBeenCalledWith("set_active_target", {
      targetId: "ssh-demo",
    });
  });

  it("persists quarantine status across repeated target reloads", async () => {
    const status = {
      version: 1 as const,
      incidents: [
        {
          domain: "ssh" as const,
          detectedAt: "2026-07-27T10:00:00Z",
          reasonCode: "invalid_json",
          sourceBytes: 128,
          sourceSha256: "a".repeat(64),
        },
      ],
      activeTargetReset: true,
    };
    vi.mocked(invoke)
      .mockResolvedValueOnce([localTarget])
      .mockResolvedValueOnce(status)
      .mockResolvedValueOnce([localTarget])
      .mockResolvedValueOnce(status);

    await useTargetStore.getState().loadTargets();
    await useTargetStore.getState().loadTargets();

    expect(useTargetStore.getState().quarantineStatus).toEqual(status);
  });

  it("keeps the last quarantine status when the status command fails", async () => {
    useTargetStore.setState({ quarantineStatus: healthyQuarantineStatus });
    vi.mocked(invoke)
      .mockResolvedValueOnce([localTarget])
      .mockRejectedValueOnce("status unavailable");

    await useTargetStore.getState().loadTargets();

    expect(useTargetStore.getState().targets).toEqual([localTarget]);
    expect(useTargetStore.getState().quarantineStatus).toEqual(healthyQuarantineStatus);
    expect(useTargetStore.getState().quarantineStatusError).toBe("status unavailable");
  });
});
