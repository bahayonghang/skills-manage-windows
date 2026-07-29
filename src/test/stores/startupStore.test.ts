import { beforeEach, describe, expect, it, vi } from "vitest";

import {
  resetStartupStoreForTests,
  useStartupStore,
} from "@/stores/startupStore";
import {
  ipcInvokeCalls,
  mockIpcCommand,
  mockIpcCommands,
} from "@/test/support/ipcMock";

describe("startupStore", () => {
  beforeEach(() => {
    resetStartupStoreForTests();
  });

  it("deduplicates initial status loading and enters ready", async () => {
    let resolveStatus!: (value: { phase: "ready" }) => void;
    mockIpcCommand(
      "get_startup_status",
      () =>
        new Promise((resolve) => {
          resolveStatus = resolve;
        }),
    );

    const first = useStartupStore.getState().loadStatus();
    const second = useStartupStore.getState().loadStatus();

    expect(ipcInvokeCalls("get_startup_status")).toHaveLength(1);
    resolveStatus({ phase: "ready" });
    await Promise.all([first, second]);
    expect(useStartupStore.getState().status).toEqual({ phase: "ready" });
  });

  it("converts a raw load failure into a safe fatal state", async () => {
    mockIpcCommand("get_startup_status", () => {
      throw new Error("C:\\private\\db.sqlite sqlx failure");
    });

    await useStartupStore.getState().loadStatus();

    expect(useStartupStore.getState()).toMatchObject({
      status: { phase: "fatal", issue: "startup_state_unavailable" },
      actionError: "load",
    });
    expect(JSON.stringify(useStartupStore.getState().status)).not.toContain("db.sqlite");
  });

  it("retries, rebuilds, and clears stale action errors", async () => {
    const recovery = {
      phase: "recovery_required" as const,
      issue: "database_open_failed" as const,
      diagnostic: "corrupt" as const,
      canRebuild: true,
      backupCreated: false,
    };
    mockIpcCommands({
      retry_startup: () => {
        throw new Error("retry failed");
      },
      rebuild_startup_database: { phase: "ready" },
    });
    useStartupStore.setState({ status: recovery });

    await useStartupStore.getState().retry();
    expect(useStartupStore.getState().actionError).toBe("retry");

    await useStartupStore.getState().rebuild();
    expect(useStartupStore.getState()).toMatchObject({
      status: { phase: "ready" },
      actionError: null,
      activeAction: null,
    });
  });

  it("prevents duplicate recovery actions while one is running", async () => {
    let resolveRetry!: (value: { phase: "ready" }) => void;
    mockIpcCommand(
      "retry_startup",
      () =>
        new Promise((resolve) => {
          resolveRetry = resolve;
        }),
    );

    const first = useStartupStore.getState().retry();
    const second = useStartupStore.getState().retry();

    expect(ipcInvokeCalls("retry_startup")).toHaveLength(1);
    resolveRetry({ phase: "ready" });
    await Promise.all([first, second]);
  });

  it("routes exit through typed startup IPC", async () => {
    const exit = vi.fn();
    mockIpcCommand("exit_startup", exit);

    await useStartupStore.getState().exit();

    expect(exit).toHaveBeenCalledTimes(1);
    expect(ipcInvokeCalls("exit_startup")).toHaveLength(1);
  });
});
