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

export interface UnusedSkillEntry {
  /** Central skill id；平台散件为 null */
  skillId: string | null;
  name: string;
  matchStatus: UsageSkillMatchStatus;
  origin: UnusedSkillOrigin;
  /** 安装/链接的平台 agent id（升序去重） */
  agents: string[];
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
