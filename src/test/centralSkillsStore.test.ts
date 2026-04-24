import { describe, it, expect, vi, beforeEach } from "vitest";
import { AgentWithStatus, SkillRepositoryWithStats, SkillTag, SkillWithLinks } from "../types";
import * as tauriBridge from "@/lib/tauri";

// Mock Tauri core before importing the store
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCentralSkillsStore } from "../stores/centralSkillsStore";

// ─── Fixtures ─────────────────────────────────────────────────────────────────

const mockSkills: SkillWithLinks[] = [
  {
    id: "frontend-design",
    name: "frontend-design",
    description: "Build distinctive frontend UIs",
    file_path: "~/.agents/skills/frontend-design/SKILL.md",
    canonical_path: "~/.agents/skills/frontend-design",
    is_central: true,
    scanned_at: "2026-04-09T00:00:00Z",
    linked_agents: ["claude-code", "cursor"],
    shared_root_agents: [],
  },
  {
    id: "code-reviewer",
    name: "code-reviewer",
    description: "Review code changes and identify bugs",
    file_path: "~/.agents/skills/code-reviewer/SKILL.md",
    canonical_path: "~/.agents/skills/code-reviewer",
    is_central: true,
    scanned_at: "2026-04-09T00:00:00Z",
    linked_agents: [],
    shared_root_agents: [],
  },
];

const mockAgents: AgentWithStatus[] = [
  {
    id: "claude-code",
    display_name: "Claude Code",
    category: "coding",
    global_skills_dir: "~/.claude/skills/",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
  {
    id: "cursor",
    display_name: "Cursor",
    category: "coding",
    global_skills_dir: "~/.cursor/skills/",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
  {
    id: "central",
    display_name: "Central Skills",
    category: "central",
    global_skills_dir: "~/.agents/skills/",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
];

const mockRepositories: SkillRepositoryWithStats[] = [
  {
    id: "local-unknown",
    name: "本地 / 未知来源",
    source_type: "local",
    is_unknown: true,
    created_at: "2026-04-17T00:00:00.000Z",
    updated_at: "2026-04-17T00:00:00.000Z",
    skill_count: 1,
    unknown_skill_count: 1,
  },
];

const mockTags: SkillTag[] = [
  {
    id: "programming-agent-engineering",
    name: "编程与 Agent 工程",
    is_builtin: true,
    created_at: "2026-04-17T00:00:00.000Z",
    updated_at: "2026-04-17T00:00:00.000Z",
  },
];

// ─── Tests ────────────────────────────────────────────────────────────────────

describe("centralSkillsStore", () => {
  beforeEach(() => {
    useCentralSkillsStore.setState({
      skills: [],
      agents: [],
      repositories: [],
      tags: [],
      aiTagReviews: [],
      aiTagJob: {
        jobId: null,
        status: "idle",
        total: 0,
        completed: 0,
        succeeded: 0,
        failed: 0,
        lowConfidenceCount: 0,
        items: {},
      },
      aiTaggingAvailable: false,
      isLoading: false,
      isInstalling: false,
      isMetadataUpdating: false,
      isSuggestingTags: false,
      togglingAgentId: null,
      error: null,
    });
    vi.clearAllMocks();
  });

  // ── Initial State ─────────────────────────────────────────────────────────

  it("has correct initial state", () => {
    const state = useCentralSkillsStore.getState();
    expect(state.skills).toEqual([]);
    expect(state.agents).toEqual([]);
    expect(state.repositories).toEqual([]);
    expect(state.tags).toEqual([]);
    expect(state.aiTagReviews).toEqual([]);
    expect(state.aiTagJob.status).toBe("idle");
    expect(state.aiTaggingAvailable).toBe(false);
    expect(state.isLoading).toBe(false);
    expect(state.isInstalling).toBe(false);
    expect(state.isMetadataUpdating).toBe(false);
    expect(state.isSuggestingTags).toBe(false);
    expect(state.togglingAgentId).toBeNull();
    expect(state.error).toBeNull();
  });

  // ── loadCentralSkills ─────────────────────────────────────────────────────

  it("calls get_central_skills and get_agents on loadCentralSkills", async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce(mockSkills) // get_central_skills
      .mockResolvedValueOnce(mockAgents) // get_agents
      .mockResolvedValueOnce(mockRepositories) // get_skill_repositories
      .mockResolvedValueOnce(mockTags) // get_skill_tags
      .mockResolvedValueOnce([]) // get_pending_ai_tag_reviews
      .mockResolvedValueOnce("test-key"); // get_setting

    await useCentralSkillsStore.getState().loadCentralSkills();

    expect(invoke).toHaveBeenCalledWith("get_central_skills");
    expect(invoke).toHaveBeenCalledWith("get_agents");
    expect(invoke).toHaveBeenCalledWith("get_skill_repositories");
    expect(invoke).toHaveBeenCalledWith("get_skill_tags");
    expect(invoke).toHaveBeenCalledWith("get_pending_ai_tag_reviews");
    expect(invoke).toHaveBeenCalledWith("get_setting", { key: "ai_api_key" });
  });

  it("populates skills and agents after successful loadCentralSkills", async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce(mockSkills)
      .mockResolvedValueOnce(mockAgents)
      .mockResolvedValueOnce(mockRepositories)
      .mockResolvedValueOnce(mockTags)
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce("test-key");

    await useCentralSkillsStore.getState().loadCentralSkills();

    const state = useCentralSkillsStore.getState();
    expect(state.skills).toEqual(mockSkills);
    expect(state.agents).toEqual(mockAgents);
    expect(state.repositories).toEqual(mockRepositories);
    expect(state.tags).toEqual(mockTags);
    expect(state.aiTagReviews).toEqual([]);
    expect(state.aiTaggingAvailable).toBe(true);
    expect(state.isLoading).toBe(false);
    expect(state.error).toBeNull();
  });

  it("sets error when loadCentralSkills fails", async () => {
    vi.mocked(invoke).mockRejectedValueOnce(new Error("DB error"));

    await useCentralSkillsStore.getState().loadCentralSkills();

    const state = useCentralSkillsStore.getState();
    expect(state.error).toContain("DB error");
    expect(state.isLoading).toBe(false);
  });

  it("returns deterministic browser fixture data when Tauri runtime is unavailable", async () => {
    const isTauriSpy = vi.spyOn(tauriBridge, "isTauriRuntime").mockReturnValue(false);

    await useCentralSkillsStore.getState().loadCentralSkills();

    expect(invoke).not.toHaveBeenCalled();
    const state = useCentralSkillsStore.getState();
    expect(state.skills).toEqual([
      expect.objectContaining({
        id: "fixture-central-skill",
        linked_agents: ["claude-code", "cursor"],
        shared_root_agents: ["cursor"],
        is_source_unknown: true,
      }),
    ]);
    expect(state.agents).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ id: "claude-code" }),
        expect.objectContaining({ id: "central" }),
      ])
    );
    expect(state.aiTagReviews).toEqual([]);

    isTauriSpy.mockRestore();
  });

  // ── installSkill ──────────────────────────────────────────────────────────

  it("calls batch_install_to_agents then refreshes skills", async () => {
    const batchResult = { succeeded: ["cursor"], failed: [] };
    const updatedSkills = [
      { ...mockSkills[0], linked_agents: ["claude-code", "cursor", "gemini-cli"] },
      mockSkills[1],
    ];

    vi.mocked(invoke)
      .mockResolvedValueOnce(batchResult) // batch_install_to_agents
      .mockResolvedValueOnce(updatedSkills) // get_central_skills (refresh)
      .mockResolvedValueOnce(mockRepositories); // get_skill_repositories

    await useCentralSkillsStore
      .getState()
      .installSkill("frontend-design", ["cursor"], "symlink");

    expect(invoke).toHaveBeenCalledWith("batch_install_to_agents", {
      skillId: "frontend-design",
      agentIds: ["cursor"],
      method: "symlink",
    });
    // Refresh call
    expect(invoke).toHaveBeenCalledWith("get_central_skills");

    const state = useCentralSkillsStore.getState();
    expect(state.skills).toEqual(updatedSkills);
    expect(state.isInstalling).toBe(false);
  });

  it("forwards 'copy' method to batch_install_to_agents", async () => {
    const batchResult = { succeeded: ["cursor"], failed: [] };
    vi.mocked(invoke)
      .mockResolvedValueOnce(batchResult)
      .mockResolvedValueOnce(mockSkills)
      .mockResolvedValueOnce(mockRepositories);

    await useCentralSkillsStore
      .getState()
      .installSkill("frontend-design", ["cursor"], "copy");

    expect(invoke).toHaveBeenCalledWith("batch_install_to_agents", {
      skillId: "frontend-design",
      agentIds: ["cursor"],
      method: "copy",
    });
  });

  it("returns the BatchInstallResult from installSkill", async () => {
    const batchResult = { succeeded: ["cursor"], failed: [] };
    vi.mocked(invoke)
      .mockResolvedValueOnce(batchResult)
      .mockResolvedValueOnce(mockSkills)
      .mockResolvedValueOnce(mockRepositories);

    const result = await useCentralSkillsStore
      .getState()
      .installSkill("frontend-design", ["cursor"], "symlink");

    expect(result).toEqual(batchResult);
  });

  it("sets error and re-throws when installSkill fails", async () => {
    vi.mocked(invoke).mockRejectedValueOnce(new Error("symlink failed"));

    await expect(
      useCentralSkillsStore
        .getState()
        .installSkill("frontend-design", ["cursor"], "symlink")
    ).rejects.toThrow("symlink failed");

    const state = useCentralSkillsStore.getState();
    expect(state.error).toContain("symlink failed");
    expect(state.isInstalling).toBe(false);
  });

  // ── togglePlatformLink ────────────────────────────────────────────────────

  it("calls uninstall when skill is already linked to the agent", async () => {
    // Pre-populate skills so the toggle can check linked_agents
    useCentralSkillsStore.setState({ skills: mockSkills });

    const updatedSkills = [
      { ...mockSkills[0], linked_agents: ["claude-code"] }, // cursor removed
      mockSkills[1],
    ];
    vi.mocked(invoke)
      .mockResolvedValueOnce(undefined) // uninstall_skill_from_agent
      .mockResolvedValueOnce(updatedSkills) // get_central_skills (refresh)
      .mockResolvedValueOnce(mockRepositories); // get_skill_repositories

    await useCentralSkillsStore
      .getState()
      .togglePlatformLink("frontend-design", "cursor");

    expect(invoke).toHaveBeenCalledWith("uninstall_skill_from_agent", {
      skillId: "frontend-design",
      agentId: "cursor",
    });
    expect(invoke).toHaveBeenCalledWith("get_central_skills");

    const state = useCentralSkillsStore.getState();
    expect(state.skills).toEqual(updatedSkills);
    expect(state.togglingAgentId).toBeNull();
  });

  it("calls install when skill is not linked to the agent", async () => {
    useCentralSkillsStore.setState({ skills: mockSkills });

    const updatedSkills = [
      mockSkills[0],
      { ...mockSkills[1], linked_agents: ["claude-code"] }, // added
    ];
    vi.mocked(invoke)
      .mockResolvedValueOnce(undefined) // install_skill_to_agent
      .mockResolvedValueOnce(updatedSkills) // get_central_skills (refresh)
      .mockResolvedValueOnce(mockRepositories); // get_skill_repositories

    await useCentralSkillsStore
      .getState()
      .togglePlatformLink("code-reviewer", "claude-code");

    expect(invoke).toHaveBeenCalledWith("install_skill_to_agent", {
      skillId: "code-reviewer",
      agentId: "claude-code",
      method: "auto",
    });

    const state = useCentralSkillsStore.getState();
    expect(state.skills).toEqual(updatedSkills);
    expect(state.togglingAgentId).toBeNull();
  });

  it("sets error and re-throws when togglePlatformLink fails", async () => {
    useCentralSkillsStore.setState({ skills: mockSkills });

    vi.mocked(invoke).mockRejectedValueOnce(new Error("toggle failed"));

    await expect(
      useCentralSkillsStore
        .getState()
        .togglePlatformLink("frontend-design", "cursor")
    ).rejects.toThrow("toggle failed");

    const state = useCentralSkillsStore.getState();
    expect(state.error).toContain("toggle failed");
    expect(state.togglingAgentId).toBeNull();
  });

  // ── Repository / Tag Metadata ────────────────────────────────────────────

  it("assigns skills to a repository and refreshes metadata", async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce(mockSkills)
      .mockResolvedValueOnce(mockRepositories);

    await useCentralSkillsStore
      .getState()
      .assignSkillsToRepository(["frontend-design"], "github-openai-skills-main");

    expect(invoke).toHaveBeenCalledWith("assign_skills_to_repository", {
      skillIds: ["frontend-design"],
      repositoryId: "github-openai-skills-main",
    });
    expect(invoke).toHaveBeenCalledWith("get_central_skills");
    expect(invoke).toHaveBeenCalledWith("get_skill_repositories");
    expect(useCentralSkillsStore.getState().isMetadataUpdating).toBe(false);
  });

  it("assigns skill tags and refreshes central skills", async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce(mockSkills);

    await useCentralSkillsStore
      .getState()
      .assignSkillTags(["frontend-design"], ["programming-agent-engineering"]);

    expect(invoke).toHaveBeenCalledWith("assign_skill_tags", {
      skillIds: ["frontend-design"],
      tagIds: ["programming-agent-engineering"],
    });
    expect(invoke).toHaveBeenCalledWith("get_central_skills");
  });

  it("runs bulk AI tag suggestions and refreshes central skills", async () => {
    const suggestions = [
      {
        skill_id: "frontend-design",
        suggestions: [],
        succeeded: true,
        low_confidence_count: 0,
      },
    ];
    vi.mocked(invoke)
      .mockResolvedValueOnce(suggestions)
      .mockResolvedValueOnce(mockSkills)
      .mockResolvedValueOnce([]);

    const result = await useCentralSkillsStore
      .getState()
      .bulkSuggestSkillTags(["frontend-design"]);

    expect(result).toEqual(suggestions);
    expect(invoke).toHaveBeenCalledWith("bulk_suggest_skill_tags", {
      skillIds: ["frontend-design"],
    });
    expect(invoke).toHaveBeenCalledWith("get_central_skills");
    expect(invoke).toHaveBeenCalledWith("get_pending_ai_tag_reviews");
    expect(useCentralSkillsStore.getState().aiTagJob.status).toBe("completed");
  });

  it("updates AI tag job state from progress events", async () => {
    let handler: ((event: { payload: unknown }) => void) | undefined;
    const unlisten = vi.fn();
    vi.mocked(listen).mockImplementation(async (_event, callback) => {
      handler = callback as (event: { payload: unknown }) => void;
      return unlisten;
    });

    await useCentralSkillsStore.getState().subscribeAiTagProgress();
    handler?.({
      payload: {
        jobId: "job-1",
        skillId: "frontend-design",
        skillName: "frontend-design",
        status: "running",
        total: 2,
        completed: 0,
        succeeded: 0,
        failed: 0,
        lowConfidenceCount: 0,
      },
    });

    let state = useCentralSkillsStore.getState();
    expect(state.aiTagJob.status).toBe("running");
    expect(state.aiTagJob.currentSkillName).toBe("frontend-design");
    expect(state.aiTagJob.items["frontend-design"]).toBe("running");

    handler?.({
      payload: {
        jobId: "job-1",
        skillId: "frontend-design",
        status: "succeeded",
        total: 2,
        completed: 1,
        succeeded: 1,
        failed: 0,
        lowConfidenceCount: 1,
      },
    });

    state = useCentralSkillsStore.getState();
    expect(state.aiTagJob.completed).toBe(1);
    expect(state.aiTagJob.lowConfidenceCount).toBe(1);
    expect(state.aiTagJob.items["frontend-design"]).toBe("succeeded");
  });

  it("cancels the active AI tag job", async () => {
    useCentralSkillsStore.setState({
      aiTagJob: {
        jobId: "job-1",
        status: "running",
        total: 2,
        completed: 1,
        succeeded: 1,
        failed: 0,
        lowConfidenceCount: 0,
        items: {
          "frontend-design": "succeeded",
          "code-reviewer": "queued",
        },
      },
      isSuggestingTags: true,
    });
    vi.mocked(invoke).mockResolvedValueOnce(undefined);

    await useCentralSkillsStore.getState().cancelAiTagJob();

    expect(invoke).toHaveBeenCalledWith("cancel_ai_tag_job", { jobId: "job-1" });
    expect(useCentralSkillsStore.getState().isSuggestingTags).toBe(true);
    expect(useCentralSkillsStore.getState().aiTagJob.status).toBe("cancelled");
  });
});
