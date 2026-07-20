import { registerIpcFixtures } from "@/lib/ipc";
import { BROWSER_PLATFORM_PATHS } from "@/lib/platformPathPolicy";
import type {
  AgentWithStatus,
  BootstrapSnapshot,
  CentralTopTag,
  DashboardCentralSummary,
  ScanResult,
} from "@/types";

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
    id: "codex",
    display_name: "Codex CLI",
    category: "coding",
    global_skills_dir: "",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
  {
    id: "grok",
    display_name: "Grok",
    category: "coding",
    global_skills_dir: "",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
  {
    id: "antigravity",
    display_name: "Antigravity",
    category: "coding",
    global_skills_dir: "",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
  {
    id: "antigravity-cli",
    display_name: "Antigravity CLI",
    category: "coding",
    global_skills_dir: "",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
  {
    id: "gemini-cli",
    display_name: "Gemini CLI (legacy)",
    category: "coding",
    global_skills_dir: "",
    is_detected: true,
    is_builtin: true,
    is_enabled: false,
  },
  {
    id: "opencode",
    display_name: "OpenCode",
    category: "coding",
    global_skills_dir: "",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
  {
    id: "kiro",
    display_name: "Kiro",
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
    is_enabled: false,
  },
  {
    id: "openclaw",
    display_name: "OpenClaw",
    category: "lobster",
    global_skills_dir: "",
    is_detected: true,
    is_builtin: true,
    is_enabled: false,
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

export const BROWSER_FIXTURE_COUNTS: ScanResult = {
  total_skills: 6,
  agents_scanned: 11,
  skills_by_agent: {
    "claude-code": 1,
    codex: 1,
    grok: 0,
    antigravity: 1,
    "antigravity-cli": 1,
    "gemini-cli": 0,
    opencode: 1,
    kiro: 1,
    cursor: 0,
    openclaw: 0,
    central: 5,
  },
};

const BROWSER_FIXTURE_LAST_SCAN_AT = "2026-04-23T00:00:00.000Z";

const BROWSER_FIXTURE_DASHBOARD_CENTRAL_SUMMARY: DashboardCentralSummary = {
  centralSkillCount: BROWSER_FIXTURE_COUNTS.skills_by_agent.central,
  updatesAvailable: 0,
  aiReviewCount: 0,
  uncategorizedCount: 0,
  unassignedSourceCount: 0,
  readiness: {
    score: 0,
    categorizedRatio: 0,
    describedRatio: 0,
    sourcedRatio: 0,
    installHealthRatio: 0,
  },
  sourceRepositories: [],
};

const BROWSER_FIXTURE_TOP_TAGS: CentralTopTag[] = [
  { id: "web", name: "Web", count: 3 },
  { id: "docs", name: "Docs", count: 2 },
  { id: "automation", name: "Automation", count: 1 },
];

const BROWSER_FIXTURE_BOOTSTRAP: BootstrapSnapshot = {
  agents: BROWSER_FIXTURE_AGENTS,
  cachedSkillCounts: BROWSER_FIXTURE_COUNTS.skills_by_agent,
  collectionCount: 0,
  dashboardCentralSummary: BROWSER_FIXTURE_DASHBOARD_CENTRAL_SUMMARY,
  lastScanAt: BROWSER_FIXTURE_LAST_SCAN_AT,
  scanState: "idle",
};

export function registerPlatformFixtures(): void {
  registerIpcFixtures({
    get_bootstrap_snapshot: () => BROWSER_FIXTURE_BOOTSTRAP,
    get_dashboard_central_summary: () => BROWSER_FIXTURE_DASHBOARD_CENTRAL_SUMMARY,
    get_central_top_tags: ({ limit }) =>
      BROWSER_FIXTURE_TOP_TAGS.slice(0, Math.max(1, limit)),
    list_platform_paths: () => BROWSER_PLATFORM_PATHS,
    scan_all_skills: () => BROWSER_FIXTURE_COUNTS,
    get_skill_counts_summary: () => ({
      cachedSkillCounts: BROWSER_FIXTURE_COUNTS.skills_by_agent,
      lastScanAt: BROWSER_FIXTURE_LAST_SCAN_AT,
      scanState: "idle",
    }),
    set_agent_enabled: ({ agentId, isEnabled }) => {
      const agent = BROWSER_FIXTURE_AGENTS.find((item) => item.id === agentId);
      if (!agent) {
        throw new Error(`Unknown fixture agent '${agentId}'`);
      }
      return { ...agent, is_enabled: isEnabled };
    },
  });
}
