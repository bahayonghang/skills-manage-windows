import { describe, it, expect, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";

import {
  useSkillCallCounts,
  clearSkillCallCountsCache,
} from "../hooks/useSkillCallCounts";
import { useTargetStore } from "../stores/targetStore";
import { ipcInvokeCalls, mockIpcCommand } from "./ipcMock";

beforeEach(() => {
  clearSkillCallCountsCache();
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

describe("useSkillCallCounts", () => {
  it("invokes usage_get_skill_counts with skill names + days", async () => {
    mockIpcCommand("usage_get_skill_counts", { review: 5, "git-commit": 12 });

    const { result } = renderHook(() =>
      useSkillCallCounts(["git-commit", "review"], 30),
    );

    await waitFor(() => {
      expect(result.current.review).toBe(5);
    });
    expect(ipcInvokeCalls("usage_get_skill_counts")).toEqual([
      {
        command: "usage_get_skill_counts",
        args: { skills: ["git-commit", "review"], days: 30 },
      },
    ]);
  });

  it("dedupes via module-level cache when names list is identical", async () => {
    mockIpcCommand("usage_get_skill_counts", { review: 3 });
    const { result: r1 } = renderHook(() => useSkillCallCounts(["review"], 30));
    await waitFor(() => expect(r1.current.review).toBe(3));

    // Second hook with the same names should hit cache and not re-invoke
    const { result: r2 } = renderHook(() => useSkillCallCounts(["review"], 30));
    await waitFor(() => expect(r2.current.review).toBe(3));
    expect(ipcInvokeCalls("usage_get_skill_counts")).toHaveLength(1);
  });

  it("keeps cache entries isolated by active target id", async () => {
    mockIpcCommand("usage_get_skill_counts", { review: 3 });
    const { result: localResult, unmount } = renderHook(() =>
      useSkillCallCounts(["review"], 30),
    );
    await waitFor(() => expect(localResult.current.review).toBe(3));
    unmount();

    mockIpcCommand("usage_get_skill_counts", { review: 9 });
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
      useSkillCallCounts(["review"], 30),
    );
    await waitFor(() => expect(remoteResult.current.review).toBe(9));
    // 目标切换后缓存不复用 —— 第二次真实 invoke
    expect(ipcInvokeCalls("usage_get_skill_counts")).toHaveLength(2);
  });

  it("returns empty map and silently swallows errors", async () => {
    mockIpcCommand("usage_get_skill_counts", () => {
      throw new Error("io");
    });
    const { result } = renderHook(() => useSkillCallCounts(["x"], 30));
    await waitFor(() =>
      expect(ipcInvokeCalls("usage_get_skill_counts")).toHaveLength(1),
    );
    // 错误被吞掉，state 维持空 map
    expect(result.current).toEqual({});
  });
});
