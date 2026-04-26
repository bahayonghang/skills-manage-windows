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

describe("targetStore", () => {
  beforeEach(() => {
    useTargetStore.setState({
      targets: [localTarget],
      activeTarget: localTarget,
      isLoading: false,
      isCreating: false,
      testingTargetId: null,
      updatingPasswordTargetId: null,
      switchingTargetId: null,
      deletingTargetId: null,
      error: null,
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
        message: "saved",
      })
      .mockResolvedValueOnce([localTarget, sshTarget]);

    const result = await useTargetStore
      .getState()
      .updateSshTargetPassword("ssh-demo", "secret");

    expect(result.ok).toBe(true);
    expect(invoke).toHaveBeenCalledWith("update_ssh_target_password", {
      targetId: "ssh-demo",
      password: "secret",
    });
    expect(invoke).toHaveBeenCalledWith("list_targets");
  });
});
