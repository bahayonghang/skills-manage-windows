import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { MemoryRouter, useLocation } from "react-router-dom";

import i18n from "@/i18n";
import { DashboardView } from "@/pages/DashboardView";
import { useCentralSkillsStore } from "@/stores/centralSkillsStore";
import { useCollectionStore } from "@/stores/collectionStore";
import { useMarketplaceStore } from "@/stores/marketplaceStore";
import { useOperationLogStore } from "@/stores/operationLogStore";
import { usePlatformStore } from "@/stores/platformStore";
import { useTargetStore } from "@/stores/targetStore";
import type {
  AgentWithStatus,
  CentralSkillUpdateState,
  Collection,
  OperationLogEntry,
  SkillRegistry,
  SkillRepositoryWithStats,
  SkillTag,
  SkillWithLinks,
  TargetSummary,
} from "@/types";

vi.mock("@/stores/centralSkillsStore", () => ({
  useCentralSkillsStore: vi.fn(),
}));

vi.mock("@/stores/collectionStore", () => ({
  useCollectionStore: vi.fn(),
}));

vi.mock("@/stores/marketplaceStore", () => ({
  useMarketplaceStore: vi.fn(),
}));

vi.mock("@/stores/operationLogStore", () => ({
  useOperationLogStore: vi.fn(),
}));

vi.mock("@/stores/platformStore", () => ({
  usePlatformStore: vi.fn(),
}));

vi.mock("@/stores/targetStore", () => ({
  useTargetStore: vi.fn(),
}));

const mockUseCentralSkillsStore = vi.mocked(useCentralSkillsStore);
const mockUseCollectionStore = vi.mocked(useCollectionStore);
const mockUseMarketplaceStore = vi.mocked(useMarketplaceStore);
const mockUseOperationLogStore = vi.mocked(useOperationLogStore);
const mockUsePlatformStore = vi.mocked(usePlatformStore);
const mockUseTargetStore = vi.mocked(useTargetStore);

const mockLoadCentralSkills = vi.fn();
const mockSubscribeAiTagProgress = vi.fn();
const mockSubscribeUpdateProgress = vi.fn();
const mockLoadCollections = vi.fn();
const mockLoadRegistries = vi.fn();
const mockSyncRegistry = vi.fn();
const mockLoadLogs = vi.fn();

const agents: AgentWithStatus[] = [
  {
    id: "central",
    display_name: "Central Skills",
    category: "central",
    global_skills_dir: "~/.skillsmanage/skills/",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
  {
    id: "codex",
    display_name: "Codex CLI",
    category: "coding",
    global_skills_dir: "~/.agents/skills/",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
  {
    id: "claude-code",
    display_name: "Claude Code",
    category: "coding",
    global_skills_dir: "~/.claude/skills/",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
];

const repositories: SkillRepositoryWithStats[] = [
  {
    id: "repo-known",
    name: "openai/skills",
    source_type: "github",
    owner: "openai",
    repo: "skills",
    pinned: false,
    is_unknown: false,
    created_at: "2026-04-10T00:00:00Z",
    updated_at: "2026-04-10T00:00:00Z",
    skill_count: 2,
    unknown_skill_count: 0,
  },
  {
    id: "local-unknown",
    name: "Local / Unknown",
    source_type: "local",
    pinned: false,
    is_unknown: true,
    created_at: "2026-04-10T00:00:00Z",
    updated_at: "2026-04-10T00:00:00Z",
    skill_count: 1,
    unknown_skill_count: 1,
  },
];

const tags: SkillTag[] = [
  {
    id: "frontend",
    name: "Frontend",
    is_builtin: true,
    created_at: "2026-04-10T00:00:00Z",
    updated_at: "2026-04-10T00:00:00Z",
  },
  {
    id: "uncategorized",
    name: "Uncategorized",
    is_builtin: true,
    created_at: "2026-04-10T00:00:00Z",
    updated_at: "2026-04-10T00:00:00Z",
  },
];

const skills: SkillWithLinks[] = [
  {
    id: "frontend-design",
    name: "frontend-design",
    description: "Build frontend interfaces",
    file_path: "~/.skillsmanage/skills/frontend-design/SKILL.md",
    canonical_path: "~/.skillsmanage/skills/frontend-design",
    is_central: true,
    scanned_at: "2026-04-10T00:00:00Z",
    linked_agents: ["codex"],
    shared_root_agents: [],
    repository: repositories[0],
    tags: [tags[0]],
    is_source_unknown: false,
  },
  {
    id: "code-reviewer",
    name: "code-reviewer",
    description: "Review code",
    file_path: "~/.skillsmanage/skills/code-reviewer/SKILL.md",
    canonical_path: "~/.skillsmanage/skills/code-reviewer",
    is_central: true,
    scanned_at: "2026-04-10T00:00:00Z",
    linked_agents: [],
    shared_root_agents: [],
    repository: repositories[1],
    tags: [],
    is_source_unknown: true,
  },
  {
    id: "writer",
    name: "writer",
    description: "Write prose",
    file_path: "~/.skillsmanage/skills/writer/SKILL.md",
    canonical_path: "~/.skillsmanage/skills/writer",
    is_central: true,
    scanned_at: "2026-04-10T00:00:00Z",
    linked_agents: ["claude-code"],
    shared_root_agents: [],
    repository: repositories[0],
    tags: [tags[1]],
    is_source_unknown: false,
  },
];

const updateStatuses: Record<string, CentralSkillUpdateState> = {
  "frontend-design": {
    skill_id: "frontend-design",
    source_type: "github",
    status: "update_available",
  },
};

const collections: Collection[] = [
  {
    id: "frontend-kit",
    name: "Frontend Kit",
    created_at: "2026-04-10T00:00:00Z",
    updated_at: "2026-04-10T00:00:00Z",
  },
];

const registries: SkillRegistry[] = [
  {
    id: "official",
    name: "Official",
    source_type: "github",
    url: "https://github.com/openai/skills",
    is_builtin: true,
    is_enabled: true,
    last_synced: "2026-04-10T00:00:00Z",
    created_at: "2026-04-10T00:00:00Z",
  },
];

const logs: OperationLogEntry[] = [
  {
    id: "log-1",
    createdAt: "2026-04-27T10:00:00Z",
    level: "info",
    targetKind: "local",
    targetId: "local",
    targetLabel: "Local",
    category: "scan",
    action: "scan.all",
    status: "succeeded",
    summary: "Scanned 3 skills",
  },
];

const localTarget: TargetSummary = {
  id: "local",
  kind: "local",
  label: "Local",
  isActive: true,
};

let centralState: Record<string, unknown>;
let collectionState: Record<string, unknown>;
let marketplaceState: Record<string, unknown>;
let operationLogState: Record<string, unknown>;
let platformState: Record<string, unknown>;
let targetState: Record<string, unknown>;

function LocationProbe() {
  const location = useLocation();
  return <div data-testid="location">{location.pathname}</div>;
}

function renderDashboard() {
  render(
    <MemoryRouter initialEntries={["/dashboard"]}>
      <DashboardView />
      <LocationProbe />
    </MemoryRouter>
  );
}

function installStoreMocks() {
  mockUseCentralSkillsStore.mockImplementation((selector?: unknown) => {
    if (typeof selector === "function") return selector(centralState);
    return centralState;
  });
  mockUseCollectionStore.mockImplementation((selector?: unknown) => {
    if (typeof selector === "function") return selector(collectionState);
    return collectionState;
  });
  mockUseMarketplaceStore.mockImplementation((selector?: unknown) => {
    if (typeof selector === "function") return selector(marketplaceState);
    return marketplaceState;
  });
  mockUseOperationLogStore.mockImplementation((selector?: unknown) => {
    if (typeof selector === "function") return selector(operationLogState);
    return operationLogState;
  });
  mockUsePlatformStore.mockImplementation((selector?: unknown) => {
    if (typeof selector === "function") return selector(platformState);
    return platformState;
  });
  mockUseTargetStore.mockImplementation((selector?: unknown) => {
    if (typeof selector === "function") return selector(targetState);
    return targetState;
  });
}

describe("DashboardView", () => {
  beforeEach(async () => {
    vi.clearAllMocks();
    await i18n.changeLanguage("en");

    mockLoadCentralSkills.mockResolvedValue(undefined);
    mockSubscribeAiTagProgress.mockResolvedValue(() => {});
    mockSubscribeUpdateProgress.mockResolvedValue(() => {});
    mockLoadCollections.mockResolvedValue(undefined);
    mockLoadRegistries.mockResolvedValue(undefined);
    mockSyncRegistry.mockResolvedValue(undefined);
    mockLoadLogs.mockResolvedValue({
      entries: logs,
      total: logs.length,
      limit: 5,
      offset: 0,
    });

    centralState = {
      skills,
      agents,
      repositories,
      tags,
      aiTagReviews: [
        {
          skill_id: "writer",
          skill_name: "writer",
          tag: tags[0],
          confidence: 0.76,
          reason: "Looks like writing support",
          suggested_at: "2026-04-10T00:00:00Z",
          updated_at: "2026-04-10T00:00:00Z",
        },
      ],
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
      updateStatuses,
      updateJob: {
        phase: null,
        status: "idle",
        total: 0,
        completed: 0,
        succeeded: 0,
        failed: 0,
        skipped: 0,
        items: {},
      },
      isLoading: false,
      error: null,
      loadCentralSkills: mockLoadCentralSkills,
      subscribeAiTagProgress: mockSubscribeAiTagProgress,
      subscribeUpdateProgress: mockSubscribeUpdateProgress,
    };
    collectionState = {
      collections,
      isLoading: false,
      error: null,
      loadCollections: mockLoadCollections,
    };
    marketplaceState = {
      registries,
      isLoading: false,
      error: null,
      loadRegistries: mockLoadRegistries,
      syncRegistry: mockSyncRegistry,
    };
    operationLogState = {
      entries: logs,
      total: logs.length,
      isLoading: false,
      error: null,
      loadLogs: mockLoadLogs,
    };
    platformState = {
      agents,
      skillsByAgent: {
        central: 3,
        codex: 2,
        "claude-code": 1,
      },
      collectionCount: collections.length,
      dashboardCentralSummary: {
        centralSkillCount: 3,
        updatesAvailable: 1,
        aiReviewCount: 1,
        uncategorizedCount: 2,
        unassignedSourceCount: 1,
        sourceRepositories: repositories,
      },
      categoryVisibility: {
        coding: true,
        lobster: true,
      },
      lastScanAt: "2026-04-27T10:00:00Z",
      scanState: "idle",
      isLoading: false,
      isRefreshing: false,
    };
    targetState = {
      activeTarget: localTarget,
    };
    installStoreMocks();
  });

  it("renders dashboard summaries from existing stores", () => {
    renderDashboard();

    expect(screen.getByRole("heading", { name: /Dashboard/ })).toBeInTheDocument();
    expect(screen.getByTestId("dashboard-metric-central")).toHaveTextContent("3");
    expect(screen.getByTestId("dashboard-metric-collections")).toHaveTextContent("1");
    expect(screen.getByTestId("dashboard-metric-updates")).toHaveTextContent("1");
    expect(screen.getByTestId("dashboard-metric-ai")).toHaveTextContent("1");
    expect(screen.getByTestId("dashboard-metric-uncategorized")).toHaveTextContent("2");
    expect(screen.getByTestId("dashboard-metric-unassigned")).toHaveTextContent("1");
    expect(screen.getByTestId("dashboard-metric-targets")).toHaveTextContent("2");
    expect(screen.getByText(/Enabled platforms|启用平台/)).toBeInTheDocument();
    expect(screen.getByText("Scanned 3 skills")).toBeInTheDocument();
  });

  it("loads local metadata without syncing registries", async () => {
    centralState = {
      ...centralState,
      skills: [],
      repositories: [],
      tags: [],
      aiTagReviews: [],
      updateStatuses: {},
    };
    collectionState = { ...collectionState, collections: [] };
    marketplaceState = { ...marketplaceState, registries: [] };
    operationLogState = { ...operationLogState, entries: [], total: 0 };

    renderDashboard();

    await waitFor(() => {
      expect(mockLoadCollections).toHaveBeenCalledTimes(1);
      expect(mockLoadRegistries).toHaveBeenCalledTimes(1);
      expect(mockLoadLogs).toHaveBeenCalledWith({ limit: 5, offset: 0 });
    });
    expect(mockLoadCentralSkills).not.toHaveBeenCalled();
    expect(mockSubscribeAiTagProgress).toHaveBeenCalledTimes(1);
    expect(mockSubscribeUpdateProgress).toHaveBeenCalledTimes(1);
    expect(mockSyncRegistry).not.toHaveBeenCalled();
  });

  it("navigates through quick actions only", () => {
    renderDashboard();

    fireEvent.click(screen.getByTestId("dashboard-action-marketplace"));

    expect(screen.getByTestId("location")).toHaveTextContent("/marketplace");
    expect(mockSyncRegistry).not.toHaveBeenCalled();
  });

  it("renders empty and error states without crashing", () => {
    centralState = {
      ...centralState,
      skills: [],
      repositories: [],
      tags: [],
      aiTagReviews: [],
      updateStatuses: {},
      error: "central offline",
    };
    operationLogState = { ...operationLogState, entries: [], total: 0 };
    platformState = {
      ...platformState,
      dashboardCentralSummary: {
        centralSkillCount: 0,
        updatesAvailable: 0,
        aiReviewCount: 0,
        uncategorizedCount: 0,
        unassignedSourceCount: 0,
        sourceRepositories: [],
      },
    };

    renderDashboard();

    expect(
      screen.getByText(/Some dashboard data could not be loaded|部分 Dashboard 数据加载失败/)
    ).toHaveAttribute("title", "central offline");
    expect(screen.getByText(/No recent operation logs|暂无最近操作日志/)).toBeInTheDocument();
    expect(screen.getByText(/No Central tags yet|暂无 Central 标签/)).toBeInTheDocument();
  });
});
