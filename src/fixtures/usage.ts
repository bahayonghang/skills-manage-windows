import { registerIpcFixtures } from "@/lib/ipc";
import type {
  ProviderHealth,
  RecentSkillCall,
  SkillUsageDetail,
  SkillUsageSummary,
  UnusedSkillEntry,
  UnusedSkillsReport,
  UsageOverview,
  UsageScopeInfo,
} from "@/types/usage";

const now = Date.now();

function localDate(daysAgo: number): string {
  const date = new Date();
  date.setHours(12, 0, 0, 0);
  date.setDate(date.getDate() - daysAgo);
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

const BROWSER_FIXTURE_SKILLS: SkillUsageSummary[] = [
  {
    skill: "skill-creator",
    count: 38,
    projects: 6,
    sessions: 14,
    lastUsedMs: now - 35 * 60_000,
    matchStatus: "matched",
    resolvedSkillId: "skill-creator",
    staticTokenEstimate: 1840,
    staticByteCount: 7216,
  },
  {
    skill: "git-commit",
    count: 24,
    projects: 4,
    sessions: 11,
    lastUsedMs: now - 4 * 3_600_000,
    matchStatus: "ambiguous",
    resolvedSkillId: null,
    staticTokenEstimate: null,
    staticByteCount: null,
  },
  {
    skill: "trellis-check",
    count: 17,
    projects: 5,
    sessions: 9,
    lastUsedMs: now - 26 * 3_600_000,
    matchStatus: "matched",
    resolvedSkillId: "trellis-check",
    staticTokenEstimate: null,
    staticByteCount: null,
  },
  {
    skill: "legacy-review",
    count: 9,
    projects: 2,
    sessions: 5,
    lastUsedMs: now - 4 * 86_400_000,
    matchStatus: "unmatched",
    resolvedSkillId: null,
    staticTokenEstimate: null,
    staticByteCount: null,
  },
];

const BROWSER_FIXTURE_RECENT: RecentSkillCall[] = BROWSER_FIXTURE_SKILLS.map(
  (skill, index) => ({
    skill: skill.skill,
    timestampMs: skill.lastUsedMs,
    project: `C:/Users/demo/projects/${["skills-manage", "codex-next", "docs", "legacy"][index]}`,
    sessionId: `fixture-${index + 1}`,
    source: index % 2 === 0 ? "Codex CLI" : "Claude Code",
    matchStatus: skill.matchStatus,
    resolvedSkillId: skill.resolvedSkillId,
  }),
);

const BROWSER_FIXTURE_OVERVIEW: UsageOverview = {
  kpis: {
    totalCalls: 88,
    uniqueSkills: 4,
    uniqueProjects: 9,
    uniqueSources: 2,
    uniqueSessions: 31,
  },
  topSkills: BROWSER_FIXTURE_SKILLS,
  heatmap: Array.from({ length: 16 * 7 }, (_, i) => ({
    date: localDate(16 * 7 - 1 - i),
    count: i % 13 === 0 ? 7 : i % 7 === 0 ? 3 : i % 5 === 0 ? 1 : 0,
  })),
  lastScanMs: now - 5 * 60_000,
};

const BROWSER_FIXTURE_PROVIDERS: ProviderHealth[] = [
  "claude-code",
  "codex",
  "droid",
  "opencode",
  "grok",
  "antigravity",
  "kiro",
  "zed",
].map((id, index) => ({
  providerId: id,
  displayName:
    id === "claude-code" ? "Claude Code" : id === "codex" ? "Codex CLI" : id,
  available: index < 5,
  callCount: index === 0 ? 36 : index === 1 ? 52 : 0,
  scannedAtMs: now - 5 * 60_000,
}));

// 未使用报告 fixture：覆盖 matched/ambiguous/unmatched、缺失静态估算、
// never_used 与不同未用时长，供阈值切换的视图本地重分类演示。
const UNUSED_FIXTURE_ENTRIES: UnusedSkillEntry[] = [
  {
    skillId: "legacy-cleanup",
    name: "legacy-cleanup",
    matchStatus: "matched",
    origin: "central",
    agents: ["claude-code"],
    installedPath: "C:/Users/demo/.skillport/skills/legacy-cleanup",
    callCount: 0,
    lastUsedMs: null,
    staticTokenEstimate: 960,
    staticByteCount: 3_840,
    status: "never_used",
  },
  {
    skillId: "trellis-check",
    name: "trellis-check",
    matchStatus: "matched",
    origin: "central",
    agents: ["claude-code", "codex"],
    installedPath: "C:/Users/demo/.skillport/skills/trellis-check",
    callCount: 17,
    lastUsedMs: now - 45 * 86_400_000,
    staticTokenEstimate: null,
    staticByteCount: null,
    status: "stale",
  },
  {
    skillId: null,
    name: "prompt-helper",
    matchStatus: "ambiguous",
    origin: "platform",
    agents: ["codex"],
    installedPath: "C:/Users/demo/.codex/skills/prompt-helper",
    callCount: 3,
    lastUsedMs: now - 120 * 86_400_000,
    staticTokenEstimate: null,
    staticByteCount: null,
    status: "stale",
  },
  {
    skillId: null,
    name: "local-notes",
    matchStatus: "unmatched",
    origin: "platform",
    agents: ["claude-code", "zed"],
    installedPath: "C:/Users/demo/.claude/skills/local-notes",
    callCount: 0,
    lastUsedMs: null,
    staticTokenEstimate: null,
    staticByteCount: null,
    status: "never_used",
  },
];

function unusedReportFor(thresholdDays: number | null): UnusedSkillsReport {
  const threshold = (thresholdDays ?? 90) * 86_400_000;
  const visible = UNUSED_FIXTURE_ENTRIES.filter(
    (entry) =>
      entry.callCount === 0 ||
      entry.lastUsedMs === null ||
      now - entry.lastUsedMs >= threshold,
  );
  return {
    central: visible.filter((entry) => entry.origin === "central"),
    platforms: visible.filter((entry) => entry.origin === "platform"),
  };
}

function detailFor(skill: string): SkillUsageDetail {
  const summary =
    BROWSER_FIXTURE_SKILLS.find((item) => item.skill === skill) ??
    BROWSER_FIXTURE_SKILLS[0];
  return {
    skill: summary.skill,
    count: summary.count,
    sessions: summary.sessions,
    firstUsedMs: now - 45 * 86_400_000,
    lastUsedMs: summary.lastUsedMs,
    byProject: [
      {
        project: "C:/Users/demo/projects/skills-manage",
        count: Math.max(1, summary.count - 8),
        sessions: Math.max(1, summary.sessions - 3),
        lastUsedMs: summary.lastUsedMs,
      },
      {
        project: "C:/Users/demo/projects/codex-next",
        count: 8,
        sessions: 3,
        lastUsedMs: summary.lastUsedMs - 86_400_000,
      },
    ],
    weekly: BROWSER_FIXTURE_OVERVIEW.heatmap.map((day, index) => ({
      ...day,
      count: index % 11 === 0 ? 2 : index % 7 === 0 ? 1 : 0,
    })),
    matchStatus: summary.matchStatus,
    resolvedSkillId: summary.resolvedSkillId,
    staticTokenEstimate: summary.staticTokenEstimate,
    staticByteCount: summary.staticByteCount,
  };
}

const BROWSER_FIXTURE_SCOPE: UsageScopeInfo = {
  targetId: "local",
  label: "Local",
  isRemote: false,
  remoteReachable: false,
};

function overviewFor(source: string | null): UsageOverview {
  if (!source) return BROWSER_FIXTURE_OVERVIEW;
  const names = new Set(
    BROWSER_FIXTURE_RECENT.filter((call) => call.source === source).map(
      (call) => call.skill,
    ),
  );
  const topSkills = BROWSER_FIXTURE_SKILLS.filter((skill) => names.has(skill.skill));
  return {
    ...BROWSER_FIXTURE_OVERVIEW,
    kpis: {
      totalCalls: topSkills.reduce((total, skill) => total + skill.count, 0),
      uniqueSkills: topSkills.length,
      uniqueProjects: new Set(topSkills.flatMap((skill) => [skill.skill])).size,
      uniqueSources: topSkills.length > 0 ? 1 : 0,
      uniqueSessions: topSkills.reduce(
        (total, skill) => total + skill.sessions,
        0,
      ),
    },
    topSkills,
    heatmap: BROWSER_FIXTURE_OVERVIEW.heatmap.map((day) => ({
      ...day,
      count: Math.floor(day.count / 2),
    })),
  };
}

export function registerUsageFixtures(): void {
  registerIpcFixtures({
    usage_refresh: () => ({
      summary: {
        cached: false,
        callsWritten: 88,
        providersAvailable: 5,
        scannedAtMs: now,
      },
      overview: BROWSER_FIXTURE_OVERVIEW,
      recent: BROWSER_FIXTURE_RECENT,
      providers: BROWSER_FIXTURE_PROVIDERS,
      scope: BROWSER_FIXTURE_SCOPE,
      usedCachedData: false,
      scanning: false,
      refreshError: null,
    }),
    usage_get_overview: ({ source }) => overviewFor(source),
    usage_get_recent: ({ source }) =>
      source
        ? BROWSER_FIXTURE_RECENT.filter((call) => call.source === source)
        : BROWSER_FIXTURE_RECENT,
    usage_get_providers: () => BROWSER_FIXTURE_PROVIDERS,
    usage_get_skill_detail: ({ skill }) => detailFor(skill),
    usage_get_unused_skills: ({ thresholdDays }) =>
      unusedReportFor(thresholdDays),
    usage_get_scope_info: () => BROWSER_FIXTURE_SCOPE,
    usage_resolve_skill_id: () => null,
    usage_get_skill_counts: () => ({}),
  });
}
