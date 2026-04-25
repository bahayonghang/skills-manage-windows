import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { MemoryRouter, Route, Routes } from "react-router-dom";
import { SkillDetailPage } from "../pages/SkillDetailPage";
import * as scrollRestoration from "../lib/scrollRestoration";
import { AgentWithStatus, SkillDetail as SkillDetailType } from "../types";

// ─── Mock stores ──────────────────────────────────────────────────────────────

vi.mock("../stores/skillDetailStore", () => ({
  useSkillDetailStore: vi.fn(),
}));

vi.mock("../stores/platformStore", () => ({
  usePlatformStore: vi.fn(),
}));

// ─── Mock CollectionPickerDialog (same as the view test) ──────────────────────

vi.mock("../components/collection/CollectionPickerDialog", () => ({
  CollectionPickerDialog: ({ open }: { open: boolean }) =>
    open ? <div data-testid="collection-picker-dialog" /> : null,
}));

import { useSkillDetailStore } from "../stores/skillDetailStore";
import { usePlatformStore } from "../stores/platformStore";

vi.mock("react-markdown", () => ({
  default: ({ children }: { children: string }) => (
    <div data-testid="react-markdown">{children}</div>
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
];

const mockDetail: SkillDetailType = {
  id: "frontend-design",
  name: "frontend-design",
  description: "Build distinctive, production-grade frontend interfaces",
  file_path: "~/.skillsmanage/skills/frontend-design/SKILL.md",
  canonical_path: "~/.skillsmanage/skills/frontend-design",
  is_central: true,
  source: "native",
  scanned_at: "2026-04-09T00:00:00Z",
  installations: [],
};

function applyStoreMocks(detail: SkillDetailType | null = mockDetail) {
  vi.mocked(useSkillDetailStore).mockImplementation((selector?: unknown) => {
    const state = {
      detail,
      content: null,
      isLoading: false,
      installingAgentId: null,
      error: null,
      explanation: null,
      isExplanationLoading: false,
      isExplanationStreaming: false,
      explanationError: null,
      explanationErrorInfo: null,
      loadDetail: vi.fn(),
      loadCachedExplanation: vi.fn(),
      generateExplanation: vi.fn(),
      refreshExplanation: vi.fn(),
      installSkill: vi.fn(),
      uninstallSkill: vi.fn(),
      cleanupExplanationListeners: vi.fn(),
      reset: vi.fn(),
    };
    if (typeof selector === "function") return selector(state);
    return state;
  });
  vi.mocked(usePlatformStore).mockImplementation((selector?: unknown) => {
    const state = {
      agents: mockAgents,
      skillsByAgent: {},
      isLoading: false,
      error: null,
      initialize: vi.fn(),
      rescan: vi.fn(),
    };
    if (typeof selector === "function") return selector(state);
    return state;
  });
}

function renderPage(
  initialEntry:
    | string
    | { pathname: string; state?: unknown } = "/skill/frontend-design"
) {
  return render(
    <MemoryRouter initialEntries={[initialEntry as string]}>
      <Routes>
        <Route path="/skill/:skillId" element={<SkillDetailPage />} />
      </Routes>
    </MemoryRouter>
  );
}

// ─── Tests ────────────────────────────────────────────────────────────────────

describe("SkillDetailPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    applyStoreMocks();
  });

  // ── PageHeader: back button ──────────────────────────────────────────────

  it("renders a back button with the goBack aria label", () => {
    renderPage();
    expect(screen.getByRole("button", { name: /返回/i })).toBeInTheDocument();
  });

  it("does not throw when back button is clicked", () => {
    renderPage();
    const backBtn = screen.getByRole("button", { name: /返回/i });
    fireEvent.click(backBtn);
  });

  it("saves scroll position from location.state.scrollRestoration before going back", () => {
    const saveSpy = vi.spyOn(scrollRestoration, "saveScrollPosition");
    render(
      <MemoryRouter
        initialEntries={[
          {
            pathname: "/skill/frontend-design",
            state: {
              scrollRestoration: { key: "central:list", scrollTop: 420 },
            },
          },
        ]}
      >
        <Routes>
          <Route path="/skill/:skillId" element={<SkillDetailPage />} />
        </Routes>
      </MemoryRouter>
    );
    fireEvent.click(screen.getByRole("button", { name: /返回/i }));
    expect(saveSpy).toHaveBeenCalledWith("central:list", 420);
    saveSpy.mockRestore();
  });

  // ── PageHeader: breadcrumb ───────────────────────────────────────────────

  it("renders a single-segment breadcrumb with skill name when no location.state.from is present", () => {
    renderPage();
    const breadcrumb = screen.getByRole("navigation");
    // Only the skill name appears, no separator, no other link
    const items = breadcrumb.querySelectorAll("li");
    expect(items).toHaveLength(1);
    expect(items[0]).toHaveTextContent("frontend-design");
  });

  it("falls back to skillId when detail has not loaded yet", () => {
    applyStoreMocks(null);
    renderPage("/skill/unknown-skill");
    const breadcrumb = screen.getByRole("navigation");
    expect(breadcrumb).toHaveTextContent("unknown-skill");
  });

  it("renders a two-segment breadcrumb with a link to location.state.from.route when from is provided", () => {
    render(
      <MemoryRouter
        initialEntries={[
          {
            pathname: "/skill/frontend-design",
            state: {
              from: { pageLabel: "Central Skills", route: "/central" },
            },
          },
        ]}
      >
        <Routes>
          <Route path="/skill/:skillId" element={<SkillDetailPage />} />
        </Routes>
      </MemoryRouter>
    );
    const breadcrumb = screen.getByRole("navigation");
    const items = breadcrumb.querySelectorAll("li");
    // first crumb (link) + separator + current crumb
    expect(items.length).toBe(3);
    const link = breadcrumb.querySelector("a");
    expect(link).not.toBeNull();
    expect(link).toHaveAttribute("href", "/central");
    expect(link).toHaveTextContent("Central Skills");
    expect(breadcrumb).toHaveTextContent("frontend-design");
  });

  // ── Delegation to SkillDetailView ────────────────────────────────────────

  it("renders the shared SkillDetailView (title, metadata, tabs)", () => {
    renderPage();
    // The view's h1 title
    expect(
      screen.getByRole("heading", { name: /frontend-design/i })
    ).toBeInTheDocument();
    // The view's TabToggle
    expect(screen.getByRole("tab", { name: /Markdown/i })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: /原始源码/i })).toBeInTheDocument();
    expect(screen.getByRole("tab", { name: /AI 解释/i })).toBeInTheDocument();
    // The view's metadata section
    expect(screen.getByRole("region", { name: /技能基本信息/i })).toBeInTheDocument();
  });
});
