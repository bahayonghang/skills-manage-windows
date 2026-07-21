import { renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";

import { useSkillExplanationSummaries } from "@/hooks/useSkillExplanationSummaries";

import { ipcInvokeCalls, ipcInvokedCommands, mockIpcCommand } from "@/test/support/ipcMock";

describe("useSkillExplanationSummaries", () => {
  it("loads cached summaries in one deduplicated batch without generation commands", async () => {
    mockIpcCommand("get_skill_explanation_summaries", { alpha: "中文总结" });

    const { result } = renderHook(() =>
      useSkillExplanationSummaries(["alpha", "beta", "alpha", "", null], "zh"),
    );

    await waitFor(() => {
      expect(result.current).toEqual({ alpha: "中文总结" });
    });

    expect(ipcInvokeCalls("get_skill_explanation_summaries")).toEqual([
      {
        command: "get_skill_explanation_summaries",
        args: { skillIds: ["alpha", "beta"], lang: "zh" },
      },
    ]);
    // 只发一次批量查询，不触发任何生成类命令（explain_skill_stream 等）
    expect(ipcInvokedCommands()).toEqual(["get_skill_explanation_summaries"]);
  });

  it("returns empty map and swallows command errors", async () => {
    mockIpcCommand("get_skill_explanation_summaries", () => {
      throw new Error("io");
    });

    const { result } = renderHook(() =>
      useSkillExplanationSummaries(["alpha"], "zh"),
    );

    await waitFor(() =>
      expect(ipcInvokeCalls("get_skill_explanation_summaries")).toHaveLength(1),
    );
    expect(result.current).toEqual({});
  });
});
