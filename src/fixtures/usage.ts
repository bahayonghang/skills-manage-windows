import { registerIpcFixtures } from "@/lib/ipc";
import type {
  ProviderHealth,
  UsageOverview,
  UsageScopeInfo,
} from "@/types/usage";

const BROWSER_FIXTURE_OVERVIEW: UsageOverview = {
  kpis: {
    totalCalls: 0,
    uniqueSkills: 0,
    uniqueProjects: 0,
    uniqueSources: 0,
    uniqueSessions: 0,
  },
  topSkills: [],
  heatmap: Array.from({ length: 16 * 7 }, (_, i) => ({
    date: new Date(Date.now() - (16 * 7 - 1 - i) * 86_400_000)
      .toISOString()
      .slice(0, 10),
    count: 0,
  })),
  lastScanMs: null,
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
].map((id) => ({
  providerId: id,
  displayName: id,
  available: false,
  callCount: 0,
  scannedAtMs: 0,
}));

const BROWSER_FIXTURE_SCOPE: UsageScopeInfo = {
  targetId: "local",
  label: "Local",
  isRemote: false,
  remoteReachable: false,
};

export function registerUsageFixtures(): void {
  registerIpcFixtures({
    usage_refresh: () => ({
      summary: {
        cached: false,
        callsWritten: 0,
        providersAvailable: 0,
        scannedAtMs: Date.now(),
      },
      overview: BROWSER_FIXTURE_OVERVIEW,
      recent: [],
      providers: BROWSER_FIXTURE_PROVIDERS,
      scope: BROWSER_FIXTURE_SCOPE,
      usedCachedData: false,
      refreshError: null,
    }),
    usage_get_overview: () => BROWSER_FIXTURE_OVERVIEW,
    usage_get_recent: () => [],
    usage_get_providers: () => BROWSER_FIXTURE_PROVIDERS,
    usage_get_skill_detail: () => null,
    usage_get_scope_info: () => BROWSER_FIXTURE_SCOPE,
    usage_resolve_skill_id: () => null,
    usage_get_skill_counts: () => ({}),
  });
}
