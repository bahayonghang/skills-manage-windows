/* eslint-disable react-refresh/only-export-components */
import { vi } from "vitest";
import { render, screen, fireEvent, waitFor, cleanup, within } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { CentralSkillsView as CentralSkillsViewComponent } from "../pages/CentralSkillsView";
import type {
  AgentWithStatus,
  SkillDetail,
  SkillRepositoryWithStats,
  SkillTag,
  SkillWithLinks,
  TargetSummary,
} from "../types";

const mockedToast = vi.hoisted(() => ({
  success: vi.fn(),
  error: vi.fn(),
  info: vi.fn(),
}));

const mockedUpdateCenter = vi.hoisted(() => {
  const emptyInventory = {
    updatable: [],
    remoteAdded: [],
    remoteMissing: [],
    failedRepositories: [],
    platformDuplicates: [],
    deletedPlatformCopies: [],
    orphans: [],
    generatedAt: "2026-05-30T00:00:00Z",
  };
  const refresh = vi.fn().mockResolvedValue(emptyInventory);
  const openDialog = vi.fn();
  const setRefreshMode = vi.fn();
  return {
    emptyInventory,
    refresh,
    openDialog,
    closeDialog: vi.fn(),
    setActiveTab: vi.fn(),
    state: {
      inventory: null,
      isRefreshing: false,
      isApplying: false,
      lastRefreshedAt: null,
      isDialogOpen: false,
      activeTab: "updatable",
      refreshContext: { repositoryIds: [], skillIds: [], agentIds: [] },
      refreshMode: "sync",
      error: null,
      refresh,
      apply: vi.fn(),
      clear: vi.fn(),
      loadInventory: vi.fn(),
      scanDuplicates: vi.fn(),
      scanDeletedPlatformCopies: vi.fn(),
      openDialog,
      closeDialog: vi.fn(),
      setActiveTab: vi.fn(),
      setRefreshMode,
    },
  };
});

// Mock stores
vi.mock("@/stores/centralSkillsStore", () => ({
  useCentralSkillsStore: vi.fn(),
}));

vi.mock("@/stores/platformStore", () => ({
  usePlatformStore: vi.fn(),
}));

vi.mock("@/stores/skillStore", () => ({
  useSkillStore: vi.fn(),
}));

vi.mock("@/stores/marketplaceStore", () => ({
  useMarketplaceStore: vi.fn(),
}));

vi.mock("@/stores/skillDetailStore", () => ({
  useSkillDetailStore: vi.fn(),
}));

vi.mock("@/stores/updateCenterStore", () => ({
  useUpdateCenterStore: vi.fn((selector?: unknown) => {
    if (typeof selector === "function") {
      return selector(mockedUpdateCenter.state);
    }
    return mockedUpdateCenter.state;
  }),
  selectSkillInventoryFlags: vi.fn(() => ({
    hasUpdate: false,
    isMissing: false,
    isAdded: false,
    hasDuplicate: false,
    isOrphan: false,
  })),
  selectSkillInventoryFlagsFromInventory: vi.fn(() => ({
    hasUpdate: false,
    isMissing: false,
    isAdded: false,
    hasDuplicate: false,
    isOrphan: false,
  })),
}));

vi.mock("@/components/marketplace/GitHubRepoImportWizard", async () => {
  const React = await vi.importActual<typeof import("react")>("react");

  return {
    GitHubRepoImportWizard: ({
      open,
      preview,
      importResult,
      onImport,
      onOpenChange,
    }: {
      open: boolean;
      preview: {
        skills?: Array<{
          sourcePath: string;
          skillId: string;
          conflict?: unknown;
        }>;
      } | null;
      importResult: unknown;
      onImport: (selections: Array<{
        sourcePath: string;
        resolution: "skip" | "overwrite";
        renamedSkillId: string | null;
      }>) => Promise<unknown> | unknown;
      onOpenChange: (open: boolean) => void;
    }) => {
      const [step, setStep] = React.useState<"input" | "preview" | "confirm" | "result">(
        importResult ? "result" : preview ? "preview" : "input"
      );
      const [localResult, setLocalResult] = React.useState(importResult);
      const lastPreviewRef = React.useRef(preview);

      React.useEffect(() => {
        const previewChanged = lastPreviewRef.current !== preview;
        lastPreviewRef.current = preview;

        if (!open) {
          setStep("input");
          setLocalResult(null);
          return;
        }

        if (importResult) {
          setLocalResult(importResult);
          setStep("result");
          return;
        }

        if (preview) {
          setStep((current) =>
            current === "input" || previewChanged ? "preview" : current
          );
          return;
        }

        setLocalResult(null);
        setStep("input");
      }, [importResult, open, preview]);

      if (!open) return null;

      const selections =
        preview?.skills?.map((skill) => ({
          sourcePath: skill.sourcePath,
          resolution: skill.conflict ? ("skip" as const) : ("overwrite" as const),
          renamedSkillId: skill.skillId ?? null,
        })) ?? [];

      const result = localResult ?? importResult;

      return (
        <div role="dialog" aria-label="GitHub import wizard">
          {step === "preview" ? (
            <>
              <div>GitHub import preview</div>
              <button type="button" onClick={() => setStep("confirm")}>
                检查导入内容
              </button>
            </>
          ) : null}

          {step === "confirm" ? (
            <>
              <div data-testid="github-import-confirm-summary">
                selected:{selections.length}
              </div>
              <button type="button" onClick={() => setStep("preview")}>
                返回预览修改
              </button>
              <button
                type="button"
                onClick={() => {
                  void Promise.resolve(onImport(selections))
                    .then((nextResult) => {
                      if (nextResult) {
                        setLocalResult(nextResult);
                        setStep("result");
                      }
                    })
                    .catch(() => {});
                }}
              >
                导入
              </button>
            </>
          ) : null}

          {step === "result" && result ? (
            <>
              <div>GitHub import result</div>
              <button type="button">安装到平台</button>
            </>
          ) : null}

          <button type="button" onClick={() => onOpenChange(false)}>
            关闭
          </button>
        </div>
      );
    },
  };
});

vi.mock("sonner", () => ({
  toast: mockedToast,
}));

vi.mock("@/components/skill/SkillDetailDrawer", () => ({
  SkillDetailDrawer: ({
    open,
    skillId,
    onOpenChange,
    returnFocusRef,
    onInstallClick,
  }: {
    open: boolean;
    skillId: string | null;
    onOpenChange: (open: boolean) => void;
    returnFocusRef?: { current: HTMLElement | null };
    onInstallClick?: (skillId: string) => void;
  }) =>
    open ? (
      <div data-testid="skill-detail-drawer">
        <div>drawer-skill:{skillId}</div>
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

import { useCentralSkillsStore } from "@/stores/centralSkillsStore";
import { usePlatformStore } from "@/stores/platformStore";
import { useSkillStore } from "@/stores/skillStore";
import { useMarketplaceStore } from "@/stores/marketplaceStore";
import { useSkillDetailStore } from "@/stores/skillDetailStore";
import { useTargetStore as targetStoreHook } from "@/stores/targetStore";
import {
  createSettingsStoreInitialState,
  useSettingsStore,
} from "@/stores/settingsStore";
import * as tauriBridgeModule from "@/lib/ipc";

// ─── Fixtures ─────────────────────────────────────────────────────────────────

export const ASYNC_UI_TIMEOUT_MS = 5_000;

export const mockAgents: AgentWithStatus[] = [
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

export const mockSkills: SkillWithLinks[] = [
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
      pinned: false,
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
      pinned: false,
      is_unknown: true,
      created_at: "2026-04-10T00:00:00Z",
      updated_at: "2026-04-10T00:00:00Z",
    },
    tags: [],
    is_source_unknown: true,
  },
];

export const mockRepositories: SkillRepositoryWithStats[] = [
  {
    id: "local-unknown",
    name: "本地 / 未知来源",
    source_type: "local",
    pinned: false,
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
    pinned: false,
    is_unknown: false,
    created_at: "2026-04-10T00:00:00Z",
    updated_at: "2026-04-10T00:00:00Z",
    skill_count: 1,
    unknown_skill_count: 0,
  },
];

export const mockTags: SkillTag[] = [
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

export const mockDeletePreview: SkillDetail = {
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

export const mockLoadCentralSkills = vi.fn();
export const mockInstallSkill = vi.fn();
export const mockBatchInstallSkills = vi.fn();
export const mockLoadDeletePreview = vi.fn();
export const mockLoadBatchDeletePreview = vi.fn();
export const mockLoadRepositoryDeletePreview = vi.fn();
export const mockDeleteCentralSkill = vi.fn();
export const mockDeleteCentralSkills = vi.fn();
export const mockDeleteSkillRepository = vi.fn();
export const mockTogglePlatformLink = vi.fn();
export const mockCreateRepository = vi.fn();
export const mockAssignSkillsToRepository = vi.fn();
export const mockCreateTag = vi.fn();
export const mockAssignSkillTags = vi.fn();
export const mockBulkSuggestSkillTags = vi.fn();
export const mockCheckSkillUpdates = vi.fn();
export const mockCheckRepositorySync = vi.fn();
export const mockApplyRepositorySync = vi.fn();
export const mockUpdateSkills = vi.fn();
export const mockKeepRemoteMissingSkills = vi.fn();
export const mockPreviewCentralStoreLocationChange = vi.fn();
export const mockApplyCentralStoreLocationChange = vi.fn();
export const mockCancelAiTagJob = vi.fn();
export const mockAcceptAiTagReview = vi.fn();
export const mockSkipAiTagReview = vi.fn();
export const mockSubscribeAiTagProgress = vi.fn();
export const mockSubscribeUpdateProgress = vi.fn();
export const mockRescan = vi.fn();
export const mockGetSkillsByAgent = vi.fn();
export const mockBatchUninstallSkillsFromAgent = vi.fn();
export const mockPreviewGitHubRepoImport = vi.fn();
export const mockImportGitHubRepoSkills = vi.fn();
export const mockResetGitHubImport = vi.fn();
export const mockExportSkillportState = vi.fn();
export const mockPreviewSkillportStateImport = vi.fn();
export const mockImportSkillportState = vi.fn();
export const mockRefreshInstallations = vi.fn();
export const mockUseCentralSkillsStore = vi.mocked(useCentralSkillsStore);
export const mockUsePlatformStore = vi.mocked(usePlatformStore);
export const mockUseSkillStore = vi.mocked(useSkillStore);
export const mockUseMarketplaceStore = vi.mocked(useMarketplaceStore);
export const mockUseSkillDetailStore = vi.mocked(useSkillDetailStore);
export const mockRefreshUpdateInventory = mockedUpdateCenter.refresh;
export const mockOpenUpdateCenterDialog = mockedUpdateCenter.openDialog;
export const mockEmptyUpdateInventory = mockedUpdateCenter.emptyInventory;
export const localTarget: TargetSummary = {
  id: "local",
  kind: "local",
  label: "Local",
  isActive: true,
};

export function buildCentralStoreState(overrides = {}) {
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
    updateStatuses: {},
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
    aiTaggingAvailable: true,
    isLoading: false,
    isInstalling: false,
    isDeleting: false,
    isMetadataUpdating: false,
    isSuggestingTags: false,
    isCheckingUpdates: false,
    updatingSkillIds: [],
    togglingAgentId: null,
    error: null,
    loadCentralSkills: mockLoadCentralSkills,
    installSkill: mockInstallSkill,
    batchInstallSkills: mockBatchInstallSkills,
    loadDeletePreview: mockLoadDeletePreview,
    loadBatchDeletePreview: mockLoadBatchDeletePreview,
    loadRepositoryDeletePreview: mockLoadRepositoryDeletePreview,
    deleteCentralSkill: mockDeleteCentralSkill,
    deleteCentralSkills: mockDeleteCentralSkills,
    deleteSkillRepository: mockDeleteSkillRepository,
    togglePlatformLink: mockTogglePlatformLink,
    createRepository: mockCreateRepository,
    assignSkillsToRepository: mockAssignSkillsToRepository,
    createTag: mockCreateTag,
    assignSkillTags: mockAssignSkillTags,
    bulkSuggestSkillTags: mockBulkSuggestSkillTags,
    checkSkillUpdates: mockCheckSkillUpdates,
    checkRepositorySync: mockCheckRepositorySync,
    applyRepositorySync: mockApplyRepositorySync,
    updateSkills: mockUpdateSkills,
    keepRemoteMissingSkills: mockKeepRemoteMissingSkills,
    previewCentralStoreLocationChange: mockPreviewCentralStoreLocationChange,
    applyCentralStoreLocationChange: mockApplyCentralStoreLocationChange,
    cancelAiTagJob: mockCancelAiTagJob,
    acceptAiTagReview: mockAcceptAiTagReview,
    skipAiTagReview: mockSkipAiTagReview,
    subscribeAiTagProgress: mockSubscribeAiTagProgress.mockResolvedValue(() => {}),
    subscribeUpdateProgress: mockSubscribeUpdateProgress.mockResolvedValue(() => {}),
    exportSkillportState: mockExportSkillportState,
    previewSkillportStateImport: mockPreviewSkillportStateImport,
    importSkillportState: mockImportSkillportState,
    ...overrides,
  };
}

export function buildPlatformStoreState(overrides = {}) {
  return {
    agents: mockAgents,
    platformPaths: {},
    skillsByAgent: {},
    collectionCount: 0,
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
    ...overrides,
  };
}

export function buildSkillStoreState(overrides = {}) {
  return {
    platformPaths: {},
    skillsByAgent: {},
    loadingByAgent: {},
    pendingSkillActionKeys: {},
    error: null,
    getSkillsByAgent: mockGetSkillsByAgent,
    batchUninstallSkillsFromAgent: mockBatchUninstallSkillsFromAgent,
    ...overrides,
  };
}

export function buildSkillDetailStoreState(overrides = {}) {
  return {
    detail: null,
    refreshInstallations: mockRefreshInstallations,
    ...overrides,
  };
}

export function buildMarketplaceStoreState(overrides = {}) {
  return {
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
    ...overrides,
  };
}

export function renderCentralSkillsView({
  centralOverrides = {},
  platformOverrides = {},
  skillOverrides = {},
  skillDetailOverrides = {},
  marketplaceOverrides = {},
}: {
  centralOverrides?: Record<string, unknown>;
  platformOverrides?: Record<string, unknown>;
  skillOverrides?: Record<string, unknown>;
  skillDetailOverrides?: Record<string, unknown>;
  marketplaceOverrides?: Record<string, unknown>;
} = {}) {
  const centralState = buildCentralStoreState(centralOverrides);
  const platformState = buildPlatformStoreState(platformOverrides);
  const skillState = buildSkillStoreState(skillOverrides);
  const marketplaceState = buildMarketplaceStoreState(marketplaceOverrides);
  const skillDetailState = buildSkillDetailStoreState(skillDetailOverrides);

  mockUseCentralSkillsStore.mockImplementation((selector?: unknown) => {
    if (typeof selector === "function") return selector(centralState);
    return centralState;
  });
  mockUsePlatformStore.mockImplementation((selector?: unknown) => {
    if (typeof selector === "function") return selector(platformState);
    return platformState;
  });
  mockUseSkillStore.mockImplementation((selector?: unknown) => {
    if (typeof selector === "function") return selector(skillState);
    return skillState;
  });
  mockUseMarketplaceStore.mockImplementation((selector?: unknown) => {
    if (typeof selector === "function") return selector(marketplaceState);
    return marketplaceState;
  });
  mockUseSkillDetailStore.mockImplementation((selector?: unknown) => {
    if (typeof selector === "function") return selector(skillDetailState);
    return skillDetailState;
  });

  return render(
    <MemoryRouter>
      <CentralSkillsViewComponent />
    </MemoryRouter>
  );
}

export { render, screen, fireEvent, waitFor, cleanup, within };
export { MemoryRouter };
export const toast = mockedToast;
export const CentralSkillsView = CentralSkillsViewComponent;
export const useTargetStore = targetStoreHook;
export const tauriBridge = tauriBridgeModule;
export const settingsStore = useSettingsStore;

export function resetCentralSkillsViewTestState() {
  vi.clearAllMocks();
  // 重置 jsdom URL：CentralSkillsView 用 useCentralViewStateUrl 把 viewState（搜索/筛选/排序）写到 location.search，
  // 测试间共享 jsdom 的 window.location 会导致前一个 case 的搜索文本污染下一个 case 的初始 state。
  if (typeof window !== "undefined" && window.history && typeof window.history.replaceState === "function") {
    window.history.replaceState({}, "", "/");
  }
  useTargetStore.setState({
    targets: [localTarget],
    activeTarget: localTarget,
  });
  useSettingsStore.setState({
    ...createSettingsStoreInitialState(),
    centralUpdateCheckModeLoaded: true,
  });
  mockCheckSkillUpdates.mockResolvedValue([]);
  mockCheckRepositorySync.mockResolvedValue({
    states: [],
    remoteAdded: [],
    skippedRemoteAdded: [],
    remoteMissing: [],
    repositories: [],
    failedRepositories: [],
  });
  mockApplyRepositorySync.mockResolvedValue({
    keptSkillIds: [],
    deleteResult: { succeeded: [], failed: [] },
    importResults: [],
    skippedAdditions: [],
    unskippedAdditions: [],
    failedRepositories: [],
    states: [],
  });
  mockUpdateSkills.mockResolvedValue({ succeeded: [], failed: [], skipped: [], states: [] });
  mockKeepRemoteMissingSkills.mockResolvedValue([]);
  mockBatchUninstallSkillsFromAgent.mockResolvedValue({ succeeded: [], failed: [] });
  mockRefreshUpdateInventory.mockResolvedValue(mockEmptyUpdateInventory);
  mockOpenUpdateCenterDialog.mockClear();
  mockPreviewCentralStoreLocationChange.mockResolvedValue({
    sourcePath: "/Users/test/.skillsmanage/skills",
    targetPath: "/Users/test/SkillPort/skills",
    skillsToCopy: 1,
    skillsToOverwrite: 1,
    targetOnlySkills: 1,
  });
  mockApplyCentralStoreLocationChange.mockResolvedValue({
    sourcePath: "/Users/test/.skillsmanage/skills",
    targetPath: "/Users/test/SkillPort/skills",
    copied: 1,
    overwritten: 1,
    targetOnlyImported: 1,
    symlinkRebuildFailed: 0,
    symlinkFailures: [],
    completedAt: "2026-05-24T00:00:00Z",
  });
  mockExportSkillportState.mockResolvedValue(
    JSON.stringify({
      kind: "skillport/state-export",
      version: 1,
      exportedAt: "2026-04-25T00:00:00Z",
      exportedFrom: { app: "SkillPort" },
      githubSources: [],
      centralSkills: [],
      unrestorableSkills: [],
    })
  );
}

export function cleanupCentralSkillsViewTestState() {
  cleanup();
  vi.restoreAllMocks();
}
