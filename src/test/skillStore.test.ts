import { describe, it, expect, vi, beforeEach } from "vitest";
import { ScannedSkill } from "../types";
import * as tauriBridge from "@/lib/ipc";
import { useSkillStore } from "../stores/skillStore";
import {
  ipcInvokeCalls,
  ipcInvokedCommands,
  mockIpcCommand,
  mockIpcCommands,
} from "./ipcMock";

// ─── Fixtures ─────────────────────────────────────────────────────────────────

const mockSkills: ScannedSkill[] = [
  {
    id: "frontend-design",
    name: "frontend-design",
    description: "Build distinctive, production-grade frontend interfaces",
    file_path: "~/.claude/skills/frontend-design/SKILL.md",
    dir_path: "~/.claude/skills/frontend-design",
    link_type: "symlink",
    symlink_target: "~/.skillsmanage/skills/frontend-design",
    is_central: true,
  },
  {
    id: "code-reviewer",
    name: "code-reviewer",
    description:
      "Review code changes and identify high-confidence, actionable bugs",
    file_path: "~/.claude/skills/code-reviewer/SKILL.md",
    dir_path: "~/.claude/skills/code-reviewer",
    link_type: "copy",
    is_central: false,
  },
];

// ─── Tests ────────────────────────────────────────────────────────────────────

describe("skillStore", () => {
  beforeEach(() => {
    // Reset store to initial state before each test
    useSkillStore.setState({
      skillsByAgent: {},
      loadingByAgent: {},
      pendingSkillActionKeys: {},
      error: null,
    });
  });

  // ── Initial State ─────────────────────────────────────────────────────────

  it("has correct initial state", () => {
    const state = useSkillStore.getState();
    expect(state.skillsByAgent).toEqual({});
    expect(state.loadingByAgent).toEqual({});
    expect(state.pendingSkillActionKeys).toEqual({});
    expect(state.error).toBeNull();
  });

  // ── getSkillsByAgent ──────────────────────────────────────────────────────

  it("calls invoke('get_skills_by_agent') with agentId (camelCase)", async () => {
    mockIpcCommand("get_skills_by_agent", mockSkills);

    await useSkillStore.getState().getSkillsByAgent("claude-code");

    expect(ipcInvokeCalls("get_skills_by_agent")[0].args).toEqual({
      agentId: "claude-code",
    });
  });

  it("populates skillsByAgent after successful fetch", async () => {
    mockIpcCommand("get_skills_by_agent", mockSkills);

    await useSkillStore.getState().getSkillsByAgent("claude-code");

    const state = useSkillStore.getState();
    expect(state.skillsByAgent["claude-code"]).toEqual(mockSkills);
    expect(state.loadingByAgent["claude-code"]).toBe(false);
    expect(state.error).toBeNull();
  });

  it("sets loading to true while fetching", async () => {
    let resolveSkills!: (value: ScannedSkill[]) => void;
    mockIpcCommand(
      "get_skills_by_agent",
      () => new Promise<ScannedSkill[]>((r) => (resolveSkills = r)),
    );

    const fetchPromise = useSkillStore
      .getState()
      .getSkillsByAgent("claude-code");

    // Loading should be true while the call is pending
    expect(useSkillStore.getState().loadingByAgent["claude-code"]).toBe(true);

    resolveSkills(mockSkills);
    await fetchPromise;

    expect(useSkillStore.getState().loadingByAgent["claude-code"]).toBe(false);
  });

  it("sets error and clears loading when fetch fails", async () => {
    mockIpcCommand("get_skills_by_agent", () =>
      Promise.reject(new Error("Agent not found")),
    );

    await useSkillStore.getState().getSkillsByAgent("claude-code");

    const state = useSkillStore.getState();
    expect(state.error).toContain("Agent not found");
    expect(state.loadingByAgent["claude-code"]).toBe(false);
    expect(state.skillsByAgent["claude-code"]).toBeUndefined();
  });

  it("can hold skills for multiple agents independently", async () => {
    const cursorSkills: ScannedSkill[] = [
      {
        id: "deploy",
        name: "deploy",
        description: "Deploy the application",
        file_path: "~/.cursor/skills/deploy/SKILL.md",
        dir_path: "~/.cursor/skills/deploy",
        link_type: "symlink",
        is_central: true,
      },
    ];

    mockIpcCommand("get_skills_by_agent", ({ agentId }: { agentId: string }) =>
      agentId === "cursor" ? cursorSkills : mockSkills,
    );

    await useSkillStore.getState().getSkillsByAgent("claude-code");
    await useSkillStore.getState().getSkillsByAgent("cursor");

    const state = useSkillStore.getState();
    expect(state.skillsByAgent["claude-code"]).toEqual(mockSkills);
    expect(state.skillsByAgent["cursor"]).toEqual(cursorSkills);
  });

  it("returns deterministic browser fixture skills when Tauri runtime is unavailable", async () => {
    const isTauriSpy = vi
      .spyOn(tauriBridge, "isTauriRuntime")
      .mockReturnValue(false);

    await useSkillStore.getState().getSkillsByAgent("claude-code");

    expect(ipcInvokeCalls()).toHaveLength(0);
    expect(useSkillStore.getState().skillsByAgent["claude-code"]).toEqual([
      expect.objectContaining({
        id: "fixture-central-skill",
        link_type: "symlink",
        is_central: true,
      }),
    ]);

    isTauriSpy.mockRestore();
  });

  // ── uninstallSkillFromAgent ──────────────────────────────────────────────

  it("calls uninstall_skill_from_agent and refreshes the agent skill list", async () => {
    useSkillStore.setState({
      skillsByAgent: { "claude-code": mockSkills },
      loadingByAgent: {},
      pendingSkillActionKeys: {},
      error: null,
    });

    const remainingSkills = [mockSkills[1]];
    mockIpcCommands({
      uninstall_skill_from_agent: undefined,
      get_skills_by_agent: remainingSkills,
    });

    await useSkillStore
      .getState()
      .uninstallSkillFromAgent("frontend-design", "claude-code");

    expect(ipcInvokedCommands()).toEqual([
      "uninstall_skill_from_agent",
      "get_skills_by_agent",
    ]);
    expect(ipcInvokeCalls("uninstall_skill_from_agent")[0].args).toEqual({
      skillId: "frontend-design",
      agentId: "claude-code",
    });
    expect(ipcInvokeCalls("get_skills_by_agent")[0].args).toEqual({
      agentId: "claude-code",
    });
    expect(useSkillStore.getState().skillsByAgent["claude-code"]).toEqual(
      remainingSkills,
    );
    expect(useSkillStore.getState().pendingSkillActionKeys).toEqual({});
    expect(useSkillStore.getState().error).toBeNull();
  });

  it("passes rowId and tracks pending state by Claude row identity", async () => {
    let resolveUninstall!: () => void;
    mockIpcCommands({
      uninstall_skill_from_agent: () =>
        new Promise<void>((resolve) => {
          resolveUninstall = resolve;
        }),
      get_skills_by_agent: [],
    });

    const uninstallPromise = useSkillStore
      .getState()
      .uninstallSkillFromAgent(
        "shared-skill",
        "claude-code",
        "claude-code::user::shared-skill",
      );

    expect(
      useSkillStore.getState().pendingSkillActionKeys[
        "claude-code::user::shared-skill"
      ],
    ).toBe(true);
    expect(
      useSkillStore.getState().pendingSkillActionKeys[
        "claude-code::shared-skill"
      ],
    ).toBeUndefined();

    resolveUninstall();
    await uninstallPromise;

    expect(ipcInvokeCalls("uninstall_skill_from_agent")[0].args).toEqual({
      skillId: "shared-skill",
      agentId: "claude-code",
      rowId: "claude-code::user::shared-skill",
    });
    expect(
      useSkillStore.getState().pendingSkillActionKeys[
        "claude-code::user::shared-skill"
      ],
    ).toBeUndefined();
  });

  it("tracks in-flight uninstall mutations by agent and skill", async () => {
    let resolveUninstall!: () => void;
    mockIpcCommands({
      uninstall_skill_from_agent: () =>
        new Promise<void>((resolve) => {
          resolveUninstall = resolve;
        }),
      get_skills_by_agent: [],
    });

    const uninstallPromise = useSkillStore
      .getState()
      .uninstallSkillFromAgent("frontend-design", "claude-code");

    expect(
      useSkillStore.getState().pendingSkillActionKeys[
        "claude-code::frontend-design"
      ],
    ).toBe(true);

    resolveUninstall();
    await uninstallPromise;

    expect(
      useSkillStore.getState().pendingSkillActionKeys[
        "claude-code::frontend-design"
      ],
    ).toBeUndefined();
  });

  it("sets error and clears pending uninstall state when uninstall fails", async () => {
    mockIpcCommand("uninstall_skill_from_agent", () =>
      Promise.reject(new Error("Permission denied")),
    );

    await expect(
      useSkillStore
        .getState()
        .uninstallSkillFromAgent("frontend-design", "claude-code"),
    ).rejects.toThrow("Permission denied");

    expect(useSkillStore.getState().error).toContain("Permission denied");
    expect(
      useSkillStore.getState().pendingSkillActionKeys[
        "claude-code::frontend-design"
      ],
    ).toBeUndefined();
  });

  it("batch uninstalls skills with row-aware IPC payload and refreshes the agent list", async () => {
    const remainingSkills = [mockSkills[1]];
    mockIpcCommands({
      batch_uninstall_skills_from_agent: {
        succeeded: [
          {
            skill_id: "frontend-design",
          },
          {
            skill_id: "shared-skill",
            row_id: "claude-code::user::shared-skill",
          },
        ],
        failed: [],
      },
      get_skills_by_agent: remainingSkills,
    });

    const result = await useSkillStore
      .getState()
      .batchUninstallSkillsFromAgent("claude-code", [
        { skill_id: "frontend-design" },
        { skill_id: "shared-skill", row_id: "claude-code::user::shared-skill" },
      ]);

    expect(ipcInvokedCommands()).toEqual([
      "batch_uninstall_skills_from_agent",
      "get_skills_by_agent",
    ]);
    expect(ipcInvokeCalls("batch_uninstall_skills_from_agent")[0].args).toEqual(
      {
        agentId: "claude-code",
        requests: [
          { skill_id: "frontend-design" },
          {
            skill_id: "shared-skill",
            row_id: "claude-code::user::shared-skill",
          },
        ],
      },
    );
    expect(ipcInvokeCalls("get_skills_by_agent")[0].args).toEqual({
      agentId: "claude-code",
    });
    expect(result.failed).toEqual([]);
    expect(useSkillStore.getState().skillsByAgent["claude-code"]).toEqual(
      remainingSkills,
    );
  });

  it("tracks batch uninstall pending state by row-level action keys", async () => {
    let resolveBatch!: (value: {
      succeeded: Array<{ skill_id: string; row_id?: string }>;
      failed: never[];
    }) => void;
    mockIpcCommands({
      batch_uninstall_skills_from_agent: () =>
        new Promise((resolve) => {
          resolveBatch = resolve;
        }),
      get_skills_by_agent: [],
    });

    const uninstallPromise = useSkillStore
      .getState()
      .batchUninstallSkillsFromAgent("claude-code", [
        { skill_id: "frontend-design" },
        { skill_id: "shared-skill", row_id: "claude-code::user::shared-skill" },
      ]);

    expect(
      useSkillStore.getState().pendingSkillActionKeys[
        "claude-code::frontend-design"
      ],
    ).toBe(true);
    expect(
      useSkillStore.getState().pendingSkillActionKeys[
        "claude-code::user::shared-skill"
      ],
    ).toBe(true);

    resolveBatch({
      succeeded: [
        { skill_id: "frontend-design" },
        { skill_id: "shared-skill", row_id: "claude-code::user::shared-skill" },
      ],
      failed: [],
    });
    await uninstallPromise;

    expect(useSkillStore.getState().pendingSkillActionKeys).toEqual({});
  });

  it("rejects batch uninstall outside the Tauri desktop runtime", async () => {
    const isTauriSpy = vi
      .spyOn(tauriBridge, "isTauriRuntime")
      .mockReturnValue(false);

    await expect(
      useSkillStore
        .getState()
        .batchUninstallSkillsFromAgent("claude-code", [
          { skill_id: "frontend-design" },
        ]),
    ).rejects.toThrow(
      "Uninstalling skills requires the Tauri desktop runtime.",
    );

    expect(ipcInvokeCalls()).toHaveLength(0);
    expect(useSkillStore.getState().error).toBe(
      "Uninstalling skills requires the Tauri desktop runtime.",
    );
    expect(useSkillStore.getState().pendingSkillActionKeys).toEqual({});

    isTauriSpy.mockRestore();
  });
});
