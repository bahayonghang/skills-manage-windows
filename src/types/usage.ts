// 与 Rust 端 services/usage + commands/usage 保持同名 camelCase。

export interface SkillCall {
  skill: string;
  timestampMs: number;
  project: string;
  sessionId: string;
  source: string;
}

export interface SkillCount {
  skill: string;
  count: number;
  projects: number;
  sessions: number;
  lastUsedMs: number;
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
  topSkills: SkillCount[];
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
  recent: SkillCall[];
  providers: ProviderHealth[];
  scope: UsageScopeInfo;
  usedCachedData: boolean;
  refreshError: string | null;
}

export interface SkillUsageDetail {
  skill: string;
  count: number;
  sessions: number;
  firstUsedMs: number;
  lastUsedMs: number;
  byProject: SkillCount[];
  /** 16w x 7d，仅本 skill 的子集 */
  weekly: DayCount[];
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
