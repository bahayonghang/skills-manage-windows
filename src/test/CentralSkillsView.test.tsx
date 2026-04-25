import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, fireEvent, waitFor, cleanup } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { CentralSkillsView } from "../pages/CentralSkillsView";
import { AgentWithStatus, SkillDetail, SkillRepositoryWithStats, SkillTag, SkillWithLinks } from "../types";

// Mock stores
vi.mock("../stores/centralSkillsStore", () => ({
  useCentralSkillsStore: vi.fn(),
}));

vi.mock("../stores/platformStore", () => ({
  usePlatformStore: vi.fn(),
}));

vi.mock("../stores/skillStore", () => ({
  useSkillStore: vi.fn(),
}));

vi.mock("../stores/marketplaceStore", () => ({
  useMarketplaceStore: vi.fn(),
}));

vi.mock("../components/skill/SkillDetailDrawer", () => ({
  SkillDetailDrawer: ({
    open,
    skillId,
    onOpenChange,
    returnFocusRef,
  }: {
    open: boolean;
    skillId: string | null;
    onOpenChange: (open: boolean) => void;
    returnFocusRef?: { current: HTMLElement | null };
  }) =>
    open ? (
      <div data-testid="skill-detail-drawer">
        <div>drawer-skill:{skillId}</div>
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

import { useCentralSkillsStore } from "../stores/centralSkillsStore";
import { usePlatformStore } from "../stores/platformStore";
import { useSkillStore } from "../stores/skillStore";
import { useMarketplaceStore } from "../stores/marketplaceStore";
import * as tauriBridge from "@/lib/tauri";

// ─── Fixtures ─────────────────────────────────────────────────────────────────

const mockAgents: AgentWithStatus[] = [
  {
    id: "claude-code",
    display_name: "Claude Code",
    category: "coding",
    global_skills_dir: "/Users/test/.claude/skills/",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
  {
    id: "cursor",
    display_name: "Cursor",
    category: "coding",
    global_skills_dir: "/Users/test/.agents/skills/",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
  {
    id: "codex",
    display_name: "Codex",
    category: "coding",
    global_skills_dir: "/Users/test/.agents/skills/",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
  {
    id: "central",
    display_name: "Central Skills",
    category: "central",
    global_skills_dir: "/Users/test/.skillsmanage/skills/",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
];

const mockSkills: SkillWithLinks[] = [
  {
    id: "frontend-design",
    name: "frontend-design",
    description: "Build distinctive, production-grade frontend interfaces",
    file_path: "~/.skillsmanage/skills/frontend-design/SKILL.md",
    canonical_path: "~/.skillsmanage/skills/frontend-design",
    is_central: true,
    scanned_at: "2026-04-09T00:00:00Z",
    created_at: "2026-04-10T00:00:00Z",
    updated_at: "2026-04-12T00:00:00Z",
    linked_agents: ["claude-code", "codex"],
    shared_root_agents: [],
    repository: {
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
    },
    tags: [
      {
        id: "frontend-visual-design",
        name: "前端与视觉设计",
        is_builtin: true,
        created_at: "2026-04-10T00:00:00Z",
        updated_at: "2026-04-10T00:00:00Z",
      },
    ],
    source_path: "skills/frontend-design",
    is_source_unknown: false,
  },
  {
    id: "code-reviewer",
    name: "code-reviewer",
    description: "Review code changes and identify high-confidence, actionable bugs",
    file_path: "~/.skillsmanage/skills/code-reviewer/SKILL.md",
    canonical_path: "~/.skillsmanage/skills/code-reviewer",
    is_central: true,
    scanned_at: "2026-04-09T00:00:00Z",
    created_at: "2026-04-08T00:00:00Z",
    updated_at: "2026-04-20T00:00:00Z",
    linked_agents: ["codex"],
    shared_root_agents: [],
    repository: {
      id: "local-unknown",
      name: "本地 / 未知来源",
      source_type: "local",
      is_unknown: true,
      created_at: "2026-04-10T00:00:00Z",
      updated_at: "2026-04-10T00:00:00Z",
    },
    tags: [],
    is_source_unknown: true,
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
    id: "uncategorized",
    name: "未分类",
    is_builtin: true,
    created_at: "2026-04-10T00:00:00Z",
    updated_at: "2026-04-10T00:00:00Z",
  },
];

const mockDeletePreview: SkillDetail = {
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
  installations: [
    {
      skill_id: "frontend-design",
      agent_id: "cursor",
      installed_path: "/Users/test/.cursor/skills/frontend-design",
      link_type: "copy",
      symlink_target: undefined,
      installed_at: "2026-04-11T00:00:00Z",
    },
    {
      skill_id: "frontend-design",
      agent_id: "claude-code",
      installed_path: "/Users/test/.claude/skills/frontend-design",
      link_type: "symlink",
      symlink_target: "/Users/test/.skillsmanage/skills/frontend-design",
      installed_at: "2026-04-10T00:00:00Z",
    },
  ],
  collections: [],
  repository: mockRepositories[1],
  tags: mockTags,
  is_source_unknown: false,
};

const mockLoadCentralSkills = vi.fn();
const mockInstallSkill = vi.fn();
const mockLoadDeletePreview = vi.fn();
const mockDeleteCentralSkill = vi.fn();
const mockTogglePlatformLink = vi.fn();
const mockCreateRepository = vi.fn();
const mockAssignSkillsToRepository = vi.fn();
const mockCreateTag = vi.fn();
const mockAssignSkillTags = vi.fn();
const mockBulkSuggestSkillTags = vi.fn();
const mockCancelAiTagJob = vi.fn();
const mockAcceptAiTagReview = vi.fn();
const mockSkipAiTagReview = vi.fn();
const mockSubscribeAiTagProgress = vi.fn();
const mockRescan = vi.fn();
const mockGetSkillsByAgent = vi.fn();
const mockPreviewGitHubRepoImport = vi.fn();
const mockImportGitHubRepoSkills = vi.fn();
const mockResetGitHubImport = vi.fn();
const mockUseCentralSkillsStore = vi.mocked(useCentralSkillsStore);
const mockUsePlatformStore = vi.mocked(usePlatformStore);
const mockUseSkillStore = vi.mocked(useSkillStore);
const mockUseMarketplaceStore = vi.mocked(useMarketplaceStore);

function buildCentralStoreState(overrides = {}) {
  return {
    skills: mockSkills,
    agents: mockAgents,
    repositories: mockRepositories,
    tags: mockTags,
    aiTagReviews: [],
    aiTagJob: {
      jobId: null,
      status: "idle",
      total: 0,
      completed: 0,
      succeeded: 0,
      failed: 0,
      lowConfidenceCount: 0,
      items: {},
    },
    aiTaggingAvailable: true,
    isLoading: false,
    isInstalling: false,
    isDeleting: false,
    isMetadataUpdating: false,
    isSuggestingTags: false,
    togglingAgentId: null,
    error: null,
    loadCentralSkills: mockLoadCentralSkills,
    installSkill: mockInstallSkill,
    loadDeletePreview: mockLoadDeletePreview,
    deleteCentralSkill: mockDeleteCentralSkill,
    togglePlatformLink: mockTogglePlatformLink,
    createRepository: mockCreateRepository,
    assignSkillsToRepository: mockAssignSkillsToRepository,
    createTag: mockCreateTag,
    assignSkillTags: mockAssignSkillTags,
    bulkSuggestSkillTags: mockBulkSuggestSkillTags,
    cancelAiTagJob: mockCancelAiTagJob,
    acceptAiTagReview: mockAcceptAiTagReview,
    skipAiTagReview: mockSkipAiTagReview,
    subscribeAiTagProgress: mockSubscribeAiTagProgress.mockResolvedValue(() => {}),
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
    refreshCounts: mockRescan,
    applyScanSummary: vi.fn(),
    setCollectionCount: vi.fn(),
    setDiscoveredCount: vi.fn(),
    ...overrides,
  };
}

function buildSkillStoreState(overrides = {}) {
  return {
    skillsByAgent: {},
    loadingByAgent: {},
    error: null,
    getSkillsByAgent: mockGetSkillsByAgent,
    ...overrides,
  };
}

function renderCentralSkillsView() {
  mockUseCentralSkillsStore.mockImplementation((selector?: unknown) => {
    const state = buildCentralStoreState();
    if (typeof selector === "function") return selector(state);
    return state;
  });
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
  mockUseMarketplaceStore.mockImplementation((selector?: unknown) => {
    const state = {
      githubImport: {
        isPreviewLoading: false,
        isImporting: false,
        preview: null,
        importResult: null,
        previewedRepoUrl: null,
        error: null,
      },
      previewGitHubRepoImport: mockPreviewGitHubRepoImport,
      importGitHubRepoSkills: mockImportGitHubRepoSkills,
      resetGitHubImport: mockResetGitHubImport,
    };
    if (typeof selector === "function") return selector(state);
    return state;
  });

  return render(
    <MemoryRouter>
      <CentralSkillsView />
    </MemoryRouter>
  );
}

// ─── Tests ────────────────────────────────────────────────────────────────────

describe("CentralSkillsView", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  // ── Header ────────────────────────────────────────────────────────────────

  it("shows page title in header", () => {
    renderCentralSkillsView();
    expect(screen.getByText("中央技能库")).toBeInTheDocument();
  });

  it("shows the central skills directory path", () => {
    renderCentralSkillsView();
    expect(screen.getByText("/Users/test/.skillsmanage/skills/")).toBeInTheDocument();
  });

  it("shows a refresh button", () => {
    renderCentralSkillsView();
    expect(
      screen.getByRole("button", { name: /刷新中央技能库/i })
    ).toBeInTheDocument();
  });

  it("shows the shared github import launcher", () => {
    renderCentralSkillsView();
    expect(screen.getByRole("button", { name: /从 GitHub 导入/i })).toBeInTheDocument();
  });

  it("shows a search input", () => {
    renderCentralSkillsView();
    expect(
      screen.getByPlaceholderText(/搜索中央技能库/i)
    ).toBeInTheDocument();
  });

  it("shows explicit sort field and direction controls", () => {
    renderCentralSkillsView();

    expect(screen.getByRole("group", { name: "排序字段" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "名称" })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("button", { name: "创建时间" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "修改时间" })).toBeInTheDocument();

    expect(screen.getByRole("group", { name: "排序方向" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "正排" })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("button", { name: "倒排" })).toBeInTheDocument();
  });

  it("shows repository and tag workspace controls", () => {
    renderCentralSkillsView();

    expect(screen.getByText("仓库来源")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /未归仓/i })).toBeInTheDocument();
    expect(screen.getByRole("combobox", { name: "分类筛选" })).toBeInTheDocument();
    expect(screen.getAllByText("openai/skills").length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText("前端与视觉设计").length).toBeGreaterThanOrEqual(1);
  });

  // ── Skills List ───────────────────────────────────────────────────────────

  it("renders all central skills", () => {
    renderCentralSkillsView();
    expect(screen.getByText("frontend-design")).toBeInTheDocument();
    expect(screen.getByText("code-reviewer")).toBeInTheDocument();
  });

  it("sorts by modified time and reverses direction explicitly", async () => {
    renderCentralSkillsView();

    fireEvent.click(screen.getByRole("button", { name: "修改时间" }));

    await waitFor(() => {
      const detailButtons = screen.getAllByRole("button", {
        name: /查看 .* 的详情/i,
      });
      expect(detailButtons[0]).toHaveTextContent("frontend-design");
      expect(detailButtons[1]).toHaveTextContent("code-reviewer");
    });

    fireEvent.click(screen.getByRole("button", { name: "倒排" }));

    await waitFor(() => {
      const detailButtons = screen.getAllByRole("button", {
        name: /查看 .* 的详情/i,
      });
      expect(detailButtons[0]).toHaveTextContent("code-reviewer");
      expect(detailButtons[1]).toHaveTextContent("frontend-design");
    });
  });

  it("renders skill descriptions", () => {
    renderCentralSkillsView();
    expect(
      screen.getByText(/Build distinctive, production-grade frontend interfaces/)
    ).toBeInTheDocument();
  });

  it("shows Install to... button for each skill", () => {
    renderCentralSkillsView();
    const installButtons = screen.getAllByRole("button", {
      name: /将 .* 安装到平台/i,
    });
    expect(installButtons).toHaveLength(2);
  });

  it("shows delete button for each central skill", () => {
    renderCentralSkillsView();

    expect(screen.getByTestId("delete-central-skill-frontend-design")).toBeInTheDocument();
    expect(screen.getByTestId("delete-central-skill-code-reviewer")).toBeInTheDocument();
  });

  it("opens delete dialog and deletes selected platform copies", async () => {
    mockLoadDeletePreview.mockResolvedValueOnce(mockDeletePreview);
    mockDeleteCentralSkill.mockResolvedValueOnce(undefined);
    renderCentralSkillsView();

    fireEvent.click(screen.getByTestId("delete-central-skill-frontend-design"));

    await waitFor(() => {
      expect(mockLoadDeletePreview).toHaveBeenCalledWith("frontend-design");
    });
    expect(screen.getByText("/Users/test/.cursor/skills/frontend-design")).toBeInTheDocument();
    expect(screen.getByText(/Claude Code/)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("checkbox", { name: /Cursor/i }));
    fireEvent.click(screen.getByRole("button", { name: /\u5220\u9664\u4e2d\u592e\u6280\u80fd/i }));

    await waitFor(() => {
      expect(mockDeleteCentralSkill).toHaveBeenCalledWith("frontend-design", ["cursor"]);
    });
    expect(mockRescan).toHaveBeenCalled();
  });

  it("renders browser fixture skill card on the localhost validation surface without Tauri", async () => {
    const isTauriSpy = vi.spyOn(tauriBridge, "isTauriRuntime").mockReturnValue(false);
    mockUseCentralSkillsStore.mockRestore();
    mockUsePlatformStore.mockRestore();

    render(
      <MemoryRouter>
        <CentralSkillsView />
      </MemoryRouter>
    );

    expect(await screen.findByRole("button", { name: /查看 fixture-central-skill 的详情/i })).toBeInTheDocument();

    isTauriSpy.mockRestore();
  });

  it("skill name is a clickable button for detail navigation", () => {
    renderCentralSkillsView();
    // The skill name itself is the detail link (no separate [详情] button).
    const detailBtns = screen.getAllByRole("button", {
      name: /查看 frontend-design 的详情/i,
    });
    expect(detailBtns.length).toBeGreaterThanOrEqual(1);
  });

  // ── Per-platform link status ──────────────────────────────────────────────

  it("shows platform toggle icons for each non-central agent", () => {
    renderCentralSkillsView();
    const toggleButtons = screen.getAllByRole("button", {
      name: /切换 .* 的链接状态/i,
    });
    expect(toggleButtons.length).toBe(4);
  });

  // ── Empty State ───────────────────────────────────────────────────────────

  it("renders Universal platform icon as toggleable", () => {
    renderCentralSkillsView();

    const codexButton = screen.getAllByRole("button", {
      name: /切换 frontend-design 在 Universal 的链接状态/i,
    })[0];
    expect(codexButton).not.toBeDisabled();

    fireEvent.click(codexButton);
    expect(mockTogglePlatformLink).toHaveBeenCalledWith("frontend-design", "codex");
  });

  it("shows first-visit empty state when no skills exist", () => {
    mockUseCentralSkillsStore.mockImplementation((selector?: unknown) => {
      const state = buildCentralStoreState({ skills: [] });
      if (typeof selector === "function") return selector(state);
      return state;
    });

    render(
      <MemoryRouter>
        <CentralSkillsView />
      </MemoryRouter>
    );

    expect(
      screen.getByText(/欢迎使用 SkillPort/)
    ).toBeInTheDocument();
    // Should show guidance about creating a skill
    expect(
      screen.getAllByText(/skillsmanage\/skills/).length
    ).toBeGreaterThanOrEqual(1);
  });

  it("shows loading state", () => {
    mockUseCentralSkillsStore.mockImplementation((selector?: unknown) => {
      const state = buildCentralStoreState({ isLoading: true, skills: [] });
      if (typeof selector === "function") return selector(state);
      return state;
    });

    render(
      <MemoryRouter>
        <CentralSkillsView />
      </MemoryRouter>
    );

    expect(screen.getByText("正在加载技能...")).toBeInTheDocument();
  });

  // ── Search / Filter ───────────────────────────────────────────────────────

  it("filters skills by name when searching", async () => {
    renderCentralSkillsView();
    const searchInput = screen.getByPlaceholderText(/搜索中央技能库/i);
    fireEvent.change(searchInput, { target: { value: "frontend" } });

    await waitFor(() => {
      expect(screen.getByText("frontend-design")).toBeInTheDocument();
      expect(screen.queryByText("code-reviewer")).not.toBeInTheDocument();
    });
  });

  it("filters skills by description when searching", async () => {
    renderCentralSkillsView();
    const searchInput = screen.getByPlaceholderText(/搜索中央技能库/i);
    fireEvent.change(searchInput, { target: { value: "actionable" } });

    await waitFor(() => {
      expect(screen.getByText("code-reviewer")).toBeInTheDocument();
      expect(screen.queryByText("frontend-design")).not.toBeInTheDocument();
    });
  });

  it("filters skills by repository shortcut", async () => {
    renderCentralSkillsView();

    fireEvent.click(screen.getByRole("button", { name: /未归仓/i }));

    await waitFor(() => {
      expect(screen.getByText("code-reviewer")).toBeInTheDocument();
      expect(screen.queryByText("frontend-design")).not.toBeInTheDocument();
    });
  });

  it("filters skills by tag", async () => {
    renderCentralSkillsView();

    fireEvent.change(screen.getByRole("combobox", { name: "分类筛选" }), {
      target: { value: "frontend-visual-design" },
    });

    await waitFor(() => {
      expect(screen.getByText("frontend-design")).toBeInTheDocument();
      expect(screen.queryByText("code-reviewer")).not.toBeInTheDocument();
    });
  });

  it("shows empty state when search has no results", async () => {
    renderCentralSkillsView();
    const searchInput = screen.getByPlaceholderText(/搜索中央技能库/i);
    fireEvent.change(searchInput, { target: { value: "zzz-nonexistent" } });

    await waitFor(() => {
      expect(screen.getByText(/没有匹配的技能/)).toBeInTheDocument();
    });
  });

  it("restores all skills when search is cleared", async () => {
    renderCentralSkillsView();
    const searchInput = screen.getByPlaceholderText(/搜索中央技能库/i);
    fireEvent.change(searchInput, { target: { value: "frontend" } });
    fireEvent.change(searchInput, { target: { value: "" } });

    await waitFor(() => {
      expect(screen.getByText("frontend-design")).toBeInTheDocument();
      expect(screen.getByText("code-reviewer")).toBeInTheDocument();
    });
  });

  // ── Load on Mount ─────────────────────────────────────────────────────────

  it("calls loadCentralSkills on mount", () => {
    renderCentralSkillsView();
    expect(mockLoadCentralSkills).toHaveBeenCalledTimes(1);
  });

  // ── Refresh Button ────────────────────────────────────────────────────────

  it("calls rescan then loadCentralSkills when refresh button is clicked", async () => {
    renderCentralSkillsView();
    const refreshBtn = screen.getByRole("button", {
      name: /刷新中央技能库/i,
    });
    fireEvent.click(refreshBtn);

    await waitFor(() => {
      // rescan is called once (only on refresh, not on mount)
      expect(mockRescan).toHaveBeenCalledTimes(1);
      // loadCentralSkills is called twice: once on mount, once on refresh
      expect(mockLoadCentralSkills).toHaveBeenCalledTimes(2);
    });
  });

  // ── Install Dialog ────────────────────────────────────────────────────────

  it("opens install dialog when 'Install to...' is clicked", async () => {
    renderCentralSkillsView();
    const installBtn = screen.getAllByRole("button", {
      name: /将 .* 安装到平台/i,
    })[0];
    fireEvent.click(installBtn);

    // Dialog should open (skill name should appear in dialog title)
    await waitFor(() => {
      expect(screen.getByRole("dialog")).toBeInTheDocument();
    });
  });

  it("assigns selected skills to multiple tags from the categorize panel", async () => {
    mockAssignSkillTags.mockResolvedValue(undefined);
    renderCentralSkillsView();

    fireEvent.click(screen.getByRole("button", { name: /全选当前筛选/i }));
    fireEvent.click(screen.getByRole("button", { name: "前端与视觉设计" }));
    const uncategorizedButtons = screen.getAllByRole("button", { name: "未分类" });
    fireEvent.click(uncategorizedButtons[uncategorizedButtons.length - 1]);
    fireEvent.click(screen.getByRole("button", { name: /应用手动标签/i }));

    await waitFor(() => {
      expect(mockAssignSkillTags).toHaveBeenCalledWith(
        ["code-reviewer", "frontend-design"],
        ["frontend-visual-design", "uncategorized"]
      );
    });
  });

  it("shows AI config hint when AI tagging is unavailable", () => {
    mockUseCentralSkillsStore.mockImplementation((selector?: unknown) => {
      const state = buildCentralStoreState({ aiTaggingAvailable: false });
      if (typeof selector === "function") return selector(state);
      return state;
    });

    render(
      <MemoryRouter>
        <CentralSkillsView />
      </MemoryRouter>
    );

    fireEvent.click(screen.getByRole("tab", { name: "AI" }));
    expect(screen.getByText(/配置 AI API Key 后可批量自动标注/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /AI 标注/i })).toBeDisabled();
  });

  it("shows AI tagging progress and review queue entry", () => {
    mockUseCentralSkillsStore.mockImplementation((selector?: unknown) => {
      const state = buildCentralStoreState({
        aiTagReviews: [
          {
            skill_id: "code-reviewer",
            skill_name: "code-reviewer",
            tag: mockTags[1],
            confidence: 0.42,
            reason: "不确定",
            suggested_at: "2026-04-24T00:00:00Z",
            updated_at: "2026-04-24T00:00:00Z",
          },
        ],
        aiTagJob: {
          jobId: "job-1",
          status: "completed",
          total: 2,
          completed: 2,
          succeeded: 1,
          failed: 1,
          lowConfidenceCount: 1,
          items: {
            "frontend-design": "succeeded",
            "code-reviewer": "failed",
          },
        },
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });

    render(
      <MemoryRouter>
        <CentralSkillsView />
      </MemoryRouter>
    );

    expect(screen.getByText("AI 标注进度")).toBeInTheDocument();
    expect(screen.getByText("成功 1")).toBeInTheDocument();
    expect(screen.getByText("失败 1")).toBeInTheDocument();
    expect(screen.getAllByText("AI 待复核").length).toBeGreaterThanOrEqual(1);
  });

  it("shows a cancel button while AI tagging is running", async () => {
    mockCancelAiTagJob.mockResolvedValue(undefined);
    mockUseCentralSkillsStore.mockImplementation((selector?: unknown) => {
      const state = buildCentralStoreState({
        aiTagJob: {
          jobId: "job-1",
          status: "running",
          total: 2,
          completed: 1,
          succeeded: 1,
          failed: 0,
          lowConfidenceCount: 0,
          currentSkillName: "code-reviewer",
          items: {
            "frontend-design": "succeeded",
            "code-reviewer": "running",
          },
        },
        isSuggestingTags: true,
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });

    render(
      <MemoryRouter>
        <CentralSkillsView />
      </MemoryRouter>
    );

    fireEvent.click(screen.getByRole("button", { name: /中断 AI Tag/i }));

    await waitFor(() => {
      expect(mockCancelAiTagJob).toHaveBeenCalled();
    });
  });

  it("opens the skill detail drawer without navigating away", async () => {
    renderCentralSkillsView();

    fireEvent.click(screen.getByRole("button", { name: /查看 frontend-design 的详情/i }));

    await waitFor(() => {
      expect(screen.getByTestId("skill-detail-drawer")).toBeInTheDocument();
    });
    expect(screen.getByText("drawer-skill:frontend-design")).toBeInTheDocument();
  });

  it("offers post-import platform installation for imported skills", async () => {
    mockUseMarketplaceStore.mockImplementation((selector?: unknown) => {
      const state = {
        githubImport: {
          isPreviewLoading: false,
          isImporting: false,
          preview: null,
          importResult: {
            repo: {
              owner: "dorukardahan",
              repo: "twitterapi-io-skill",
              branch: "main",
              normalizedUrl: "https://github.com/dorukardahan/twitterapi-io-skill",
            },
            importedSkills: [
              {
                sourcePath: "twitterapi-io-skill/SKILL.md",
                originalSkillId: "frontend-design",
                importedSkillId: "frontend-design",
                skillName: "frontend-design",
                targetDirectory: "/Users/test/.skillsmanage/skills/frontend-design",
                resolution: "overwrite",
              },
            ],
            skippedSkills: [],
          },
          previewedRepoUrl: "https://github.com/dorukardahan/twitterapi-io-skill",
          error: null,
        },
        previewGitHubRepoImport: mockPreviewGitHubRepoImport,
        importGitHubRepoSkills: mockImportGitHubRepoSkills,
        resetGitHubImport: mockResetGitHubImport,
      };
      if (typeof selector === "function") return selector(state);
      return state;
    });

    render(
      <MemoryRouter>
        <CentralSkillsView />
      </MemoryRouter>
    );

    fireEvent.click(screen.getByRole("button", { name: /从 GitHub 导入/i }));

    expect(await screen.findByRole("button", { name: /安装到平台/i })).toBeInTheDocument();
  });

  it("shows the redesigned github confirm summary in the shared wizard", async () => {
    mockUseMarketplaceStore.mockImplementation((selector?: unknown) => {
      const state = {
        githubImport: {
          isPreviewLoading: false,
          isImporting: false,
          preview: {
            repo: {
              owner: "anthropics",
              repo: "skills",
              branch: "main",
              normalizedUrl: "https://github.com/anthropics/skills",
            },
            skills: [
              {
                sourcePath: "skills/first/SKILL.md",
                skillId: "frontend-design",
                skillName: "frontend-design",
                description: "First imported skill",
                rootDirectory: "skills",
                skillDirectoryName: "first",
                downloadUrl: "https://example.com/first",
                conflict: {
                  existingSkillId: "frontend-design",
                  existingName: "frontend-design",
                  existingCanonicalPath: "/Users/test/.skillsmanage/skills/frontend-design",
                  proposedSkillId: "frontend-design",
                  proposedName: "frontend-design",
                },
              },
            ],
            importResult: null,
            previewedRepoUrl: "https://github.com/anthropics/skills",
            error: null,
          },
          previewGitHubRepoImport: mockPreviewGitHubRepoImport,
          importGitHubRepoSkills: mockImportGitHubRepoSkills,
          resetGitHubImport: mockResetGitHubImport,
        },
        previewGitHubRepoImport: mockPreviewGitHubRepoImport,
        importGitHubRepoSkills: mockImportGitHubRepoSkills,
        resetGitHubImport: mockResetGitHubImport,
      };
      if (typeof selector === "function") return selector(state);
      return state;
    });

    render(
      <MemoryRouter>
        <CentralSkillsView />
      </MemoryRouter>
    );

    fireEvent.click(screen.getByRole("button", { name: /从 GitHub 导入/i }));
    fireEvent.click(screen.getByRole("button", { name: /检查导入内容/i }));

    expect(await screen.findByTestId("github-import-confirm-summary")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /返回预览修改/i })).toBeInTheDocument();
  });

  it("preserves search and scroll state when closing the drawer and restores focus", async () => {
    renderCentralSkillsView();

    const searchInput = screen.getByPlaceholderText(/搜索中央技能库/i);
    fireEvent.change(searchInput, { target: { value: "frontend" } });

    const scroller = searchInput.closest(".flex.flex-col.h-full")?.querySelector(".flex-1.overflow-auto.p-6");
    expect(scroller).not.toBeNull();
    if (!scroller) return;
    (scroller as HTMLDivElement).scrollTop = 240;

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
    expect((scroller as HTMLDivElement).scrollTop).toBe(240);
    expect(trigger).toHaveFocus();
  });
});
