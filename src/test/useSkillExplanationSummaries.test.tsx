import { renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@/lib/ipc", () => ({
  invoke: vi.fn(),
  isTauriRuntime: vi.fn(),
}));

import { invoke, isTauriRuntime } from "@/lib/ipc";
import { useSkillExplanationSummaries } from "@/hooks/useSkillExplanationSummaries";

const mockInvoke = vi.mocked(invoke);
const mockIsTauriRuntime = vi.mocked(isTauriRuntime);

describe("useSkillExplanationSummaries", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    mockIsTauriRuntime.mockReset();
    mockIsTauriRuntime.mockReturnValue(true);
  });

  it("loads cached summaries in one deduplicated batch without generation commands", async () => {
    mockInvoke.mockResolvedValue({ alpha: "中文总结" });

    const { result } = renderHook(() =>
      useSkillExplanationSummaries(["alpha", "beta", "alpha", "", null], "zh")
    );

    await waitFor(() => {
      expect(result.current).toEqual({ alpha: "中文总结" });
    });

    expect(mockInvoke).toHaveBeenCalledTimes(1);
    expect(mockInvoke).toHaveBeenCalledWith("get_skill_explanation_summaries", {
      skillIds: ["alpha", "beta"],
      lang: "zh",
    });
    expect(mockInvoke).not.toHaveBeenCalledWith(
      "explain_skill_stream",
      expect.anything()
    );
    expect(mockInvoke).not.toHaveBeenCalledWith(
      "refresh_skill_explanation",
      expect.anything()
    );
  });

  it("does not call IPC outside the Tauri runtime", () => {
    mockIsTauriRuntime.mockReturnValue(false);

    const { result } = renderHook(() => useSkillExplanationSummaries(["alpha"], "zh"));

    expect(result.current).toEqual({});
    expect(mockInvoke).not.toHaveBeenCalled();
  });
});
