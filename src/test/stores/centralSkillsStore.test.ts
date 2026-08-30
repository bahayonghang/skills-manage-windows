import { describe, it, expect, vi, beforeEach } from "vitest";
import { AgentWithStatus, CentralSkillUpdateState, DeleteCentralSkillPreview, SkillRepositoryWithStats, SkillTag, SkillWithLinks } from "@/types";
import type {
  CentralRepositorySyncApplyResult,
  CentralRepositorySyncPreview,
} from "@/types/centralRepositorySync";
import * as tauriBridge from "@/lib/ipc";
import { ipcFixtureError } from "@/lib/ipc/errors";

// Mock Tauri core before importing the store
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCentralSkillsStore } from "@/stores/centralSkillsStore";
import { usePlatformStore } from "@/stores/platformStore";

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
    owner: null,
    repo: null,
    branch: null,
    url: null,
    pinned: false,
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

const mockDeletePreview: DeleteCentralSkillPreview = {
  skill_id: "frontend-design",
  skill_name: "frontend-design",
  central_path: "~/.skillsmanage/skills/frontend-design",
  copy_installations: [
    {
      skill_id: "frontend-design",
      agent_id: "cursor",
      installed_path: "~/.cursor/skills/frontend-design",
      link_type: "copy",
      symlink_target: undefined,
      installed_at: "2026-04-10T00:00:00Z",
    },
  ],
  auto_removed_agent_ids: ["claude-code"],
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
        jobId: null,
        phase: null,
        status: "idle",
        total: 0,
        completed: 0,
        succeeded: 0,
        failed: 0,
        skipped: 0,
        items: {},
      },
      portabilityJob: {
        jobId: null,
        phase: null,
        status: "idle",
        total: 0,
        completed: 0,
      },
      aiTaggingAvailable: false,
      isLoading: false,
      isRefreshingList: false,
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
    expect(state.portabilityJob.status).toBe("idle");
    expect(state.aiTaggingAvailable).toBe(false);
    expect(state.isLoading).toBe(false);
    expect(state.isRefreshingList).toBe(false);
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
      .mockResolvedValueOnce({ configured: true }); // get_ai_api_key_state

    await useCentralSkillsStore.getState().loadCentralSkills();

    expect(invoke).toHaveBeenCalledWith("get_central_skills");
    expect(invoke).toHaveBeenCalledWith("get_agents");
    expect(invoke).toHaveBeenCalledWith("get_skill_repositories");
    expect(invoke).toHaveBeenCalledWith("get_skill_tags");
    expect(invoke).toHaveBeenCalledWith("get_pending_ai_tag_reviews");
    expect(invoke).toHaveBeenCalledWith("get_central_skill_update_states");
    expect(invoke).toHaveBeenCalledWith("get_ai_api_key_state");
  });

  it("populates skills and agents after successful loadCentralSkills", async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce(mockSkills)
      .mockResolvedValueOnce(mockAgents)
      .mockResolvedValueOnce(mockRepositories)
      .mockResolvedValueOnce(mockTags)
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce(mockUpdateStates)
      .mockResolvedValueOnce({ configured: true });

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
    vi.mocked(invoke).mockRejectedValueOnce(
      ipcFixtureError("storage.unavailable", "DB error"),
    );

    await useCentralSkillsStore.getState().loadCentralSkills();

    const state = useCentralSkillsStore.getState();
    expect(state.error).toBe("DB error");
    expect(state.isLoading).toBe(false);
  });

  it("rethrows on failure when throwOnError is true and still writes store error", async () => {
    vi.mocked(invoke).mockRejectedValueOnce(
      ipcFixtureError("storage.unavailable", "DB error"),
    );

    await expect(
      useCentralSkillsStore.getState().loadCentralSkills({ throwOnError: true })
    ).rejects.toThrow("DB error");

    const state = useCentralSkillsStore.getState();
    expect(state.error).toBe("DB error");
    expect(state.isLoading).toBe(false);
    expect(state.isRefreshingList).toBe(false);
  });

  it("refreshes in place via isRefreshingList when skills already exist", async () => {
    useCentralSkillsStore.setState({ skills: mockSkills });
    let resolveSkills: ((value: SkillWithLinks[]) => void) | undefined;
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === "get_central_skills") {
        return new Promise<SkillWithLinks[]>((resolve) => {
          resolveSkills = resolve;
        });
      }
      return [];
    });

    const pending = useCentralSkillsStore.getState().loadCentralSkills();

    // 入口同步分流：isLoading 保持 false，旧列表内容保留。
    const refreshingState = useCentralSkillsStore.getState();
    expect(refreshingState.isRefreshingList).toBe(true);
    expect(refreshingState.isLoading).toBe(false);
    expect(refreshingState.skills).toEqual(mockSkills);

    resolveSkills?.([mockSkills[1]]);
    await pending;

    const state = useCentralSkillsStore.getState();
    expect(state.isRefreshingList).toBe(false);
    expect(state.isLoading).toBe(false);
    expect(state.skills).toEqual([mockSkills[1]]);
  });

  it("keeps using isLoading for the initial empty-store load", async () => {
    let resolveSkills: ((value: SkillWithLinks[]) => void) | undefined;
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === "get_central_skills") {
        return new Promise<SkillWithLinks[]>((resolve) => {
          resolveSkills = resolve;
        });
      }
      return [];
    });

    const pending = useCentralSkillsStore.getState().loadCentralSkills();

    const loadingState = useCentralSkillsStore.getState();
    expect(loadingState.isLoading).toBe(true);
    expect(loadingState.isRefreshingList).toBe(false);

    resolveSkills?.(mockSkills);
    await pending;

    const state = useCentralSkillsStore.getState();
    expect(state.isLoading).toBe(false);
    expect(state.skills).toEqual(mockSkills);
  });

  it("applies only the latest loadCentralSkills result when requests overlap", async () => {
    const secondSkills = [mockSkills[1]];
    let getCentralSkillsCalls = 0;
    let resolveFirstSkills: ((value: SkillWithLinks[]) => void) | undefined;
    vi.mocked(invoke).mockImplementation(async (command) => {
      if (command === "get_central_skills") {
        getCentralSkillsCalls += 1;
        if (getCentralSkillsCalls === 1) {
          return new Promise<SkillWithLinks[]>((resolve) => {
            resolveFirstSkills = resolve;
          });
        }
        return secondSkills;
      }
      return [];
    });

    const first = useCentralSkillsStore.getState().loadCentralSkills();
    const second = useCentralSkillsStore.getState().loadCentralSkills();

    // 后到请求先完成，结果生效。
    await second;
    expect(useCentralSkillsStore.getState().skills).toEqual(secondSkills);

    // 先到请求更晚完成，其过期写入被 latest-wins 门控丢弃。
    resolveFirstSkills?.(mockSkills);
    await first;
    expect(useCentralSkillsStore.getState().skills).toEqual(secondSkills);
    expect(useCentralSkillsStore.getState().isLoading).toBe(false);
  });

  it("returns deterministic browser fixture data when Tauri runtime is unavailable", async () => {
    const isTauriSpy = vi.spyOn(tauriBridge, "isTauriRuntime").mockReturnValue(false);

    await useCentralSkillsStore.getState().loadCentralSkills();

    expect(invoke).not.toHaveBeenCalled();
    const state = useCentralSkillsStore.getState();
    expect(state.skills).toEqual(expect.arrayContaining([
      expect.objectContaining({
        id: "fixture-central-skill",
        linked_agents: ["claude-code", "cursor"],
        shared_root_agents: [],
        is_source_unknown: true,
      }),
    ]));
    expect(state.skills.length).toBeGreaterThan(60);
    expect(
      state.skills.some((skill) => skill.name.includes("中文高密度排版验证技能")),
    ).toBe(true);
    expect(
      state.skills.some((skill) => skill.canonical_path?.startsWith("C:\\Users\\")),
    ).toBe(true);
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

  it("loads delete preview from preview_delete_central_skills", async () => {
    vi.mocked(invoke).mockResolvedValueOnce({
      previews: [mockDeletePreview],
      failed: [],
    });

    const preview = await useCentralSkillsStore
      .getState()
      .loadDeletePreview("frontend-design");

    expect(preview).toEqual(mockDeletePreview);
    expect(invoke).toHaveBeenCalledWith("preview_delete_central_skills", {
      skillIds: ["frontend-design"],
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
      force: false,
    });
    expect(invoke).toHaveBeenCalledWith("get_central_skills");
    expect(invoke).toHaveBeenCalledWith("get_skill_repositories");
    expect(invoke).toHaveBeenCalledWith("get_skill_tags");
    expect(invoke).toHaveBeenCalledWith("get_pending_ai_tag_reviews");
    expect(useCentralSkillsStore.getState().skills).toEqual([mockSkills[1]]);
    expect(useCentralSkillsStore.getState().isDeleting).toBe(false);
  });

  it("passes the force flag to delete_central_skill", async () => {
    vi.mocked(invoke)
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce([mockSkills[1]])
      .mockResolvedValueOnce(mockRepositories)
      .mockResolvedValueOnce(mockTags)
      .mockResolvedValueOnce([]);

    await useCentralSkillsStore
      .getState()
      .deleteCentralSkill("frontend-design", ["cursor"], true);

    expect(invoke).toHaveBeenCalledWith("delete_central_skill", {
      skillId: "frontend-design",
      removeAgentIds: ["cursor"],
      force: true,
    });
  });

  it("loads batch delete preview for selected central skills", async () => {
    const preview = {
      previews: [
        {
          skill_id: "frontend-design",
          skill_name: "frontend-design",
          central_path: "~/.skillsmanage/skills/frontend-design",
          copy_installations: mockDeletePreview.copy_installations,
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

  it("loads unknown-source reset preview from the active target", async () => {
    const preview = {
      skillIds: ["npx-skill"],
      preview: {
        previews: [
          {
            skill_id: "npx-skill",
            skill_name: "npx-skill",
            central_path: "~/.skillsmanage/skills/npx-skill",
            copy_installations: [],
            auto_removed_agent_ids: [],
          },
        ],
        failed: [],
      },
    };
    vi.mocked(invoke).mockResolvedValueOnce(preview);

    const result = await useCentralSkillsStore
      .getState()
      .loadUnknownSourceResetPreview();

    expect(result).toEqual(preview);
    expect(invoke).toHaveBeenCalledWith("preview_reset_unknown_source_skills");
  });

  it("resets unknown-source skills and reloads central plus inventory", async () => {
    const result = {
      succeeded: [
        {
          skill_id: "npx-skill",
          removed_central_path: "~/.skillsmanage/skills/npx-skill",
          removed_agent_ids: [],
          retained_agent_ids: [],
        },
      ],
      failed: [],
    };
    vi.mocked(invoke)
      .mockResolvedValueOnce(result)
      .mockResolvedValueOnce([mockSkills[1]])
      .mockResolvedValueOnce(mockRepositories)
      .mockResolvedValueOnce(mockTags)
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce([])
      .mockResolvedValueOnce({
        updatable: [],
        remoteAdded: [],
        remoteMissing: [],
        unsupported: [],
        platformDuplicates: [],
        deletedPlatformCopies: [],
        orphans: [],
        failedRepositories: [],
        generatedAt: "2026-08-14T00:00:00Z",
      });

    const actual = await useCentralSkillsStore
      .getState()
      .resetUnknownSourceSkills(["npx-skill"], []);

    expect(actual).toEqual(result);
    expect(invoke).toHaveBeenCalledWith("reset_unknown_source_skills", {
      skillIds: ["npx-skill"],
      removeCopyAgentIds: [],
    });
    expect(invoke).toHaveBeenCalledWith("get_central_skills");
    expect(invoke).toHaveBeenCalledWith("get_skill_update_inventory", {
      scope: null,
    });
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
      pinned: false,
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
            copy_installations: mockDeletePreview.copy_installations,
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
        pinned: false,
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
    vi.mocked(invoke).mockRejectedValueOnce(
      ipcFixtureError("storage.unavailable", "delete failed"),
    );

    await expect(
      useCentralSkillsStore
        .getState()
        .deleteCentralSkill("frontend-design", [])
    ).rejects.toThrow("delete failed");

    expect(useCentralSkillsStore.getState().error).toBe(
      "storage.unavailable:delete failed",
    );
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

    expect(result).toEqual({ ...batchResult, skipped: [] });
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
      skipped: [
        {
          skill_id: "frontend-design",
          agent_id: "codex",
          target_path: "D:\\work\\demo\\.agents\\skills\\frontend-design",
          reason: "already_installed",
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
      skipped: [
        {
          agent_id: "codex",
          target_path: "D:\\work\\demo\\.agents\\skills\\frontend-design",
          reason: "already_installed",
        },
      ],
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
      skipped: [],
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
    vi.mocked(invoke).mockRejectedValueOnce(
      ipcFixtureError("storage.unavailable", "symlink failed"),
    );

    await expect(
      useCentralSkillsStore
        .getState()
        .installSkill("frontend-design", ["cursor"], "symlink")
    ).rejects.toThrow("symlink failed");

    const state = useCentralSkillsStore.getState();
    expect(state.error).toBe("symlink failed");
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

    vi.mocked(invoke).mockRejectedValueOnce(
      ipcFixtureError("storage.unavailable", "toggle failed"),
    );

    await expect(
      useCentralSkillsStore
        .getState()
        .togglePlatformLink("frontend-design", "cursor")
    ).rejects.toThrow("toggle failed");

    const state = useCentralSkillsStore.getState();
    expect(state.error).toBe("toggle failed");
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

  it("sets repository pin state and refreshes repositories", async () => {
    const pinnedRepository: SkillRepositoryWithStats = {
      id: "github-openai-skills-main",
      name: "openai/skills",
      source_type: "github",
      owner: "openai",
      repo: "skills",
      branch: "main",
      url: "https://github.com/openai/skills",
      pinned: true,
      is_unknown: false,
      created_at: "2026-04-17T00:00:00.000Z",
      updated_at: "2026-04-17T00:00:00.000Z",
      skill_count: 1,
      unknown_skill_count: 0,
    };
    const pinnedRepositories = [mockRepositories[0], pinnedRepository];
    vi.mocked(invoke)
      .mockResolvedValueOnce(pinnedRepository)
      .mockResolvedValueOnce(pinnedRepositories);

    await useCentralSkillsStore
      .getState()
      .setRepositoryPinned("github-openai-skills-main", true);

    expect(invoke).toHaveBeenCalledWith("set_skill_repository_pinned", {
      repositoryId: "github-openai-skills-main",
      pinned: true,
    });
    expect(invoke).toHaveBeenCalledWith("get_skill_repositories");
    expect(useCentralSkillsStore.getState().repositories).toEqual(pinnedRepositories);
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
      jobId: expect.any(String),
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

  it("checks repository sync and merges checked states without storing remote additions", async () => {
    const preview: CentralRepositorySyncPreview = {
      states: mockUpdateStates,
      remoteAdded: [
        {
          repositoryId: "github-owner-repo-main",
          repo: {
            owner: "owner",
            repo: "repo",
            branch: "main",
            normalizedUrl: "https://github.com/owner/repo",
          },
          preview: {
            sourcePath: "skills/new-skill",
            skillId: "new-skill",
            skillName: "New Skill",
            description: null,
            rootDirectory: "skills",
            skillDirectoryName: "new-skill",
            downloadUrl: "https://raw.githubusercontent.com/owner/repo/main/skills/new-skill/SKILL.md",
            conflict: null,
          },
        },
      ],
      skippedRemoteAdded: [],
      remoteMissing: [],
      repositories: [],
      failedRepositories: [],
    };
    vi.mocked(invoke).mockResolvedValueOnce(preview);

    const result = await useCentralSkillsStore
      .getState()
      .checkRepositorySync(["github-owner-repo-main"], ["frontend-design"]);

    expect(result).toEqual(preview);
    expect(invoke).toHaveBeenCalledWith("check_central_repository_sync", {
      jobId: expect.any(String),
      repositoryIds: ["github-owner-repo-main"],
      skillIds: ["frontend-design"],
    });
    expect(useCentralSkillsStore.getState().updateStatuses["frontend-design"]).toEqual(
      mockUpdateStates[0]
    );
    expect(useCentralSkillsStore.getState().updateStatuses["new-skill"]).toBeUndefined();
  });

  it("checks repository sync with wrapped remote-missing payload and still only indexes checked states", async () => {
    const remoteMissingState: CentralSkillUpdateState = {
      ...mockUpdateStates[0],
      skill_id: "code-reviewer",
      source_path: "skills/code-reviewer",
      last_remote_hash: null,
      latest_remote_hash: null,
      status: "remote_missing",
      error: "removed remotely",
    };
    const preview: CentralRepositorySyncPreview = {
      states: [remoteMissingState],
      remoteAdded: [
        {
          repositoryId: "github-owner-repo-main",
          repo: {
            owner: "owner",
            repo: "repo",
            branch: "main",
            normalizedUrl: "https://github.com/owner/repo",
          },
          preview: {
            sourcePath: "skills/new-skill",
            skillId: "new-skill",
            skillName: "New Skill",
            description: null,
            rootDirectory: "skills",
            skillDirectoryName: "new-skill",
            downloadUrl: "https://raw.githubusercontent.com/owner/repo/main/skills/new-skill/SKILL.md",
            conflict: null,
          },
        },
      ],
      skippedRemoteAdded: [],
      remoteMissing: [
        {
          state: remoteMissingState,
          repositoryId: "github-owner-repo-main",
          repositoryName: "owner/repo",
          repo: {
            owner: "owner",
            repo: "repo",
            branch: "main",
            normalizedUrl: "https://github.com/owner/repo",
          },
        },
      ],
      repositories: [],
      failedRepositories: [],
    };
    vi.mocked(invoke).mockResolvedValueOnce(preview);

    const result = await useCentralSkillsStore
      .getState()
      .checkRepositorySync(["github-owner-repo-main"], ["code-reviewer"]);

    expect(result).toEqual(preview);
    expect(useCentralSkillsStore.getState().updateStatuses["code-reviewer"]).toEqual(
      remoteMissingState
    );
    expect(useCentralSkillsStore.getState().updateStatuses["new-skill"]).toBeUndefined();
  });

  it("updates central skills and merges returned states without a second state refresh", async () => {
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
      .mockResolvedValueOnce(mockSkills);

    const result = await useCentralSkillsStore
      .getState()
      .updateSkills(["frontend-design"]);

    expect(result).toEqual(updateResult);
    expect(invoke).toHaveBeenCalledWith("update_central_skills", {
      jobId: expect.any(String),
      skillIds: ["frontend-design"],
    });
    expect(invoke).toHaveBeenCalledWith("get_central_skills");
    expect(invoke).not.toHaveBeenCalledWith("get_skill_repositories");
    expect(invoke).not.toHaveBeenCalledWith("get_central_skill_update_states");
    expect(useCentralSkillsStore.getState().updateStatuses["frontend-design"]).toEqual(
      updatedState
    );
    expect(useCentralSkillsStore.getState().repositories).toEqual([]);
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

  it("applies repository sync and refreshes skills, repositories, tags, and update states", async () => {
    const applyResult: CentralRepositorySyncApplyResult = {
      keptSkillIds: ["code-reviewer"],
      deleteResult: { succeeded: [], failed: [] },
      importResults: [
        {
          repo: {
            owner: "owner",
            repo: "repo",
            branch: "main",
            normalizedUrl: "https://github.com/owner/repo",
          },
          importedSkills: [
            {
              sourcePath: "skills/new-skill",
              originalSkillId: "new-skill",
              importedSkillId: "new-skill",
              skillName: "New Skill",
              targetDirectory: "~/.skillsmanage/skills/new-skill",
              resolution: "overwrite",
            },
          ],
          skippedSkills: [],
        },
      ],
      skippedAdditions: [],
      unskippedAdditions: [],
      failedRepositories: [],
      states: mockUpdateStates,
    };
    vi.mocked(invoke)
      .mockResolvedValueOnce(applyResult)
      .mockResolvedValueOnce(mockSkills)
      .mockResolvedValueOnce(mockRepositories)
      .mockResolvedValueOnce(mockTags)
      .mockResolvedValueOnce(mockUpdateStates);

    const result = await useCentralSkillsStore.getState().applyRepositorySync({
      keepSkillIds: ["code-reviewer"],
      deleteRequests: [],
      additions: [
        {
          repositoryId: "github-owner-repo-main",
          selections: [
            {
              sourcePath: "skills/new-skill",
              resolution: "overwrite",
              renamedSkillId: null,
            },
          ],
        },
      ],
      skipAdditions: [],
      unskipAdditions: [],
    });

    expect(result).toEqual(applyResult);
    expect(invoke).toHaveBeenCalledWith("apply_central_repository_sync", {
      decisions: {
        keepSkillIds: ["code-reviewer"],
        deleteRequests: [],
        additions: [
          {
            repositoryId: "github-owner-repo-main",
            selections: [
              {
                sourcePath: "skills/new-skill",
                resolution: "overwrite",
                renamedSkillId: null,
              },
            ],
          },
        ],
        skipAdditions: [],
        unskipAdditions: [],
      },
    });
    expect(invoke).toHaveBeenCalledWith("get_central_skills");
    expect(invoke).toHaveBeenCalledWith("get_skill_repositories");
    expect(invoke).toHaveBeenCalledWith("get_skill_tags");
    expect(invoke).toHaveBeenCalledWith("get_central_skill_update_states");
    expect(useCentralSkillsStore.getState().updateStatuses["frontend-design"]).toEqual(
      mockUpdateStates[0]
    );
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

  it("marks queued and running AI tag items cancelled on job-level cancellation", async () => {
    useCentralSkillsStore.setState({
      aiTagJob: {
        jobId: "job-1",
        status: "running",
        total: 3,
        completed: 1,
        succeeded: 1,
        failed: 0,
        lowConfidenceCount: 0,
        items: {
          "frontend-design": "succeeded",
          "code-reviewer": "running",
          queued: "queued",
        },
      },
    });
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
        status: "cancelled",
        total: 3,
        completed: 1,
        succeeded: 1,
        failed: 0,
        lowConfidenceCount: 0,
      },
    });

    const state = useCentralSkillsStore.getState();
    expect(state.aiTagJob.status).toBe("cancelled");
    expect(state.aiTagJob.items["frontend-design"]).toBe("succeeded");
    expect(state.aiTagJob.items["code-reviewer"]).toBe("cancelled");
    expect(state.aiTagJob.items.queued).toBe("cancelled");
  });

  it("updates central update job state from progress events", async () => {
    useCentralSkillsStore.setState((state) => ({
      updateJob: { ...state.updateJob, jobId: "update-job", status: "running" },
    }));
    let handler: ((event: { payload: unknown }) => void) | undefined;
    const unlisten = vi.fn();
    vi.mocked(listen).mockImplementation(async (_event, callback) => {
      handler = callback as (event: { payload: unknown }) => void;
      return unlisten;
    });

    await useCentralSkillsStore.getState().subscribeUpdateProgress();
    handler?.({
      payload: {
        jobId: "update-job",
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
        jobId: "stale-update-job",
        phase: "checking",
        status: "completed",
        total: 99,
        completed: 99,
        succeeded: 99,
        failed: 0,
        skipped: 0,
      },
    });
    state = useCentralSkillsStore.getState();
    expect(state.updateJob.status).toBe("running");
    expect(state.updateJob.total).not.toBe(99);

    handler?.({
      payload: {
        jobId: "update-job",
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
        jobId: "update-job",
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
        jobId: "update-job",
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

  it("updates SkillPort portability job state from progress events", async () => {
    useCentralSkillsStore.setState((state) => ({
      portabilityJob: {
        ...state.portabilityJob,
        jobId: "portability-job",
        status: "running",
      },
    }));
    let handler: ((event: { payload: unknown }) => void) | undefined;
    const unlisten = vi.fn();
    vi.mocked(listen).mockImplementation(async (_event, callback) => {
      handler = callback as (event: { payload: unknown }) => void;
      return unlisten;
    });

    await useCentralSkillsStore.getState().subscribePortabilityProgress();
    handler?.({
      payload: {
        jobId: "portability-job",
        phase: "previewing",
        status: "running",
        total: 3,
        completed: 1,
        message: "Checking GitHub source catalogs",
      },
    });

    let state = useCentralSkillsStore.getState();
    expect(state.portabilityJob.status).toBe("running");
    expect(state.portabilityJob.phase).toBe("previewing");
    expect(state.portabilityJob.completed).toBe(1);
    expect(state.portabilityJob.message).toBe("Checking GitHub source catalogs");

    handler?.({
      payload: {
        jobId: "stale-portability-job",
        phase: "previewing",
        status: "failed",
        total: 99,
        completed: 99,
        error: "stale",
      },
    });
    state = useCentralSkillsStore.getState();
    expect(state.portabilityJob.status).toBe("running");
    expect(state.portabilityJob.error).toBeUndefined();

    handler?.({
      payload: {
        jobId: "portability-job",
        phase: "previewing",
        status: "cancelled",
        total: 3,
        completed: 1,
        error: "cancelled",
      },
    });

    state = useCentralSkillsStore.getState();
    expect(state.portabilityJob.status).toBe("cancelled");
    expect(state.portabilityJob.error).toBe("cancelled");
  });

  it("cancels the active SkillPort portability job", async () => {
    useCentralSkillsStore.setState({
      portabilityJob: {
        jobId: "active-portability-job",
        phase: "importing",
        status: "running",
        total: 2,
        completed: 1,
      },
    });
    vi.mocked(invoke).mockResolvedValueOnce(undefined);

    await useCentralSkillsStore.getState().cancelSkillportStatePortability();

    expect(invoke).toHaveBeenCalledWith("cancel_skillport_state_portability", {
      jobId: "active-portability-job",
    });
    expect(useCentralSkillsStore.getState().portabilityJob.status).toBe("cancelling");
  });

  it("cancels only the active Central update job ID", async () => {
    useCentralSkillsStore.setState((state) => ({
      updateJob: {
        ...state.updateJob,
        jobId: "active-update-job",
        status: "running",
      },
    }));
    vi.mocked(invoke).mockResolvedValueOnce(undefined);

    await useCentralSkillsStore.getState().cancelCentralUpdates();

    expect(invoke).toHaveBeenCalledWith("cancel_central_skill_updates", {
      jobId: "active-update-job",
    });
    expect(useCentralSkillsStore.getState().updateJob.status).toBe("cancelling");
  });

  it("does not replace active update or portability jobs with duplicate starts", async () => {
    useCentralSkillsStore.setState((state) => ({
      updateJob: { ...state.updateJob, jobId: "active-update", status: "running" },
      portabilityJob: {
        ...state.portabilityJob,
        jobId: "active-portability",
        status: "running",
      },
    }));

    await expect(
      useCentralSkillsStore.getState().updateSkills(["frontend-design"]),
    ).rejects.toThrow("job.central_update_busy");
    await expect(
      useCentralSkillsStore.getState().exportSkillportState(),
    ).rejects.toThrow("job.portability_busy");
    expect(invoke).not.toHaveBeenCalled();
    expect(useCentralSkillsStore.getState().updateJob.jobId).toBe("active-update");
    expect(useCentralSkillsStore.getState().portabilityJob.jobId).toBe(
      "active-portability",
    );
  });

  it("exports the SkillPort portable state manifest", async () => {
    vi.mocked(invoke).mockResolvedValueOnce("{\"kind\":\"skillport/state-export\"}");

    const json = await useCentralSkillsStore.getState().exportSkillportState();

    expect(json).toBe("{\"kind\":\"skillport/state-export\"}");
    expect(invoke).toHaveBeenCalledWith("export_skillport_state", {
      jobId: expect.any(String),
      options: {},
    });
  });

  it("saves portable state through the backend file adapter", async () => {
    vi.mocked(invoke).mockResolvedValueOnce(undefined);

    await useCentralSkillsStore
      .getState()
      .saveSkillportStateExport("D:\\exports\\state.json", "{\"kind\":\"skillport/state-export\"}");

    expect(invoke).toHaveBeenCalledWith("save_skillport_state_export", {
      path: "D:\\exports\\state.json",
      json: "{\"kind\":\"skillport/state-export\"}",
    });
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
      warnings: [],
    };
    vi.mocked(invoke).mockResolvedValueOnce(preview);

    const result = await useCentralSkillsStore.getState().previewSkillportStateImport("{}");

    expect(result).toEqual(preview);
    expect(invoke).toHaveBeenCalledWith("preview_skillport_state_import", {
      jobId: expect.any(String),
      json: "{}",
    });
  });

  it("reads and previews a portable state file through one backend command", async () => {
    const result = {
      json: "{}",
      preview: {
        githubSources: [],
        skills: [],
        summary: {
          sourcesToAdd: 0,
          sourcesExisting: 0,
          sourcesDuplicate: 0,
          ready: 0,
          conflicts: 0,
          missing: 0,
          unrestorable: 0,
          duplicateSkipped: 0,
        },
        warnings: [],
      },
    };
    vi.mocked(invoke).mockResolvedValueOnce(result);

    await expect(
      useCentralSkillsStore
        .getState()
        .previewSkillportStateImportFile("D:\\imports\\state.json"),
    ).resolves.toEqual(result);
    expect(invoke).toHaveBeenCalledWith("preview_skillport_state_import_file", {
      jobId: expect.any(String),
      path: "D:\\imports\\state.json",
    });
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
      jobId: expect.any(String),
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

  it("marks portable state import failed when post-import refresh rejects", async () => {
    const refreshError = ipcFixtureError(
      "storage.unavailable",
      "Central refresh unavailable.",
    );
    const importResult = {
      sourcesAdded: 0,
      sourcesSkipped: 0,
      importedSkills: [],
      skippedSkills: [],
      failedSkills: [],
      tagsRestored: 0,
    };
    vi.mocked(invoke)
      .mockResolvedValueOnce(importResult)
      .mockRejectedValueOnce(refreshError)
      .mockResolvedValueOnce(mockRepositories)
      .mockResolvedValueOnce(mockTags)
      .mockResolvedValueOnce(mockUpdateStates);

    const importPromise = useCentralSkillsStore
      .getState()
      .importSkillportState("{\"kind\":\"skillport/state-export\"}", []);
    const jobId = useCentralSkillsStore.getState().portabilityJob.jobId;

    await expect(importPromise).rejects.toThrow("Central refresh unavailable.");

    const state = useCentralSkillsStore.getState();
    expect(jobId).toEqual(expect.any(String));
    expect(state.portabilityJob).toMatchObject({
      jobId,
      status: "failed",
      error: "Central refresh unavailable.",
    });
    expect(["running", "cancelling"]).not.toContain(state.portabilityJob.status);
  });

  it("ignores stale portable state refresh completion after target reset", async () => {
    let resolveSkills!: (skills: SkillWithLinks[]) => void;
    const importResult = {
      sourcesAdded: 0,
      sourcesSkipped: 0,
      importedSkills: [],
      skippedSkills: [],
      failedSkills: [],
      tagsRestored: 0,
    };
    vi.mocked(invoke)
      .mockResolvedValueOnce(importResult)
      .mockReturnValueOnce(new Promise<SkillWithLinks[]>((resolve) => {
        resolveSkills = resolve;
      }))
      .mockResolvedValueOnce(mockRepositories)
      .mockResolvedValueOnce(mockTags)
      .mockResolvedValueOnce(mockUpdateStates);

    const importPromise = useCentralSkillsStore
      .getState()
      .importSkillportState("{\"kind\":\"skillport/state-export\"}", []);
    await vi.waitFor(() => expect(invoke).toHaveBeenCalledTimes(5));

    useCentralSkillsStore.getState().resetForTargetChange();
    useCentralSkillsStore.setState({ skills: [mockSkills[1]] });
    resolveSkills(mockSkills);

    await expect(importPromise).resolves.toEqual(importResult);
    const state = useCentralSkillsStore.getState();
    expect(state.skills).toEqual([mockSkills[1]]);
    expect(state.repositories).toEqual([]);
    expect(state.tags).toEqual([]);
    expect(state.updateStatuses).toEqual({});
    expect(state.portabilityJob).toMatchObject({ jobId: null, status: "idle" });
  });

  it("ignores stale portable state refresh error after target reset", async () => {
    let rejectSkills!: (error: unknown) => void;
    const refreshError = ipcFixtureError(
      "storage.unavailable",
      "Stale Central refresh unavailable.",
    );
    const importResult = {
      sourcesAdded: 0,
      sourcesSkipped: 0,
      importedSkills: [],
      skippedSkills: [],
      failedSkills: [],
      tagsRestored: 0,
    };
    vi.mocked(invoke)
      .mockResolvedValueOnce(importResult)
      .mockReturnValueOnce(new Promise<SkillWithLinks[]>((_resolve, reject) => {
        rejectSkills = reject;
      }))
      .mockResolvedValueOnce(mockRepositories)
      .mockResolvedValueOnce(mockTags)
      .mockResolvedValueOnce(mockUpdateStates);

    const importPromise = useCentralSkillsStore
      .getState()
      .importSkillportState("{\"kind\":\"skillport/state-export\"}", []);
    await vi.waitFor(() => expect(invoke).toHaveBeenCalledTimes(5));

    useCentralSkillsStore.getState().resetForTargetChange();
    useCentralSkillsStore.setState({ skills: [mockSkills[1]] });
    rejectSkills(refreshError);

    await expect(importPromise).rejects.toThrow("Stale Central refresh unavailable.");
    const state = useCentralSkillsStore.getState();
    expect(state.skills).toEqual([mockSkills[1]]);
    expect(state.error).toBeNull();
    expect(state.portabilityJob).toMatchObject({ jobId: null, status: "idle" });
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

  it("refreshes dashboard central summary after checking skill updates (AC6)", async () => {
    const summary = {
      centralSkillCount: 5,
      updatesAvailable: 1,
      aiReviewCount: 0,
      uncategorizedCount: 0,
      unassignedSourceCount: 0,
      readiness: {
        score: 80,
        categorizedRatio: 1,
        describedRatio: 1,
        sourcedRatio: 1,
        installHealthRatio: 0.6,
      },
      sourceRepositories: [],
    };
    vi.mocked(invoke)
      .mockResolvedValueOnce(mockUpdateStates)
      .mockResolvedValueOnce(summary);

    await useCentralSkillsStore.getState().checkSkillUpdates(["frontend-design"]);

    expect(invoke).toHaveBeenCalledWith("get_dashboard_central_summary");
    await vi.waitFor(() => {
      expect(usePlatformStore.getState().dashboardCentralSummary).toEqual(
        summary,
      );
    });
  });

  it("refreshes dashboard central summary after applying skill updates (AC6)", async () => {
    const summary = {
      centralSkillCount: 4,
      updatesAvailable: 0,
      aiReviewCount: 0,
      uncategorizedCount: 0,
      unassignedSourceCount: 0,
      readiness: {
        score: 100,
        categorizedRatio: 1,
        describedRatio: 1,
        sourcedRatio: 1,
        installHealthRatio: 1,
      },
      sourceRepositories: [],
    };
    const updateResult = {
      succeeded: ["frontend-design"],
      failed: [],
      skipped: [],
      states: mockUpdateStates,
    };
    vi.mocked(invoke)
      .mockResolvedValueOnce(updateResult)
      .mockResolvedValueOnce(mockSkills)
      .mockResolvedValueOnce(summary);

    await useCentralSkillsStore.getState().updateSkills(["frontend-design"]);

    expect(invoke).toHaveBeenCalledWith("get_dashboard_central_summary");
    await vi.waitFor(() => {
      expect(usePlatformStore.getState().dashboardCentralSummary).toEqual(
        summary,
      );
    });
  });
});
