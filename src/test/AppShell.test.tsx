import { act, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { MemoryRouter, Route, Routes, useNavigate } from "react-router-dom";
import { AppShell } from "@/components/layout/AppShell";
import { usePlatformStore } from "@/stores/platformStore";
import { useCentralSkillsStore } from "@/stores/centralSkillsStore";
import { useTargetStore } from "@/stores/targetStore";
import { useSkillStore } from "@/stores/skillStore";
import { useMarketplaceStore } from "@/stores/marketplaceStore";
import type { TargetSummary } from "@/types";
import { listen, isTauriRuntime } from "@/lib/tauri";
import { toast } from "sonner";

let triggerRescanInMock = false;

vi.mock("@/stores/platformStore", () => ({
  usePlatformStore: vi.fn(),
}));

vi.mock("@/stores/centralSkillsStore", () => ({
  useCentralSkillsStore: vi.fn(),
}));

vi.mock("@/stores/targetStore", () => ({
  useTargetStore: vi.fn(),
}));

vi.mock("@/stores/skillStore", () => ({
  useSkillStore: vi.fn(),
}));

vi.mock("@/stores/marketplaceStore", () => ({
  useMarketplaceStore: vi.fn(),
}));

vi.mock("@/lib/tauri", () => ({
  listen: vi.fn(),
  isTauriRuntime: vi.fn(() => true),
}));

vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

vi.mock("@/components/layout/Sidebar", () => ({
  Sidebar: () => <div data-testid="sidebar" />,
}));

vi.mock("@/components/layout/TopBar", () => ({
  TopBar: ({ onSearchClick }: { onSearchClick: () => void }) => (
    <button type="button" onClick={onSearchClick}>
      open-search
    </button>
  ),
}));

vi.mock("@/components/layout/GlobalSearchDialog", () => ({
  GlobalSearchDialog: ({
    open,
    onAction,
  }: {
    open: boolean;
    onAction: (action: string) => void;
  }) =>
    open ? (
      triggerRescanInMock ? (
        <button type="button" onClick={() => onAction("rescan")}>
          trigger-rescan
        </button>
      ) : (
        <div data-testid="global-search-dialog" />
      )
    ) : null,
}));

const mockUsePlatformStore = vi.mocked(usePlatformStore);
const mockUseCentralSkillsStore = vi.mocked(useCentralSkillsStore);
const mockUseTargetStore = vi.mocked(useTargetStore);
const mockUseSkillStore = vi.mocked(useSkillStore);
const mockUseMarketplaceStore = vi.mocked(useMarketplaceStore);
const mockListen = vi.mocked(listen);
const mockIsTauriRuntime = vi.mocked(isTauriRuntime);

let testNavigate: ReturnType<typeof useNavigate> | null = null;

function createPlatformState(overrides?: Partial<{
  initialize: ReturnType<typeof vi.fn>;
  rescan: ReturnType<typeof vi.fn>;
  resetForTargetChange: ReturnType<typeof vi.fn>;
}>) {
  return {
    initialize: vi.fn().mockResolvedValue(undefined),
    rescan: vi.fn().mockResolvedValue(undefined),
    resetForTargetChange: vi.fn(),
    ...overrides,
  };
}

function createCentralSkillsState(overrides?: Partial<{
  loadCentralSkills: ReturnType<typeof vi.fn>;
  resetForTargetChange: ReturnType<typeof vi.fn>;
}>) {
  return {
    loadCentralSkills: vi.fn().mockResolvedValue(undefined),
    resetForTargetChange: vi.fn(),
    ...overrides,
  };
}

function createSkillState(overrides?: Partial<{
  resetForTargetChange: ReturnType<typeof vi.fn>;
}>) {
  return {
    resetForTargetChange: vi.fn(),
    ...overrides,
  };
}

function createMarketplaceState(overrides?: Partial<{
  resetForTargetChange: ReturnType<typeof vi.fn>;
}>) {
  return {
    resetForTargetChange: vi.fn(),
    ...overrides,
  };
}

function createTargetState(overrides?: Partial<{
  activeTarget: TargetSummary;
  loadTargets: ReturnType<typeof vi.fn>;
}>) {
  return {
    activeTarget: {
      id: "local",
      kind: "local" as const,
      label: "Local",
      isActive: true,
    },
    loadTargets: vi.fn().mockResolvedValue(undefined),
    ...overrides,
  };
}

function NavigationHarness() {
  testNavigate = useNavigate();
  return null;
}

function DummyPage({ label }: { label: string }) {
  return (
    <div className="flex h-full flex-col">
      <div>{label}</div>
      <div className="flex-1 overflow-auto p-4">
        <div style={{ height: 1600 }}>content</div>
      </div>
    </div>
  );
}

describe("AppShell", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    testNavigate = null;
    triggerRescanInMock = false;
    mockIsTauriRuntime.mockReturnValue(true);
    mockListen.mockResolvedValue(vi.fn());

    mockUsePlatformStore.mockImplementation((selector?: unknown) => {
      const state = createPlatformState();
      if (typeof selector === "function") return selector(state);
      return state;
    });
    mockUseCentralSkillsStore.mockImplementation((selector?: unknown) => {
      const state = createCentralSkillsState();
      if (typeof selector === "function") return selector(state);
      return state;
    });
    mockUseTargetStore.mockImplementation((selector?: unknown) => {
      const state = createTargetState();
      if (typeof selector === "function") return selector(state);
      return state;
    });
    mockUseSkillStore.mockImplementation((selector?: unknown) => {
      const state = createSkillState();
      if (typeof selector === "function") return selector(state);
      return state;
    });
    mockUseMarketplaceStore.mockImplementation((selector?: unknown) => {
      const state = createMarketplaceState();
      if (typeof selector === "function") return selector(state);
      return state;
    });
  });

  it("resets shell scroll and keeps main non-scrollable when the route changes", async () => {
    render(
      <MemoryRouter initialEntries={["/a"]}>
        <NavigationHarness />
        <Routes>
          <Route path="/" element={<AppShell />}>
            <Route path="a" element={<DummyPage label="page-a" />} />
            <Route path="b" element={<DummyPage label="page-b" />} />
          </Route>
        </Routes>
      </MemoryRouter>
    );

    const main = document.querySelector("main");
    expect(main).not.toBeNull();
    if (!main) return;

    expect(main.className).toContain("overflow-hidden");
    expect(main.className).not.toContain("overflow-auto");

    (main as HTMLElement).scrollTop = 240;

    await act(async () => {
      testNavigate?.("/b");
    });

    await waitFor(() => {
      expect(screen.getByText("page-b")).toBeInTheDocument();
    });

    expect((main as HTMLElement).scrollTop).toBe(0);
  });

  it("routes the global rescan action to the platform and central stores", async () => {
    const mockRescan = vi.fn().mockResolvedValue(undefined);
    const mockLoadCentralSkills = vi.fn().mockResolvedValue(undefined);
    triggerRescanInMock = true;

    mockUsePlatformStore.mockImplementation((selector?: unknown) => {
      const state = createPlatformState({
        rescan: mockRescan,
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });
    mockUseCentralSkillsStore.mockImplementation((selector?: unknown) => {
      const state = createCentralSkillsState({
        loadCentralSkills: mockLoadCentralSkills,
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });

    render(
      <MemoryRouter initialEntries={["/a"]}>
        <Routes>
          <Route path="/" element={<AppShell />}>
            <Route path="a" element={<DummyPage label="page-a" />} />
          </Route>
        </Routes>
      </MemoryRouter>
    );

    await act(async () => {
      screen.getByRole("button", { name: /open-search/i }).click();
    });

    await act(async () => {
      screen.getByRole("button", { name: /trigger-rescan/i }).click();
    });

    expect(mockRescan).toHaveBeenCalledTimes(1);
    expect(mockLoadCentralSkills).toHaveBeenCalledTimes(1);
  });

  it("waits for the platform rescan before refreshing central", async () => {
    let resolveRescan!: () => void;
    const rescanPromise = new Promise<void>((resolve) => {
      resolveRescan = () => resolve();
    });
    const mockRescan = vi.fn().mockReturnValue(rescanPromise);
    const mockLoadCentralSkills = vi.fn().mockResolvedValue(undefined);
    triggerRescanInMock = true;

    mockUsePlatformStore.mockImplementation((selector?: unknown) => {
      const state = createPlatformState({
        rescan: mockRescan,
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });
    mockUseCentralSkillsStore.mockImplementation((selector?: unknown) => {
      const state = createCentralSkillsState({
        loadCentralSkills: mockLoadCentralSkills,
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });

    render(
      <MemoryRouter initialEntries={["/a"]}>
        <Routes>
          <Route path="/" element={<AppShell />}>
            <Route path="a" element={<DummyPage label="page-a" />} />
          </Route>
        </Routes>
      </MemoryRouter>
    );

    await act(async () => {
      screen.getByRole("button", { name: /open-search/i }).click();
    });

    await act(async () => {
      screen.getByRole("button", { name: /trigger-rescan/i }).click();
    });

    expect(mockRescan).toHaveBeenCalledTimes(1);
    expect(mockLoadCentralSkills).not.toHaveBeenCalled();

    resolveRescan();

    await waitFor(() => {
      expect(mockLoadCentralSkills).toHaveBeenCalledTimes(1);
    });
  });

  it("clears stale counts and reloads target-bound data after active target changes", async () => {
    const mockInitialize = vi.fn().mockResolvedValue(undefined);
    const mockResetPlatformForTargetChange = vi.fn();
    const mockResetCentralForTargetChange = vi.fn();
    const mockResetSkillsForTargetChange = vi.fn();
    const mockResetMarketplaceForTargetChange = vi.fn();
    const mockRescan = vi.fn().mockResolvedValue(undefined);
    const mockLoadCentralSkills = vi.fn().mockResolvedValue(undefined);
    let activeTarget: TargetSummary = {
      id: "local",
      kind: "local" as const,
      label: "Local",
      isActive: true,
    };

    mockUsePlatformStore.mockImplementation((selector?: unknown) => {
      const state = createPlatformState({
        initialize: mockInitialize,
        rescan: mockRescan,
        resetForTargetChange: mockResetPlatformForTargetChange,
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });
    mockUseCentralSkillsStore.mockImplementation((selector?: unknown) => {
      const state = createCentralSkillsState({
        loadCentralSkills: mockLoadCentralSkills,
        resetForTargetChange: mockResetCentralForTargetChange,
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });
    mockUseSkillStore.mockImplementation((selector?: unknown) => {
      const state = createSkillState({
        resetForTargetChange: mockResetSkillsForTargetChange,
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });
    mockUseMarketplaceStore.mockImplementation((selector?: unknown) => {
      const state = createMarketplaceState({
        resetForTargetChange: mockResetMarketplaceForTargetChange,
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });
    mockUseTargetStore.mockImplementation((selector?: unknown) => {
      const state = createTargetState({ activeTarget });
      if (typeof selector === "function") return selector(state);
      return state;
    });

    const { rerender } = render(
      <MemoryRouter initialEntries={["/a"]}>
        <Routes>
          <Route path="/" element={<AppShell />}>
            <Route path="a" element={<DummyPage label="page-a" />} />
          </Route>
        </Routes>
      </MemoryRouter>
    );

    await waitFor(() => {
      expect(mockInitialize).toHaveBeenCalledTimes(1);
    });

    activeTarget = {
      id: "ssh-demo",
      kind: "ssh" as const,
      label: "Demo",
      isActive: true,
    };

    rerender(
      <MemoryRouter initialEntries={["/a"]}>
        <Routes>
          <Route path="/" element={<AppShell />}>
            <Route path="a" element={<DummyPage label="page-a" />} />
          </Route>
        </Routes>
      </MemoryRouter>
    );

    await waitFor(() => {
      expect(mockResetPlatformForTargetChange).toHaveBeenCalledTimes(1);
      expect(mockResetCentralForTargetChange).toHaveBeenCalledTimes(1);
      expect(mockResetSkillsForTargetChange).toHaveBeenCalledTimes(1);
      expect(mockResetMarketplaceForTargetChange).toHaveBeenCalledTimes(1);
      expect(mockRescan).toHaveBeenCalledTimes(1);
      expect(mockLoadCentralSkills).toHaveBeenCalledTimes(1);
    });
  });

  it("does not reset target-bound stores when the saved SSH target loads during startup", async () => {
    const mockInitialize = vi.fn().mockResolvedValue(undefined);
    const mockResetPlatformForTargetChange = vi.fn();
    const mockResetCentralForTargetChange = vi.fn();
    const mockResetSkillsForTargetChange = vi.fn();
    const mockResetMarketplaceForTargetChange = vi.fn();
    const mockRescan = vi.fn().mockResolvedValue(undefined);
    const sshTarget: TargetSummary = {
      id: "ssh-demo",
      kind: "ssh",
      label: "Demo",
      isActive: true,
    };
    let activeTarget: TargetSummary = {
      id: "local",
      kind: "local",
      label: "Local",
      isActive: true,
    };
    const mockLoadTargets = vi.fn().mockImplementation(async () => {
      activeTarget = sshTarget;
    });

    mockUsePlatformStore.mockImplementation((selector?: unknown) => {
      const state = createPlatformState({
        initialize: mockInitialize,
        rescan: mockRescan,
        resetForTargetChange: mockResetPlatformForTargetChange,
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });
    mockUseCentralSkillsStore.mockImplementation((selector?: unknown) => {
      const state = createCentralSkillsState({
        resetForTargetChange: mockResetCentralForTargetChange,
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });
    mockUseSkillStore.mockImplementation((selector?: unknown) => {
      const state = createSkillState({
        resetForTargetChange: mockResetSkillsForTargetChange,
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });
    mockUseMarketplaceStore.mockImplementation((selector?: unknown) => {
      const state = createMarketplaceState({
        resetForTargetChange: mockResetMarketplaceForTargetChange,
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });
    mockUseTargetStore.mockImplementation((selector?: unknown) => {
      const state = createTargetState({
        activeTarget,
        loadTargets: mockLoadTargets,
      });
      if (typeof selector === "function") return selector(state);
      return state;
    });

    render(
      <MemoryRouter initialEntries={["/a"]}>
        <Routes>
          <Route path="/" element={<AppShell />}>
            <Route path="a" element={<DummyPage label="page-a" />} />
          </Route>
        </Routes>
      </MemoryRouter>
    );

    await waitFor(() => {
      expect(mockInitialize).toHaveBeenCalledTimes(1);
    });

    expect(mockLoadTargets).toHaveBeenCalledTimes(1);
    expect(mockResetPlatformForTargetChange).not.toHaveBeenCalled();
    expect(mockResetCentralForTargetChange).not.toHaveBeenCalled();
    expect(mockResetSkillsForTargetChange).not.toHaveBeenCalled();
    expect(mockResetMarketplaceForTargetChange).not.toHaveBeenCalled();
    expect(mockRescan).not.toHaveBeenCalled();
  });

  it("shows a toast when legacy Central migration completes with copied or failed items", async () => {
    let migrationHandler: ((event: { payload: { phase: "completed"; copied: number; skipped: number; failed: number } }) => void) | undefined;
    mockListen.mockImplementation(async (eventName, handler) => {
      if (eventName === "system://migration-progress") {
        migrationHandler = handler as typeof migrationHandler;
      }
      return () => {};
    });

    render(
      <MemoryRouter initialEntries={["/a"]}>
        <Routes>
          <Route path="/" element={<AppShell />}>
            <Route path="a" element={<DummyPage label="page-a" />} />
          </Route>
        </Routes>
      </MemoryRouter>
    );

    await waitFor(() => {
      expect(mockListen).toHaveBeenCalled();
    });

    act(() => {
      migrationHandler?.({
        payload: { phase: "completed", copied: 2, skipped: 1, failed: 0 },
      });
    });

    expect(toast.success).toHaveBeenCalledWith(
      "旧 Central 仓库迁移完成：复制 2 个，跳过 1 个，失败 0 个"
    );
  });

  it("shows an error toast when legacy Central migration fails", async () => {
    let migrationHandler: ((event: { payload: { phase: "failed"; error: string } }) => void) | undefined;
    mockListen.mockImplementation(async (eventName, handler) => {
      if (eventName === "system://migration-progress") {
        migrationHandler = handler as typeof migrationHandler;
      }
      return () => {};
    });

    render(
      <MemoryRouter initialEntries={["/a"]}>
        <Routes>
          <Route path="/" element={<AppShell />}>
            <Route path="a" element={<DummyPage label="page-a" />} />
          </Route>
        </Routes>
      </MemoryRouter>
    );

    await waitFor(() => {
      expect(mockListen).toHaveBeenCalled();
    });

    act(() => {
      migrationHandler?.({
        payload: { phase: "failed", error: "copy failed" },
      });
    });

    expect(toast.error).toHaveBeenCalledWith(
      "旧 Central 仓库迁移失败：copy failed"
    );
  });
});
