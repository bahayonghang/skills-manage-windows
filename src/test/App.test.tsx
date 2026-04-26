import { describe, it, expect, vi } from "vitest";
import { render, screen, act } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import App from "../App";

// Mock platformStore to prevent real Tauri invoke calls during tests
vi.mock("@/stores/platformStore", () => ({
  usePlatformStore: vi.fn().mockImplementation((selector?: unknown) => {
    const state = {
      agents: [],
      skillsByAgent: {},
      collectionCount: 0,
      discoveredCount: 0,
      lastScanAt: null,
      scanState: "idle",
      isLoading: false,
      isRefreshing: false,
      error: null,
      initialize: vi.fn().mockResolvedValue(undefined),
      hydrateShell: vi.fn().mockResolvedValue(undefined),
      refreshScanInBackground: vi.fn().mockResolvedValue(undefined),
      rescan: vi.fn().mockResolvedValue(undefined),
      refreshCounts: vi.fn().mockResolvedValue(undefined),
      resetForTargetChange: vi.fn(),
      applyScanSummary: vi.fn(),
      setCollectionCount: vi.fn(),
      setDiscoveredCount: vi.fn(),
    };
    if (typeof selector === "function") {
      return selector(state);
    }
    return state;
  }),
}));

// Mock collectionStore to prevent real Tauri invoke calls during tests
vi.mock("@/stores/collectionStore", () => ({
  useCollectionStore: vi.fn().mockImplementation((selector?: unknown) => {
    const state = {
      collections: [],
      currentDetail: null,
      isLoading: false,
      isLoadingDetail: false,
      error: null,
      loadCollections: vi.fn(),
      createCollection: vi.fn(),
      updateCollection: vi.fn(),
      deleteCollection: vi.fn(),
      loadCollectionDetail: vi.fn(),
      addSkillToCollection: vi.fn(),
      removeSkillFromCollection: vi.fn(),
      batchInstallCollection: vi.fn(),
      exportCollection: vi.fn(),
      importCollection: vi.fn(),
    };
    if (typeof selector === "function") {
      return selector(state);
    }
    return state;
  }),
}));

// Mock centralSkillsStore to prevent async state updates that cause act() warnings
vi.mock("@/stores/centralSkillsStore", () => ({
  useCentralSkillsStore: vi.fn().mockImplementation((selector?: unknown) => {
    const state = {
      skills: [],
      agents: [],
      isLoading: false,
      isInstalling: false,
      error: null,
      loadCentralSkills: vi.fn().mockResolvedValue(undefined),
      installSkill: vi.fn(),
      resetForTargetChange: vi.fn(),
    };
    if (typeof selector === "function") {
      return selector(state);
    }
    return state;
  }),
}));

vi.mock("@/stores/discoverStore", () => ({
  useDiscoverStore: vi.fn().mockImplementation((selector?: unknown) => {
    const state = {
      rescanFromDisk: vi.fn().mockResolvedValue(undefined),
      refreshCounts: vi.fn().mockResolvedValue(undefined),
      resetForTargetChange: vi.fn(),
    };
    if (typeof selector === "function") {
      return selector(state);
    }
    return state;
  }),
}));

vi.mock("@/stores/targetStore", () => ({
  useTargetStore: vi.fn().mockImplementation((selector?: unknown) => {
    const state = {
      activeTarget: {
        id: "local",
        kind: "local",
        label: "Local",
        isActive: true,
      },
      targets: [],
      loadTargets: vi.fn().mockResolvedValue(undefined),
    };
    if (typeof selector === "function") {
      return selector(state);
    }
    return state;
  }),
}));

vi.mock("@/stores/skillStore", () => ({
  useSkillStore: vi.fn().mockImplementation((selector?: unknown) => {
    const state = {
      getSkillsByAgent: vi.fn().mockResolvedValue(undefined),
      resetForTargetChange: vi.fn(),
    };
    if (typeof selector === "function") {
      return selector(state);
    }
    return state;
  }),
}));

vi.mock("@/stores/marketplaceStore", () => ({
  useMarketplaceStore: vi.fn().mockImplementation((selector?: unknown) => {
    const state = {
      githubImport: {
        importProgress: null,
        importStartedAt: null,
        skillMarkdown: {},
        aiSummaries: {},
      },
      resetForTargetChange: vi.fn(),
      loadRegistries: vi.fn().mockResolvedValue(undefined),
      fetchGitHubSkillMarkdown: vi.fn().mockResolvedValue(undefined),
      generateGitHubImportAiSummary: vi.fn().mockResolvedValue(undefined),
    };
    if (typeof selector === "function") {
      return selector(state);
    }
    return state;
  }),
}));

describe("App", () => {
  it("renders the app shell with top bar", async () => {
    await act(async () => {
      render(
        <MemoryRouter initialEntries={["/central"]}>
          <App />
        </MemoryRouter>
      );
    });
    // TopBar shows the app name
    expect(screen.getByText("SkillPort")).toBeInTheDocument();
  });

  it("renders sidebar with icon-only navigation", async () => {
    await act(async () => {
      render(
        <MemoryRouter initialEntries={["/central"]}>
          <App />
        </MemoryRouter>
      );
    });
    // "中央技能库" appears as icon button tooltip in sidebar + possibly in main content header
    expect(screen.getAllByText("中央技能库").length).toBeGreaterThanOrEqual(1);
    // Icon-only sidebar has no "By Tool" section header
    expect(screen.queryByText("按工具")).not.toBeInTheDocument();
  });
});
