import { create } from "zustand";
import { invoke, isTauriRuntime, listen } from "@/lib/tauri";
import {
  AgentWithStatus,
  AiTagJob,
  AiTagProgressPayload,
  BatchInstallResult,
  SkillAiTagReview,
  SkillRepository,
  SkillRepositoryWithStats,
  SkillTag,
  SkillTagSuggestionResult,
  SkillWithLinks,
} from "@/types";

const AI_TAG_PROGRESS_EVENT = "central://ai-tag-progress";

const BROWSER_UNKNOWN_REPOSITORY: SkillRepositoryWithStats = {
  id: "local-unknown",
  name: "本地 / 未知来源",
  source_type: "local",
  is_unknown: true,
  created_at: "2026-04-17T00:00:00.000Z",
  updated_at: "2026-04-17T00:00:00.000Z",
  skill_count: 0,
  unknown_skill_count: 0,
};

const BROWSER_TAGS: SkillTag[] = [
  {
    id: "programming-agent-engineering",
    name: "编程与 Agent 工程",
    description: "Browser fixture category",
    color: "#7c3aed",
    is_builtin: true,
    created_at: "2026-04-17T00:00:00.000Z",
    updated_at: "2026-04-17T00:00:00.000Z",
  },
  {
    id: "uncategorized",
    name: "未分类",
    description: "Browser fixture category",
    color: "#71717a",
    is_builtin: true,
    created_at: "2026-04-17T00:00:00.000Z",
    updated_at: "2026-04-17T00:00:00.000Z",
  },
];

export const BROWSER_FIXTURE_AGENTS: AgentWithStatus[] = [
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
    global_skills_dir: "~/.agents/skills/",
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

export const BROWSER_FIXTURE_SKILLS: SkillWithLinks[] = [
  {
    id: "fixture-central-skill",
    name: "fixture-central-skill",
    description: "Browser validation fixture for Central and drawer entry flows.",
    file_path: "~/.agents/skills/fixture-central-skill/SKILL.md",
    canonical_path: "~/.agents/skills/fixture-central-skill",
    is_central: true,
    source: "browser-fixture",
    scanned_at: "2026-04-17T00:00:00.000Z",
    created_at: "2026-04-17T00:00:00.000Z",
    updated_at: "2026-04-17T00:00:00.000Z",
    linked_agents: ["claude-code", "cursor"],
    shared_root_agents: ["cursor"],
    repository: BROWSER_UNKNOWN_REPOSITORY,
    tags: [BROWSER_TAGS[0]],
    is_source_unknown: true,
  },
];

function createIdleAiTagJob(): AiTagJob {
  return {
    jobId: null,
    status: "idle",
    total: 0,
    completed: 0,
    succeeded: 0,
    failed: 0,
    lowConfidenceCount: 0,
    items: {},
  };
}

function createRunningAiTagJob(skillIds: string[]): AiTagJob {
  return {
    jobId: createLocalJobId(),
    status: "running",
    total: skillIds.length,
    completed: 0,
    succeeded: 0,
    failed: 0,
    lowConfidenceCount: 0,
    items: Object.fromEntries(skillIds.map((skillId) => [skillId, "queued"])),
  };
}

function createLocalJobId(): string {
  return globalThis.crypto?.randomUUID?.() ?? `ai-tag-${Date.now()}`;
}

function mergeAiTagProgress(current: AiTagJob, payload: AiTagProgressPayload): AiTagJob {
  const items = { ...current.items };
  if (payload.skillId && payload.status === "running") {
    items[payload.skillId] = "running";
  }
  if (payload.skillId && payload.status === "succeeded") {
    items[payload.skillId] = "succeeded";
  }
  if (payload.skillId && payload.status === "failed") {
    items[payload.skillId] = "failed";
  }
  if (payload.skillId && payload.status === "cancelled") {
    items[payload.skillId] = "cancelled";
  }

  const status =
    payload.status === "completed"
      ? "completed"
      : payload.status === "cancelled"
        ? "cancelled"
        : "running";

  return {
    ...current,
    jobId: payload.jobId,
    status,
    total: payload.total,
    completed: payload.completed,
    succeeded: payload.succeeded,
    failed: payload.failed,
    lowConfidenceCount: payload.lowConfidenceCount ?? current.lowConfidenceCount,
    currentSkillName: payload.skillName ?? current.currentSkillName,
    error: payload.error ?? current.error,
    items,
  };
}

function summarizeAiTagResults(skillIds: string[], results: SkillTagSuggestionResult[]): AiTagJob {
  const items: AiTagJob["items"] = Object.fromEntries(
    skillIds.map((skillId) => [skillId, "failed"])
  );
  let succeeded = 0;
  let failed = 0;
  let lowConfidenceCount = 0;
  for (const result of results) {
    const didSucceed = result.succeeded !== false && !result.error;
    items[result.skill_id] = didSucceed ? "succeeded" : "failed";
    if (didSucceed) {
      succeeded += 1;
    } else {
      failed += 1;
    }
    lowConfidenceCount += result.low_confidence_count ?? 0;
  }

  return {
    jobId: createLocalJobId(),
    status: "completed",
    total: skillIds.length,
    completed: results.length,
    succeeded,
    failed,
    lowConfidenceCount,
    items,
  };
}

// ─── State ────────────────────────────────────────────────────────────────────

interface CentralSkillsState {
  skills: SkillWithLinks[];
  agents: AgentWithStatus[];
  repositories: SkillRepositoryWithStats[];
  tags: SkillTag[];
  aiTagReviews: SkillAiTagReview[];
  aiTagJob: AiTagJob;
  aiTaggingAvailable: boolean;
  isLoading: boolean;
  isInstalling: boolean;
  isMetadataUpdating: boolean;
  isSuggestingTags: boolean;
  /** Agent ID currently being toggled (null = idle). */
  togglingAgentId: string | null;
  error: string | null;

  // Actions
  loadCentralSkills: () => Promise<void>;
  installSkill: (
    skillId: string,
    agentIds: string[],
    method: string
  ) => Promise<BatchInstallResult>;
  togglePlatformLink: (skillId: string, agentId: string) => Promise<void>;
  createRepository: (name: string) => Promise<SkillRepository>;
  assignSkillsToRepository: (skillIds: string[], repositoryId: string) => Promise<void>;
  createTag: (name: string) => Promise<SkillTag>;
  assignSkillTags: (skillIds: string[], tagIds: string[]) => Promise<void>;
  bulkSuggestSkillTags: (skillIds: string[]) => Promise<SkillTagSuggestionResult[]>;
  cancelAiTagJob: () => Promise<void>;
  loadAiTagReviews: () => Promise<void>;
  acceptAiTagReview: (skillId: string, tagIds: string[]) => Promise<void>;
  skipAiTagReview: (skillId: string) => Promise<void>;
  subscribeAiTagProgress: () => Promise<() => void>;
}

// ─── Store ────────────────────────────────────────────────────────────────────

export const useCentralSkillsStore = create<CentralSkillsState>((set, get) => ({
  skills: [],
  agents: [],
  repositories: [],
  tags: [],
  aiTagReviews: [],
  aiTagJob: createIdleAiTagJob(),
  aiTaggingAvailable: false,
  isLoading: false,
  isInstalling: false,
  isMetadataUpdating: false,
  isSuggestingTags: false,
  togglingAgentId: null,
  error: null,

  /**
   * Load all Central Skills with per-platform link status, along with the
   * list of all registered agents. Called when navigating to /central.
   */
  loadCentralSkills: async () => {
    set({ isLoading: true, error: null });
    if (!isTauriRuntime()) {
      set({
        skills: BROWSER_FIXTURE_SKILLS,
        agents: BROWSER_FIXTURE_AGENTS,
        repositories: [BROWSER_UNKNOWN_REPOSITORY],
        tags: BROWSER_TAGS,
        aiTagReviews: [],
        aiTaggingAvailable: false,
        isLoading: false,
      });
      return;
    }
    try {
      const [skills, agents, repositories, tags, reviews, aiApiKey] = await Promise.all([
        invoke<SkillWithLinks[]>("get_central_skills"),
        invoke<AgentWithStatus[]>("get_agents"),
        invoke<SkillRepositoryWithStats[]>("get_skill_repositories"),
        invoke<SkillTag[]>("get_skill_tags"),
        invoke<SkillAiTagReview[]>("get_pending_ai_tag_reviews"),
        Promise.resolve(invoke<string | null>("get_setting", { key: "ai_api_key" })).catch(() => null),
      ]);
      set({
        skills: skills ?? [],
        agents: agents ?? [],
        repositories: repositories ?? [],
        tags: tags ?? [],
        aiTagReviews: reviews ?? [],
        aiTaggingAvailable: !!aiApiKey,
        isLoading: false,
      });
    } catch (err) {
      set({ error: String(err), isLoading: false });
    }
  },

  /**
   * Install a skill to one or more agents. Refreshes the skill list after
   * a successful (or partial) install so link status icons update.
   */
  installSkill: async (skillId, agentIds, method) => {
    set({ isInstalling: true, error: null });
    try {
      const result = await invoke<BatchInstallResult>("batch_install_to_agents", {
        skillId,
        agentIds,
        method,
      });

      // Refresh central skills to get updated link status.
      const skills = await invoke<SkillWithLinks[]>("get_central_skills");
      const repositories = await invoke<SkillRepositoryWithStats[]>("get_skill_repositories");
      set({ skills, repositories: repositories ?? get().repositories, isInstalling: false });

      return result;
    } catch (err) {
      set({ error: String(err), isInstalling: false });
      throw err;
    }
  },

  /**
   * Toggle a single platform link for a skill.
   * If linked, uninstalls; if not linked, installs via the backend default method.
   * Refreshes the skill list afterward so linked_agents updates.
   */
  togglePlatformLink: async (skillId, agentId) => {
    set({ togglingAgentId: agentId, error: null });
    try {
      const skill = get().skills.find((s) => s.id === skillId);
      const isLinked = skill?.linked_agents.includes(agentId) ?? false;

      if (isLinked) {
        await invoke("uninstall_skill_from_agent", { skillId, agentId });
      } else {
        await invoke("install_skill_to_agent", { skillId, agentId, method: "auto" });
      }

      const skills = await invoke<SkillWithLinks[]>("get_central_skills");
      const repositories = await invoke<SkillRepositoryWithStats[]>("get_skill_repositories");
      set({ skills, repositories: repositories ?? get().repositories, togglingAgentId: null });
    } catch (err) {
      set({ error: String(err), togglingAgentId: null });
      throw err;
    }
  },

  createRepository: async (name) => {
    if (!isTauriRuntime()) {
      throw new Error("Desktop-only feature: repository metadata is available in the Tauri app.");
    }
    set({ isMetadataUpdating: true, error: null });
    try {
      const repository = await invoke<SkillRepository>("create_or_update_skill_repository", {
        name,
        sourceType: "manual",
      });
      const repositories = await invoke<SkillRepositoryWithStats[]>("get_skill_repositories");
      set({ repositories, isMetadataUpdating: false });
      return repository;
    } catch (err) {
      set({ error: String(err), isMetadataUpdating: false });
      throw err;
    }
  },

  assignSkillsToRepository: async (skillIds, repositoryId) => {
    set({ isMetadataUpdating: true, error: null });
    try {
      await invoke("assign_skills_to_repository", { skillIds, repositoryId });
      const [skills, repositories] = await Promise.all([
        invoke<SkillWithLinks[]>("get_central_skills"),
        invoke<SkillRepositoryWithStats[]>("get_skill_repositories"),
      ]);
      set({ skills, repositories, isMetadataUpdating: false });
    } catch (err) {
      set({ error: String(err), isMetadataUpdating: false });
      throw err;
    }
  },

  createTag: async (name) => {
    if (!isTauriRuntime()) {
      throw new Error("Desktop-only feature: tag metadata is available in the Tauri app.");
    }
    set({ isMetadataUpdating: true, error: null });
    try {
      const tag = await invoke<SkillTag>("create_skill_tag", { name });
      const tags = await invoke<SkillTag[]>("get_skill_tags");
      set({ tags, isMetadataUpdating: false });
      return tag;
    } catch (err) {
      set({ error: String(err), isMetadataUpdating: false });
      throw err;
    }
  },

  assignSkillTags: async (skillIds, tagIds) => {
    set({ isMetadataUpdating: true, error: null });
    try {
      await invoke("assign_skill_tags", { skillIds, tagIds });
      const skills = await invoke<SkillWithLinks[]>("get_central_skills");
      set({ skills, isMetadataUpdating: false });
    } catch (err) {
      set({ error: String(err), isMetadataUpdating: false });
      throw err;
    }
  },

  loadAiTagReviews: async () => {
    if (!isTauriRuntime()) {
      set({ aiTagReviews: [] });
      return;
    }
    const reviews = await invoke<SkillAiTagReview[]>("get_pending_ai_tag_reviews");
    set({ aiTagReviews: reviews ?? [] });
  },

  acceptAiTagReview: async (skillId, tagIds) => {
    set({ isMetadataUpdating: true, error: null });
    try {
      await invoke("accept_ai_tag_review", { skillId, tagIds });
      const [skills, reviews] = await Promise.all([
        invoke<SkillWithLinks[]>("get_central_skills"),
        invoke<SkillAiTagReview[]>("get_pending_ai_tag_reviews"),
      ]);
      set({ skills, aiTagReviews: reviews ?? [], isMetadataUpdating: false });
    } catch (err) {
      set({ error: String(err), isMetadataUpdating: false });
      throw err;
    }
  },

  skipAiTagReview: async (skillId) => {
    set({ isMetadataUpdating: true, error: null });
    try {
      await invoke("skip_ai_tag_review", { skillId });
      const reviews = await invoke<SkillAiTagReview[]>("get_pending_ai_tag_reviews");
      set({ aiTagReviews: reviews ?? [], isMetadataUpdating: false });
    } catch (err) {
      set({ error: String(err), isMetadataUpdating: false });
      throw err;
    }
  },

  bulkSuggestSkillTags: async (skillIds) => {
    if (skillIds.length === 0) {
      return [];
    }

    set({
      isSuggestingTags: true,
      error: null,
      aiTagJob: createRunningAiTagJob(skillIds),
    });
    try {
      const result = await invoke<SkillTagSuggestionResult[]>("bulk_suggest_skill_tags", {
        skillIds,
      });
      const [skills, reviews] = await Promise.all([
        invoke<SkillWithLinks[]>("get_central_skills"),
        invoke<SkillAiTagReview[]>("get_pending_ai_tag_reviews"),
      ]);
      set((state) => ({
        skills,
        aiTagReviews: reviews ?? [],
        isSuggestingTags: false,
        aiTagJob:
          state.aiTagJob.status === "completed" || state.aiTagJob.status === "cancelled"
            ? state.aiTagJob
            : summarizeAiTagResults(skillIds, result),
      }));
      return result;
    } catch (err) {
      set((state) => ({
        error: String(err),
        isSuggestingTags: false,
        aiTagJob: {
          ...state.aiTagJob,
          status: "failed",
          error: String(err),
        },
      }));
      throw err;
    }
  },

  cancelAiTagJob: async () => {
    const jobId = get().aiTagJob.jobId;
    if (!jobId) {
      return;
    }

    await invoke("cancel_ai_tag_job", { jobId });
    set((state) => ({
      aiTagJob: {
        ...state.aiTagJob,
        status: "cancelled",
        error: state.aiTagJob.error ?? "AI tagging cancellation requested",
      },
    }));
  },

  subscribeAiTagProgress: async () => {
    if (!isTauriRuntime()) {
      return () => {};
    }

    return listen<AiTagProgressPayload>(AI_TAG_PROGRESS_EVENT, (event) => {
      set((state) => ({
        aiTagJob: mergeAiTagProgress(state.aiTagJob, event.payload),
      }));
    });
  },
}));
