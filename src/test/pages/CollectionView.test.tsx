import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { useEffect } from "react";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import {
  MemoryRouter,
  Route,
  Routes,
  useLocation,
  type Location,
} from "react-router-dom";
import { CollectionView } from "@/pages/CollectionView";
import { CollectionDetail, AgentWithStatus } from "@/types";
import {
  consumeScrollPosition,
  saveScrollPosition,
} from "@/lib/scrollRestoration";

// Subscribes to location changes so tests can assert on navigation state
// without depending on `window.history.state`, which MemoryRouter does not
// update.
function LocationProbe({
  onChange,
}: {
  onChange: (location: Location) => void;
}) {
  const location = useLocation();
  useEffect(() => {
    onChange(location);
  }, [location, onChange]);
  return null;
}

// Mock stores
vi.mock("@/stores/collectionStore", () => ({
  useCollectionStore: vi.fn(),
}));

vi.mock("@/stores/platformStore", () => ({
  usePlatformStore: vi.fn(),
}));

import { useCollectionStore } from "@/stores/collectionStore";
import { usePlatformStore } from "@/stores/platformStore";

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

const mockCollectionDetail: CollectionDetail = {
  id: "col-1",
  name: "Frontend",
  description: "Frontend skills collection",
  created_at: "2026-04-09T00:00:00Z",
  updated_at: "2026-04-09T00:00:00Z",
  skills: [
    {
      id: "frontend-design",
      name: "frontend-design",
      description: "Build distinctive frontend UIs",
      file_path: "~/.skillsmanage/skills/frontend-design/SKILL.md",
      is_central: true,
      scanned_at: "2026-04-09T00:00:00Z",
    },
    {
      id: "code-reviewer",
      name: "code-reviewer",
      description: "Review code changes",
      file_path: "~/.skillsmanage/skills/code-reviewer/SKILL.md",
      is_central: true,
      scanned_at: "2026-04-09T00:00:00Z",
    },
  ],
};

const mockLoadCollectionDetail = vi.fn();
const mockRemoveSkillFromCollection = vi.fn();
const mockDeleteCollection = vi.fn();
const mockExportCollection = vi.fn();
const mockUseCollectionStore = vi.mocked(useCollectionStore);
const mockUsePlatformStore = vi.mocked(usePlatformStore);

function buildCollectionStoreState(overrides = {}) {
  return {
    collections: [],
    currentDetail: mockCollectionDetail,
    isLoading: false,
    isLoadingDetail: false,
    hasLoaded: true,
    detailTargetId: "col-1",
    error: null,
    loadCollections: vi.fn(),
    createCollection: vi.fn(),
    updateCollection: vi.fn(),
    deleteCollection: mockDeleteCollection,
    loadCollectionDetail: mockLoadCollectionDetail,
    addSkillToCollection: vi.fn(),
    removeSkillFromCollection: mockRemoveSkillFromCollection,
    batchInstallCollection: vi.fn(),
    exportCollection: mockExportCollection,
    importCollection: vi.fn(),
    refreshCounts: vi.fn(),
    ...overrides,
  };
}

function buildPlatformStoreState(overrides = {}) {
  return {
    agents: mockAgents,
    platformPaths: {},
    skillsByAgent: {},
    collectionCount: 0,
    categoryVisibility: {
      coding: true,
      lobster: true,
    },
    lastScanAt: "2026-04-23T01:00:00Z",
    scanState: "idle" as const,
    isLoading: false,
    isRefreshing: false,
    error: null,
    topTags: [],
    isTopTagsLoading: false,
    topTagsError: null,
    initialize: vi.fn(),
    hydrateShell: vi.fn(),
    refreshScanInBackground: vi.fn(),
    rescan: vi.fn(),
    refreshCounts: vi.fn(),
    refreshDashboardSummary: vi.fn(),
    loadTopTags: vi.fn(),
    resetForTargetChange: vi.fn(),
    setCategoryVisibility: vi.fn(),
    setAgentEnabled: vi.fn(),
    applyScanSummary: vi.fn(),
    setCollectionCount: vi.fn(),
    addCustomAgent: vi.fn(),
    updateCustomAgent: vi.fn(),
    removeCustomAgent: vi.fn(),
    ...overrides,
  };
}

function renderCollectionView(collectionId = "col-1", storeOverrides = {}) {
  mockUseCollectionStore.mockImplementation((selector) =>
    selector(buildCollectionStoreState(storeOverrides))
  );
  mockUsePlatformStore.mockImplementation((selector) =>
    selector(buildPlatformStoreState())
  );

  return render(
    <MemoryRouter initialEntries={[`/collection/${collectionId}`]}>
      <Routes>
        <Route path="/collection/:collectionId" element={<CollectionView />} />
        <Route path="/central" element={<div>Central Skills</div>} />
      </Routes>
    </MemoryRouter>
  );
}

// ─── Tests ────────────────────────────────────────────────────────────────────

describe("CollectionView", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  // ── Loading and Data Display ───────────────────────────────────────────────

  it("calls loadCollectionDetail on mount", () => {
    renderCollectionView("col-1");
    expect(mockLoadCollectionDetail).toHaveBeenCalledWith("col-1");
  });

  it("renders collection name and description", () => {
    renderCollectionView();
    expect(screen.getByText("Frontend")).toBeInTheDocument();
    expect(screen.getByText("Frontend skills collection")).toBeInTheDocument();
  });

  it("renders member skills list", () => {
    renderCollectionView();
    expect(screen.getByText("frontend-design")).toBeInTheDocument();
    expect(screen.getByText("code-reviewer")).toBeInTheDocument();
  });

  it("shows loading state when isLoadingDetail is true", () => {
    renderCollectionView("col-1", { isLoadingDetail: true, currentDetail: null });
    expect(screen.getByText(/正在加载技能集/i)).toBeInTheDocument();
  });

  it("shows empty skills state when collection has no skills", () => {
    renderCollectionView("col-1", {
      currentDetail: { ...mockCollectionDetail, skills: [] },
    });
    expect(screen.getByText(/此技能集还没有技能/i)).toBeInTheDocument();
  });

  // ── Remove Skill ───────────────────────────────────────────────────────────

  it("calls removeSkillFromCollection only after inline confirmation", async () => {
    mockRemoveSkillFromCollection.mockResolvedValueOnce(undefined);
    renderCollectionView();

    const removeButtons = screen.getAllByRole("button", { name: /从技能集中移除/i });
    fireEvent.click(removeButtons[0]);
    expect(mockRemoveSkillFromCollection).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: /确认删除/i }));

    await waitFor(() => {
      expect(mockRemoveSkillFromCollection).toHaveBeenCalledWith("col-1", "frontend-design");
    });
  });

  // ── Action Buttons ─────────────────────────────────────────────────────────

  it("renders Edit, Delete, Export, Add Skill, and Batch Install buttons", () => {
    renderCollectionView();
    expect(screen.getByRole("button", { name: /编辑技能集/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /删除技能集/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /导出技能集/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /添加技能到技能集/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /批量安装技能集/i })).toBeInTheDocument();
  });

  // ── Export ────────────────────────────────────────────────────────────────

  it("calls exportCollection when Export button is clicked", async () => {
    mockExportCollection.mockResolvedValueOnce(
      JSON.stringify({ version: 1, name: "Frontend", skills: ["frontend-design"] })
    );

    // Mock URL.createObjectURL and anchor click. The stub must be restored:
    // react-router's navigator calls `new URL(...)` while navigating, so a
    // leaked non-constructor URL breaks every later navigation in this file.
    const createObjectURL = vi.fn().mockReturnValue("blob:mock");
    const revokeObjectURL = vi.fn();
    const anchorClick = vi
      .spyOn(HTMLAnchorElement.prototype, "click")
      .mockImplementation(() => undefined);
    vi.stubGlobal("URL", { createObjectURL, revokeObjectURL });

    try {
      renderCollectionView();
      const exportButton = screen.getByRole("button", { name: /导出技能集/i });
      fireEvent.click(exportButton);

      await waitFor(() => {
        expect(mockExportCollection).toHaveBeenCalledWith("col-1");
      });
      expect(anchorClick).toHaveBeenCalledOnce();
    } finally {
      vi.unstubAllGlobals();
      anchorClick.mockRestore();
    }
  });

  // ── Error State ───────────────────────────────────────────────────────────

  it("shows error when loading fails", () => {
    renderCollectionView("col-1", {
      currentDetail: null,
      isLoadingDetail: false,
      error: "Collection not found",
    });
    expect(screen.getByText(/Collection not found/i)).toBeInTheDocument();
  });

  // ── Return-position restoration ───────────────────────────────────────────

  describe("scroll restoration", () => {
    afterEach(() => {
      // Clear any leftover scroll map entries between tests.
      consumeScrollPosition("collection:col-1");
      consumeScrollPosition("collection:col-2");
    });

    type InitialEntry =
      | string
      | {
          pathname: string;
          state?: unknown;
          search?: string;
          hash?: string;
        };

    function renderWithState(
      initialEntry: InitialEntry,
      storeOverrides: Record<string, unknown> = {},
      onLocationChange?: (location: Location) => void
    ) {
      mockUseCollectionStore.mockImplementation((selector) =>
        selector(buildCollectionStoreState(storeOverrides))
      );
      mockUsePlatformStore.mockImplementation((selector) =>
        selector(buildPlatformStoreState())
      );

      return render(
        <MemoryRouter initialEntries={[initialEntry]}>
          {onLocationChange && (
            <LocationProbe onChange={onLocationChange} />
          )}
          <Routes>
            <Route path="/collection/:collectionId" element={<CollectionView />} />
            <Route path="/skill/:skillId" element={<div>detail-route</div>} />
          </Routes>
        </MemoryRouter>
      );
    }

    it("navigates to skill detail with collection context and scroll restoration state", async () => {
      const locations: Location[] = [];
      renderWithState(`/collection/col-1`, {}, (loc) => {
        locations.push(loc);
      });

      // Locate the scroll container and fake a scroll offset.
      const scroller = screen
        .getByText("frontend-design")
        .closest("[class*='overflow-auto']");
      expect(scroller).not.toBeNull();
      if (!scroller) return;
      Object.defineProperty(scroller, "scrollTop", {
        value: 180,
        writable: true,
        configurable: true,
      });

      fireEvent.click(
        screen.getByRole("button", { name: /查看 frontend-design 的详情/i })
      );

      await waitFor(() => {
        expect(screen.getByText("detail-route")).toBeInTheDocument();
      });

      const detailLocation = locations.find((l) =>
        l.pathname.startsWith("/skill/")
      );
      expect(detailLocation).toBeDefined();
      const state = detailLocation?.state as
        | {
            collectionContext?: { collectionId?: string };
            scrollRestoration?: { key?: string; scrollTop?: number };
          }
        | null
        | undefined;
      expect(state?.collectionContext).toEqual({ collectionId: "col-1" });
      expect(state?.scrollRestoration).toEqual({
        key: "collection:col-1",
        scrollTop: 180,
      });
    });

    it("restores scroll position from the in-memory map after data hydrates", async () => {
      // Simulate what SkillDetail.handleGoBack does before navigating back.
      saveScrollPosition("collection:col-1", 420);

      renderWithState({
        pathname: "/collection/col-1",
        state: {
          collectionContext: { collectionId: "col-1" },
          scrollRestoration: { key: "collection:col-1", scrollTop: 0 },
        },
      });

      const scroller = screen
        .getByText("frontend-design")
        .closest("[class*='overflow-auto']");
      expect(scroller).not.toBeNull();
      if (!scroller) return;

      await waitFor(() => {
        expect((scroller as HTMLDivElement).scrollTop).toBe(420);
      });
      // Map should have been consumed after restoration.
      expect(consumeScrollPosition("collection:col-1")).toBeNull();
    });

    it("falls back to location.state.scrollTop when the in-memory map is empty", async () => {
      renderWithState({
        pathname: "/collection/col-1",
        state: {
          collectionContext: { collectionId: "col-1" },
          scrollRestoration: { key: "collection:col-1", scrollTop: 360 },
        },
      });

      const scroller = screen
        .getByText("frontend-design")
        .closest("[class*='overflow-auto']");
      expect(scroller).not.toBeNull();
      if (!scroller) return;

      await waitFor(() => {
        expect((scroller as HTMLDivElement).scrollTop).toBe(360);
      });
    });

    it("does not restore scroll when the restoration key targets a different collection", async () => {
      // The stored key points at col-2 but the route is col-1 — this should be
      // ignored so we don't cross-contaminate other collection contexts.
      renderWithState({
        pathname: "/collection/col-1",
        state: {
          collectionContext: { collectionId: "col-2" },
          scrollRestoration: { key: "collection:col-2", scrollTop: 999 },
        },
      });

      const scroller = screen
        .getByText("frontend-design")
        .closest("[class*='overflow-auto']");
      expect(scroller).not.toBeNull();
      if (!scroller) return;

      // Give the effect an opportunity to run — scrollTop should remain 0.
      await waitFor(() => {
        expect((scroller as HTMLDivElement).scrollTop).toBe(0);
      });
    });

    it("restoration remains stable when collection membership changes", async () => {
      // Simulate the case where, while the user was on skill detail, the
      // collection gained a new skill. The stored key still matches the
      // collection id, so restoration should proceed even though the skill
      // list length differs from what it was on navigation-away.
      const detailWithNewSkill: CollectionDetail = {
        ...mockCollectionDetail,
        skills: [
          ...mockCollectionDetail.skills,
          {
            id: "late-arrival",
            name: "late-arrival",
            description: "Added while user was away",
            file_path: "~/.skillsmanage/skills/late-arrival/SKILL.md",
            is_central: true,
            scanned_at: "2026-04-09T00:00:00Z",
          },
        ],
      };

      renderWithState(
        {
          pathname: "/collection/col-1",
          state: {
            collectionContext: { collectionId: "col-1" },
            scrollRestoration: { key: "collection:col-1", scrollTop: 240 },
          },
        },
        { currentDetail: detailWithNewSkill }
      );

      const scroller = screen
        .getByText("frontend-design")
        .closest("[class*='overflow-auto']");
      expect(scroller).not.toBeNull();
      if (!scroller) return;

      await waitFor(() => {
        expect((scroller as HTMLDivElement).scrollTop).toBe(240);
      });
      // And the newly-added skill is still rendered — context is valid.
      expect(screen.getByText("late-arrival")).toBeInTheDocument();
    });

    it("restores scroll from the in-memory map on back-navigation when location.state is null", async () => {
      // Simulate SkillDetail.handleGoBack: it saves the scroll position under
      // the collection's key, then navigates back. React Router replays the
      // original /collection/col-1 entry which had no state attached, so the
      // view mounts with location.state === null. Restoration must still
      // succeed because the scroll map still holds the offset keyed by
      // collection id.
      saveScrollPosition("collection:col-1", 280);

      renderWithState("/collection/col-1");

      const scroller = screen
        .getByText("frontend-design")
        .closest("[class*='overflow-auto']");
      expect(scroller).not.toBeNull();
      if (!scroller) return;

      await waitFor(() => {
        expect((scroller as HTMLDivElement).scrollTop).toBe(280);
      });
    });
  });
});
