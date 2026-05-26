import { describe, it, expect, vi, beforeEach } from "vitest";
import { renderHook, waitFor } from "@testing-library/react";

import {
  useSkillCallCounts,
  clearSkillCallCountsCache,
} from "../hooks/useSkillCallCounts";
import { useTargetStore } from "../stores/targetStore";

vi.mock("@/lib/tauri", () => ({
  invoke: vi.fn(),
  isTauriRuntime: vi.fn(() => true),
}));

import { invoke, isTauriRuntime } from "@/lib/tauri";

const invokeMock = vi.mocked(invoke);
const runtimeMock = vi.mocked(isTauriRuntime);

beforeEach(() => {
  invokeMock.mockReset();
  runtimeMock.mockReset();
  runtimeMock.mockReturnValue(true);
  clearSkillCallCountsCache();
  useTargetStore.setState({
    targets: [{ id: "local", kind: "local", label: "Local", isActive: true }],
    activeTarget: { id: "local", kind: "local", label: "Local", isActive: true },
  });
});

describe("useSkillCallCounts", () => {
  it("invokes usage_get_skill_counts with sorted skill names + days", async () => {
    invokeMock.mockResolvedValue({ review: 5, "git-commit": 12 });

    const { result } = renderHook(() =>
      useSkillCallCounts(["git-commit", "review"], 30)
    );

    await waitFor(() => {
      expect(result.current.review).toBe(5);
    });
    expect(invokeMock).toHaveBeenCalledWith("usage_get_skill_counts", {
      skills: ["git-commit", "review"],
      days: 30,
    });
  });

  it("returns empty map when not running in Tauri", () => {
    runtimeMock.mockReturnValue(false);
    const { result } = renderHook(() => useSkillCallCounts(["x"], 30));
    expect(result.current).toEqual({});
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("dedupes via module-level cache when names list is identical", async () => {
    invokeMock.mockResolvedValue({ review: 3 });
    const { result: r1 } = renderHook(() => useSkillCallCounts(["review"], 30));
    await waitFor(() => expect(r1.current.review).toBe(3));

    invokeMock.mockClear();
    // Second hook with the same names should hit cache and not re-invoke
    const { result: r2 } = renderHook(() => useSkillCallCounts(["review"], 30));
    await waitFor(() => expect(r2.current.review).toBe(3));
    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("keeps cache entries isolated by active target id", async () => {
    invokeMock.mockResolvedValueOnce({ review: 3 });
    const { result: localResult, unmount } = renderHook(() =>
      useSkillCallCounts(["review"], 30)
    );
    await waitFor(() => expect(localResult.current.review).toBe(3));
    unmount();

    invokeMock.mockClear();
    invokeMock.mockResolvedValueOnce({ review: 9 });
    useTargetStore.setState({
      targets: [
        { id: "local", kind: "local", label: "Local", isActive: false },
        { id: "ssh-prod", kind: "ssh", label: "prod", isActive: true },
      ],
      activeTarget: { id: "ssh-prod", kind: "ssh", label: "prod", isActive: true },
    });

    const { result: remoteResult } = renderHook(() =>
      useSkillCallCounts(["review"], 30)
    );
    await waitFor(() => expect(remoteResult.current.review).toBe(9));
    expect(invokeMock).toHaveBeenCalledTimes(1);
  });

  it("returns empty map and silently swallows errors", async () => {
    invokeMock.mockRejectedValue(new Error("io"));
    const { result } = renderHook(() => useSkillCallCounts(["x"], 30));
    // After mount, error path keeps state at {}
    await waitFor(() => expect(result.current).toEqual({}));
  });
});
