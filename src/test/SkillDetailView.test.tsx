import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, fireEvent, waitFor, within, cleanup } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { SkillDetailView } from "../components/skill/SkillDetailView";
import {
  AgentWithStatus,
  CentralSkillUpdateState,
  SkillDetail as SkillDetailType,
  SkillRepositoryWithStats,
  SkillTag,
} from "../types";

// ─── Mock stores ──────────────────────────────────────────────────────────────

vi.mock("../stores/skillDetailStore", () => ({
  useSkillDetailStore: vi.fn(),
}));

vi.mock("../stores/platformStore", () => ({
  usePlatformStore: vi.fn(),
}));

// ─── Mock CollectionPickerDialog ──────────────────────────────────────────────

vi.mock("../components/collection/CollectionPickerDialog", () => ({
  CollectionPickerDialog: ({
    open,
    onOpenChange,
    onAdded,
    currentCollectionIds,
  }: {
    open: boolean;
    onOpenChange: (open: boolean) => void;
    skillId: string;
    currentCollectionIds: string[];
    onAdded: () => void;
  }) =>
    open ? (
      <div
        data-testid="collection-picker-dialog"
        data-current-collection-ids={currentCollectionIds.join(",")}
      >
        <button onClick={() => { onAdded(); onOpenChange(false); }}>
          Confirm add to collection
        </button>
        <button onClick={() => onOpenChange(false)}>Cancel picker</button>
      </div>
    ) : null,
}));

import { useSkillDetailStore } from "../stores/skillDetailStore";
import { usePlatformStore } from "../stores/platformStore";
import { useCentralSkillsStore } from "../stores/centralSkillsStore";

// ─── Mock react-markdown ──────────────────────────────────────────────────────

vi.mock("react-markdown", () => ({
  default: ({
    children,
    remarkPlugins,
  }: {
    children: string;
    remarkPlugins?: unknown[];
  }) => (
    <div
      data-testid="react-markdown"
      data-has-remark-gfm={remarkPlugins && remarkPlugins.length > 0 ? "true" : "false"}
    >
      {children}
    </div>
  ),
}));

// ─── Fixtures ─────────────────────────────────────────────────────────────────

const mockAgents: AgentWithStatus[] = [
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
    global_skills_dir: "~/.cursor/skills/",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
  {
    id: "central",
    display_name: "Central Skills",
    category: "central",
    global_skills_dir: "~/.skillsmanage/skills/",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
];

const mockRepositories: SkillRepositoryWithStats[] = [
  {
    id: "local-unknown",
    name: "本地 / 未知来源",
    source_type: "local",
    is_unknown: true,
    created_at: "2026-04-10T00:00:00Z",
    updated_at: "2026-04-10T00:00:00Z",
    skill_count: 1,
    unknown_skill_count: 1,
  },
  {
    id: "github-openai-skills-main",
    name: "openai/skills",
    source_type: "github",
    owner: "openai",
    repo: "skills",
    branch: "main",
    url: "https://github.com/openai/skills",
    is_unknown: false,
    created_at: "2026-04-10T00:00:00Z",
    updated_at: "2026-04-10T00:00:00Z",
    skill_count: 1,
    unknown_skill_count: 0,
  },
];

const mockTags: SkillTag[] = [
  {
    id: "frontend-visual-design",
    name: "前端与视觉设计",
    is_builtin: true,
    created_at: "2026-04-10T00:00:00Z",
    updated_at: "2026-04-10T00:00:00Z",
  },
  {
    id: "programming-agent-engineering",
    name: "编程与 Agent 工程",
    is_builtin: true,
    created_at: "2026-04-10T00:00:00Z",
    updated_at: "2026-04-10T00:00:00Z",
  },
];

const mockDetail: SkillDetailType = {
  id: "frontend-design",
  row_id: "frontend-design",
  name: "frontend-design",
  description: "Build distinctive, production-grade frontend interfaces",
  file_path: "~/.skillsmanage/skills/frontend-design/SKILL.md",
  dir_path: "~/.skillsmanage/skills/frontend-design",
  canonical_path: "~/.skillsmanage/skills/frontend-design",
  is_central: true,
  source: "native",
  scanned_at: "2026-04-09T00:00:00Z",
  source_kind: null,
  source_root: null,
  is_read_only: false,
  conflict_group: null,
  conflict_count: 0,
  collections: [],
  installations: [
    {
      skill_id: "frontend-design",
      agent_id: "claude-code",
      installed_path: "~/.claude/skills/frontend-design",
      link_type: "symlink",
      symlink_target: "~/.skillsmanage/skills/frontend-design",
      installed_at: "2026-04-09T12:00:00Z",
    },
  ],
};

const mockPluginDetail: SkillDetailType = {
  ...mockDetail,
  row_id: "claude-code::plugin::frontend-design",
  file_path: "~/.claude/plugins/cache/publisher/frontend-design/unknown/skills/frontend-design/SKILL.md",
  dir_path: "~/.claude/plugins/cache/publisher/frontend-design/unknown/skills/frontend-design",
  canonical_path: undefined,
  is_central: false,
  source: "plugin",
  source_kind: "plugin",
  source_root: "~/.claude/plugins/cache/publisher/frontend-design/unknown",
  is_read_only: true,
  installations: [],
  collections: [],
};

const mockClaudeUserDetail: SkillDetailType = {
  ...mockDetail,
  row_id: "claude-code::user::frontend-design",
  file_path: "~/.claude/skills/frontend-design/SKILL.md",
  dir_path: "~/.claude/skills/frontend-design",
  is_central: false,
  source: "user",
  source_kind: "user",
  source_root: "~/.claude/skills",
  is_read_only: false,
  collections: [
    {
      id: "claude-user",
      name: "Claude User",
      description: "User-managed Claude skills",
      created_at: "2026-04-09T00:00:00Z",
      updated_at: "2026-04-09T00:00:00Z",
    },
  ],
};

const mockContent =
  "---\nname: frontend-design\ndescription: Build distinctive, production-grade frontend interfaces\nmetadata:\n  openclaw:\n    requires:\n      anyBins:\n        - bun\n        - npx\n---\n\n# Frontend Design\n\nContent here.";

const mockPluginContent =
  "---\nname: frontend-design\ndescription: Plugin copy\n---\n\n# Plugin Frontend Design\n\nPlugin content.";

const mockUserContent =
  "---\nname: frontend-design\ndescription: User copy\n---\n\n# User Frontend Design\n\nUser content.";

const mockLoadDetail = vi.fn();
const mockInstallSkill = vi.fn();
const mockUninstallSkill = vi.fn();
const mockLoadCachedExplanation = vi.fn();
const mockGenerateExplanation = vi.fn();
const mockRefreshExplanation = vi.fn();
const mockCleanupExplanationListeners = vi.fn();
const mockReset = vi.fn();
const mockRescan = vi.fn();
const mockRefreshCounts = vi.fn();
const mockRefreshInstallations = vi.fn();
const mockAssignSkillsToRepository = vi.fn();
const mockAssignSkillTags = vi.fn();
const mockCheckSkillUpdates = vi.fn();
const mockUpdateSkills = vi.fn();

function buildDetailStoreState(overrides = {}) {
  return {
    detail: mockDetail,
    content: mockContent,
    isLoading: false,
    installingAgentId: null,
    error: null,
    explanation: null,
    isExplanationLoading: false,
    isExplanationStreaming: false,
    explanationError: null,
    explanationErrorInfo: null,
    loadDetail: mockLoadDetail,
    loadCachedExplanation: mockLoadCachedExplanation,
    generateExplanation: mockGenerateExplanation,
    refreshExplanation: mockRefreshExplanation,
    installSkill: mockInstallSkill,
    uninstallSkill: mockUninstallSkill,
    refreshInstallations: mockRefreshInstallations,
    cleanupExplanationListeners: mockCleanupExplanationListeners,
    reset: mockReset,
    ...overrides,
  };
}

function buildPlatformStoreState(overrides = {}) {
  return {
    agents: mockAgents,
    skillsByAgent: {},
    collectionCount: 0,
    discoveredCount: 0,
    lastScanAt: "2026-04-23T01:00:00Z",
    scanState: "idle",
    isLoading: false,
    isRefreshing: false,
    error: null,
    initialize: vi.fn(),
    hydrateShell: vi.fn(),
    refreshScanInBackground: vi.fn(),
    rescan: mockRescan,
    refreshCounts: mockRefreshCounts,
    resetForTargetChange: vi.fn(),
    applyScanSummary: vi.fn(),
    setCollectionCount: vi.fn(),
    setDiscoveredCount: vi.fn(),
    addCustomAgent: vi.fn(),
    updateCustomAgent: vi.fn(),
    removeCustomAgent: vi.fn(),
    ...overrides,
  };
}

function applyStoreMocks(detailOverrides = {}, platformOverrides = {}) {
  vi.mocked(useSkillDetailStore).mockImplementation((selector?: unknown) => {
    const state = buildDetailStoreState(detailOverrides);
    if (typeof selector === "function") return selector(state);
    return state;
  });
  vi.mocked(usePlatformStore).mockImplementation((selector?: unknown) => {
    const state = buildPlatformStoreState(platformOverrides);
    if (typeof selector === "function") return selector(state);
    return state;
  });
}

function renderView(
  skillId = "frontend-design",
  variant: "page" | "drawer" = "page",
  options?: { skipMockSetup?: boolean }
) {
  if (!options?.skipMockSetup) {
    applyStoreMocks();
  }
  return render(
    <MemoryRouter>
      <SkillDetailView skillId={skillId} variant={variant} />
    </MemoryRouter>
  );
}

const updateStatusCases: Array<{
  label: string;
  updateStatuses: Record<string, CentralSkillUpdateState>;
  expectedBadge: string;
  updateEnabled: boolean;
  expectedError?: string;
}> = [
  {
    label: "not checked",
    updateStatuses: {},
    expectedBadge: "未检查",
    updateEnabled: false,
  },
  {
    label: "up to date",
    updateStatuses: {
      "frontend-design": {
        skill_id: "frontend-design",
        source_type: "github",
        status: "up_to_date",
        source_url: "https://github.com/openai/skills",
        last_checked_at: "2026-04-29T01:23:45Z",
      },
    },
    expectedBadge: "已是最新",
    updateEnabled: false,
  },
  {
    label: "update available",
    updateStatuses: {
      "frontend-design": {
        skill_id: "frontend-design",
        source_type: "github",
        status: "update_available",
        source_url: "https://github.com/openai/skills",
        ref: "main",
        last_checked_at: "2026-04-29T01:23:45Z",
      },
    },
    expectedBadge: "有更新",
    updateEnabled: true,
  },
  {
    label: "error",
    updateStatuses: {
      "frontend-design": {
        skill_id: "frontend-design",
        source_type: "github",
        status: "error",
        source_url: "https://github.com/openai/skills",
        error: "Network timeout",
        last_checked_at: "2026-04-29T01:23:45Z",
      },
    },
    expectedBadge: "检查失败",
    updateEnabled: false,
    expectedError: "Network timeout",
  },
];

// ─── Tests ────────────────────────────────────────────────────────────────────

describe("SkillDetailView", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockAssignSkillsToRepository.mockResolvedValue(undefined);
    mockAssignSkillTags.mockResolvedValue(undefined);
    mockCheckSkillUpdates.mockResolvedValue([]);
    mockUpdateSkills.mockResolvedValue({
      succeeded: [],
      failed: [],
      skipped: [],
      states: [],
    });
    useCentralSkillsStore.setState({
      repositories: mockRepositories,
      tags: mockTags,
      isMetadataUpdating: false,
      updateStatuses: {},
      isCheckingUpdates: false,
      updatingSkillIds: [],
      assignSkillsToRepository: mockAssignSkillsToRepository,
      assignSkillTags: mockAssignSkillTags,
      checkSkillUpdates: mockCheckSkillUpdates,
      updateSkills: mockUpdateSkills,
    });
  });

  afterEach(() => {
    cleanup();
  });

  // ── Shell-agnostic: no back button / breadcrumb is rendered here ─────────

  it("does not render a back button (that belongs to the outer shell)", () => {
    renderView();
    expect(screen.queryByRole("button", { name: /返回/i })).toBeNull();
  });

  // ── Skill name & description ──────────────────────────────────────────────

  it("shows skill name in ViewHeader h1", () => {
    renderView();
    expect(screen.getByRole("heading", { name: /frontend-design/i })).toBeInTheDocument();
  });

  it("shows skill description in ViewHeader", () => {
    renderView();
    expect(
      screen.getAllByText("Build distinctive, production-grade frontend interfaces")[0]
    ).toBeInTheDocument();
  });

  it("renders optional leading slot when provided", () => {
    applyStoreMocks();
    render(
      <MemoryRouter>
        <SkillDetailView
          skillId="frontend-design"
          variant="drawer"
          leading={<span data-testid="leading-slot">L</span>}
        />
      </MemoryRouter>
    );
    expect(screen.getByTestId("leading-slot")).toBeInTheDocument();
  });

  // ── Metadata ──────────────────────────────────────────────────────────────

  it("shows metadata section", () => {
    renderView();
    expect(screen.getByRole("region", { name: /技能基本信息/i })).toBeInTheDocument();
  });

  it("shows file path", () => {
    renderView();
    expect(
      screen.getByText("~/.skillsmanage/skills/frontend-design/SKILL.md")
    ).toBeInTheDocument();
  });

  it("uses the widened inspector rail classes in page variant", () => {
    renderView();

    const layout = screen.getByTestId("skill-detail-two-column-layout");
    const sidebar = screen.getByTestId("skill-detail-right-sidebar");

    expect(layout.className).toContain("flex-col");
    expect(layout.className).toContain("lg:flex-row");
    expect(layout.className).not.toContain("md:flex-row");
    expect(sidebar.className).toContain("overflow-x-hidden");
    expect(sidebar.className).toContain("lg:w-72");
    expect(sidebar.className).toContain("xl:w-80");
    expect(sidebar.className).not.toContain("md:w-64");
  });

  it.each(updateStatusCases)("renders update status inspector layout for $label", ({ updateStatuses, expectedBadge, updateEnabled, expectedError }) => {
    useCentralSkillsStore.setState({ updateStatuses });
    renderView();

    const card = screen.getByTestId("detail-update-status-card");
    const actions = screen.getByTestId("detail-update-actions");
    const checkButton = within(actions).getByRole("button", { name: "检查" });
    const updateButton = within(actions).getByRole("button", { name: "更新" });

    expect(card).toHaveTextContent(expectedBadge);
    expect(actions.className).toContain("grid");
    expect(actions.className).toContain("grid-cols-2");
    expect(actions.className).toContain("gap-2");
    expect(checkButton.className).toContain("w-full");
    expect(updateButton.className).toContain("w-full");

    if (updateEnabled) {
      expect(updateButton).toBeEnabled();
    } else {
      expect(updateButton).toBeDisabled();
    }

    if (Object.keys(updateStatuses).length > 0) {
      expect(card).toHaveTextContent("https://github.com/openai/skills");
    }

    if (expectedError) {
      expect(card).toHaveTextContent(expectedError);
    }
  });

  it("stacks repository and tag management into vertical field groups", () => {
    renderView();

    const repositoryGroup = screen.getByTestId("detail-repository-management");
    const tagGroup = screen.getByTestId("detail-tag-management");
    const repositorySelect = within(repositoryGroup).getByLabelText("仓库");
    const repositoryButton = within(repositoryGroup).getByRole("button", { name: "归仓" });
    const tagSelect = within(tagGroup).getByLabelText("分类");
    const tagButton = within(tagGroup).getByRole("button", { name: "打标签" });

    expect(within(repositoryGroup).getByText("仓库")).toBeInTheDocument();
    expect(repositoryGroup.className).toContain("space-y-2");
    expect(repositorySelect.className).toContain("w-full");
    expect(repositoryButton.className).toContain("w-full");

    expect(within(tagGroup).getByText("分类")).toBeInTheDocument();
    expect(tagGroup.className).toContain("space-y-2");
    expect(tagSelect.className).toContain("w-full");
    expect(tagButton.className).toContain("w-full");
  });

  it("assigns repository from detail metadata controls", async () => {
    renderView();

    fireEvent.change(screen.getByLabelText("仓库"), {
      target: { value: "github-openai-skills-main" },
    });
    fireEvent.click(screen.getByRole("button", { name: "归仓" }));

    await waitFor(() => {
      expect(mockAssignSkillsToRepository).toHaveBeenCalledWith(
        ["frontend-design"],
        "github-openai-skills-main"
      );
    });
    expect(mockLoadDetail).toHaveBeenCalledWith({ skillId: "frontend-design" });
  });

  it("assigns tag from detail metadata controls", async () => {
    renderView();

    fireEvent.change(screen.getByLabelText("分类"), {
      target: { value: "programming-agent-engineering" },
    });
    fireEvent.click(screen.getByRole("button", { name: "打标签" }));

    await waitFor(() => {
      expect(mockAssignSkillTags).toHaveBeenCalledWith(
        ["frontend-design"],
        ["programming-agent-engineering"]
      );
    });
    expect(mockLoadDetail).toHaveBeenCalledWith({ skillId: "frontend-design" });
  });

  it("shows canonical path", () => {
    renderView();
    expect(screen.getAllByText("~/.skillsmanage/skills/frontend-design").length).toBeGreaterThan(0);
  });

  it("shows source", () => {
    renderView();
    expect(screen.getByText("native")).toBeInTheDocument();
  });

  it("shows a read-only plugin source state and blocks management actions", () => {
    applyStoreMocks({
      detail: mockPluginDetail,
      content: mockPluginContent,
    });

    render(
      <MemoryRouter>
        <SkillDetailView
          skillId="frontend-design"
          agentId="claude-code"
          rowId="claude-code::plugin::frontend-design"
          variant="drawer"
        />
      </MemoryRouter>
    );

    const sourceStatusRegion = screen.getByRole("region", { name: /来源状态|Source status/i });
    expect(
      within(sourceStatusRegion).getByText(/插件来源|Plugin source/i)
    ).toBeInTheDocument();
    expect(
      within(sourceStatusRegion).getByText(/只读来源|Read-only source/i)
    ).toBeInTheDocument();
    expect(
      screen.getByText("~/.claude/plugins/cache/publisher/frontend-design/unknown/skills/frontend-design/SKILL.md")
    ).toBeInTheDocument();
    expect(screen.getByText("~/.claude/plugins/cache/publisher/frontend-design/unknown")).toBeInTheDocument();
    expect(
      screen.getByText(/插件安装的副本仅供查看|display-only/i)
    ).toBeInTheDocument();
    expect(
      screen.getByText(/不可安装或卸载|Install and uninstall are unavailable/i)
    ).toBeInTheDocument();
    expect(
      screen.getByText(/不可调整技能集|Collection management is unavailable/i)
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /切换 .* 的链接状态/i })
    ).toBeNull();
    expect(
      screen.queryByRole("button", { name: /加入技能集/i })
    ).toBeNull();
  });

  it("keeps user-source Claude detail manageable", () => {
    applyStoreMocks({
      detail: mockClaudeUserDetail,
      content: mockUserContent,
    });

    render(
      <MemoryRouter>
        <SkillDetailView
          skillId="frontend-design"
          agentId="claude-code"
          rowId="claude-code::user::frontend-design"
          variant="drawer"
        />
      </MemoryRouter>
    );

    expect(screen.getByText(/用户来源|User source/i)).toBeInTheDocument();
    expect(screen.queryByText(/只读来源|Read-only source/i)).toBeNull();
    expect(screen.getByText("~/.claude/skills/frontend-design/SKILL.md")).toBeInTheDocument();
    expect(screen.getByText("~/.claude/skills")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /切换 frontend-design 在 Universal 的链接状态/i })
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /加入技能集/i })
    ).toBeInTheDocument();
    expect(screen.getByText("Claude User")).toBeInTheDocument();
  });

  // ── Installation status ───────────────────────────────────────────────────

  it("shows installation status section", () => {
    renderView();
    expect(
      screen.getByRole("region", { name: /安装状态/i })
    ).toBeInTheDocument();
  });

  it("shows platform toggle icons for non-central agents", () => {
    renderView();
    // Each non-central agent should have a toggle icon button
    const toggleButtons = screen.getAllByRole("button", {
      name: /切换 .* 的链接状态/i,
    });
    // 2 non-central targets (Claude Code, Universal)
    expect(toggleButtons).toHaveLength(2);
  });

  it("shows platform name in tooltip on toggle icon", () => {
    renderView();
    // Claude Code is installed — tooltip includes linked status
    const claudeToggle = screen.getByRole("button", {
      name: /切换 frontend-design 在 Claude Code 的链接状态/i,
    });
    expect(claudeToggle).toHaveAttribute("title", expect.stringContaining("Claude Code"));
  });

  it("calls installSkill when unlinked platform icon is clicked", async () => {
    renderView();
    // Universal is NOT installed
    const universalToggle = screen.getByRole("button", {
      name: /切换 frontend-design 在 Universal 的链接状态/i,
    });
    fireEvent.click(universalToggle);
    await waitFor(() => {
      expect(mockInstallSkill).toHaveBeenCalledWith("frontend-design", "cursor");
    });
    expect(mockRefreshCounts).toHaveBeenCalledTimes(1);
    expect(mockRefreshInstallations).toHaveBeenCalledWith("frontend-design");
  });

  it("calls uninstallSkill when linked platform icon is clicked", async () => {
    renderView();
    // Claude Code IS installed
    const claudeToggle = screen.getByRole("button", {
      name: /切换 frontend-design 在 Claude Code 的链接状态/i,
    });
    fireEvent.click(claudeToggle);
    await waitFor(() => {
      expect(mockUninstallSkill).toHaveBeenCalledWith("frontend-design", "claude-code");
    });
    expect(mockRefreshCounts).toHaveBeenCalledTimes(1);
    expect(mockRefreshInstallations).toHaveBeenCalledWith("frontend-design");
  });

  // ── Collections ───────────────────────────────────────────────────────────

  it("shows collections section", () => {
    renderView();
    expect(screen.getByRole("region", { name: /技能集/i })).toBeInTheDocument();
  });

  it("shows Add to collection button", () => {
    renderView();
    expect(
      screen.getByRole("button", { name: /加入技能集/i })
    ).toBeInTheDocument();
  });

  it("shows collection tags when collections are present", () => {
    applyStoreMocks({
      detail: {
        ...mockDetail,
        collections: [
          {
            id: "frontend",
            name: "Frontend",
            description: "Frontend patterns",
            created_at: "2026-04-09T00:00:00Z",
            updated_at: "2026-04-09T00:00:00Z",
          },
          {
            id: "design-system",
            name: "Design System",
            description: "Shared UI system",
            created_at: "2026-04-09T00:00:00Z",
            updated_at: "2026-04-09T00:00:00Z",
          },
        ],
      },
    });
    render(
      <MemoryRouter>
        <SkillDetailView skillId="frontend-design" variant="page" />
      </MemoryRouter>
    );
    expect(screen.getByText("Frontend")).toBeInTheDocument();
    expect(screen.getByText("Design System")).toBeInTheDocument();
  });

  // ── SKILL.md Preview ──────────────────────────────────────────────────────

  it("shows SKILL.md preview as markdown content", () => {
    renderView();
    expect(screen.getByRole("tabpanel", { name: /Markdown/i })).toBeInTheDocument();
  });

  it("shows Markdown tab button", () => {
    renderView();
    expect(screen.getByRole("tab", { name: /Markdown/i })).toBeInTheDocument();
  });

  it("shows Raw Source tab button", () => {
    renderView();
    expect(screen.getByRole("tab", { name: /原始源码/i })).toBeInTheDocument();
  });

  it("shows AI Explanation tab button", () => {
    renderView();
    expect(screen.getByRole("tab", { name: /AI 解释/i })).toBeInTheDocument();
  });

  it("renders markdown content by default in Markdown tab", () => {
    renderView();
    const markdownPane = screen.getByRole("tabpanel", { name: /Markdown/i });
    expect(markdownPane).toBeInTheDocument();
    expect(screen.getByTestId("react-markdown")).toBeInTheDocument();
    expect(screen.getByTestId("react-markdown")).toHaveAttribute("data-has-remark-gfm", "true");
    expect(
      markdownPane.querySelector('[data-skill-markdown-variant="detail"]')
    ).not.toBeNull();
  });

  it("renders frontmatter card in Markdown tab", () => {
    renderView();
    const markdown = screen.getByRole("tabpanel", { name: /Markdown/i });
    expect(within(markdown).getByRole("heading", { name: /Frontmatter/i })).toBeInTheDocument();
    expect(within(markdown).getByText("frontend-design")).toBeInTheDocument();
    expect(within(markdown).getByText("Build distinctive, production-grade frontend interfaces")).toBeInTheDocument();
    expect(within(markdown).getByText("bun")).toBeInTheDocument();
    expect(within(markdown).getByText("npx")).toBeInTheDocument();
    expect(screen.getByTestId("react-markdown")).toHaveTextContent("# Frontend Design");
  });

  it("strips BOM-prefixed frontmatter before rendering markdown", () => {
    applyStoreMocks({
      content:
        "\uFEFF---\r\nname: wrangler\r\ndescription: Cloudflare Workers CLI\r\n---\r\n\r\n# Wrangler CLI\r\n\r\nBody.",
    });
    renderView("frontend-design", "page", { skipMockSetup: true });

    const markdown = screen.getByRole("tabpanel", { name: /Markdown/i });
    expect(within(markdown).getByText("wrangler")).toBeInTheDocument();
    expect(screen.getByTestId("react-markdown")).toHaveTextContent("# Wrangler CLI");
    expect(screen.getByTestId("react-markdown")).not.toHaveTextContent("name: wrangler");
    expect(screen.getByTestId("react-markdown")).not.toHaveTextContent("---");
  });

  it("keeps malformed frontmatter out of markdown body while preserving summary fields", () => {
    applyStoreMocks({
      content:
        "---\nname: broken-skill\ndescription: Broken summary\nmetadata: [oops\n---\n\n# Broken Skill\n\nBody.",
    });
    renderView("frontend-design", "page", { skipMockSetup: true });

    const markdown = screen.getByRole("tabpanel", { name: /Markdown/i });
    expect(within(markdown).getByRole("heading", { name: /Frontmatter/i })).toBeInTheDocument();
    expect(within(markdown).getByText("broken-skill")).toBeInTheDocument();
    expect(within(markdown).getByText("Broken summary")).toBeInTheDocument();
    expect(screen.getByTestId("react-markdown")).toHaveTextContent("# Broken Skill");
    expect(screen.getByTestId("react-markdown")).not.toHaveTextContent("name: broken-skill");
    expect(screen.getByTestId("react-markdown")).not.toHaveTextContent("metadata: [oops");
  });

  it("switches to raw source tab when Raw Source is clicked", async () => {
    renderView();
    const rawTab = screen.getByRole("tab", { name: /原始源码/i });
    fireEvent.click(rawTab);
    await waitFor(() => {
      expect(screen.getByRole("tabpanel", { name: /原始源码/i })).toBeInTheDocument();
    });
  });

  it("shows raw content including frontmatter in raw source tab", async () => {
    renderView();
    const rawTab = screen.getByRole("tab", { name: /原始源码/i });
    fireEvent.click(rawTab);
    await waitFor(() => {
      const rawPane = screen.getByRole("tabpanel", { name: /原始源码/i });
      expect(rawPane).toHaveTextContent("---");
      expect(rawPane).toHaveTextContent("name: frontend-design");
    });
  });

  it("loads cached explanation when content is available", async () => {
    renderView();
    await waitFor(() => {
      expect(mockLoadCachedExplanation).toHaveBeenCalledWith("frontend-design", "zh");
    });
  });

  it("loads cached explanation with the selected Claude row id", async () => {
    applyStoreMocks({
      detail: mockPluginDetail,
      content: mockPluginContent,
    });

    render(
      <MemoryRouter>
        <SkillDetailView
          skillId="frontend-design"
          agentId="claude-code"
          rowId="claude-code::plugin::frontend-design"
          variant="page"
        />
      </MemoryRouter>
    );

    await waitFor(() => {
      expect(mockLoadCachedExplanation).toHaveBeenCalledWith(
        "claude-code::plugin::frontend-design",
        "zh"
      );
    });
  });

  it("uses the resolved Claude detail row id for cached explanation lookup when rowId is omitted", async () => {
    applyStoreMocks({
      detail: mockClaudeUserDetail,
      content: mockUserContent,
    });

    render(
      <MemoryRouter>
        <SkillDetailView
          skillId="frontend-design"
          agentId="claude-code"
          variant="page"
        />
      </MemoryRouter>
    );

    await waitFor(() => {
      expect(mockLoadCachedExplanation).toHaveBeenCalledWith(
        "claude-code::user::frontend-design",
        "zh"
      );
    });
  });

  it("shows cached AI explanation in AI Explanation tab", async () => {
    applyStoreMocks({ explanation: "这是缓存的技能解释。" });
    render(
      <MemoryRouter>
        <SkillDetailView skillId="frontend-design" variant="page" />
      </MemoryRouter>
    );

    fireEvent.click(screen.getByRole("tab", { name: /AI 解释/i }));
    await waitFor(() => {
      expect(screen.getByText("这是缓存的技能解释。")).toBeInTheDocument();
    });
    const explanationPane = screen.getByRole("tabpanel", { name: /AI 解释/i });
    expect(
      explanationPane.querySelector('[data-skill-markdown-variant="compact"]')
    ).not.toBeNull();
  });

  it("calls generateExplanation from empty AI Explanation tab", async () => {
    renderView();
    fireEvent.click(screen.getByRole("tab", { name: /AI 解释/i }));
    // Two buttons can carry the "生成解释" label (header action + empty-state
    // CTA); both invoke `handleGenerateExplanation`, so clicking either one is
    // equivalent. Use the first match to keep the assertion stable.
    const generateButtons = screen.getAllByRole("button", { name: /生成解释/i });
    fireEvent.click(generateButtons[0]);
    await waitFor(() => {
      expect(mockGenerateExplanation).toHaveBeenCalledWith("frontend-design", mockContent, "zh");
    });
  });

  it("calls generateExplanation with the selected Claude row id", async () => {
    applyStoreMocks({
      detail: mockPluginDetail,
      content: mockPluginContent,
    });

    render(
      <MemoryRouter>
        <SkillDetailView
          skillId="frontend-design"
          agentId="claude-code"
          rowId="claude-code::plugin::frontend-design"
          variant="page"
        />
      </MemoryRouter>
    );

    fireEvent.click(screen.getByRole("tab", { name: /AI 解释/i }));
    fireEvent.click(screen.getAllByRole("button", { name: /生成解释/i })[0]);

    await waitFor(() => {
      expect(mockGenerateExplanation).toHaveBeenCalledWith(
        "claude-code::plugin::frontend-design",
        mockPluginContent,
        "zh"
      );
    });
  });

  it("calls refreshExplanation when cached explanation exists", async () => {
    applyStoreMocks({ explanation: "这是缓存的技能解释。" });
    render(
      <MemoryRouter>
        <SkillDetailView skillId="frontend-design" variant="page" />
      </MemoryRouter>
    );

    fireEvent.click(screen.getByRole("tab", { name: /AI 解释/i }));
    fireEvent.click(screen.getByRole("button", { name: /重新生成/i }));
    await waitFor(() => {
      expect(mockRefreshExplanation).toHaveBeenCalledWith("frontend-design", mockContent, "zh");
    });
  });

  it("calls refreshExplanation with the resolved Claude row id", async () => {
    applyStoreMocks({
      detail: mockClaudeUserDetail,
      content: mockUserContent,
      explanation: "这是用户来源缓存的技能解释。",
    });

    render(
      <MemoryRouter>
        <SkillDetailView
          skillId="frontend-design"
          agentId="claude-code"
          variant="page"
        />
      </MemoryRouter>
    );

    fireEvent.click(screen.getByRole("tab", { name: /AI 解释/i }));
    fireEvent.click(screen.getByRole("button", { name: /重新生成/i }));

    await waitFor(() => {
      expect(mockRefreshExplanation).toHaveBeenCalledWith(
        "claude-code::user::frontend-design",
        mockUserContent,
        "zh"
      );
    });
  });

  it("shows explanation loading state while a request is in flight", async () => {
    applyStoreMocks({
      explanation: null,
      isExplanationLoading: true,
      isExplanationStreaming: true,
    });

    render(
      <MemoryRouter>
        <SkillDetailView skillId="frontend-design" variant="page" />
      </MemoryRouter>
    );

    fireEvent.click(screen.getByRole("tab", { name: /AI 解释/i }));

    await waitFor(() => {
      expect(screen.getByText(/正在加载 AI 解释/i)).toBeInTheDocument();
    });
    // Only the header action button is present while loading (the empty-state
    // CTA is replaced by the loading indicator). Disable state applies to it.
    expect(screen.getByRole("button", { name: /生成解释/i })).toBeDisabled();
  });

  it("shows streaming indicator once explanation content starts arriving", async () => {
    applyStoreMocks({
      explanation: "第一段解释",
      isExplanationLoading: false,
      isExplanationStreaming: true,
    });

    render(
      <MemoryRouter>
        <SkillDetailView skillId="frontend-design" variant="page" />
      </MemoryRouter>
    );

    fireEvent.click(screen.getByRole("tab", { name: /AI 解释/i }));

    expect(screen.getByText("第一段解释")).toBeInTheDocument();
    // Matches the current i18n string for `detail.explanationStreaming`.
    expect(screen.getByText(/正在生成解释/i)).toBeInTheDocument();
  });

  it("shows recoverable explanation error state without leaving stale explanation visible", async () => {
    applyStoreMocks({
      explanation: null,
      explanationError: "代理连接失败",
      explanationErrorInfo: {
        message: "代理连接失败",
        details: "error sending request",
        kind: "proxy",
        retryable: true,
        fallbackTried: true,
      },
    });

    render(
      <MemoryRouter>
        <SkillDetailView skillId="frontend-design" variant="page" />
      </MemoryRouter>
    );

    fireEvent.click(screen.getByRole("tab", { name: /AI 解释/i }));

    await waitFor(() => {
      expect(screen.getByText("代理连接失败")).toBeInTheDocument();
    });
    expect(screen.getByText(/备用端点也无法访问/i)).toBeInTheDocument();
    expect(screen.getByText(/暂无 AI 解释/i)).toBeInTheDocument();
    expect(screen.queryByText("这是缓存的技能解释。")).not.toBeInTheDocument();
    // Both the header action and the empty-state CTA are enabled when an
    // error is present but no request is in flight.
    const generateButtons = screen.getAllByRole("button", { name: /生成解释/i });
    expect(generateButtons.length).toBeGreaterThan(0);
    generateButtons.forEach((btn) => expect(btn).toBeEnabled());
  });

  it("keeps retry action available after explanation failure", async () => {
    applyStoreMocks({
      explanation: null,
      explanationError: "temporary failure",
    });

    render(
      <MemoryRouter>
        <SkillDetailView skillId="frontend-design" variant="page" />
      </MemoryRouter>
    );

    fireEvent.click(screen.getByRole("tab", { name: /AI 解释/i }));
    // Two buttons match /生成解释/i after a failure (header + empty state).
    // Clicking either one retries via `handleGenerateExplanation`.
    const generateButtons = screen.getAllByRole("button", { name: /生成解释/i });
    fireEvent.click(generateButtons[0]);

    await waitFor(() => {
      expect(mockGenerateExplanation).toHaveBeenCalledWith("frontend-design", mockContent, "zh");
    });
  });

  it("reveals structured explanation error details on demand", async () => {
    applyStoreMocks({
      explanation: null,
      explanationError: "temporary failure",
      explanationErrorInfo: {
        message: "temporary failure",
        details: "connect ECONNREFUSED 127.0.0.1:3000",
        kind: "connect",
        retryable: true,
        fallbackTried: false,
      },
    });

    render(
      <MemoryRouter>
        <SkillDetailView skillId="frontend-design" variant="page" />
      </MemoryRouter>
    );

    fireEvent.click(screen.getByRole("tab", { name: /AI 解释/i }));
    fireEvent.click(screen.getByRole("button", { name: /查看详情/i }));

    expect(screen.getByText(/ECONNREFUSED 127.0.0.1:3000/i)).toBeInTheDocument();
    const generateButtons = screen.getAllByRole("button", { name: /生成解释/i });
    expect(generateButtons.length).toBeGreaterThan(0);
    generateButtons.forEach((btn) => expect(btn).toBeEnabled());
  });

  // ── Loading state ─────────────────────────────────────────────────────────

  it("shows loading state when isLoading is true", () => {
    applyStoreMocks({ isLoading: true, detail: null });
    render(
      <MemoryRouter>
        <SkillDetailView skillId="frontend-design" variant="page" />
      </MemoryRouter>
    );
    expect(screen.getByText(/正在加载技能详情/i)).toBeInTheDocument();
  });

  // ── Error state ───────────────────────────────────────────────────────────

  it("shows error message when error occurs", () => {
    applyStoreMocks({ error: "Skill not found", detail: null });
    render(
      <MemoryRouter>
        <SkillDetailView skillId="frontend-design" variant="page" />
      </MemoryRouter>
    );
    expect(screen.getByText("Skill not found")).toBeInTheDocument();
  });

  it("renders a safe browser fallback when the Tauri bridge is unavailable", async () => {
    // `setup.ts` defines these without `configurable: true`, so we cannot
    // reuse `Object.defineProperty` to swap them out — assign directly
    // instead, which is allowed because `writable: true` was set initially.
    const w = window as unknown as {
      __TAURI__?: unknown;
      __TAURI_INTERNALS__?: unknown;
    };
    const prevTauri = w.__TAURI__;
    const prevInternals = w.__TAURI_INTERNALS__;
    w.__TAURI__ = undefined;
    w.__TAURI_INTERNALS__ = undefined;

    try {
      applyStoreMocks({ detail: null, content: null, error: null, isLoading: false });

      render(
        <MemoryRouter>
          <SkillDetailView skillId="defuddle" variant="page" />
        </MemoryRouter>
      );

      expect(await screen.findByText(/技能详情需要桌面运行时/i)).toBeInTheDocument();
      expect(screen.getByText(/浏览器预览中该路由现在会安全渲染/i)).toBeInTheDocument();
    } finally {
      w.__TAURI__ = prevTauri;
      w.__TAURI_INTERNALS__ = prevInternals;
    }
  });

  // ── Store calls ───────────────────────────────────────────────────────────

  it("calls loadDetail on mount with skillId prop", () => {
    renderView("frontend-design");
    expect(mockLoadDetail).toHaveBeenCalledWith({
      skillId: "frontend-design",
      agentId: undefined,
      rowId: undefined,
    });
  });

  it("passes source-aware Claude row identity into loadDetail when provided", () => {
    applyStoreMocks();
    render(
      <MemoryRouter>
        <SkillDetailView
          skillId="frontend-design"
          agentId="claude-code"
          rowId="claude-code::plugin::frontend-design"
          variant="drawer"
        />
      </MemoryRouter>
    );

    expect(mockLoadDetail).toHaveBeenCalledWith({
      skillId: "frontend-design",
      agentId: "claude-code",
      rowId: "claude-code::plugin::frontend-design",
    });
  });

  it("switching duplicate Claude rows updates path, content, and management affordances", async () => {
    applyStoreMocks({
      detail: mockPluginDetail,
      content: mockPluginContent,
    });

    const { rerender } = render(
      <MemoryRouter>
        <SkillDetailView
          skillId="frontend-design"
          agentId="claude-code"
          rowId="claude-code::plugin::frontend-design"
          variant="drawer"
        />
      </MemoryRouter>
    );

    expect(screen.getByText("~/.claude/plugins/cache/publisher/frontend-design/unknown/skills/frontend-design/SKILL.md")).toBeInTheDocument();
    expect(screen.getByTestId("react-markdown")).toHaveTextContent("# Plugin Frontend Design");
    expect(screen.queryByRole("button", { name: /加入技能集/i })).toBeNull();

    mockLoadDetail.mockClear();
    applyStoreMocks({
      detail: mockClaudeUserDetail,
      content: mockUserContent,
    });

    rerender(
      <MemoryRouter>
        <SkillDetailView
          skillId="frontend-design"
          agentId="claude-code"
          rowId="claude-code::user::frontend-design"
          variant="drawer"
        />
      </MemoryRouter>
    );

    await waitFor(() => {
      expect(mockLoadDetail).toHaveBeenCalledWith({
        skillId: "frontend-design",
        agentId: "claude-code",
        rowId: "claude-code::user::frontend-design",
      });
    });

    expect(screen.getByText("~/.claude/skills/frontend-design/SKILL.md")).toBeInTheDocument();
    expect(screen.getByTestId("react-markdown")).toHaveTextContent("# User Frontend Design");
    expect(screen.getByRole("button", { name: /加入技能集/i })).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /切换 frontend-design 在 Universal 的链接状态/i })
    ).toBeInTheDocument();
    expect(screen.queryByText(/只读来源|Read-only source/i)).toBeNull();
  });

  it("retries a failed Claude duplicate detail load with the same row identity", async () => {
    applyStoreMocks({
      detail: null,
      content: null,
      error: "Multiple Claude rows found",
    });

    render(
      <MemoryRouter>
        <SkillDetailView
          skillId="frontend-design"
          agentId="claude-code"
          rowId="claude-code::user::frontend-design"
          variant="drawer"
        />
      </MemoryRouter>
    );

    mockLoadDetail.mockClear();
    fireEvent.click(screen.getByRole("button", { name: /重试/i }));

    await waitFor(() => {
      expect(mockLoadDetail).toHaveBeenCalledWith({
        skillId: "frontend-design",
        agentId: "claude-code",
        rowId: "claude-code::user::frontend-design",
      });
    });
  });

  it("calls reset on unmount", () => {
    const { unmount } = renderView();
    unmount();
    expect(mockReset).toHaveBeenCalled();
  });

  // ── Spinner during install/uninstall ──────────────────────────────────────

  it("disables toggle icon when that agent is installing", () => {
    applyStoreMocks({ installingAgentId: "cursor" });
    render(
      <MemoryRouter>
        <SkillDetailView skillId="frontend-design" variant="page" />
      </MemoryRouter>
    );
    const cursorToggle = screen.getByRole("button", {
      name: /切换 frontend-design 在 Universal 的链接状态/i,
    });
    expect(cursorToggle).toBeDisabled();
  });

  // ── CollectionPickerDialog integration ────────────────────────────────────

  it("does not render CollectionPickerDialog by default", () => {
    renderView();
    expect(screen.queryByTestId("collection-picker-dialog")).toBeNull();
  });

  it("opens CollectionPickerDialog when Add to collection is clicked", async () => {
    renderView();
    const addBtn = screen.getByRole("button", { name: /加入技能集/i });
    fireEvent.click(addBtn);
    await waitFor(() => {
      expect(screen.getByTestId("collection-picker-dialog")).toBeInTheDocument();
    });
  });

  it("passes current collection ids into CollectionPickerDialog for preselection", async () => {
    applyStoreMocks({
      detail: {
        ...mockDetail,
        collections: [
          {
            id: "frontend",
            name: "Frontend",
            description: "Frontend patterns",
            created_at: "2026-04-09T00:00:00Z",
            updated_at: "2026-04-09T00:00:00Z",
          },
          {
            id: "design-system",
            name: "Design System",
            description: "Shared UI system",
            created_at: "2026-04-09T00:00:00Z",
            updated_at: "2026-04-09T00:00:00Z",
          },
        ],
      },
    });
    renderView("frontend-design", "page", { skipMockSetup: true });

    fireEvent.click(screen.getByRole("button", { name: /加入技能集/i }));

    await waitFor(() => {
      expect(screen.getByTestId("collection-picker-dialog")).toHaveAttribute(
        "data-current-collection-ids",
        "frontend,design-system"
      );
    });
  });

  it("closes CollectionPickerDialog when cancel is clicked inside it", async () => {
    renderView();
    fireEvent.click(screen.getByRole("button", { name: /加入技能集/i }));
    await waitFor(() => {
      expect(screen.getByTestId("collection-picker-dialog")).toBeInTheDocument();
    });
    fireEvent.click(screen.getByRole("button", { name: /Cancel picker/i }));
    await waitFor(() => {
      expect(screen.queryByTestId("collection-picker-dialog")).toBeNull();
    });
  });

  it("restores focus to the add-to-collection trigger after closing the picker", async () => {
    renderView();
    const addBtn = screen.getByRole("button", { name: /加入技能集/i });
    fireEvent.click(addBtn);

    await waitFor(() => {
      expect(screen.getByTestId("collection-picker-dialog")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /Cancel picker/i }));

    await waitFor(() => {
      expect(screen.queryByTestId("collection-picker-dialog")).toBeNull();
    });

    expect(addBtn).toHaveFocus();
  });

  it("calls loadDetail to refresh skill after collections are added", async () => {
    renderView();
    mockLoadDetail.mockClear(); // clear the initial load call

    fireEvent.click(screen.getByRole("button", { name: /加入技能集/i }));
    await waitFor(() => {
      expect(screen.getByTestId("collection-picker-dialog")).toBeInTheDocument();
    });

    // Simulate confirming the picker (which calls onAdded then closes)
    fireEvent.click(screen.getByRole("button", { name: /Confirm add to collection/i }));

    await waitFor(() => {
      expect(mockLoadDetail).toHaveBeenCalledWith({
        skillId: "frontend-design",
        agentId: undefined,
        rowId: undefined,
      });
    });
  });
});
