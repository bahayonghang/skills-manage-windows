import {
  AgentWithStatus,
  AiTagJob,
  AiTagProgressPayload,
  CentralSkillUpdateJob,
  CentralSkillUpdateProgressPayload,
  CentralSkillUpdateState,
  SkillRepositoryWithStats,
  SkillTag,
  SkillTagSuggestionResult,
  SkillWithLinks,
} from "@/types";
import {
  applyPlatformPathsToAgents,
  BROWSER_PLATFORM_PATHS,
  getPlatformSkillDir,
  getPlatformSkillFilePath,
} from "@/lib/platformPathPolicy";

export const AI_TAG_PROGRESS_EVENT = "central://ai-tag-progress";
export const CENTRAL_UPDATE_PROGRESS_EVENT = "central://skill-update-progress";

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
    global_skills_dir: "",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
  {
    id: "cursor",
    display_name: "Cursor",
    category: "coding",
    global_skills_dir: "",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
  {
    id: "central",
    display_name: "Central Skills",
    category: "central",
    global_skills_dir: "",
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
    file_path: getPlatformSkillFilePath(
      BROWSER_PLATFORM_PATHS,
      "central",
      "fixture-central-skill"
    ),
    canonical_path: getPlatformSkillDir(
      BROWSER_PLATFORM_PATHS,
      "central",
      "fixture-central-skill"
    ),
    is_central: true,
    source: "browser-fixture",
    scanned_at: "2026-04-17T00:00:00.000Z",
    created_at: "2026-04-17T00:00:00.000Z",
    updated_at: "2026-04-17T00:00:00.000Z",
    linked_agents: ["claude-code", "cursor"],
    shared_root_agents: [],
    repository: BROWSER_UNKNOWN_REPOSITORY,
    tags: [BROWSER_TAGS[0]],
    is_source_unknown: true,
  },
];

export function createIdleAiTagJob(): AiTagJob {
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

export function createIdleUpdateJob(): CentralSkillUpdateJob {
  return {
    phase: null,
    status: "idle",
    total: 0,
    completed: 0,
    succeeded: 0,
    failed: 0,
    skipped: 0,
    items: {},
  };
}

export function createRunningUpdateJob(
  phase: NonNullable<CentralSkillUpdateJob["phase"]>,
  skillIds: string[]
): CentralSkillUpdateJob {
  return {
    phase,
    status: "running",
    total: skillIds.length,
    completed: 0,
    succeeded: 0,
    failed: 0,
    skipped: 0,
    items: Object.fromEntries(skillIds.map((skillId) => [skillId, "queued"])),
  };
}

export function createRunningAiTagJob(skillIds: string[]): AiTagJob {
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

export function mergeAiTagProgress(current: AiTagJob, payload: AiTagProgressPayload): AiTagJob {
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

export function mergeUpdateProgress(
  current: CentralSkillUpdateJob,
  payload: CentralSkillUpdateProgressPayload
): CentralSkillUpdateJob {
  const items = { ...current.items };
  if (payload.skillId && payload.status === "running") {
    items[payload.skillId] = "running";
  }
  if (payload.skillId && (payload.status === "up_to_date" || payload.status === "update_available")) {
    items[payload.skillId] = "succeeded";
  }
  if (payload.skillId && (payload.status === "unsupported" || payload.status === "remote_missing")) {
    items[payload.skillId] = "skipped";
  }
  if (payload.skillId && payload.status === "error") {
    items[payload.skillId] = "failed";
  }

  const nextStatus: CentralSkillUpdateJob["status"] =
    payload.status === "cancelled"
      ? "cancelled"
      : payload.status === "completed"
        ? payload.failed > 0
          ? "failed"
          : "completed"
        : current.status === "cancelling"
          ? "cancelling"
          : "running";

  return {
    ...current,
    phase: payload.phase,
    status: nextStatus,
    total: payload.total,
    completed: payload.completed,
    succeeded: payload.succeeded,
    failed: payload.failed,
    skipped: payload.skipped,
    currentSkillName: payload.skillName ?? current.currentSkillName,
    error:
      payload.status === "completed" && payload.failed === 0
        ? undefined
        : payload.error ?? current.error,
    items,
  };
}

export function indexUpdateStates(
  states: CentralSkillUpdateState[]
): Record<string, CentralSkillUpdateState> {
  return Object.fromEntries(states.map((state) => [state.skill_id, state]));
}

export function mergeUpdateStates(
  current: Record<string, CentralSkillUpdateState>,
  states: CentralSkillUpdateState[]
): Record<string, CentralSkillUpdateState> {
  return {
    ...current,
    ...indexUpdateStates(states),
  };
}

export function summarizeAiTagResults(skillIds: string[], results: SkillTagSuggestionResult[]): AiTagJob {
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

export function createCentralBrowserFixtureState() {
  return {
    skills: BROWSER_FIXTURE_SKILLS,
    agents: applyPlatformPathsToAgents(BROWSER_FIXTURE_AGENTS, BROWSER_PLATFORM_PATHS),
    repositories: [BROWSER_UNKNOWN_REPOSITORY],
    tags: BROWSER_TAGS,
    aiTagReviews: [],
    updateStatuses: {},
    aiTaggingAvailable: false,
    isLoading: false,
    error: null,
  };
}

export function createCentralSkillsInitialState() {
  return {
    skills: [],
    agents: [],
    repositories: [],
    tags: [],
    aiTagReviews: [],
    aiTagJob: createIdleAiTagJob(),
    updateStatuses: {},
    updateJob: createIdleUpdateJob(),
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
  };
}
