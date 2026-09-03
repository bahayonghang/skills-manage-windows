import {
  AgentWithStatus,
  AiTagJob,
  AiTagProgressPayload,
  CentralSkillUpdateJob,
  CentralSkillUpdateProgressPayload,
  CentralSkillUpdateState,
  SkillportStatePortabilityJob,
  SkillportStatePortabilityProgressPayload,
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
export const PORTABILITY_PROGRESS_EVENT = "central://state-portability-progress";

const BROWSER_UNKNOWN_REPOSITORY: SkillRepositoryWithStats = {
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

const BROWSER_FIXTURE_SKILL_COUNT = 72;
const BROWSER_FIXTURE_LONG_WINDOWS_PATH =
  "C:\\Users\\fixture-user\\Documents\\SkillPort Validation Workspace\\deeply-nested-project\\.agents\\skills";

const BROWSER_FIXTURE_PRIMARY_SKILL: SkillWithLinks = {
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
};

function createDenseBrowserFixtureSkill(index: number): SkillWithLinks {
  const paddedIndex = String(index).padStart(2, "0");
  const skillName =
    index % 3 === 0
      ? `中文高密度排版验证技能-${paddedIndex}`
      : `dense-typography-validation-skill-with-a-deliberately-long-name-${paddedIndex}`;
  const canonicalPath = `${BROWSER_FIXTURE_LONG_WINDOWS_PATH}\\${skillName}`;

  return {
    ...BROWSER_FIXTURE_PRIMARY_SKILL,
    id: `fixture-central-skill-${paddedIndex}`,
    name: skillName,
    description:
      index % 2 === 0
        ? "用于验证中文元数据、状态标签、长路径以及三档字号缩放下卡片布局不会重叠或截断。"
        : "Browser fixture with deliberately long English metadata for validating dense card layout, truncation, and fixed-height virtualization across every supported font scale.",
    file_path: `${canonicalPath}\\SKILL.md`,
    canonical_path: canonicalPath,
    linked_agents: index % 2 === 0 ? ["claude-code", "cursor"] : ["claude-code"],
    tags: index % 4 === 0 ? BROWSER_TAGS : [BROWSER_TAGS[index % BROWSER_TAGS.length]],
  };
}

export const BROWSER_FIXTURE_SKILLS: SkillWithLinks[] = [
  BROWSER_FIXTURE_PRIMARY_SKILL,
  ...Array.from(
    { length: BROWSER_FIXTURE_SKILL_COUNT - 1 },
    (_, offset) => createDenseBrowserFixtureSkill(offset + 1),
  ),
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
    jobId: null,
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

export function createIdlePortabilityJob(): SkillportStatePortabilityJob {
  return {
    jobId: null,
    phase: null,
    status: "idle",
    total: 0,
    completed: 0,
  };
}

export function createRunningUpdateJob(
  phase: NonNullable<CentralSkillUpdateJob["phase"]>,
  skillIds: string[],
  jobId = createLocalJobId(),
): CentralSkillUpdateJob {
  return {
    jobId,
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

export function createLocalJobId(): string {
  return globalThis.crypto?.randomUUID?.() ?? `job-${Date.now()}-${Math.random().toString(16).slice(2)}`;
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
  if (payload.status === "cancelled") {
    for (const [skillId, status] of Object.entries(items)) {
      if (status === "queued" || status === "running") {
        items[skillId] = "cancelled";
      }
    }
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
  if (payload.jobId !== current.jobId) {
    return current;
  }
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

export function mergePortabilityProgress(
  current: SkillportStatePortabilityJob,
  payload: SkillportStatePortabilityProgressPayload
): SkillportStatePortabilityJob {
  if (payload.jobId !== current.jobId) {
    return current;
  }
  const status =
    payload.status === "completed"
      ? "completed"
      : payload.status === "cancelled"
        ? "cancelled"
        : payload.status === "failed"
          ? "failed"
          : current.status === "cancelling"
            ? "cancelling"
            : "running";

  return {
    ...current,
    phase: payload.phase,
    status,
    total: payload.total,
    completed: payload.completed,
    message: payload.message ?? current.message,
    currentItem: payload.currentItem ?? current.currentItem,
    error: status === "completed" ? undefined : payload.error ?? current.error,
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
    portabilityJob: createIdlePortabilityJob(),
    aiTaggingAvailable: false,
    isLoading: false,
    hasLoaded: true,
    isRefreshingList: false,
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
    portabilityJob: createIdlePortabilityJob(),
    aiTaggingAvailable: false,
    isLoading: false,
    hasLoaded: false,
    isRefreshingList: false,
    isInstalling: false,
    isDeleting: false,
    isMetadataUpdating: false,
    isSuggestingTags: false,
    isCheckingUpdates: false,
    updatingSkillIds: [],
    togglingAgentId: null,
    requiresCentralReload: false,
    error: null,
  };
}
