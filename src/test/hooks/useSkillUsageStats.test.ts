import { describe, it, expect, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";

import {
  useSkillUsageStats,
  clearSkillUsageStatsCache,
} from "@/hooks/useSkillUsageStats";
import { useTargetStore } from "@/stores/targetStore";
import { ipcInvokeCalls, mockIpcCommand } from "@/test/support/ipcMock";

beforeEach(() => {
  clearSkillUsageStatsCache();
  useTargetStore.setState({
    targets: [{ id: "local", kind: "local", label: "Local", isActive: true }],
    activeTarget: {
      id: "local",
      kind: "local",
      label: "Local",
      isActive: true,
    },
  });
});

describe("useSkillUsageStats", () => {
  it("invokes usage_get_skill_usage_stats with skill names + days null", async () => {
    mockIpcCommand("usage_get_skill_usage_stats", {
      review: { count: 5, lastUsedMs: 100 },
      "git-commit": { count: 12, lastUsedMs: 200 },
    });

    const { result } = renderHook(() =>
      useSkillUsageStats(["git-commit", "review"], { days: null }),
    );

    expect(result.current.ready).toBe(false);

    await waitFor(() => {
      expect(result.current.ready).toBe(true);
    });
    expect(result.current.stats.review).toEqual({ count: 5, lastUsedMs: 100 });
    expect(ipcInvokeCalls("usage_get_skill_usage_stats")).toEqual([
      {
        command: "usage_get_skill_usage_stats",
        args: { skills: ["git-commit", "review"], days: null },
      },
    ]);
  });

  it("dedupes via module-level cache when names list is identical", async () => {
    mockIpcCommand("usage_get_skill_usage_stats", {
      review: { count: 3, lastUsedMs: 10 },
    });
    const { result: r1 } = renderHook(() =>
      useSkillUsageStats(["review"], { days: null }),
    );
    await waitFor(() => expect(r1.current.stats.review.count).toBe(3));

    const { result: r2 } = renderHook(() =>
      useSkillUsageStats(["review"], { days: null }),
    );
    await waitFor(() => expect(r2.current.ready).toBe(true));
    expect(r2.current.stats.review.count).toBe(3);
    expect(ipcInvokeCalls("usage_get_skill_usage_stats")).toHaveLength(1);
  });

  it("keeps cache entries isolated by active target id and window", async () => {
    mockIpcCommand("usage_get_skill_usage_stats", {
      review: { count: 3, lastUsedMs: 10 },
    });
    const { result: localResult, unmount } = renderHook(() =>
      useSkillUsageStats(["review"], { days: null }),
    );
    await waitFor(() => expect(localResult.current.stats.review.count).toBe(3));
    unmount();

    mockIpcCommand("usage_get_skill_usage_stats", {
      review: { count: 9, lastUsedMs: 20 },
    });
    useTargetStore.setState({
      targets: [
        { id: "local", kind: "local", label: "Local", isActive: false },
        { id: "ssh-prod", kind: "ssh", label: "prod", isActive: true },
      ],
      activeTarget: {
        id: "ssh-prod",
        kind: "ssh",
        label: "prod",
        isActive: true,
      },
    });

    const { result: remoteResult } = renderHook(() =>
      useSkillUsageStats(["review"], { days: null }),
    );
    await waitFor(() => expect(remoteResult.current.stats.review.count).toBe(9));
    expect(ipcInvokeCalls("usage_get_skill_usage_stats")).toHaveLength(2);

    mockIpcCommand("usage_get_skill_usage_stats", {
      review: { count: 1, lastUsedMs: 5 },
    });
    const { result: windowed } = renderHook(() =>
      useSkillUsageStats(["review"], { days: 30 }),
    );
    await waitFor(() => expect(windowed.current.stats.review.count).toBe(1));
    expect(ipcInvokeCalls("usage_get_skill_usage_stats")).toHaveLength(3);
  });

  it("keeps ready false and swallows errors so an empty map is not all-zero", async () => {
    mockIpcCommand("usage_get_skill_usage_stats", () => {
      throw new Error("io");
    });
    const { result } = renderHook(() =>
      useSkillUsageStats(["x"], { days: null }),
    );
    await waitFor(() =>
      expect(ipcInvokeCalls("usage_get_skill_usage_stats")).toHaveLength(1),
    );
    expect(result.current.stats).toEqual({});
    expect(result.current.ready).toBe(false);
  });
});
