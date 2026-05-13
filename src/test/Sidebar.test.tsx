import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, within } from "@testing-library/react";
import { MemoryRouter, useLocation } from "react-router-dom";
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
    id: "codex",
    display_name: "Codex CLI",
    category: "coding",
    global_skills_dir: "~/.agents/skills/",
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
    global_skills_dir: "~/.skillsmanage/skills/",
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
      codex: 7,
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
    resetForTargetChange: vi.fn(),
    setCategoryVisibility: vi.fn(),
    setAgentEnabled: vi.fn(),
    applyScanSummary: vi.fn(),
    setCollectionCount: vi.fn(),
    setDiscoveredCount: vi.fn(),
    addCustomAgent: vi.fn(),
    updateCustomAgent: vi.fn(),
    removeCustomAgent: vi.fn(),
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
      <LocationProbe />
    </MemoryRouter>
  );
}

function LocationProbe() {
  const location = useLocation();
  return <div data-testid="location">{location.pathname}</div>;
}

describe("Sidebar", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders expanded sidebar by default", () => {
    const { container } = renderSidebar();
    const nav = container.querySelector("nav");
    expect(nav).toHaveStyle({ width: "208px" });
  });

  it("defaults to collapsed sidebar on narrow viewports", () => {
    const originalInnerWidth = window.innerWidth;
    Object.defineProperty(window, "innerWidth", {
      configurable: true,
      value: 390,
    });

    try {
      const { container } = renderSidebar();
      const nav = container.querySelector("nav");
      expect(nav).toHaveStyle({ width: "56px" });
      expect(screen.getByRole("button", { name: /Dashboard/ })).toBeInTheDocument();
    } finally {
      Object.defineProperty(window, "innerWidth", {
        configurable: true,
        value: originalInnerWidth,
      });
    }
  });

  it("supports keyboard resizing from the right edge", () => {
    const { container } = renderSidebar();
    const nav = container.querySelector("nav");

    fireEvent.keyDown(screen.getByRole("separator"), { key: "ArrowRight" });

    expect(nav).toHaveStyle({ width: "224px" });
  });

  it("renders central, discover, collections, and platform entries", () => {
    renderSidebar();
    expect(screen.getByRole("button", { name: /中央技能库/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /项目技能库/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /技能集合/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /操作日志/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Claude Code/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Universal/ })).toBeInTheDocument();
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

  it("does not render Settings in the sidebar", () => {
    renderSidebar();
    expect(screen.queryByRole("button", { name: /设置/ })).not.toBeInTheDocument();
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
    const claudeButton = screen.getByRole("button", { name: /Claude Code/ });
    expect(claudeButton.className).toContain("bg-sidebar-primary");
  });

  it("highlights Central Skills when on /central", () => {
    renderSidebar("/central");
    const centralButton = screen.getByRole("button", { name: /中央技能库/ });
    expect(centralButton.className).toContain("bg-sidebar-primary");
  });

  it("routes Universal to its platform page", () => {
    renderSidebar();
    fireEvent.click(screen.getByRole("button", { name: /Universal/ }));
    expect(screen.getByTestId("location")).toHaveTextContent("/platform/universal-agents");
  });

  it("highlights Universal without highlighting Central", () => {
    renderSidebar("/platform/universal-agents");
    const universalButton = screen.getByRole("button", { name: /Universal/ });
    const centralButton = screen.getByRole("button", { name: /中央技能库/ });

    expect(universalButton.className).toContain("bg-sidebar-primary");
    expect(centralButton.className).not.toContain("bg-sidebar-primary");
  });

  it("uses the Universal representative count instead of the Central count", () => {
    renderSidebar();
    const universalButton = screen.getByRole("button", { name: /Universal/ });

    expect(within(universalButton).getByText("7")).toBeInTheDocument();
    expect(within(universalButton).queryByText("10")).not.toBeInTheDocument();
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

  it("renders Dashboard as the first top-level destination", () => {
    renderSidebar();
    const dashboardButton = screen.getByRole("button", { name: /Dashboard/ });
    const centralButton = screen.getByRole("button", { name: /中央技能库/ });

    expect(dashboardButton).toBeInTheDocument();
    expect(
      dashboardButton.compareDocumentPosition(centralButton) &
        Node.DOCUMENT_POSITION_FOLLOWING
    ).toBeTruthy();
  });

  it("highlights Dashboard when on /dashboard", () => {
    renderSidebar("/dashboard");
    const dashboardButton = screen.getByRole("button", { name: /Dashboard/ });
    const centralButton = screen.getByRole("button", { name: /中央技能库/ });

    expect(dashboardButton.className).toContain("bg-sidebar-primary");
    expect(centralButton.className).not.toContain("bg-sidebar-primary");
  });

  it("keeps Dashboard accessible when collapsed", () => {
    renderSidebar("/dashboard");
    fireEvent.click(screen.getByRole("button", { name: /折叠侧边栏/ }));

    expect(screen.getByRole("button", { name: /Dashboard/ })).toBeInTheDocument();
  });

  it("central and platform entries remain clickable", () => {
    renderSidebar();
    fireEvent.click(screen.getByRole("button", { name: /中央技能库/ }));
    fireEvent.click(screen.getByRole("button", { name: /Claude Code/ }));
  });
});
