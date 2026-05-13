import React from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { SkillDetailDrawer } from "@/components/skill/SkillDetailDrawer";
import { useSkillDetailStore } from "@/stores/skillDetailStore";
import { usePlatformStore } from "@/stores/platformStore";
import { useTargetStore } from "@/stores/targetStore";
import type { AgentWithStatus, SkillDetail as SkillDetailType } from "@/types";

vi.mock("@/stores/skillDetailStore", () => ({
  useSkillDetailStore: vi.fn(),
}));

vi.mock("@/stores/platformStore", () => ({
  usePlatformStore: vi.fn(),
}));

vi.mock("@/stores/targetStore", () => ({
  useTargetStore: vi.fn(),
}));

vi.mock("@/components/collection/CollectionPickerDialog", () => ({
  CollectionPickerDialog: ({ open }: { open: boolean }) =>
    open ? <div data-testid="collection-picker-dialog" /> : null,
}));

vi.mock("react-markdown", () => ({
  default: ({ children }: { children: string }) => <div data-testid="react-markdown">{children}</div>,
}));

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
  collections: [],
};

const mockLoadDetail = vi.fn();
const mockLoadCachedExplanation = vi.fn();
const mockInstallSkill = vi.fn();
const mockUninstallSkill = vi.fn();
const mockGenerateExplanation = vi.fn();
const mockRefreshExplanation = vi.fn();
const mockReset = vi.fn();
const mockRefreshCounts = vi.fn();
const mockRefreshInstallations = vi.fn();

function applyStoreMocks(detailOverrides = {}, platformOverrides = {}) {
  vi.mocked(useSkillDetailStore).mockImplementation((selector?: unknown) => {
    const state = {
      detail: mockDetail,
      content: "# Frontend Design",
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
      cleanupExplanationListeners: vi.fn(),
      reset: mockReset,
      ...detailOverrides,
    };
    if (typeof selector === "function") return selector(state);
    return state;
  });

  vi.mocked(usePlatformStore).mockImplementation((selector?: unknown) => {
    const state = {
      agents: mockAgents,
      skillsByAgent: {},
      isLoading: false,
      isRefreshing: false,
      error: null,
      initialize: vi.fn(),
      rescan: vi.fn(),
      refreshCounts: mockRefreshCounts,
      ...platformOverrides,
    };
    if (typeof selector === "function") return selector(state);
    return state;
  });

  vi.mocked(useTargetStore).mockImplementation((selector?: unknown) => {
    const state = {
      activeTarget: { id: "local", kind: "local", label: "Local", isActive: true },
    };
    if (typeof selector === "function") return selector(state);
    return state;
  });
}

function TestHarness({
  initialOpen = true,
  skillId = "frontend-design",
}: {
  initialOpen?: boolean;
  skillId?: string | null;
}) {
  const [open, setOpen] = React.useState(initialOpen);
  const triggerRef = React.useRef<HTMLButtonElement>(null);
  const renderCount = React.useRef(0);
  renderCount.current += 1;

  return (
    <MemoryRouter>
      <div data-testid="parent-shell" data-render-count={renderCount.current}>
        <button ref={triggerRef} onClick={() => setOpen(true)}>
          Open drawer
        </button>
        <SkillDetailDrawer
          open={open}
          skillId={skillId}
          onOpenChange={setOpen}
          returnFocusRef={triggerRef}
        />
      </div>
    </MemoryRouter>
  );
}

function PlatformViewLikeHarness() {
  const [open, setOpen] = React.useState(false);
  const [activeSkillId, setActiveSkillId] = React.useState<string | null>(null);
  const triggerRef = React.useRef<HTMLButtonElement>(null);

  return (
    <MemoryRouter>
      <button
        ref={triggerRef}
        onClick={() => {
          setActiveSkillId("frontend-design");
          setOpen(true);
        }}
      >
        Open drawer
      </button>
      <SkillDetailDrawer
        open={open}
        skillId={activeSkillId}
        onOpenChange={(nextOpen) => {
          setOpen(nextOpen);
          if (!nextOpen) {
            setActiveSkillId(null);
          }
        }}
        returnFocusRef={
          activeSkillId
            ? triggerRef
            : undefined
        }
      />
    </MemoryRouter>
  );
}

describe("SkillDetailDrawer", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    applyStoreMocks();
  });

  it("renders a dialog with overlay and close button when open", async () => {
    render(<TestHarness />);

    const drawer = await screen.findByTestId("skill-detail-modal");
    expect(drawer).toHaveAttribute("role", "dialog");
    expect(drawer).toHaveAttribute("aria-modal", "true");
    expect(screen.getByTestId("skill-detail-modal-overlay")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /关闭/i })).toBeInTheDocument();
  });

  it("does not render drawer contents when closed", () => {
    render(<TestHarness initialOpen={false} />);
    expect(screen.queryByTestId("skill-detail-modal")).toBeNull();
  });

  it("closes via close button and restores focus to returnFocusRef", async () => {
    render(<TestHarness />);

    const closeButton = await screen.findByRole("button", { name: /关闭/i });
    fireEvent.click(closeButton);

    await waitFor(() => {
      expect(screen.queryByTestId("skill-detail-modal")).toBeNull();
    });
    expect(screen.getByRole("button", { name: /open drawer/i })).toHaveFocus();
    expect(mockReset).toHaveBeenCalled();
  });

  it("restores focus even when the parent clears the selected skill during close", async () => {
    render(<PlatformViewLikeHarness />);

    fireEvent.click(screen.getByRole("button", { name: /open drawer/i }));

    const closeButton = await screen.findByRole("button", { name: /关闭/i });
    fireEvent.click(closeButton);

    await waitFor(() => {
      expect(screen.queryByTestId("skill-detail-modal")).toBeNull();
    });

    expect(screen.getByRole("button", { name: /open drawer/i })).toHaveFocus();
  });

  it("closes on Escape key", async () => {
    render(<TestHarness />);

    const drawer = await screen.findByTestId("skill-detail-modal");
    fireEvent.keyDown(drawer, { key: "Escape" });

    await waitFor(() => {
      expect(screen.queryByTestId("skill-detail-modal")).toBeNull();
    });
  });

  it("closes on overlay click", async () => {
    render(<TestHarness />);

    fireEvent.click(await screen.findByTestId("skill-detail-modal-overlay"));

    await waitFor(() => {
      expect(screen.queryByTestId("skill-detail-modal")).toBeNull();
    });
  });

  it("fully unmounts the shared overlay after close", async () => {
    render(<TestHarness />);

    expect(await screen.findByTestId("skill-detail-modal-overlay")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /关闭/i }));

    await waitFor(() => {
      expect(screen.queryByTestId("skill-detail-modal")).toBeNull();
      expect(screen.queryByTestId("skill-detail-modal-overlay")).toBeNull();
      expect(document.querySelector("[data-base-ui-inert]")).toBeNull();
    });
  });

  it("removes panel and overlay together when closed from the shell", async () => {
    render(<TestHarness />);

    const closeButton = await screen.findByRole("button", { name: /关闭/i });
    const overlay = screen.getByTestId("skill-detail-modal-overlay");
    const drawer = screen.getByTestId("skill-detail-modal");

    expect(overlay).toBeInTheDocument();
    expect(drawer).toBeInTheDocument();

    fireEvent.click(closeButton);

    await waitFor(() => {
      expect(screen.queryByTestId("skill-detail-modal")).toBeNull();
      expect(screen.queryByTestId("skill-detail-modal-overlay")).toBeNull();
    });
  });

  it("wires aria-labelledby to the SkillDetailView heading", async () => {
    render(<TestHarness />);

    const drawer = await screen.findByTestId("skill-detail-modal");
    const heading = screen.getByRole("heading", { name: /frontend-design/i });
    expect(drawer).toHaveAttribute("aria-labelledby", heading.id);
  });

  it("applies responsive modal class expectations", async () => {
    render(<TestHarness />);

    const modal = await screen.findByTestId("skill-detail-modal");

    // SkillDetailModalShell uses centered modal classes
    expect(modal.className).toContain("fixed");
    expect(modal.className).toContain("top-1/2");
    expect(modal.className).toContain("left-1/2");
    expect(modal.className).toContain("w-[min(90vw,var(--modal-max-w))]");
    expect(modal.className).toContain("lg:w-[min(70vw,var(--modal-max-w))]");
    expect(modal.className).toContain("sm:max-lg:w-[85vw]");
    expect(modal.className).toContain("max-sm:w-[95vw]");
  });

  it("does not unmount the parent container during open/close", async () => {
    render(<TestHarness />);

    const parent = screen.getByTestId("parent-shell");
    const initialNode = parent;

    fireEvent.click(await screen.findByRole("button", { name: /关闭/i }));

    await waitFor(() => {
      expect(screen.queryByTestId("skill-detail-modal")).toBeNull();
    });

    expect(screen.getByTestId("parent-shell")).toBe(initialNode);
    expect(screen.getByTestId("parent-shell")).toHaveAttribute("data-render-count", "2");
  });
});

describe("SkillDetailDrawer – modal shell integration", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    applyStoreMocks();
  });

  it("renders SkillDetailModalShell (not SkillDetailPanelShell)", async () => {
    render(<TestHarness />);

    // SkillDetailModalShell uses data-testid="skill-detail-modal"
    const modal = await screen.findByTestId("skill-detail-modal");
    expect(modal).toBeInTheDocument();
    expect(modal).toHaveAttribute("role", "dialog");
    expect(modal).toHaveAttribute("aria-modal", "true");

    // The old SkillDetailPanelShell used data-testid="skill-detail-drawer" — should NOT be present
    expect(screen.queryByTestId("skill-detail-drawer")).toBeNull();
  });

  it("renders ModalInstallButton in header when skill is not read-only", async () => {
    applyStoreMocks({ detail: { ...mockDetail, is_read_only: false } });
    render(<TestHarness />);

    await screen.findByTestId("skill-detail-modal");

    // ModalInstallButton renders a button with aria-label "安装 {name}"
    const installButton = screen.getByRole("button", { name: /安装 frontend-design/i });
    expect(installButton).toBeInTheDocument();
  });

  it("does not render ModalInstallButton when skill is read-only", async () => {
    applyStoreMocks({ detail: { ...mockDetail, is_read_only: true } });
    render(<TestHarness />);

    await screen.findByTestId("skill-detail-modal");

    // ModalInstallButton should not render when is_read_only is true
    expect(screen.queryByRole("button", { name: /安装/i })).toBeNull();
  });
});
