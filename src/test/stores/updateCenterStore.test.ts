import { beforeEach, describe, expect, it, vi } from "vitest";

const { mockInvoke, mockListen } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
  mockListen: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: mockInvoke,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: mockListen,
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ show: vi.fn() }),
}));

import { useUpdateCenterStore } from "@/stores/updateCenterStore";
import { ipcFixtureError } from "@/lib/ipc/errors";
import type {
  SkillUpdateApplyResult,
  SkillUpdateDecisions,
  SkillUpdateInventory,
} from "@/types/skillUpdateInventory";

function emptyInventory(): SkillUpdateInventory {
  return {
    updatable: [],
    remoteAdded: [],
    remoteMissing: [],
    platformDuplicates: [],
    deletedPlatformCopies: [],
    orphans: [],
    failedRepositories: [],
    generatedAt: "2026-06-11T00:00:00.000Z",
  };
}

describe("updateCenterStore", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    mockListen.mockReset();
    mockListen.mockResolvedValue(vi.fn());
    Object.defineProperty(window, "__TAURI__", {
      value: {},
      configurable: true,
    });
    useUpdateCenterStore.setState({
      inventory: null,
      isRefreshing: false,
      refreshProgress: null,
      isApplying: false,
      isForcing: false,
      lastRefreshedAt: null,
      error: null,
    });
  });

  it("bypasses the snapshot cache for manual refresh by default", async () => {
    mockInvoke.mockResolvedValueOnce(emptyInventory());

    await useUpdateCenterStore
      .getState()
      .refresh({ kind: "all", mode: "sync" });

    expect(mockInvoke).toHaveBeenCalledWith("refresh_skill_update_inventory", {
      scope: { kind: "all", mode: "sync", cachePolicy: "bypass" },
      operationId: expect.any(String),
    });
  });

  it("subscribes before invoke and tracks every active repository", async () => {
    type ProgressHandler = (event: {
      payload: {
        operationId: string;
        status: string;
        total: number;
        completed: number;
        repositoryKey?: string;
        repositoryName?: string;
      };
    }) => void;
    let progressHandler: ProgressHandler | undefined;
    const unlisten = vi.fn();
    mockListen.mockImplementation(async (_event, handler: ProgressHandler) => {
      progressHandler = handler;
      return unlisten;
    });
    let resolveInvoke: ((inventory: SkillUpdateInventory) => void) | undefined;
    mockInvoke.mockImplementation(
      () =>
        new Promise<SkillUpdateInventory>((resolve) => {
          resolveInvoke = resolve;
        }),
    );

    const refresh = useUpdateCenterStore
      .getState()
      .refresh({ kind: "all", mode: "sync" });

    await vi.waitFor(() => expect(mockInvoke).toHaveBeenCalledOnce());
    expect(mockListen.mock.invocationCallOrder[0]).toBeLessThan(
      mockInvoke.mock.invocationCallOrder[0],
    );
    const operationId = mockInvoke.mock.calls[0][1].operationId as string;
    progressHandler?.({
      payload: { operationId, status: "started", total: 3, completed: 0 },
    });
    progressHandler?.({
      payload: {
        operationId,
        status: "repository_started",
        total: 3,
        completed: 0,
        repositoryKey: "openai/skills/main",
        repositoryName: "openai/skills",
      },
    });
    progressHandler?.({
      payload: {
        operationId,
        status: "repository_started",
        total: 3,
        completed: 0,
        repositoryKey: "anthropics/skills/main",
        repositoryName: "anthropics/skills",
      },
    });

    expect(useUpdateCenterStore.getState().refreshProgress).toMatchObject({
      phase: "checking",
      total: 3,
      completed: 0,
      activeRepositories: [
        { key: "openai/skills/main", name: "openai/skills" },
        { key: "anthropics/skills/main", name: "anthropics/skills" },
      ],
    });

    progressHandler?.({
      payload: {
        operationId,
        status: "repository_completed",
        total: 3,
        completed: 1,
        repositoryKey: "openai/skills/main",
        repositoryName: "openai/skills",
      },
    });
    expect(useUpdateCenterStore.getState().refreshProgress).toMatchObject({
      completed: 1,
      activeRepositories: [
        { key: "anthropics/skills/main", name: "anthropics/skills" },
      ],
    });

    resolveInvoke?.(emptyInventory());
    await refresh;
    expect(unlisten).toHaveBeenCalledOnce();
    expect(useUpdateCenterStore.getState().refreshProgress).toBeNull();
  });

  it("ignores stale events and clears progress after failure", async () => {
    let progressHandler:
      ((event: { payload: Record<string, unknown> }) => void) | undefined;
    const unlisten = vi.fn();
    mockListen.mockImplementation(async (_event, handler) => {
      progressHandler = handler;
      return unlisten;
    });
    mockInvoke.mockRejectedValueOnce(
      ipcFixtureError("storage.unavailable", "network unavailable"),
    );

    const refresh = useUpdateCenterStore
      .getState()
      .refresh({ kind: "all", mode: "sync" });
    const rejection = expect(refresh).rejects.toThrow("network unavailable");
    await vi.waitFor(() => expect(mockInvoke).toHaveBeenCalledOnce());
    progressHandler?.({
      payload: {
        operationId: "stale-operation",
        status: "started",
        total: 99,
        completed: 50,
      },
    });

    await rejection;
    expect(unlisten).toHaveBeenCalledOnce();
    expect(useUpdateCenterStore.getState()).toMatchObject({
      isRefreshing: false,
      refreshProgress: null,
      error: "storage.unavailable:network unavailable",
    });
  });

  it("preserves the archive redirect code and drops legacy dynamic details", async () => {
    const seed =
      "ghp_secret https://example.invalid/private C:\\private\\SKILL.md";
    mockInvoke.mockRejectedValueOnce(
      `github_import.archive_redirect_rejected:${seed}`,
    );

    await expect(
      useUpdateCenterStore.getState().refresh({ kind: "all", mode: "sync" }),
    ).rejects.toThrow("GitHub repository archive redirect was rejected.");

    const error = useUpdateCenterStore.getState().error;
    expect(error).toBe(
      "github_import.archive_redirect_rejected:GitHub repository archive redirect was rejected.",
    );
    expect(error).not.toContain(seed);
  });

  it("passes a renderer-owned job ID when applying decisions", async () => {
    const decisions: SkillUpdateDecisions = {
      updates: [],
      keepMissing: [],
      deleteMissing: [],
      importAdditions: [],
      skipAdditions: [],
      unskipAdditions: [],
      removePlatformDuplicates: [],
      removeDeletedPlatformCopies: [],
    };
    const result: SkillUpdateApplyResult = {
      updatedSkillIds: [],
      keptMissingSkillIds: [],
      deletedSkillIds: [],
      importedSkillIds: [],
      skippedAdditions: [],
      unskippedAdditions: [],
      removedPlatformDuplicatePaths: [],
      removedDeletedPlatformCopyPaths: [],
      failures: [],
    };
    mockInvoke
      .mockResolvedValueOnce(result)
      .mockResolvedValueOnce(emptyInventory());

    await expect(
      useUpdateCenterStore.getState().apply(decisions, { kind: "all" }),
    ).resolves.toEqual(result);

    expect(mockInvoke).toHaveBeenNthCalledWith(
      1,
      "apply_skill_update_decisions",
      {
        jobId: expect.any(String),
        decisions,
      },
    );
  });
  describe("retryRepositories", () => {
    function inventoryWithFailure(repositoryId: string): SkillUpdateInventory {
      return {
        ...emptyInventory(),
        failedRepositories: [
          {
            repositoryId,
            error: "This repository could not be checked.",
            errorCode: "central_updates.repository_check_failed",
            retry: "retryable",
          },
        ],
      };
    }

    it("sends the panel scope, the targets, and no mode override by default", async () => {
      mockInvoke.mockResolvedValueOnce(emptyInventory());

      const result = await useUpdateCenterStore
        .getState()
        .retryRepositories(["repo-a", "repo-a"], { kind: "all", mode: "sync" });

      expect(mockInvoke).toHaveBeenCalledWith(
        "retry_failed_update_repositories",
        {
          scope: { kind: "all", mode: "sync", cachePolicy: "bypass" },
          repositoryIds: ["repo-a"],
          modeOverride: null,
          operationId: expect.any(String),
        },
      );
      expect(result).toEqual({ succeeded: ["repo-a"], failed: [] });
    });

    it("passes the incremental mode override for decision-required rows", async () => {
      mockInvoke.mockResolvedValueOnce(emptyInventory());

      await useUpdateCenterStore.getState().retryRepositories(
        ["repo-a"],
        { kind: "all", mode: "regular" },
        {
          mode: "sync",
        },
      );

      expect(mockInvoke.mock.calls[0][1]).toMatchObject({
        modeOverride: "sync",
      });
    });

    it("reports a repository that is still failing after the retry", async () => {
      mockInvoke.mockResolvedValueOnce(inventoryWithFailure("repo-a"));

      const result = await useUpdateCenterStore
        .getState()
        .retryRepositories(["repo-a", "repo-b"], { kind: "all", mode: "sync" });

      expect(result).toEqual({ succeeded: ["repo-b"], failed: ["repo-a"] });
      expect(
        useUpdateCenterStore.getState().inventory?.failedRepositories,
      ).toHaveLength(1);
    });

    it("marks the targets in flight and clears them when the call settles", async () => {
      let resolveInvoke:
        ((inventory: SkillUpdateInventory) => void) | undefined;
      mockInvoke.mockImplementation(
        () =>
          new Promise<SkillUpdateInventory>((resolve) => {
            resolveInvoke = resolve;
          }),
      );

      const pending = useUpdateCenterStore
        .getState()
        .retryRepositories(["repo-a"], { kind: "all", mode: "sync" });

      await vi.waitFor(() =>
        expect(useUpdateCenterStore.getState().retryingRepositoryIds).toEqual([
          "repo-a",
        ]),
      );
      resolveInvoke?.(emptyInventory());
      await pending;
      expect(useUpdateCenterStore.getState().retryingRepositoryIds).toEqual([]);
    });

    it("clears the in-flight marker and records the error when the call rejects", async () => {
      mockInvoke.mockRejectedValueOnce(
        ipcFixtureError("central_updates.repository_check_failed", "boom"),
      );

      await expect(
        useUpdateCenterStore
          .getState()
          .retryRepositories(["repo-a"], { kind: "all", mode: "sync" }),
      ).rejects.toBeTruthy();

      expect(useUpdateCenterStore.getState().retryingRepositoryIds).toEqual([]);
      expect(useUpdateCenterStore.getState().error).toBeTruthy();
    });

    it("skips the round trip when no repository is targeted", async () => {
      const result = await useUpdateCenterStore
        .getState()
        .retryRepositories([], { kind: "all", mode: "sync" });

      expect(result).toEqual({ succeeded: [], failed: [] });
      expect(mockInvoke).not.toHaveBeenCalled();
    });
  });
});
