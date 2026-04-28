import { describe, it, expect, vi, beforeEach } from "vitest";
import { AgentWithStatus, CentralSkillUpdateState, SkillDetail, SkillRepositoryWithStats, SkillTag, SkillWithLinks } from "../types";
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
    file_path: "~/.skillsmanage/skills/frontend-design/SKILL.md",
    canonical_path: "~/.skillsmanage/skills/frontend-design",
    is_central: true,
    scanned_at: "2026-04-09T00:00:00Z",
    linked_agents: ["claude-code", "cursor"],
    shared_root_agents: [],
  },
  {
    id: "code-reviewer",
    name: "code-reviewer",
    description: "Review code changes and identify bugs",
    file_path: "~/.skillsmanage/skills/code-reviewer/SKILL.md",
    canonical_path: "~/.skillsmanage/skills/code-reviewer",
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
    global_skills_dir: "~/.skillsmanage/skills/",
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

const mockUpdateStates: CentralSkillUpdateState[] = [
  {
    skill_id: "frontend-design",
    source_type: "github",
    source_url: "https://github.com/example/skills",
    ref: "main",
    source_path: "skills/frontend-design",
    last_remote_hash: "fnv1a64:old",
    latest_remote_hash: "fnv1a64:new",
    last_checked_at: "2026-04-25T00:00:00Z",
    status: "update_available",
  },
];

// ─── Tests ────────────────────────────────────────────────────────────────────

const mockDeletePreview: SkillDetail = {
  id: "frontend-design",
  row_id: "frontend-design",
  name: "frontend-design",
  description: "Build distinctive frontend UIs",
  file_path: "~/.skillsmanage/skills/frontend-design/SKILL.md",
  dir_path: "~/.skillsmanage/skills/frontend-design",
  canonical_path: "~/.skillsmanage/skills/frontend-design",
  is_central: true,
  source: "native",
  scanned_at: "2026-04-09T00:00:00Z",
  installations: [
    {
      skill_id: "frontend-design",
      agent_id: "cursor",
      installed_path: "~/.cursor/skills/frontend-design",
      link_type: "copy",
      symlink_target: undefined,
      installed_at: "2026-04-10T00:00:00Z",
    },
  ],
  collections: [],
  repository: mockRepositories[0],
  tags: mockTags,
  is_source_unknown: false,
};

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
      updateStatuses: {},
      updateJob: {
        phase: null,
        status: "idle",
        total: 0,
        completed: 0,
        succeeded: 0,
        failed: 0,
        skipped: 0,
        items: {},
      },
      aiTaggingAvailable: false,
      isLoading: false,
      isInstalling: false,
      isDeleting: false,
      isMetadataUpdating: false,
      isSuggestingTags: false,
      isCheckingUpdates: false,
      updatingSkillIds: [],
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
    expect(state.updateStatuses).toEqual({});
    expect(state.updateJob.status).toBe("idle");
    expect(state.aiTaggingAvailable).toBe(false);
    expect(state.isLoading).toBe(false);
    expect(state.isInstalling).toBe(false);
    expect(state.isDeleting).toBe(false);
    expect(state.isMetadataUpdating).toBe(false);
    expect(state.isSuggestingTags).toBe(false);
    expect(state.isCheckingUpdates).toBe(false);
    expect(state.updatingSkillIds).toEqual([]);
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
      .mockResolvedValueOnce([]) // get_central_skill_update_states
      .mockResolvedValueOnce("test-key"); // get_setting

    await useCentralSkillsStore.getState().loadCentralSkills();

    expect(invoke).toHaveBeenCalledWith("get_central_skills");
    expect(invoke).toHaveBeenCalledWith("get_agents");
    expect(invoke).toHaveBeenCalledWith("get_skill_repositories");
    expect(invoke).toHaveBeenCalledWith("get_skill_tags");
    expect(invoke).toHaveBeenCalledWith("get_pending_ai_tag_reviews");
    expect(invoke).toHaveBeenCalledWith("get_central_skill_update_states");
    expect(invoke).toHaveBeenCalledWith("get_setting", { key: "ai_api_key" });
  });

  it("populates skills and agents after successful loadCentralSkills", async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce(mockSkills)
      .mockResolvedValueOnce(mockAgents)
      .mockResolvedValueOnce(mockRepositories)
      .mockResolvedValueOnce(mockTags)
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce(mockUpdateStates)
      .mockResolvedValueOnce("test-key");

    await useCentralSkillsStore.getState().loadCentralSkills();

    const state = useCentralSkillsStore.getState();
    expect(state.skills).toEqual(mockSkills);
    expect(state.agents).toEqual(mockAgents);
    expect(state.repositories).toEqual(mockRepositories);
    expect(state.tags).toEqual(mockTags);
    expect(state.aiTagReviews).toEqual([]);
    expect(state.updateStatuses["frontend-design"]).toEqual(mockUpdateStates[0]);
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
        shared_root_agents: [],
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

  it("loads delete preview from skill detail", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(mockDeletePreview);

    const preview = await useCentralSkillsStore
      .getState()
      .loadDeletePreview("frontend-design");

    expect(preview).toEqual(mockDeletePreview);
    expect(invoke).toHaveBeenCalledWith("get_skill_detail", {
      skillId: "frontend-design",
    });
  });

  it("deletes a central skill and refreshes central metadata", async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce([mockSkills[1]])
      .mockResolvedValueOnce(mockRepositories)
      .mockResolvedValueOnce(mockTags)
      .mockResolvedValueOnce([]);

    await useCentralSkillsStore
      .getState()
      .deleteCentralSkill("frontend-design", ["cursor"]);

    expect(invoke).toHaveBeenCalledWith("delete_central_skill", {
      skillId: "frontend-design",
      removeAgentIds: ["cursor"],
    });
    expect(invoke).toHaveBeenCalledWith("get_central_skills");
    expect(invoke).toHaveBeenCalledWith("get_skill_repositories");
    expect(invoke).toHaveBeenCalledWith("get_skill_tags");
    expect(invoke).toHaveBeenCalledWith("get_pending_ai_tag_reviews");
    expect(useCentralSkillsStore.getState().skills).toEqual([mockSkills[1]]);
    expect(useCentralSkillsStore.getState().isDeleting).toBe(false);
  });

  it("loads batch delete preview for selected central skills", async () => {
    const preview = {
      previews: [
        {
          skill_id: "frontend-design",
          skill_name: "frontend-design",
          central_path: "~/.skillsmanage/skills/frontend-design",
          copy_installations: mockDeletePreview.installations,
          auto_removed_agent_ids: ["claude-code"],
        },
      ],
      failed: [],
    };
    vi.mocked(invoke).mockResolvedValueOnce(preview);

    const result = await useCentralSkillsStore
      .getState()
      .loadBatchDeletePreview(["frontend-design", "code-reviewer"]);

    expect(result).toEqual(preview);
    expect(invoke).toHaveBeenCalledWith("preview_delete_central_skills", {
      skillIds: ["frontend-design", "code-reviewer"],
    });
  });

  it("deletes selected central skills and refreshes central metadata", async () => {
    const result = {
      succeeded: [
        {
          skill_id: "frontend-design",
          removed_central_path: "~/.skillsmanage/skills/frontend-design",
          removed_agent_ids: ["cursor"],
          retained_agent_ids: [],
        },
      ],
      failed: [{ skill_id: "missing-skill", error: "not found" }],
    };
    vi.mocked(invoke)
      .mockResolvedValueOnce(result)
      .mockResolvedValueOnce([mockSkills[1]])
      .mockResolvedValueOnce(mockRepositories)
      .mockResolvedValueOnce(mockTags)
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([]);

    const actual = await useCentralSkillsStore.getState().deleteCentralSkills([
      { skill_id: "frontend-design", remove_agent_ids: ["cursor"] },
      { skill_id: "missing-skill", remove_agent_ids: [] },
    ]);

    expect(actual).toEqual(result);
    expect(invoke).toHaveBeenCalledWith("delete_central_skills", {
      requests: [
        { skill_id: "frontend-design", remove_agent_ids: ["cursor"] },
        { skill_id: "missing-skill", remove_agent_ids: [] },
      ],
    });
    expect(invoke).toHaveBeenCalledWith("get_central_skills");
    expect(invoke).toHaveBeenCalledWith("get_skill_repositories");
    expect(invoke).toHaveBeenCalledWith("get_skill_tags");
    expect(invoke).toHaveBeenCalledWith("get_pending_ai_tag_reviews");
    expect(invoke).toHaveBeenCalledWith("get_central_skill_update_states");
    expect(useCentralSkillsStore.getState().skills).toEqual([mockSkills[1]]);
    expect(useCentralSkillsStore.getState().isDeleting).toBe(false);
  });

  it("loads repository delete preview", async () => {
    const repository: SkillRepositoryWithStats = {
      id: "github-openai-skills-main",
      name: "openai/skills",
      source_type: "github",
      owner: "openai",
      repo: "skills",
      branch: "main",
      url: "https://github.com/openai/skills",
      is_unknown: false,
      created_at: "2026-04-17T00:00:00.000Z",
      updated_at: "2026-04-17T00:00:00.000Z",
      skill_count: 1,
      unknown_skill_count: 0,
    };
    const preview = {
      repository,
      delete_preview: {
        previews: [
          {
            skill_id: "frontend-design",
            skill_name: "frontend-design",
            central_path: "~/.skillsmanage/skills/frontend-design",
            copy_installations: mockDeletePreview.installations,
            auto_removed_agent_ids: ["claude-code"],
          },
        ],
        failed: [],
      },
    };
    vi.mocked(invoke).mockResolvedValueOnce(preview);

    const result = await useCentralSkillsStore
      .getState()
      .loadRepositoryDeletePreview("github-openai-skills-main");

    expect(result).toEqual(preview);
    expect(invoke).toHaveBeenCalledWith("preview_delete_skill_repository", {
      repositoryId: "github-openai-skills-main",
    });
  });

  it("deletes a repository and refreshes central metadata", async () => {
    const result = {
      repository: {
        id: "github-openai-skills-main",
        name: "openai/skills",
        source_type: "github",
        owner: "openai",
        repo: "skills",
        branch: "main",
        url: "https://github.com/openai/skills",
        is_unknown: false,
        created_at: "2026-04-17T00:00:00.000Z",
        updated_at: "2026-04-17T00:00:00.000Z",
      },
      deleted_repository: true,
      delete_result: {
        succeeded: [
          {
            skill_id: "frontend-design",
            removed_central_path: "~/.skillsmanage/skills/frontend-design",
            removed_agent_ids: ["cursor"],
            retained_agent_ids: [],
          },
        ],
        failed: [],
      },
    };
    vi.mocked(invoke)
      .mockResolvedValueOnce(result)
      .mockResolvedValueOnce([mockSkills[1]])
      .mockResolvedValueOnce(mockRepositories)
      .mockResolvedValueOnce(mockTags)
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([]);

    const actual = await useCentralSkillsStore.getState().deleteSkillRepository(
      "github-openai-skills-main",
      [{ skill_id: "frontend-design", remove_agent_ids: ["cursor"] }]
    );

    expect(actual).toEqual(result);
    expect(invoke).toHaveBeenCalledWith("delete_skill_repository", {
      repositoryId: "github-openai-skills-main",
      requests: [{ skill_id: "frontend-design", remove_agent_ids: ["cursor"] }],
    });
    expect(invoke).toHaveBeenCalledWith("get_central_skills");
    expect(invoke).toHaveBeenCalledWith("get_skill_repositories");
    expect(invoke).toHaveBeenCalledWith("get_skill_tags");
    expect(invoke).toHaveBeenCalledWith("get_pending_ai_tag_reviews");
    expect(invoke).toHaveBeenCalledWith("get_central_skill_update_states");
    expect(useCentralSkillsStore.getState().skills).toEqual([mockSkills[1]]);
    expect(useCentralSkillsStore.getState().isDeleting).toBe(false);
  });

  it("sets error and clears deleting state when central deletion fails", async () => {
    vi.mocked(invoke).mockRejectedValueOnce(new Error("delete failed"));

    await expect(
      useCentralSkillsStore
        .getState()
        .deleteCentralSkill("frontend-design", [])
    ).rejects.toThrow("delete failed");

    expect(useCentralSkillsStore.getState().error).toContain("delete failed");
    expect(useCentralSkillsStore.getState().isDeleting).toBe(false);
  });

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

  it("uses batch_install_central_skills for single-skill project installs", async () => {
    const batchResult = {
      succeeded: [
        {
          skill_id: "frontend-design",
          agent_id: "cursor",
          target_path: "D:\\work\\demo\\.cursor\\skills\\frontend-design",
        },
      ],
      failed: [
        {
          skill_id: "frontend-design",
          agent_id: "kiro",
          error: "No project pattern",
        },
      ],
    };
    vi.mocked(invoke)
      .mockResolvedValueOnce(batchResult)
      .mockResolvedValueOnce(mockSkills)
      .mockResolvedValueOnce(mockRepositories);

    const result = await useCentralSkillsStore
      .getState()
      .installSkill("frontend-design", ["cursor", "kiro"], "copy", "D:\\work\\demo");

    expect(result).toEqual({
      succeeded: ["cursor"],
      failed: [{ agent_id: "kiro", error: "No project pattern" }],
    });
    expect(invoke).toHaveBeenCalledWith("batch_install_central_skills", {
      skillIds: ["frontend-design"],
      agentIds: ["cursor", "kiro"],
      method: "copy",
      projectPath: "D:\\work\\demo",
    });
    expect(invoke).toHaveBeenCalledWith("get_central_skills");
    expect(invoke).toHaveBeenCalledWith("get_skill_repositories");
  });

  it("calls batch_install_central_skills then refreshes skills", async () => {
    const batchResult = {
      succeeded: [
        {
          skill_id: "frontend-design",
          agent_id: "cursor",
          target_path: "~/.cursor/skills/frontend-design",
        },
      ],
      failed: [],
    };
    vi.mocked(invoke)
      .mockResolvedValueOnce(batchResult)
      .mockResolvedValueOnce(mockSkills)
      .mockResolvedValueOnce(mockRepositories);

    const result = await useCentralSkillsStore
      .getState()
      .batchInstallSkills(["frontend-design"], ["cursor"], "copy", "D:\\work\\demo");

    expect(result).toEqual(batchResult);
    expect(invoke).toHaveBeenCalledWith("batch_install_central_skills", {
      skillIds: ["frontend-design"],
      agentIds: ["cursor"],
      method: "copy",
      projectPath: "D:\\work\\demo",
    });
    expect(invoke).toHaveBeenCalledWith("get_central_skills");
    expect(invoke).toHaveBeenCalledWith("get_skill_repositories");
    expect(useCentralSkillsStore.getState().skills).toEqual(mockSkills);
    expect(useCentralSkillsStore.getState().isInstalling).toBe(false);
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

  it("checks central skill updates and indexes returned states", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(mockUpdateStates);

    const result = await useCentralSkillsStore
      .getState()
      .checkSkillUpdates(["frontend-design"]);

    expect(result).toEqual(mockUpdateStates);
    expect(invoke).toHaveBeenCalledWith("check_central_skill_updates", {
      skillIds: ["frontend-design"],
    });
    expect(useCentralSkillsStore.getState().updateStatuses["frontend-design"]).toEqual(
      mockUpdateStates[0]
    );
    expect(useCentralSkillsStore.getState().isCheckingUpdates).toBe(false);
  });

  it("indexes remote-missing update states returned from checks", async () => {
    const remoteMissingState: CentralSkillUpdateState = {
      ...mockUpdateStates[0],
      skill_id: "code-reviewer",
      source_path: "skills/code-reviewer",
      last_remote_hash: null,
      latest_remote_hash: null,
      status: "remote_missing",
      error: "Skill source path 'skills/code-reviewer' no longer contains an importable skill.",
    };
    vi.mocked(invoke).mockResolvedValueOnce([remoteMissingState]);

    const result = await useCentralSkillsStore
      .getState()
      .checkSkillUpdates(["code-reviewer"]);

    expect(result).toEqual([remoteMissingState]);
    expect(useCentralSkillsStore.getState().updateStatuses["code-reviewer"]).toEqual(
      remoteMissingState
    );
  });

  it("updates central skills and refreshes skills plus update states", async () => {
    const updatedState: CentralSkillUpdateState = {
      ...mockUpdateStates[0],
      latest_remote_hash: "fnv1a64:new",
      last_remote_hash: "fnv1a64:new",
      status: "up_to_date",
    };
    const updateResult = {
      succeeded: ["frontend-design"],
      failed: [],
      skipped: [],
      states: [updatedState],
    };
    vi.mocked(invoke)
      .mockResolvedValueOnce(updateResult)
      .mockResolvedValueOnce(mockSkills)
      .mockResolvedValueOnce(mockRepositories)
      .mockResolvedValueOnce([updatedState]);

    const result = await useCentralSkillsStore
      .getState()
      .updateSkills(["frontend-design"]);

    expect(result).toEqual(updateResult);
    expect(invoke).toHaveBeenCalledWith("update_central_skills", {
      skillIds: ["frontend-design"],
    });
    expect(invoke).toHaveBeenCalledWith("get_central_skills");
    expect(invoke).toHaveBeenCalledWith("get_skill_repositories");
    expect(invoke).toHaveBeenCalledWith("get_central_skill_update_states");
    expect(useCentralSkillsStore.getState().updateStatuses["frontend-design"]).toEqual(
      updatedState
    );
    expect(useCentralSkillsStore.getState().updatingSkillIds).toEqual([]);
  });

  it("keeps remote-missing skills and refreshes central metadata", async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce(["frontend-design"])
      .mockResolvedValueOnce(mockSkills)
      .mockResolvedValueOnce(mockRepositories)
      .mockResolvedValueOnce([]);

    const result = await useCentralSkillsStore
      .getState()
      .keepRemoteMissingSkills(["frontend-design"]);

    expect(result).toEqual(["frontend-design"]);
    expect(invoke).toHaveBeenNthCalledWith(1, "keep_remote_missing_central_skills", {
      skillIds: ["frontend-design"],
    });
    expect(invoke).toHaveBeenCalledWith("get_central_skills");
    expect(invoke).toHaveBeenCalledWith("get_skill_repositories");
    expect(invoke).toHaveBeenCalledWith("get_central_skill_update_states");
    expect(useCentralSkillsStore.getState().skills).toEqual(mockSkills);
    expect(useCentralSkillsStore.getState().repositories).toEqual(mockRepositories);
    expect(useCentralSkillsStore.getState().updateStatuses).toEqual({});
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

  it("updates central update job state from progress events", async () => {
    let handler: ((event: { payload: unknown }) => void) | undefined;
    const unlisten = vi.fn();
    vi.mocked(listen).mockImplementation(async (_event, callback) => {
      handler = callback as (event: { payload: unknown }) => void;
      return unlisten;
    });

    await useCentralSkillsStore.getState().subscribeUpdateProgress();
    handler?.({
      payload: {
        phase: "checking",
        skillId: "frontend-design",
        skillName: "frontend-design",
        status: "running",
        total: 2,
        completed: 0,
        succeeded: 0,
        failed: 0,
        skipped: 0,
      },
    });

    let state = useCentralSkillsStore.getState();
    expect(state.updateJob.status).toBe("running");
    expect(state.updateJob.currentSkillName).toBe("frontend-design");
    expect(state.updateJob.items["frontend-design"]).toBe("running");

    handler?.({
      payload: {
        phase: "checking",
        skillId: "frontend-design",
        status: "update_available",
        total: 2,
        completed: 1,
        succeeded: 1,
        failed: 0,
        skipped: 0,
      },
    });

    state = useCentralSkillsStore.getState();
    expect(state.updateJob.completed).toBe(1);
    expect(state.updateJob.items["frontend-design"]).toBe("succeeded");

    handler?.({
      payload: {
        phase: "checking",
        skillId: "code-reviewer",
        status: "remote_missing",
        total: 2,
        completed: 2,
        succeeded: 1,
        failed: 0,
        skipped: 1,
        error: "Skill source path no longer contains an importable skill.",
      },
    });

    state = useCentralSkillsStore.getState();
    expect(state.updateJob.completed).toBe(2);
    expect(state.updateJob.skipped).toBe(1);
    expect(state.updateJob.items["code-reviewer"]).toBe("skipped");
    expect(state.updateJob.error).toBe("Skill source path no longer contains an importable skill.");

    handler?.({
      payload: {
        phase: "checking",
        status: "completed",
        total: 2,
        completed: 2,
        succeeded: 1,
        failed: 0,
        skipped: 1,
      },
    });

    state = useCentralSkillsStore.getState();
    expect(state.updateJob.status).toBe("completed");
    expect(state.updateJob.error).toBeUndefined();
  });

  it("exports the SkillPort portable state manifest", async () => {
    vi.mocked(invoke).mockResolvedValueOnce("{\"kind\":\"skillport/state-export\"}");

    const json = await useCentralSkillsStore.getState().exportSkillportState();

    expect(json).toBe("{\"kind\":\"skillport/state-export\"}");
    expect(invoke).toHaveBeenCalledWith("export_skillport_state", { options: {} });
  });

  it("previews a SkillPort portable state import", async () => {
    const preview = {
      githubSources: [],
      skills: [],
      summary: {
        sourcesToAdd: 0,
        sourcesExisting: 0,
        ready: 0,
        conflicts: 0,
        missing: 0,
        unrestorable: 0,
      },
    };
    vi.mocked(invoke).mockResolvedValueOnce(preview);

    const result = await useCentralSkillsStore.getState().previewSkillportStateImport("{}");

    expect(result).toEqual(preview);
    expect(invoke).toHaveBeenCalledWith("preview_skillport_state_import", { json: "{}" });
  });

  it("imports SkillPort portable state and refreshes Central metadata", async () => {
    const resolutions = [
      {
        skillId: "frontend-design",
        sourcePath: "skills/frontend-design/SKILL.md",
        resolution: "overwrite" as const,
        renamedSkillId: null,
      },
    ];
    const importResult = {
      sourcesAdded: 1,
      sourcesSkipped: 0,
      importedSkills: [
        {
          sourcePath: "skills/frontend-design/SKILL.md",
          importedSkillId: "frontend-design",
          skillName: "frontend-design",
        },
      ],
      skippedSkills: [],
      failedSkills: [],
      tagsRestored: 1,
    };
    vi.mocked(invoke)
      .mockResolvedValueOnce(importResult)
      .mockResolvedValueOnce(mockSkills)
      .mockResolvedValueOnce(mockRepositories)
      .mockResolvedValueOnce(mockTags)
      .mockResolvedValueOnce(mockUpdateStates);

    const result = await useCentralSkillsStore
      .getState()
      .importSkillportState("{\"kind\":\"skillport/state-export\"}", resolutions);

    expect(result).toEqual(importResult);
    expect(invoke).toHaveBeenNthCalledWith(1, "import_skillport_state", {
      json: "{\"kind\":\"skillport/state-export\"}",
      resolutions,
    });
    expect(useCentralSkillsStore.getState().skills).toEqual(mockSkills);
    expect(useCentralSkillsStore.getState().repositories).toEqual(mockRepositories);
    expect(useCentralSkillsStore.getState().tags).toEqual(mockTags);
    expect(useCentralSkillsStore.getState().updateStatuses["frontend-design"]).toEqual(
      mockUpdateStates[0]
    );
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
