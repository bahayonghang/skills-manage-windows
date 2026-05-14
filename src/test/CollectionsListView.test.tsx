import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { useEffect } from "react";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { MemoryRouter, Route, Routes, useLocation, type Location } from "react-router-dom";
import { CollectionsListView } from "../pages/CollectionsListView";
import {
  Collection,
  CollectionDetail,
  AgentWithStatus,
  SkillWithLinks,
} from "../types";
import {
  clearReturnContext,
  consumeScrollPosition,
  saveReturnContext,
  saveScrollPosition,
} from "../lib/scrollRestoration";
import * as scrollRestoration from "../lib/scrollRestoration";

// ─── Mock stores ──────────────────────────────────────────────────────────────

vi.mock("../stores/collectionStore", () => ({
  useCollectionStore: vi.fn(),
}));

vi.mock("../stores/platformStore", () => ({
  usePlatformStore: vi.fn(),
}));

vi.mock("../stores/centralSkillsStore", () => ({
  useCentralSkillsStore: vi.fn(),
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

import { useCollectionStore } from "../stores/collectionStore";
import { usePlatformStore } from "../stores/platformStore";
import { useCentralSkillsStore } from "../stores/centralSkillsStore";

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
    id: "central",
    display_name: "Central Skills",
    category: "central",
    global_skills_dir: "~/.skillsmanage/skills/",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
];

const mockCollections: Collection[] = [
  {
    id: "col-1",
    name: "Frontend",
    description: "Frontend skills",
    created_at: "2026-04-09T00:00:00Z",
    updated_at: "2026-04-09T00:00:00Z",
  },
  {
    id: "col-2",
    name: "Backend",
    description: "Backend skills",
    created_at: "2026-04-09T00:00:00Z",
    updated_at: "2026-04-09T00:00:00Z",
  },
  {
    id: "col-3",
    name: "Infra",
    description: "Infrastructure skills",
    created_at: "2026-04-09T00:00:00Z",
    updated_at: "2026-04-09T00:00:00Z",
  },
];

const mockDetailCol1: CollectionDetail = {
  id: "col-1",
  name: "Frontend",
  description: "Frontend skills",
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

const mockDetailCol2: CollectionDetail = {
  id: "col-2",
  name: "Backend",
  description: "Backend skills",
  created_at: "2026-04-09T00:00:00Z",
  updated_at: "2026-04-09T00:00:00Z",
  skills: [
    {
      id: "api-designer",
      name: "api-designer",
      description: "Design REST APIs",
      file_path: "~/.skillsmanage/skills/api-designer/SKILL.md",
      is_central: true,
      scanned_at: "2026-04-09T00:00:00Z",
    },
  ],
};

const mockCentralSkills: SkillWithLinks[] = [
  {
    id: "frontend-design",
    name: "frontend-design",
    description: "Build distinctive frontend UIs",
    file_path: "~/.skillsmanage/skills/frontend-design/SKILL.md",
    canonical_path: "~/.skillsmanage/skills/frontend-design",
    is_central: true,
    scanned_at: "2026-04-09T00:00:00Z",
    linked_agents: [],
    shared_root_agents: [],
  },
];

// ─── Store builders ───────────────────────────────────────────────────────────

const mockLoadCollections = vi.fn();
const mockLoadCollectionDetail = vi.fn();
const mockRemoveSkill = vi.fn();
const mockDeleteCollection = vi.fn();
const mockBatchInstallCollection = vi.fn();
const mockExportCollection = vi.fn();
const mockImportCollection = vi.fn();
const mockAddSkillToCollection = vi.fn();
const mockRefreshCounts = vi.fn();
const mockLoadCentralSkills = vi.fn();
const mockInstallCentralSkill = vi.fn();

const mockUseCollectionStore = vi.mocked(useCollectionStore);
const mockUsePlatformStore = vi.mocked(usePlatformStore);
const mockUseCentralSkillsStore = vi.mocked(useCentralSkillsStore);

function buildCollectionStoreState(overrides: Record<string, unknown> = {}) {
  return {
    collections: mockCollections,
    currentDetail: mockDetailCol1,
    isLoading: false,
    isLoadingDetail: false,
    error: null,
    loadCollections: mockLoadCollections,
    createCollection: vi.fn(),
    updateCollection: vi.fn(),
    deleteCollection: mockDeleteCollection,
    loadCollectionDetail: mockLoadCollectionDetail,
    addSkillToCollection: mockAddSkillToCollection,
    removeSkillFromCollection: mockRemoveSkill,
    batchInstallCollection: mockBatchInstallCollection,
    exportCollection: mockExportCollection,
    importCollection: mockImportCollection,
    refreshCounts: vi.fn(),
    ...overrides,
  };
}

function buildPlatformStoreState() {
  return {
    agents: mockAgents,
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
    rescan: vi.fn(),
    refreshCounts: mockRefreshCounts,
    resetForTargetChange: vi.fn(),
    applyScanSummary: vi.fn(),
    setCollectionCount: vi.fn(),
    addCustomAgent: vi.fn(),
    updateCustomAgent: vi.fn(),
    removeCustomAgent: vi.fn(),
  };
}

function buildCentralStoreState() {
  return {
    skills: mockCentralSkills,
    agents: mockAgents,
    isLoading: false,
    isInstalling: false,
    togglingAgentId: null,
    error: null,
    loadCentralSkills: mockLoadCentralSkills,
    installSkill: mockInstallCentralSkill,
    togglePlatformLink: vi.fn(),
  };
}

function applyStoreMocks(collectionOverrides: Record<string, unknown> = {}) {
  mockUseCollectionStore.mockImplementation((selector: unknown) => {
    const state = buildCollectionStoreState(collectionOverrides);
    if (typeof selector === "function") return selector(state);
    return state;
  });
  mockUsePlatformStore.mockImplementation((selector: unknown) => {
    const state = buildPlatformStoreState();
    if (typeof selector === "function") return selector(state);
    return state;
  });
  mockUseCentralSkillsStore.mockImplementation((selector: unknown) => {
    const state = buildCentralStoreState();
    if (typeof selector === "function") return selector(state);
    return state;
  });
}

type InitialEntry =
  | string
  | { pathname: string; state?: unknown; search?: string; hash?: string };

// Helper that subscribes to location changes and forwards them to a spy
// function. Lets tests assert on the state we emit when navigating into a
// skill detail without relying on `window.history.state`, which MemoryRouter
// does not update.
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

function renderList(
  initialEntry: InitialEntry = "/collections",
  collectionOverrides: Record<string, unknown> = {},
  onLocationChange?: (location: Location) => void
) {
  applyStoreMocks(collectionOverrides);
  return render(
    <MemoryRouter initialEntries={[initialEntry]}>
      {onLocationChange && <LocationProbe onChange={onLocationChange} />}
      <Routes>
        <Route path="/collections" element={<CollectionsListView />} />
        <Route path="/skill/:skillId" element={<div>detail-route</div>} />
      </Routes>
    </MemoryRouter>
  );
}

// ─── Tests ────────────────────────────────────────────────────────────────────

describe("CollectionsListView", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    consumeScrollPosition("collection:col-1");
    consumeScrollPosition("collection:col-2");
    consumeScrollPosition("collection:col-3");
    clearReturnContext("collections");
  });

  // ── Basic rendering ───────────────────────────────────────────────────────

  it("renders collection chips for all collections", () => {
    renderList();
    // "Frontend" appears both as a chip and in the collection header for the
    // auto-selected first collection — so we assert it shows up at least once.
    expect(screen.getAllByText("Frontend").length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText("Backend")).toBeInTheDocument();
    expect(screen.getByText("Infra")).toBeInTheDocument();
  });

  it("auto-selects the first collection when no context is provided", () => {
    renderList();
    // loadCollectionDetail should have been called for the first collection.
    expect(mockLoadCollectionDetail).toHaveBeenCalledWith("col-1");
  });

  it("requires a second confirmation click before removing a skill from the selected collection", async () => {
    mockRemoveSkill.mockResolvedValueOnce(undefined);
    renderList();

    fireEvent.click(
      screen.getByRole("button", { name: /从技能集中移除 frontend-design/i })
    );
    expect(mockRemoveSkill).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: /确认删除/i }));

    await waitFor(() => {
      expect(mockRemoveSkill).toHaveBeenCalledWith("col-1", "frontend-design");
    });
  });

  // ── Collection context restoration (COLL-RETURN-001) ─────────────────────

  it("restores the prior collection context from navigation state on entry", async () => {
    renderList(
      {
        pathname: "/collections",
        state: {
          collectionContext: { collectionId: "col-2" },
          scrollRestoration: { key: "collection:col-2", scrollTop: 0 },
        },
      },
      { currentDetail: mockDetailCol2 }
    );

    // Should load the restored collection rather than auto-selecting col-1.
    expect(mockLoadCollectionDetail).toHaveBeenCalledWith("col-2");
    await waitFor(() => {
      expect(screen.getByText("api-designer")).toBeInTheDocument();
    });
  });

  // ── Forward navigation state emission ─────────────────────────────────────

  it("opens skill detail in the drawer without navigating away from /collections", async () => {
    const locations: Location[] = [];
    renderList("/collections", {}, (loc) => {
      locations.push(loc);
    });

    fireEvent.click(
      screen.getByRole("button", { name: /查看 frontend-design 的详情/i })
    );

    await waitFor(() => {
      expect(screen.getByTestId("skill-detail-drawer")).toBeInTheDocument();
    });

    expect(screen.queryByText("detail-route")).not.toBeInTheDocument();
    expect(locations.some((l) => l.pathname.startsWith("/skill/"))).toBe(false);
    expect(locations[locations.length - 1]?.pathname).toBe("/collections");
  });

  it("opens the skill detail drawer without navigating away and does not use return-context helpers on card click", async () => {
    const saveReturnSpy = vi.spyOn(scrollRestoration, "saveReturnContext");

    renderList();

    fireEvent.click(
      screen.getByRole("button", { name: /查看 frontend-design 的详情/i })
    );

    await waitFor(() => {
      expect(screen.getByTestId("skill-detail-drawer")).toBeInTheDocument();
    });

    expect(screen.getByText("drawer-skill:frontend-design")).toBeInTheDocument();
    expect(screen.queryByText("detail-route")).not.toBeInTheDocument();
    expect(saveReturnSpy).not.toHaveBeenCalled();
  });

  it("preserves selected collection and scroll state when closing the drawer and restores focus", async () => {
    renderList(
      {
        pathname: "/collections",
        state: { collectionContext: { collectionId: "col-2" } },
      },
      { currentDetail: mockDetailCol2 }
    );

    await waitFor(() => {
      expect(screen.getByText("api-designer")).toBeInTheDocument();
    });

    const trigger = screen.getByRole("button", { name: /查看 api-designer 的详情/i });
    const scroller = trigger.closest("[class*='overflow-auto']");
    expect(scroller).not.toBeNull();
    if (!scroller) return;
    (scroller as HTMLDivElement).scrollTop = 310;

    fireEvent.click(trigger);

    await waitFor(() => {
      expect(screen.getByTestId("skill-detail-drawer")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /close drawer/i }));

    await waitFor(() => {
      expect(screen.queryByTestId("skill-detail-drawer")).not.toBeInTheDocument();
    });

    expect(screen.getByRole("button", { name: "Backend" })).toHaveClass("bg-primary/15");
    expect(screen.getByText("api-designer")).toBeInTheDocument();
    expect((scroller as HTMLDivElement).scrollTop).toBe(310);
    expect(trigger).toHaveFocus();
  });

  // ── Scroll restoration on return (COLL-RETURN-001) ───────────────────────

  it("restores scroll position from the in-memory map after collection detail hydrates", async () => {
    // Simulate what SkillDetail.handleGoBack does before navigating back.
    saveScrollPosition("collection:col-1", 360);

    renderList({
      pathname: "/collections",
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
      expect((scroller as HTMLDivElement).scrollTop).toBe(360);
    });
    // Restoration is single-use — the map entry is consumed.
    expect(consumeScrollPosition("collection:col-1")).toBeNull();
  });

  it("falls back to location.state.scrollTop when the in-memory map is empty", async () => {
    renderList({
      pathname: "/collections",
      state: {
        collectionContext: { collectionId: "col-1" },
        scrollRestoration: { key: "collection:col-1", scrollTop: 240 },
      },
    });

    const scroller = screen
      .getByText("frontend-design")
      .closest("[class*='overflow-auto']");
    expect(scroller).not.toBeNull();
    if (!scroller) return;

    await waitFor(() => {
      expect((scroller as HTMLDivElement).scrollTop).toBe(240);
    });
  });

  // ── Restoration stability when membership changes (COLL-RETURN-002) ───────

  it("restores scroll even when collection membership changed while away", async () => {
    const detailWithExtraSkill: CollectionDetail = {
      ...mockDetailCol1,
      skills: [
        ...mockDetailCol1.skills,
        {
          id: "late-arrival",
          name: "late-arrival",
          description: "Skill added while away",
          file_path: "~/.skillsmanage/skills/late-arrival/SKILL.md",
          is_central: true,
          scanned_at: "2026-04-09T00:00:00Z",
        },
      ],
    };

    renderList(
      {
        pathname: "/collections",
        state: {
          collectionContext: { collectionId: "col-1" },
          scrollRestoration: { key: "collection:col-1", scrollTop: 180 },
        },
      },
      { currentDetail: detailWithExtraSkill }
    );

    const scroller = screen
      .getByText("frontend-design")
      .closest("[class*='overflow-auto']");
    expect(scroller).not.toBeNull();
    if (!scroller) return;

    await waitFor(() => {
      expect((scroller as HTMLDivElement).scrollTop).toBe(180);
    });
    expect(screen.getByText("late-arrival")).toBeInTheDocument();
  });

  it("does not restore scroll when the restored collection id no longer exists", async () => {
    // Collection col-deleted isn't in the list — restoration should no-op and
    // the view should fall back to auto-selecting the first real collection.
    renderList({
      pathname: "/collections",
      state: {
        collectionContext: { collectionId: "col-deleted" },
        scrollRestoration: { key: "collection:col-deleted", scrollTop: 999 },
      },
    });

    // loadCollectionDetail is eventually called with the first available id.
    await waitFor(() => {
      expect(mockLoadCollectionDetail).toHaveBeenCalledWith("col-1");
    });
  });

  it("does not restore scroll when the restoration key targets a different collection", async () => {
    // key says col-2 but selectedId is col-1 (no collectionContext provided).
    renderList({
      pathname: "/collections",
      state: {
        scrollRestoration: { key: "collection:col-2", scrollTop: 999 },
      },
    });

    const scroller = screen
      .getByText("frontend-design")
      .closest("[class*='overflow-auto']");
    expect(scroller).not.toBeNull();
    if (!scroller) return;

    await waitFor(() => {
      expect((scroller as HTMLDivElement).scrollTop).toBe(0);
    });
  });

  // ── Back-navigation path (location.state is null; in-memory map) ──────────

  it("re-focuses and restores scroll from the in-memory return-context map when location.state is null (back-nav)", async () => {
    // Simulate the forward navigation side-effects that CollectionsListView
    // performs when the user clicks a skill card on col-2: save the return
    // context and the scroll position, then (in the real app) navigate to
    // /skill/:id and eventually back to /collections with state=null.
    saveReturnContext("collections", { collectionId: "col-2" });
    saveScrollPosition("collection:col-2", 300);

    renderList("/collections", { currentDetail: mockDetailCol2 });

    // The view should have re-focused col-2 rather than auto-selecting col-1.
    await waitFor(() => {
      expect(mockLoadCollectionDetail).toHaveBeenCalledWith("col-2");
    });
    expect(screen.getByText("api-designer")).toBeInTheDocument();

    const scroller = screen
      .getByText("api-designer")
      .closest("[class*='overflow-auto']");
    expect(scroller).not.toBeNull();
    if (!scroller) return;

    await waitFor(() => {
      expect((scroller as HTMLDivElement).scrollTop).toBe(300);
    });
  });

  it("re-focuses the previously viewed collection via the in-memory map even when no scroll offset was saved", async () => {
    saveReturnContext("collections", { collectionId: "col-2" });

    renderList("/collections", { currentDetail: mockDetailCol2 });

    await waitFor(() => {
      expect(mockLoadCollectionDetail).toHaveBeenCalledWith("col-2");
    });
    expect(screen.getByText("api-designer")).toBeInTheDocument();
  });

  // ── End-to-end return-position round-trip ────────────────────────────────

  it("full drawer round-trip preserves collection selection, route, and scroll", async () => {
    const locations: Location[] = [];
    renderList("/collections", {}, (loc) => {
      locations.push(loc);
    });

    const trigger = screen.getByRole("button", { name: /查看 frontend-design 的详情/i });
    const scroller = trigger.closest("[class*='overflow-auto']") as HTMLDivElement;
    expect(scroller).not.toBeNull();
    Object.defineProperty(scroller, "scrollTop", {
      value: 260,
      writable: true,
      configurable: true,
    });

    fireEvent.click(trigger);

    await waitFor(() => {
      expect(screen.getByTestId("skill-detail-drawer")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: /close drawer/i }));

    await waitFor(() => {
      expect(screen.queryByTestId("skill-detail-drawer")).not.toBeInTheDocument();
    });

    expect(locations.some((l) => l.pathname.startsWith("/skill/"))).toBe(false);
    expect(locations[locations.length - 1]?.pathname).toBe("/collections");
    expect(screen.getByText("frontend-design")).toBeInTheDocument();
    expect(scroller.scrollTop).toBe(260);
    expect(trigger).toHaveFocus();
  });
});
