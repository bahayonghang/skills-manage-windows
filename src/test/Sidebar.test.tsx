import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { Sidebar } from "../components/layout/Sidebar";

vi.mock("../stores/platformStore", () => ({
  usePlatformStore: vi.fn(),
}));

import { usePlatformStore } from "../stores/platformStore";

const mockAgents = [
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
    id: "openclaw",
    display_name: "OpenClaw",
    category: "lobster",
    global_skills_dir: "~/.openclaw/skills/",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
  {
    id: "central",
    display_name: "Central Skills",
    category: "central",
    global_skills_dir: "~/.agents/skills/",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
];

function buildPlatformStoreState(overrides = {}) {
  return {
    agents: mockAgents,
    skillsByAgent: {
      "claude-code": 5,
      cursor: 3,
      central: 10,
    },
    collectionCount: 2,
    discoveredCount: 6,
    categoryVisibility: {
      coding: true,
      lobster: true,
    },
    lastScanAt: "2026-04-23T01:00:00Z",
    scanState: "idle",
    isLoading: false,
    isRefreshing: false,
    error: null,
    initialize: vi.fn(),
    hydrateShell: vi.fn(),
    refreshScanInBackground: vi.fn(),
    rescan: vi.fn(),
    refreshCounts: vi.fn(),
    setCategoryVisibility: vi.fn(),
    setAgentEnabled: vi.fn(),
    applyScanSummary: vi.fn(),
    setCollectionCount: vi.fn(),
    setDiscoveredCount: vi.fn(),
    ...overrides,
  };
}

function renderSidebar(initialPath = "/central") {
  vi.mocked(usePlatformStore).mockImplementation((selector?: unknown) => {
    const state = buildPlatformStoreState();
    if (typeof selector === "function") return selector(state);
    return state;
  });

  return render(
    <MemoryRouter initialEntries={[initialPath]}>
      <Sidebar />
    </MemoryRouter>
  );
}

describe("Sidebar", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders expanded sidebar by default", () => {
    const { container } = renderSidebar();
    const nav = container.querySelector("nav");
    expect(nav?.className).toContain("w-52");
  });

  it("renders central, discover, collections, and platform entries", () => {
    renderSidebar();
    expect(screen.getByRole("button", { name: /中央技能库/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /项目技能库/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /技能集合/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Claude Code/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /OpenClaw/ })).toBeInTheDocument();
  });

  it("shows a blocking loading row only during shell hydration", () => {
    vi.mocked(usePlatformStore).mockImplementation((selector?: unknown) => {
      const state = buildPlatformStoreState({ isLoading: true });
      if (typeof selector === "function") return selector(state);
      return state;
    });

    render(
      <MemoryRouter>
        <Sidebar />
      </MemoryRouter>
    );

    expect(screen.getByText("扫描中...")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Claude Code/ })).not.toBeInTheDocument();
  });

  it("keeps platform entries visible during background refresh", () => {
    vi.mocked(usePlatformStore).mockImplementation((selector?: unknown) => {
      const state = buildPlatformStoreState({
        isRefreshing: true,
        scanState: "refreshing",
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });

    render(
      <MemoryRouter>
        <Sidebar />
      </MemoryRouter>
    );

    expect(screen.getByRole("button", { name: /Claude Code/ })).toBeInTheDocument();
    expect(screen.getAllByText("扫描中...").length).toBeGreaterThan(0);
  });

  it("highlights the active route", () => {
    renderSidebar("/platform/claude-code");
    const button = screen.getByRole("button", { name: /Claude Code/ });
    expect(button.className).toContain("bg-hover-bg");
  });

  it("central and platform entries remain clickable", () => {
    renderSidebar();
    fireEvent.click(screen.getByRole("button", { name: /中央技能库/ }));
    fireEvent.click(screen.getByRole("button", { name: /Claude Code/ }));
  });

  it("hides lobster platforms when the lobster group is disabled", () => {
    vi.mocked(usePlatformStore).mockImplementation((selector?: unknown) => {
      const state = buildPlatformStoreState({
        categoryVisibility: {
          coding: true,
          lobster: false,
        },
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });

    render(
      <MemoryRouter>
        <Sidebar />
      </MemoryRouter>
    );

    expect(screen.getByRole("button", { name: /Claude Code/ })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /OpenClaw/ })).not.toBeInTheDocument();
  });
});
