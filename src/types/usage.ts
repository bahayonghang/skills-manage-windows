// 与 Rust 端 services/usage + commands/usage 保持同名 camelCase。

export type UsageSkillMatchStatus = "matched" | "ambiguous" | "unmatched";

export interface RecentSkillCall {
  skill: string;
  timestampMs: number;
  project: string;
  sessionId: string;
  source: string;
  matchStatus: UsageSkillMatchStatus;
  resolvedSkillId: string | null;
}

/** Per-skill lifetime (or windowed) count + last used, from `usage_get_skill_usage_stats`. */
export interface SkillUsageStat {
  count: number;
  lastUsedMs: number | null;
}

export interface SkillUsageSummary {
  skill: string;
  count: number;
  projects: number;
  sessions: number;
  lastUsedMs: number;
  matchStatus: UsageSkillMatchStatus;
  resolvedSkillId: string | null;
  staticTokenEstimate: number | null;
  staticByteCount: number | null;
}

export interface DayCount {
  date: string; // YYYY-MM-DD
  count: number;
}

export interface UsageKpis {
  totalCalls: number;
  uniqueSkills: number;
  uniqueProjects: number;
  uniqueSources: number;
  uniqueSessions: number;
}

export interface UsageOverview {
  kpis: UsageKpis;
  topSkills: SkillUsageSummary[];
  /** 16w x 7d = 112 项，按日期升序连续 */
  heatmap: DayCount[];
  lastScanMs: number | null;
}

export interface ProviderHealth {
  providerId: string;
  displayName: string;
  available: boolean;
  callCount: number;
  scannedAtMs: number;
}

export interface RefreshSummary {
  cached: boolean;
  callsWritten: number;
  providersAvailable: number;
  scannedAtMs: number;
}

export interface UsageRefreshResult {
  summary: RefreshSummary;
  overview: UsageOverview;
  recent: RecentSkillCall[];
  providers: ProviderHealth[];
  scope: UsageScopeInfo;
  usedCachedData: boolean;
  /**
   * 本地 target 返回过期缓存页时为 true：后台重扫进行中，完成后经
   * `usage://scan-completed` 事件（payload = target id）通知静默重取。
   */
  scanning: boolean;
  refreshError: string | null;
}

export interface SkillUsageDetail {
  skill: string;
  count: number;
  sessions: number;
  firstUsedMs: number;
  lastUsedMs: number;
  byProject: SkillProjectCount[];
  /** 16w x 7d，仅本 skill 的子集 */
  weekly: DayCount[];
  matchStatus: UsageSkillMatchStatus;
  resolvedSkillId: string | null;
  staticTokenEstimate: number | null;
  staticByteCount: number | null;
}

export interface SkillProjectCount {
  project: string;
  count: number;
  sessions: number;
  lastUsedMs: number;
}

export interface UsageScopeInfo {
  targetId: string;
  label: string;
  isRemote: boolean;
  /**
   * 远程 target 是否可达；显式 refresh 返回权威值。
   * 普通只读 getter 为避免额外 SSH/WSL 建连，可能对远程乐观返回 true。
   */
  remoteReachable: boolean;
}

// ─── usage_get_unused_skills（与 Rust aggregate::UnusedSkill* 对齐）────────────

export type UnusedSkillOrigin = "central" | "platform";

export type UnusedSkillStatus = "never_used" | "stale";

/**
 * Central 条目的 per-agent 安装行。`linkType === "native"` 且
 * `installedPath` 指向 Central 目录即 shared-root 安装，不可独立 unlink。
 */
export interface UnusedAgentInstall {
  agentId: string;
  linkType: string;
  installedPath: string;
  hasPendingRecovery: boolean;
}

/**
 * 平台条目的 per-agent 安装行（对应一条 agent_skill_observations 记录）。
 * `rowId` 可直接传给 `uninstall_skill_from_agent` 做行级 unlink。
 */
export interface UnusedPlatformInstall {
  agentId: string;
  rowId: string | null;
  /** 扫描器持久化的 skills 行 id（散件）或 Central id */
  skillId: string;
  linkType: string;
  sourceKind: string | null;
  isReadOnly: boolean;
  installedPath: string;
  hasPendingRecovery: boolean;
}

export interface UnusedSkillEntry {
  /** Central skill id；平台散件为 null */
  skillId: string | null;
  name: string;
  matchStatus: UsageSkillMatchStatus;
  origin: UnusedSkillOrigin;
  /** Central 条目：per-agent 安装行（按 agentId 升序）；平台条目为 [] */
  agents: UnusedAgentInstall[];
  /** 平台条目：per-agent observation 安装行；Central 条目为 [] */
  installs: UnusedPlatformInstall[];
  /** 平台维度为观察到的 dir_path；Central 维度为 canonical_path */
  installedPath: string | null;
  /** 0 = 从未使用 */
  callCount: number;
  lastUsedMs: number | null;
  staticTokenEstimate: number | null;
  staticByteCount: number | null;
  status: UnusedSkillStatus;
}

export interface UnusedSkillsReport {
  central: UnusedSkillEntry[];
  platforms: UnusedSkillEntry[];
}

/** 批量 unlink 的单个目标（Central 条目 rowId 缺省；平台条目带 observation rowId）。 */
export interface UnusedUnlinkRequest {
  skillId: string;
  agentId: string;
  rowId?: string | null;
}

/** 批量 unlink 的逐项结果：error 为格式化后的失败原因（成功为 null）。 */
export interface UnusedUnlinkResult {
  skillId: string;
  agentId: string;
  rowId: string | null;
  ok: boolean;
  error: string | null;
}
