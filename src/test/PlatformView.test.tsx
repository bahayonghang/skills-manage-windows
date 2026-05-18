import { describe, it, expect, vi, beforeEach } from "vitest";
import { act, render, screen, fireEvent, waitFor, within } from "@testing-library/react";
import {
  MemoryRouter,
  Route,
  Routes,
  useNavigate,
} from "react-router-dom";
import { toast } from "sonner";
import { PlatformView } from "../pages/PlatformView";
import { AgentWithStatus, ScannedSkill } from "../types";

// Mock stores
vi.mock("../stores/platformStore", () => ({
  usePlatformStore: vi.fn(),
}));

vi.mock("../stores/skillStore", () => ({
  useSkillStore: vi.fn(),
}));

vi.mock("../stores/centralSkillsStore", () => ({
  useCentralSkillsStore: vi.fn(),
}));

vi.mock("../stores/skillDetailStore", () => ({
  useSkillDetailStore: vi.fn(),
}));

vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
  },
}));

vi.mock("../components/skill/SkillDetailDrawer", () => ({
  SkillDetailDrawer: ({
    open,
    skillId,
    agentId,
    rowId,
    onOpenChange,
    returnFocusRef,
    onInstallClick,
  }: {
    open: boolean;
    skillId: string | null;
    agentId?: string | null;
    rowId?: string | null;
    onOpenChange: (open: boolean) => void;
    returnFocusRef?: { current: HTMLElement | null };
    onInstallClick?: (skillId: string) => void;
  }) =>
    open ? (
      <div data-testid="skill-detail-drawer">
        <div>drawer-skill:{skillId}</div>
        <div>drawer-agent:{agentId ?? "none"}</div>
        <div>drawer-row:{rowId ?? "none"}</div>
        {skillId && onInstallClick ? (
          <button onClick={() => onInstallClick(skillId)}>
            drawer-install:{skillId}
          </button>
        ) : null}
        <button
          onClick={() => {
            onOpenChange(false);
            returnFocusRef?.current?.focus();
          }}
        >
          Close drawer
        </button>
      </div>
    ) : null,
}));

import { usePlatformStore } from "../stores/platformStore";
import { useSkillStore } from "../stores/skillStore";
import { useCentralSkillsStore } from "../stores/centralSkillsStore";
import { useSkillDetailStore } from "../stores/skillDetailStore";
import * as tauriBridge from "@/lib/tauri";

const userSourceText = /用户来源|User source/i;
const pluginSourceText = /插件来源|Plugin source/i;
const readOnlyText = /只读|Read-only/i;
const badgeQueryOptions = { selector: "span" } as const;
const claudeTabName = (label: string, count?: number) =>
  count == null
    ? new RegExp(`^${label}(?:\\s*\\(\\d+\\))?$`)
    : new RegExp(`^${label}\\s*\\(${count}\\)$`);
const getCardBadgeMatches = (matcher: RegExp) =>
  screen
    .queryAllByText(matcher, badgeQueryOptions)
    .filter((element) => element.closest(".rounded-xl"));

// ─── Fixtures ─────────────────────────────────────────────────────────────────

const mockAgent: AgentWithStatus = {
  id: "claude-code",
  display_name: "Claude Code",
  category: "coding",
  global_skills_dir: "/Users/test/.claude/skills/",
  is_detected: true,
  is_builtin: true,
  is_enabled: true,
};

const mockCursorAgent: AgentWithStatus = {
  id: "cursor",
  display_name: "Cursor",
  category: "coding",
  global_skills_dir: "/Users/test/.cursor/skills/",
  is_detected: true,
  is_builtin: true,
  is_enabled: true,
};

const mockCodexAgent: AgentWithStatus = {
  id: "codex",
  display_name: "Codex CLI",
  category: "coding",
  global_skills_dir: "/Users/test/.agents/skills/",
  is_detected: true,
  is_builtin: true,
  is_enabled: true,
};

const mockSkills: ScannedSkill[] = [
  {
    id: "frontend-design",
    name: "frontend-design",
    description: "Build distinctive, production-grade frontend interfaces",
    file_path: "~/.claude/skills/frontend-design/SKILL.md",
    dir_path: "~/.claude/skills/frontend-design",
    link_type: "symlink",
    symlink_target: "~/.skillsmanage/skills/frontend-design",
    is_central: true,
  },
  {
    id: "code-reviewer",
    name: "code-reviewer",
    description: "Review code changes and identify high-confidence actionable bugs",
    file_path: "~/.claude/skills/code-reviewer/SKILL.md",
    dir_path: "~/.claude/skills/code-reviewer",
    link_type: "copy",
    is_central: false,
  },
];

const mockCursorSkills: ScannedSkill[] = [
  {
    id: "cursor-helper",
    row_id: "cursor-helper",
    name: "cursor-helper",
    description: "Cursor-specific helper skill",
    file_path: "~/.cursor/skills/cursor-helper/SKILL.md",
    dir_path: "~/.cursor/skills/cursor-helper",
    link_type: "symlink",
    symlink_target: "~/.skillsmanage/skills/cursor-helper",
    is_central: true,
  },
];

const mockUniversalSkills: ScannedSkill[] = [
  {
    id: "universal-helper",
    name: "universal-helper",
    description: "Shared helper in .agents skills",
    file_path: "~/.agents/skills/universal-helper/SKILL.md",
    dir_path: "~/.agents/skills/universal-helper",
    link_type: "native",
    is_central: false,
  },
];

const mockDuplicateUniversalSkills: ScannedSkill[] = [
  {
    id: "shared-skill",
    row_id: "shared-skill",
    name: "shared-skill",
    description: "Universal platform copy",
    file_path: "~/.agents/skills/shared-skill/SKILL.md",
    dir_path: "~/.agents/skills/shared-skill",
    link_type: "copy",
    is_central: false,
  },
  {
    id: "shared-skill",
    row_id: "codex::~/.codex/plugins/cache/openai/example/1.0.0/skills/shared-skill",
    name: "shared-skill",
    description: "Codex plugin copy",
    file_path: "~/.codex/plugins/cache/openai/example/1.0.0/skills/shared-skill/SKILL.md",
    dir_path: "~/.codex/plugins/cache/openai/example/1.0.0/skills/shared-skill",
    link_type: "native",
    is_central: false,
    source_kind: "plugin",
    source_root: "~/.codex/plugins/cache/openai/example/1.0.0",
    is_read_only: true,
    conflict_count: 2,
  },
];

const mockDuplicateClaudeSkills: ScannedSkill[] = [
  {
    id: "shared-skill",
    row_id: "claude-code::user::shared-skill",
    name: "shared-skill",
    description: "User-source copy",
    file_path: "~/.claude/skills/shared-skill/SKILL.md",
    dir_path: "~/.claude/skills/shared-skill",
    link_type: "native",
    is_central: false,
    source_kind: "user",
    source_root: "~/.claude/skills",
    is_read_only: false,
    conflict_count: 2,
  },
  {
    id: "shared-skill",
    row_id: "claude-code::plugin::shared-skill",
    name: "shared-skill",
    description: "Plugin copy",
    file_path: "~/.claude/plugins/cache/publisher/plugin-a/1.0.0/skills/shared-skill/SKILL.md",
    dir_path: "~/.claude/plugins/cache/publisher/plugin-a/1.0.0/skills/shared-skill",
    link_type: "native",
    is_central: false,
    source_kind: "plugin",
    source_root: "~/.claude/plugins/cache/publisher/plugin-a/1.0.0",
    is_read_only: true,
    conflict_count: 2,
  },
];

const mockDuplicateClaudeSkillsWithDistinctIds: ScannedSkill[] = [
  {
    id: "shared-skill-id",
    row_id: "claude-code::user::shared-skill-id",
    name: "Shared skill",
    description: "User-source copy",
    file_path: "~/.claude/skills/shared-skill/SKILL.md",
    dir_path: "~/.claude/skills/shared-skill",
    link_type: "native",
    is_central: false,
    source_kind: "user",
    source_root: "~/.claude/skills",
    is_read_only: false,
    conflict_count: 2,
  },
  {
    id: "shared-skill-id",
    row_id: "claude-code::plugin::shared-skill-id",
    name: "Shared skill",
    description: "Plugin copy",
    file_path: "~/.claude/plugins/cache/publisher/plugin-a/1.0.0/skills/shared-skill/SKILL.md",
    dir_path: "~/.claude/plugins/cache/publisher/plugin-a/1.0.0/skills/shared-skill",
    link_type: "native",
    is_central: false,
    source_kind: "plugin",
    source_root: "~/.claude/plugins/cache/publisher/plugin-a/1.0.0",
    is_read_only: true,
    conflict_count: 2,
  },
];

const mockClaudePluginSliceDuplicates: ScannedSkill[] = [
  {
    id: "shared-skill",
    row_id: "claude-code::user::shared-skill",
    name: "shared-skill",
    description: "User-source copy",
    file_path: "~/.claude/skills/shared-skill/SKILL.md",
    dir_path: "~/.claude/skills/shared-skill",
    link_type: "native",
    is_central: false,
    source_kind: "user",
    source_root: "~/.claude/skills",
    is_read_only: false,
    conflict_count: 3,
  },
  {
    id: "shared-skill",
    row_id: "claude-code::plugin::publisher-a::shared-skill",
    name: "shared-skill",
    description: "Plugin A copy",
    file_path: "~/.claude/plugins/cache/publisher-a/plugin-a/1.0.0/skills/shared-skill/SKILL.md",
    dir_path: "~/.claude/plugins/cache/publisher-a/plugin-a/1.0.0/skills/shared-skill",
    link_type: "native",
    is_central: false,
    source_kind: "plugin",
    source_root: "~/.claude/plugins/cache/publisher-a/plugin-a/1.0.0",
    is_read_only: true,
    conflict_count: 3,
  },
  {
    id: "shared-skill",
    row_id: "claude-code::plugin::publisher-b::shared-skill",
    name: "shared-skill",
    description: "Plugin B copy",
    file_path: "~/.claude/plugins/cache/publisher-b/plugin-b/2.0.0/.claude/skills/shared-skill/SKILL.md",
    dir_path: "~/.claude/plugins/cache/publisher-b/plugin-b/2.0.0/.claude/skills/shared-skill",
    link_type: "native",
    is_central: false,
    source_kind: "plugin",
    source_root: "~/.claude/plugins/cache/publisher-b/plugin-b/2.0.0",
    is_read_only: true,
    conflict_count: 3,
  },
];

const mockSortablePlatformSkills: ScannedSkill[] = [
  {
    id: "beta-skill",
    row_id: "beta-skill",
    name: "Beta Skill",
    description: "Second skill from a repository",
    file_path: "~/.claude/skills/beta-skill/SKILL.md",
    dir_path: "~/.claude/skills/beta-skill",
    link_type: "symlink",
    symlink_target: "~/.skillsmanage/skills/beta-skill",
    is_central: true,
    scanned_at: "2026-01-01T00:00:00Z",
    installed_at: "2026-01-02T00:00:00Z",
    updated_at: "2026-01-04T00:00:00Z",
    repository: {
      id: "github-owner-beta-main",
      name: "owner/beta",
      source_type: "github",
      owner: "owner",
      repo: "beta",
      branch: "main",
      url: "https://github.com/owner/beta",
      pinned: false,
      is_unknown: false,
      created_at: "2026-01-01T00:00:00Z",
      updated_at: "2026-01-01T00:00:00Z",
    },
  },
  {
    id: "alpha-skill",
    row_id: "claude-code::plugin::publisher-a::alpha-skill",
    name: "Alpha Skill",
    description: "First plugin skill",
    file_path: "~/.claude/plugins/cache/publisher-a/plugin-a/1.0.0/skills/alpha-skill/SKILL.md",
    dir_path: "~/.claude/plugins/cache/publisher-a/plugin-a/1.0.0/skills/alpha-skill",
    link_type: "native",
    is_central: false,
    scanned_at: "2026-01-03T00:00:00Z",
    updated_at: "2026-01-03T00:00:00Z",
    source_kind: "plugin",
    source_root: "~/.claude/plugins/cache/publisher-a/plugin-a/1.0.0",
    is_read_only: true,
  },
  {
    id: "gamma-skill",
    row_id: "gamma-skill",
    name: "Gamma Skill",
    description: "Local user skill",
    file_path: "~/.claude/skills/gamma-skill/SKILL.md",
    dir_path: "~/.claude/skills/gamma-skill",
    link_type: "copy",
    is_central: false,
    scanned_at: "2026-01-05T00:00:00Z",
    installed_at: "2026-01-05T00:00:00Z",
    updated_at: "2026-01-05T00:00:00Z",
    source_kind: "user",
    source_root: "~/.claude/skills",
  },
];

const mockGetSkillsByAgent = vi.fn();
const mockLoadCentralSkills = vi.fn();
const mockInstallSkill = vi.fn();
const mockUninstallSkillFromAgent = vi.fn();
const mockRefreshCounts = vi.fn();
const mockRefreshInstallations = vi.fn();
const mockRescan = vi.fn();
const mockUsePlatformStore = vi.mocked(usePlatformStore);
const mockUseSkillStore = vi.mocked(useSkillStore);
const mockUseCentralSkillsStore = vi.mocked(useCentralSkillsStore);
const mockUseSkillDetailStore = vi.mocked(useSkillDetailStore);
const centralStoreHarness = mockUseCentralSkillsStore as typeof mockUseCentralSkillsStore & {
  setState: (nextState: Record<string, unknown>) => void;
};

function buildPlatformStoreState(overrides = {}) {
  return {
    agents: [mockAgent],
    skillsByAgent: { "claude-code": 2 },
    collectionCount: 0,
    lastScanAt: "2026-04-23T01:00:00Z",
    scanState: "idle",
    isLoading: false,
    isRefreshing: false,
    scanGeneration: 1,
    error: null,
    initialize: vi.fn(),
    hydrateShell: vi.fn(),
    refreshScanInBackground: vi.fn(),
    rescan: mockRescan,
    refreshCounts: mockRefreshCounts,
    resetForTargetChange: vi.fn(),
    applyScanSummary: vi.fn(),
    setCollectionCount: vi.fn(),
    addCustomAgent: vi.fn(),
    updateCustomAgent: vi.fn(),
    removeCustomAgent: vi.fn(),
    ...overrides,
  };
}

function buildSkillStoreState(overrides = {}) {
  return {
    skillsByAgent: { "claude-code": mockSkills },
    loadingByAgent: { "claude-code": false },
    pendingSkillActionKeys: {},
    error: null,
    getSkillsByAgent: mockGetSkillsByAgent,
    uninstallSkillFromAgent: mockUninstallSkillFromAgent,
    ...overrides,
  };
}

function buildCentralSkillsStoreState(overrides = {}) {
  return {
    skills: [],
    agents: [mockAgent],
    loadCentralSkills: mockLoadCentralSkills,
    installSkill: mockInstallSkill,
    ...overrides,
  };
}

function buildSkillDetailStoreState(overrides = {}) {
  return {
    detail: null,
    refreshInstallations: mockRefreshInstallations,
    ...overrides,
  };
}

function installDefaultStoreMocks() {
  let currentCentralState = buildCentralSkillsStoreState();

  mockUsePlatformStore.mockImplementation((selector?: unknown) => {
    const state = buildPlatformStoreState();
    if (typeof selector === "function") return selector(state);
    return state;
  });
  mockUseSkillStore.mockImplementation((selector?: unknown) => {
    const state = buildSkillStoreState();
    if (typeof selector === "function") return selector(state);
    return state;
  });
  mockUseCentralSkillsStore.mockImplementation((selector?: unknown) => {
    if (typeof selector === "function") return selector(currentCentralState);
    return currentCentralState;
  });
  mockUseSkillDetailStore.mockImplementation((selector?: unknown) => {
    const state = buildSkillDetailStoreState();
    if (typeof selector === "function") return selector(state);
    return state;
  });

  Object.assign(mockUseCentralSkillsStore, {
    getState: () => currentCentralState,
    setState: (nextState: Record<string, unknown>) => {
      currentCentralState = { ...currentCentralState, ...nextState };
    },
  });
}

function renderPlatformView(agentId = "claude-code") {
  return render(
    <MemoryRouter initialEntries={[`/platform/${agentId}`]}>
      <Routes>
        <Route path="/platform/:agentId" element={<PlatformView />} />
      </Routes>
    </MemoryRouter>
  );
}

function visibleSkillNames() {
  return screen
    .getAllByRole("button", { name: /查看 .* 的详情/i })
    .map((button) => button.textContent?.trim());
}

let testNavigate: ReturnType<typeof useNavigate> | null = null;

function NavigationHarness() {
  testNavigate = useNavigate();
  return null;
}

// ─── Tests ────────────────────────────────────────────────────────────────────

describe("PlatformView", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    testNavigate = null;
    mockRefreshCounts.mockReset();
    mockRefreshInstallations.mockReset();
    mockUninstallSkillFromAgent.mockReset();
    mockRescan.mockReset();
    vi.mocked(toast.success).mockReset();
    vi.mocked(toast.error).mockReset();
    vi.mocked(toast.info).mockReset();
    installDefaultStoreMocks();
  });

  // ── Header ────────────────────────────────────────────────────────────────

  it("shows platform name in header", () => {
    renderPlatformView();
    expect(screen.getByText("Claude Code")).toBeInTheDocument();
  });

  it("shows platform directory path in header", () => {
    renderPlatformView();
    expect(screen.getByText("/Users/test/.claude/skills/")).toBeInTheDocument();
  });

  // ── Skill List ────────────────────────────────────────────────────────────

  it("renders skill cards for all skills", () => {
    renderPlatformView();
    expect(screen.getByText("frontend-design")).toBeInTheDocument();
    expect(screen.getByText("code-reviewer")).toBeInTheDocument();
  });

  it("shows source indicator on skill cards", () => {
    renderPlatformView();
    expect(
      screen.getAllByText((_, element) => element?.textContent?.replace(/\s+/g, " ").trim() === "中央技能库 - 符号链接")
        .length
    ).toBeGreaterThan(0);
    expect(
      screen.getAllByText((_, element) => element?.textContent?.replace(/\s+/g, " ").trim() === "独立安装 - 复制安装")
        .length
    ).toBeGreaterThan(0);
  });

  it("renders browser fixture installed card on the localhost validation surface without Tauri", async () => {
    const isTauriSpy = vi.spyOn(tauriBridge, "isTauriRuntime").mockReturnValue(false);

    mockUseSkillStore.mockImplementation((selector?: unknown) => {
      const state = buildSkillStoreState({
        skillsByAgent: {
          "claude-code": [
            {
              id: "fixture-central-skill",
              name: "fixture-central-skill",
              description: "Browser fixture skill sourced from the central library",
              file_path: "~/.claude/skills/fixture-central-skill/SKILL.md",
              dir_path: "~/.claude/skills/fixture-central-skill",
              link_type: "symlink",
              symlink_target: "~/.skillsmanage/skills/fixture-central-skill",
              is_central: true,
            },
          ],
        },
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });

    render(
      <MemoryRouter initialEntries={["/platform/claude-code"]}>
        <Routes>
          <Route path="/platform/:agentId" element={<PlatformView />} />
        </Routes>
      </MemoryRouter>
    );

    expect(await screen.findByRole("button", { name: /查看 fixture-central-skill 的详情/i })).toBeInTheDocument();
    expect(
      screen.getAllByText((_, element) => element?.textContent?.replace(/\s+/g, " ").trim() === "中央技能库 - 符号链接")
        .length
    ).toBeGreaterThan(0);

    isTauriSpy.mockRestore();
  });

  // ── Empty State ───────────────────────────────────────────────────────────

  it("shows empty state when platform has no skills", () => {
    mockUsePlatformStore.mockImplementation((selector?: unknown) => {
      const state = buildPlatformStoreState({
        skillsByAgent: { "claude-code": 0 },
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });
    mockUseSkillStore.mockImplementation((selector?: unknown) => {
      const state = buildSkillStoreState({
        skillsByAgent: { "claude-code": [] },
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });

    render(
      <MemoryRouter initialEntries={["/platform/claude-code"]}>
        <Routes>
          <Route path="/platform/:agentId" element={<PlatformView />} />
        </Routes>
      </MemoryRouter>
    );

    expect(
      screen.getByText(/该平台暂无技能/)
    ).toBeInTheDocument();
  });

  // ── Platform Not Found ────────────────────────────────────────────────────

  it("shows not found when agent doesn't exist", () => {
    mockUsePlatformStore.mockImplementation((selector?: unknown) => {
      const state = buildPlatformStoreState({ agents: [] });
      if (typeof selector === "function") return selector(state);
      return state;
    });
    mockUseSkillStore.mockImplementation((selector?: unknown) => {
      const state = buildSkillStoreState({ skillsByAgent: {} });
      if (typeof selector === "function") return selector(state);
      return state;
    });

    render(
      <MemoryRouter initialEntries={["/platform/unknown"]}>
        <Routes>
          <Route path="/platform/:agentId" element={<PlatformView />} />
        </Routes>
      </MemoryRouter>
    );

    expect(screen.getByText("未找到平台")).toBeInTheDocument();
  });

  it("renders Universal as a real platform page backed by the representative agent", () => {
    mockUsePlatformStore.mockImplementation((selector?: unknown) => {
      const state = buildPlatformStoreState({
        agents: [mockAgent, mockCodexAgent, mockCursorAgent],
        skillsByAgent: {
          "claude-code": mockSkills.length,
          codex: mockUniversalSkills.length,
          cursor: mockCursorSkills.length,
        },
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });
    mockUseSkillStore.mockImplementation((selector?: unknown) => {
      const state = buildSkillStoreState({
        skillsByAgent: {
          "claude-code": mockSkills,
          codex: mockUniversalSkills,
          cursor: mockCursorSkills,
        },
        loadingByAgent: {
          "claude-code": false,
          codex: false,
          cursor: false,
        },
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });

    renderPlatformView("universal-agents");

    expect(screen.getByText("Universal")).toBeInTheDocument();
    expect(screen.getByText("通用 .agents/skills")).toBeInTheDocument();
    expect(screen.getByText("/Users/test/.agents/skills/")).toBeInTheDocument();
    expect(screen.getByText("universal-helper")).toBeInTheDocument();
    expect(mockGetSkillsByAgent).toHaveBeenCalledWith("codex");
  });

  it("keeps Claude source tabs hidden on the Universal page", () => {
    mockUsePlatformStore.mockImplementation((selector?: unknown) => {
      const state = buildPlatformStoreState({
        agents: [mockAgent, mockCodexAgent, mockCursorAgent],
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });
    mockUseSkillStore.mockImplementation((selector?: unknown) => {
      const state = buildSkillStoreState({
        skillsByAgent: { codex: mockUniversalSkills },
        loadingByAgent: { codex: false },
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });

    renderPlatformView("universal-agents");

    expect(screen.queryByRole("tab", { name: claudeTabName("全部") })).not.toBeInTheDocument();
    expect(screen.queryByRole("tab", { name: claudeTabName("用户来源") })).not.toBeInTheDocument();
    expect(screen.queryByRole("tab", { name: claudeTabName("插件来源") })).not.toBeInTheDocument();
  });

  it("opens Universal skill detail through the representative agent", async () => {
    mockUsePlatformStore.mockImplementation((selector?: unknown) => {
      const state = buildPlatformStoreState({
        agents: [mockAgent, mockCodexAgent, mockCursorAgent],
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });
    mockUseSkillStore.mockImplementation((selector?: unknown) => {
      const state = buildSkillStoreState({
        skillsByAgent: { codex: mockUniversalSkills },
        loadingByAgent: { codex: false },
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });

    renderPlatformView("universal-agents");
    fireEvent.click(screen.getByRole("button", { name: /查看 universal-helper 的详情/i }));

    await waitFor(() => {
      expect(screen.getByTestId("skill-detail-drawer")).toBeInTheDocument();
    });

    expect(screen.getByText("drawer-skill:universal-helper")).toBeInTheDocument();
    expect(screen.getByText("drawer-agent:codex")).toBeInTheDocument();
  });

  // ── Search / Filter ───────────────────────────────────────────────────────

  it("renders search input", () => {
    renderPlatformView();
    expect(
      screen.getByPlaceholderText(/搜索技能/)
    ).toBeInTheDocument();
  });

  it("filters skills by name when searching", async () => {
    renderPlatformView();
    const searchInput = screen.getByPlaceholderText(/搜索技能/);
    fireEvent.change(searchInput, { target: { value: "frontend" } });

    await waitFor(() => {
      expect(screen.getByText("frontend-design")).toBeInTheDocument();
      expect(screen.queryByText("code-reviewer")).not.toBeInTheDocument();
    });
  });

  it("filters skills by description when searching", async () => {
    renderPlatformView();
    const searchInput = screen.getByPlaceholderText(/搜索技能/);
    fireEvent.change(searchInput, { target: { value: "actionable" } });

    await waitFor(() => {
      expect(screen.getByText("code-reviewer")).toBeInTheDocument();
      expect(screen.queryByText("frontend-design")).not.toBeInTheDocument();
    });
  });

  it("shows all skills when search is cleared", async () => {
    renderPlatformView();
    const searchInput = screen.getByPlaceholderText(/搜索技能/);
    fireEvent.change(searchInput, { target: { value: "frontend" } });
    fireEvent.change(searchInput, { target: { value: "" } });

    await waitFor(() => {
      expect(screen.getByText("frontend-design")).toBeInTheDocument();
      expect(screen.getByText("code-reviewer")).toBeInTheDocument();
    });
  });

  it("shows empty state message when search has no results", async () => {
    renderPlatformView();
    const searchInput = screen.getByPlaceholderText(/搜索技能/);
    fireEvent.change(searchInput, { target: { value: "nonexistent-skill-xyz" } });

    await waitFor(() => {
      expect(screen.getByText(/没有匹配的技能/)).toBeInTheDocument();
    });
  });

  it("sorts platform skills by selected name direction", async () => {
    mockUseSkillStore.mockImplementation((selector?: unknown) => {
      const state = buildSkillStoreState({
        skillsByAgent: { "claude-code": mockSortablePlatformSkills },
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });

    renderPlatformView();

    expect(visibleSkillNames()).toEqual(["Alpha Skill", "Beta Skill", "Gamma Skill"]);

    fireEvent.click(screen.getByTestId("platform-toolbar-sort"));
    fireEvent.click(screen.getByTestId("platform-toolbar-sort-name-desc"));

    await waitFor(() => {
      expect(visibleSkillNames()).toEqual(["Gamma Skill", "Beta Skill", "Alpha Skill"]);
    });
  });

  it("combines Claude source tabs, search, and timestamp sorting", async () => {
    mockUseSkillStore.mockImplementation((selector?: unknown) => {
      const state = buildSkillStoreState({
        skillsByAgent: { "claude-code": mockSortablePlatformSkills },
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });

    renderPlatformView();

    fireEvent.click(screen.getByRole("tab", { name: claudeTabName("用户来源", 1) }));
    fireEvent.change(screen.getByPlaceholderText(/搜索技能/), {
      target: { value: "claude/skills" },
    });
    fireEvent.click(screen.getByTestId("platform-toolbar-sort"));
    fireEvent.click(screen.getByTestId("platform-toolbar-sort-installedAt-desc"));

    await waitFor(() => {
      expect(visibleSkillNames()).toEqual(["Gamma Skill"]);
    });
    expect(getCardBadgeMatches(userSourceText)).toHaveLength(1);
    expect(getCardBadgeMatches(pluginSourceText)).toHaveLength(0);
  });

  it("groups by repository/source while keeping read-only drawer row identity", async () => {
    mockUseSkillStore.mockImplementation((selector?: unknown) => {
      const state = buildSkillStoreState({
        skillsByAgent: { "claude-code": mockSortablePlatformSkills },
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });

    renderPlatformView();

    fireEvent.click(screen.getByTestId("platform-toolbar-view"));
    fireEvent.click(screen.getByTestId("platform-toolbar-view-group-repository"));

    await waitFor(() => {
      expect(screen.getAllByText("owner/beta").length).toBeGreaterThan(0);
      expect(screen.getByText("publisher-a/plugin-a")).toBeInTheDocument();
      expect(screen.getByText("本地/用户来源")).toBeInTheDocument();
    });
    expect(screen.getAllByText("owner/beta")[0].nextSibling?.textContent).toBe("1");

    fireEvent.click(screen.getByRole("button", { name: /查看 Alpha Skill 的详情/i }));

    await waitFor(() => {
      expect(screen.getByTestId("skill-detail-drawer")).toBeInTheDocument();
    });
    expect(screen.getByText("drawer-row:claude-code::plugin::publisher-a::alpha-skill")).toBeInTheDocument();
    expect(
      screen.queryByRole("button", {
        name: /从 Claude Code 卸载 Alpha Skill/i,
      })
    ).not.toBeInTheDocument();
  });

  // ── Data Loading ──────────────────────────────────────────────────────────

  it("calls getSkillsByAgent on mount", () => {
    renderPlatformView();
    expect(mockGetSkillsByAgent).toHaveBeenCalledWith("claude-code");
  });

  it("does not preload central skills before the install dialog is opened", () => {
    renderPlatformView();
    expect(mockLoadCentralSkills).not.toHaveBeenCalled();
  });

  it("loads central skills on demand when install is requested", async () => {
    renderPlatformView();

    fireEvent.click(screen.getAllByRole("button", { name: /将 .* 安装到平台/i })[0]);

    await waitFor(() => {
      expect(mockLoadCentralSkills).toHaveBeenCalledTimes(1);
    });
  });

  it("opens the install dialog from the detail drawer header install button", async () => {
    const centralSkill = {
      id: "frontend-design",
      name: "frontend-design",
      description: "Build distinctive, production-grade frontend interfaces",
      file_path: "~/.skillsmanage/skills/frontend-design/SKILL.md",
      is_central: true,
      scanned_at: "2026-04-09T00:00:00Z",
      linked_agents: ["claude-code"],
      shared_root_agents: [],
    };
    mockInstallSkill.mockResolvedValueOnce({ succeeded: ["claude-code"], skipped: [], failed: [] });
    centralStoreHarness.setState({
      skills: [centralSkill],
    });

    renderPlatformView();

    fireEvent.click(screen.getByRole("button", { name: /查看 frontend-design 的详情/i }));

    expect(await screen.findByTestId("skill-detail-drawer")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "drawer-install:frontend-design" }));

    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText("安装 frontend-design")).toBeInTheDocument();

    fireEvent.click(within(dialog).getByRole("button", { name: /安装到 1 个平台/i }));

    await waitFor(() => {
      expect(mockInstallSkill).toHaveBeenCalledWith(
        "frontend-design",
        ["claude-code"],
        "symlink",
        null
      );
    });
  });

  it("refreshes open platform detail installation state after confirming install", async () => {
    mockInstallSkill.mockResolvedValueOnce({ succeeded: ["claude-code"], skipped: [], failed: [] });
    centralStoreHarness.setState({
      skills: [
        {
          id: "frontend-design",
          name: "frontend-design",
          description: "Build distinctive, production-grade frontend interfaces",
          file_path: "~/.skillsmanage/skills/frontend-design/SKILL.md",
          is_central: true,
          scanned_at: "2026-04-09T00:00:00Z",
          linked_agents: ["claude-code"],
          shared_root_agents: [],
        },
      ],
    });
    mockUseSkillDetailStore.mockImplementation((selector?: unknown) => {
      const state = buildSkillDetailStoreState({
        detail: {
          id: "frontend-design",
          name: "frontend-design",
        },
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });

    renderPlatformView();

    fireEvent.click(screen.getByRole("button", { name: /查看 frontend-design 的详情/i }));
    expect(await screen.findByTestId("skill-detail-drawer")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "drawer-install:frontend-design" }));

    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: /安装到 1 个平台/i }));

    await waitFor(() => {
      expect(mockRefreshInstallations).toHaveBeenCalledWith("frontend-design");
    });
  });

  it("opens the skill detail drawer without navigating away", async () => {
    renderPlatformView();

    fireEvent.click(screen.getByRole("button", { name: /查看 frontend-design 的详情/i }));

    await waitFor(() => {
      expect(screen.getByTestId("skill-detail-drawer")).toBeInTheDocument();
    });
    expect(screen.getByText("drawer-skill:frontend-design")).toBeInTheDocument();
  });

  it("passes Claude row identity into the drawer when duplicate platform rows share a skill id", async () => {
    mockUseSkillStore.mockImplementation((selector?: unknown) => {
      const state = buildSkillStoreState({
        skillsByAgent: { "claude-code": mockDuplicateClaudeSkills },
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });

    renderPlatformView();

    const detailButtons = screen.getAllByRole("button", { name: /查看 shared-skill 的详情/i });
    expect(detailButtons).toHaveLength(2);

    fireEvent.click(detailButtons[1]);

    await waitFor(() => {
      expect(screen.getByTestId("skill-detail-drawer")).toBeInTheDocument();
    });

    expect(screen.getByText("drawer-skill:shared-skill")).toBeInTheDocument();
    expect(screen.getByText("drawer-agent:claude-code")).toBeInTheDocument();
    expect(
      screen.getByText("drawer-row:claude-code::plugin::shared-skill")
    ).toBeInTheDocument();
  });

  it("shows duplicate Claude rows with explicit source markers and read-only list treatment", () => {
    mockUseSkillStore.mockImplementation((selector?: unknown) => {
      const state = buildSkillStoreState({
        skillsByAgent: { "claude-code": mockDuplicateClaudeSkills },
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });

    renderPlatformView();

    expect(screen.getAllByRole("button", { name: /查看 shared-skill 的详情/i })).toHaveLength(2);

    const [userBadge] = getCardBadgeMatches(userSourceText);
    const [pluginBadge] = getCardBadgeMatches(pluginSourceText);
    const [readOnlyBadge] = getCardBadgeMatches(readOnlyText);

    expect(userBadge).toBeDefined();
    expect(pluginBadge).toBeDefined();
    expect(readOnlyBadge).toBeDefined();
    const userCard = userBadge.closest(".rounded-xl");
    const pluginCard = pluginBadge.closest(".rounded-xl");

    expect(userCard).not.toBeNull();
    expect(pluginCard).not.toBeNull();
    expect(readOnlyBadge.closest(".rounded-xl")).toBe(pluginCard);

    if (!userCard || !pluginCard) {
      return;
    }

    expect(
      within(userCard as HTMLElement).getByRole("button", {
        name: /将 shared-skill 安装到平台/i,
      })
    ).toBeInTheDocument();
    expect(
      within(userCard as HTMLElement).getByRole("button", {
        name: /从 Claude Code 卸载 shared-skill/i,
      })
    ).toBeInTheDocument();
    expect(
      within(pluginCard as HTMLElement).queryByRole("button", {
        name: /将 shared-skill 安装到平台/i,
      })
    ).not.toBeInTheDocument();
    expect(
      within(pluginCard as HTMLElement).queryByRole("button", {
        name: /从 Claude Code 卸载 shared-skill/i,
      })
    ).not.toBeInTheDocument();
  });

  it("renders uninstall actions for writable platform skills", () => {
    renderPlatformView();

    expect(
      screen.getByRole("button", { name: /从 Claude Code 卸载 frontend-design/i })
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /从 Claude Code 卸载 code-reviewer/i })
    ).toBeInTheDocument();
  });

  it("uninstalls a skill from the current platform and refreshes counts", async () => {
    renderPlatformView();

    fireEvent.click(
      screen.getByRole("button", { name: /从 Claude Code 卸载 frontend-design/i })
    );
    expect(mockUninstallSkillFromAgent).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: /确认删除/i }));

    await waitFor(() => {
      expect(mockUninstallSkillFromAgent).toHaveBeenCalledWith(
        "frontend-design",
        "claude-code"
      );
    });
    expect(mockRefreshCounts).toHaveBeenCalledTimes(1);
  });

  it("passes the selected Claude user row id when uninstalling duplicate rows", async () => {
    mockUseSkillStore.mockImplementation((selector?: unknown) => {
      const state = buildSkillStoreState({
        skillsByAgent: { "claude-code": mockDuplicateClaudeSkills },
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });

    renderPlatformView();

    const userBadge = getCardBadgeMatches(userSourceText)[0];
    const userCard = userBadge.closest(".rounded-xl");
    expect(userCard).not.toBeNull();

    fireEvent.click(
      within(userCard as HTMLElement).getByRole("button", {
        name: /从 Claude Code 卸载 shared-skill/i,
      })
    );
    fireEvent.click(screen.getByRole("button", { name: /确认删除/i }));

    await waitFor(() => {
      expect(mockUninstallSkillFromAgent).toHaveBeenCalledWith(
        "shared-skill",
        "claude-code",
        "claude-code::user::shared-skill"
      );
    });
  });

  it("scans for duplicates by rescanning and reloading the current platform list", async () => {
    mockUseSkillStore.mockImplementation((selector?: unknown) => {
      const state = buildSkillStoreState({
        skillsByAgent: { "claude-code": mockDuplicateClaudeSkills },
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });

    renderPlatformView();
    mockGetSkillsByAgent.mockClear();

    fireEvent.click(screen.getByRole("button", { name: /扫描 Claude Code 的重复技能/ }));

    await waitFor(() => {
      expect(mockRescan).toHaveBeenCalledTimes(1);
      expect(mockGetSkillsByAgent).toHaveBeenCalledWith("claude-code");
    });
    expect(await screen.findByRole("dialog")).toBeInTheDocument();
    expect(screen.getByText(/清理重复技能/)).toBeInTheDocument();
  });

  it("duplicate cleanup for Claude deletes only the writable user row", async () => {
    mockUseSkillStore.mockImplementation((selector?: unknown) => {
      const state = buildSkillStoreState({
        skillsByAgent: { "claude-code": mockDuplicateClaudeSkills },
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });

    renderPlatformView();

    fireEvent.click(screen.getByRole("button", { name: /扫描 Claude Code 的重复技能/ }));
    const dialog = await screen.findByRole("dialog");

    expect(within(dialog).getByText("插件只读副本（不会删除）")).toBeInTheDocument();
    fireEvent.click(within(dialog).getByRole("button", { name: /删除 1 个平台副本/ }));

    await waitFor(() => {
      expect(mockUninstallSkillFromAgent).toHaveBeenCalledTimes(1);
    });
    expect(mockUninstallSkillFromAgent).toHaveBeenCalledWith(
      "shared-skill",
      "claude-code",
      "claude-code::user::shared-skill"
    );
    expect(mockRefreshCounts).toHaveBeenCalled();
  });

  it("duplicate cleanup for Universal uses the Codex representative without row id", async () => {
    mockUsePlatformStore.mockImplementation((selector?: unknown) => {
      const state = buildPlatformStoreState({
        agents: [mockAgent, mockCodexAgent, mockCursorAgent],
        skillsByAgent: {
          "claude-code": mockSkills.length,
          codex: mockDuplicateUniversalSkills.length,
          cursor: mockCursorSkills.length,
        },
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });
    mockUseSkillStore.mockImplementation((selector?: unknown) => {
      const state = buildSkillStoreState({
        skillsByAgent: { codex: mockDuplicateUniversalSkills },
        loadingByAgent: { codex: false },
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });

    renderPlatformView("universal-agents");

    fireEvent.click(screen.getByRole("button", { name: /扫描 Universal 的重复技能/ }));
    const dialog = await screen.findByRole("dialog");
    fireEvent.click(within(dialog).getByRole("button", { name: /删除 1 个平台副本/ }));

    await waitFor(() => {
      expect(mockUninstallSkillFromAgent).toHaveBeenCalledTimes(1);
    });
    expect(mockUninstallSkillFromAgent.mock.calls[0]).toEqual(["shared-skill", "codex"]);
  });

  it("shows a toast and does not open the dialog when no cleanable duplicates exist", async () => {
    renderPlatformView();

    fireEvent.click(screen.getByRole("button", { name: /扫描 Claude Code 的重复技能/ }));

    await waitFor(() => {
      expect(toast.info).toHaveBeenCalledWith("未发现可清理的重复技能");
    });
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("does not pass ordinary row ids when uninstalling non-Claude platform skills", async () => {
    mockUsePlatformStore.mockImplementation((selector?: unknown) => {
      const state = buildPlatformStoreState({
        agents: [mockAgent, mockCursorAgent],
        skillsByAgent: {
          "claude-code": mockSkills.length,
          cursor: mockCursorSkills.length,
        },
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });
    mockUseSkillStore.mockImplementation((selector?: unknown) => {
      const state = buildSkillStoreState({
        skillsByAgent: {
          "claude-code": mockSkills,
          cursor: mockCursorSkills,
        },
        loadingByAgent: {
          "claude-code": false,
          cursor: false,
        },
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });

    renderPlatformView("cursor");

    fireEvent.click(
      screen.getByRole("button", { name: /从 Cursor 卸载 cursor-helper/i })
    );
    fireEvent.click(screen.getByRole("button", { name: /确认删除/i }));

    await waitFor(() => {
      expect(mockUninstallSkillFromAgent).toHaveBeenCalledWith(
        "cursor-helper",
        "cursor"
      );
    });
  });

  it("uses row-scoped pending state for duplicate Claude rows", () => {
    mockUseSkillStore.mockImplementation((selector?: unknown) => {
      const state = buildSkillStoreState({
        skillsByAgent: { "claude-code": mockDuplicateClaudeSkills },
        pendingSkillActionKeys: {
          "claude-code::user::shared-skill": true,
        },
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });

    renderPlatformView();

    const userBadge = getCardBadgeMatches(userSourceText)[0];
    const pluginBadge = getCardBadgeMatches(pluginSourceText)[0];
    const userCard = userBadge.closest(".rounded-xl");
    const pluginCard = pluginBadge.closest(".rounded-xl");
    expect(userCard).not.toBeNull();
    expect(pluginCard).not.toBeNull();

    expect(
      within(userCard as HTMLElement).getByRole("button", { name: /确认删除/i })
    ).toBeDisabled();
    expect(
      within(pluginCard as HTMLElement).queryByRole("button", {
        name: /从 Claude Code 卸载 shared-skill/i,
      })
    ).not.toBeInTheDocument();
  });

  it("cancels the armed uninstall state when clicking outside the card actions", async () => {
    renderPlatformView();

    fireEvent.click(
      screen.getByRole("button", { name: /从 Claude Code 卸载 frontend-design/i })
    );
    expect(screen.getByRole("button", { name: /确认删除/i })).toBeInTheDocument();

    fireEvent.pointerDown(document.body);

    await waitFor(() => {
      expect(screen.queryByRole("button", { name: /确认删除/i })).not.toBeInTheDocument();
    });
    expect(mockUninstallSkillFromAgent).not.toHaveBeenCalled();
  });

  it("shows Claude-only source tabs with 全部 selected by default", () => {
    mockUseSkillStore.mockImplementation((selector?: unknown) => {
      const state = buildSkillStoreState({
        skillsByAgent: { "claude-code": mockClaudePluginSliceDuplicates },
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });

    renderPlatformView();

    expect(screen.getByRole("tab", { name: claudeTabName("全部", 3) })).toHaveAttribute("aria-selected", "true");
    expect(screen.getByRole("tab", { name: claudeTabName("用户来源", 1) })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: claudeTabName("插件来源", 2) })).toBeInTheDocument();
    expect(screen.getAllByRole("button", { name: /查看 shared-skill 的详情/i })).toHaveLength(3);
  });

  it("filters Claude rows by the active source tab and keeps duplicate rows visible inside the selected slice", async () => {
    mockUseSkillStore.mockImplementation((selector?: unknown) => {
      const state = buildSkillStoreState({
        skillsByAgent: { "claude-code": mockClaudePluginSliceDuplicates },
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });

    renderPlatformView();

    fireEvent.click(screen.getByRole("tab", { name: claudeTabName("插件来源", 2) }));

    await waitFor(() => {
      expect(screen.getAllByRole("button", { name: /查看 shared-skill 的详情/i })).toHaveLength(2);
    });

    expect(getCardBadgeMatches(userSourceText)).toHaveLength(0);
    expect(getCardBadgeMatches(pluginSourceText)).toHaveLength(2);
    expect(getCardBadgeMatches(readOnlyText)).toHaveLength(2);

    fireEvent.click(screen.getByRole("tab", { name: claudeTabName("用户来源", 1) }));

    await waitFor(() => {
      expect(screen.getAllByRole("button", { name: /查看 shared-skill 的详情/i })).toHaveLength(1);
    });

    expect(getCardBadgeMatches(userSourceText)).toHaveLength(1);
    expect(getCardBadgeMatches(pluginSourceText)).toHaveLength(0);
    expect(getCardBadgeMatches(readOnlyText)).toHaveLength(0);
  });

  it("searches only inside the active Claude source tab", async () => {
    mockUseSkillStore.mockImplementation((selector?: unknown) => {
      const state = buildSkillStoreState({
        skillsByAgent: { "claude-code": mockClaudePluginSliceDuplicates },
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });

    renderPlatformView();

    fireEvent.click(screen.getByRole("tab", { name: claudeTabName("用户来源", 1) }));
    fireEvent.change(screen.getByPlaceholderText(/搜索技能/), {
      target: { value: "shared-skill" },
    });

    await waitFor(() => {
      expect(screen.getAllByRole("button", { name: /查看 shared-skill 的详情/i })).toHaveLength(1);
    });

    expect(getCardBadgeMatches(userSourceText)).toHaveLength(1);
    expect(getCardBadgeMatches(pluginSourceText)).toHaveLength(0);
    expect(getCardBadgeMatches(readOnlyText)).toHaveLength(0);
  });

  it("searching by duplicated Claude skill id keeps both source rows and badges visible", async () => {
    mockUseSkillStore.mockImplementation((selector?: unknown) => {
      const state = buildSkillStoreState({
        skillsByAgent: { "claude-code": mockDuplicateClaudeSkillsWithDistinctIds },
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });

    renderPlatformView();

    fireEvent.change(screen.getByPlaceholderText(/搜索技能/), {
      target: { value: "shared-skill-id" },
    });

    await waitFor(() => {
      expect(
        screen.getAllByRole("button", { name: /查看 Shared skill 的详情/i })
      ).toHaveLength(2);
    });

    expect(getCardBadgeMatches(userSourceText)).toHaveLength(1);
    expect(getCardBadgeMatches(pluginSourceText)).toHaveLength(1);
    expect(getCardBadgeMatches(readOnlyText)).toHaveLength(1);
  });

  it("does not render Claude source tabs on non-Claude platform pages", () => {
    mockUsePlatformStore.mockImplementation((selector?: unknown) => {
      const state = buildPlatformStoreState({
        agents: [mockAgent, mockCursorAgent],
        skillsByAgent: {
          "claude-code": mockSkills.length,
          cursor: mockCursorSkills.length,
        },
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });
    mockUseSkillStore.mockImplementation((selector?: unknown) => {
      const state = buildSkillStoreState({
        skillsByAgent: {
          "claude-code": mockSkills,
          cursor: mockCursorSkills,
        },
        loadingByAgent: {
          "claude-code": false,
          cursor: false,
        },
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });

    renderPlatformView("cursor");

    expect(screen.queryByRole("tab", { name: claudeTabName("全部") })).not.toBeInTheDocument();
    expect(screen.queryByRole("tab", { name: claudeTabName("用户来源") })).not.toBeInTheDocument();
    expect(screen.queryByRole("tab", { name: claudeTabName("插件来源") })).not.toBeInTheDocument();
  });

  it("preserves platform search and scroll state when closing the drawer and restores focus", async () => {
    renderPlatformView();

    const searchInput = screen.getByPlaceholderText(/搜索技能/);
    fireEvent.change(searchInput, { target: { value: "frontend" } });

    const scroller = searchInput.closest(".flex.flex-col.h-full")?.querySelector(".flex-1.overflow-auto.p-6");
    expect(scroller).not.toBeNull();
    if (!scroller) return;
    (scroller as HTMLDivElement).scrollTop = 180;

    const trigger = screen.getByRole("button", { name: /查看 frontend-design 的详情/i });
    fireEvent.click(trigger);

    await waitFor(() => {
      expect(screen.getByTestId("skill-detail-drawer")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /close drawer/i }));

    await waitFor(() => {
      expect(screen.queryByTestId("skill-detail-drawer")).not.toBeInTheDocument();
    });

    expect(searchInput).toHaveValue("frontend");
    expect((scroller as HTMLDivElement).scrollTop).toBe(180);
    expect(trigger).toHaveFocus();
  });

  it("restores focus to the originating duplicate Claude row trigger", async () => {
    mockUseSkillStore.mockImplementation((selector?: unknown) => {
      const state = buildSkillStoreState({
        skillsByAgent: { "claude-code": mockDuplicateClaudeSkills },
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });

    renderPlatformView();

    const [userTrigger] = screen.getAllByRole("button", {
      name: /查看 shared-skill 的详情/i,
    });
    fireEvent.click(userTrigger);

    await waitFor(() => {
      expect(screen.getByTestId("skill-detail-drawer")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /close drawer/i }));

    await waitFor(() => {
      expect(screen.queryByTestId("skill-detail-drawer")).not.toBeInTheDocument();
    });

    expect(userTrigger).toHaveFocus();
  });

  it("re-fetches the live Claude list after a scan generation change and removes stale duplicate rows without clearing the search query", async () => {
    let platformState = buildPlatformStoreState({
      scanGeneration: 1,
      skillsByAgent: { "claude-code": 2 },
    });
    let skillState = buildSkillStoreState({
      skillsByAgent: { "claude-code": mockDuplicateClaudeSkillsWithDistinctIds },
    });

    mockUsePlatformStore.mockImplementation((selector?: unknown) => {
      if (typeof selector === "function") return selector(platformState);
      return platformState;
    });
    mockUseSkillStore.mockImplementation((selector?: unknown) => {
      if (typeof selector === "function") return selector(skillState);
      return skillState;
    });

    const view = renderPlatformView();

    const searchInput = screen.getByPlaceholderText(/搜索技能/);
    fireEvent.change(searchInput, { target: { value: "shared-skill-id" } });

    await waitFor(() => {
      expect(
        screen.getAllByRole("button", { name: /查看 Shared skill 的详情/i })
      ).toHaveLength(2);
    });

    mockGetSkillsByAgent.mockClear();

    platformState = buildPlatformStoreState({
      scanGeneration: 2,
      skillsByAgent: { "claude-code": 2 },
    });
    skillState = buildSkillStoreState({
      skillsByAgent: {
        "claude-code": [
          mockDuplicateClaudeSkillsWithDistinctIds[1],
          {
            id: "other-skill",
            name: "Other skill",
            description: "Non-matching survivor",
            file_path: "~/.claude/skills/other-skill/SKILL.md",
            dir_path: "~/.claude/skills/other-skill",
            link_type: "native",
            is_central: false,
            source_kind: "user",
            source_root: "~/.claude/skills",
            is_read_only: false,
          },
        ],
      },
    });

    view.rerender(
      <MemoryRouter initialEntries={["/platform/claude-code"]}>
        <Routes>
          <Route path="/platform/:agentId" element={<PlatformView />} />
        </Routes>
      </MemoryRouter>
    );

    await waitFor(() => {
      expect(mockGetSkillsByAgent).toHaveBeenCalledWith("claude-code");
    });

    expect(searchInput).toHaveValue("shared-skill-id");
    expect(
      screen.getAllByRole("button", { name: /查看 Shared skill 的详情/i })
    ).toHaveLength(1);
    expect(getCardBadgeMatches(userSourceText)).toHaveLength(0);
    expect(getCardBadgeMatches(pluginSourceText)).toHaveLength(1);
    expect(getCardBadgeMatches(readOnlyText)).toHaveLength(1);
    expect(screen.queryByText("Other skill")).not.toBeInTheDocument();
  });

  it("resets the platform content scroll when navigating to another platform", async () => {
    mockUsePlatformStore.mockImplementation((selector?: unknown) => {
      const state = buildPlatformStoreState({
        agents: [mockAgent, mockCursorAgent],
        skillsByAgent: {
          "claude-code": mockSkills.length,
          cursor: mockCursorSkills.length,
        },
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });
    mockUseSkillStore.mockImplementation((selector?: unknown) => {
      const state = buildSkillStoreState({
        skillsByAgent: {
          "claude-code": mockSkills,
          cursor: mockCursorSkills,
        },
        loadingByAgent: {
          "claude-code": false,
          cursor: false,
        },
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });

    render(
      <MemoryRouter initialEntries={["/platform/claude-code"]}>
        <NavigationHarness />
        <Routes>
          <Route path="/platform/:agentId" element={<PlatformView />} />
        </Routes>
      </MemoryRouter>
    );

    expect(screen.getByText("Claude Code")).toBeInTheDocument();

    const searchInput = screen.getByPlaceholderText(/搜索技能/);
    const scroller = searchInput
      .closest(".flex.flex-col.h-full")
      ?.querySelector(".flex-1.overflow-auto.p-6");
    expect(scroller).not.toBeNull();
    if (!scroller) return;

    (scroller as HTMLDivElement).scrollTop = 180;

    await act(async () => {
      testNavigate?.("/platform/cursor");
    });

    await waitFor(() => {
      expect(screen.getByText("Cursor")).toBeInTheDocument();
    });

    await waitFor(() => {
      expect((scroller as HTMLDivElement).scrollTop).toBe(0);
    });
  });
});
