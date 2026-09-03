import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { MemoryRouter, useLocation } from "react-router-dom";

import { GlobalSearchDialog } from "@/components/layout/GlobalSearchDialog";
import { useCentralSkillsStore } from "@/stores/centralSkillsStore";
import { useCollectionStore } from "@/stores/collectionStore";
import { usePlatformStore } from "@/stores/platformStore";

const ASYNC_UI_TIMEOUT_MS = 5_000;

vi.mock("@/hooks/useHotkey", () => ({
  useHotkey: vi.fn(),
}));

vi.mock("@/components/ui/command", () => ({
  Command: ({ children }: { children: ReactNode }) => <div>{children}</div>,
  CommandDialog: ({
    open,
    children,
  }: {
    open: boolean;
    children: ReactNode;
  }) => (open ? <div role="dialog">{children}</div> : null),
  CommandEmpty: ({ children }: { children: ReactNode }) => <div>{children}</div>,
  CommandGroup: ({
    heading,
    children,
  }: {
    heading: ReactNode;
    children: ReactNode;
  }) => (
    <section>
      <h2>{heading}</h2>
      {children}
    </section>
  ),
  CommandInput: ({
    placeholder,
    value,
    onValueChange,
  }: {
    placeholder: string;
    value: string;
    onValueChange: (value: string) => void;
  }) => (
    <input
      placeholder={placeholder}
      value={value}
      onChange={(event) => onValueChange(event.target.value)}
    />
  ),
  CommandItem: ({
    children,
    onSelect,
  }: {
    children: ReactNode;
    onSelect: () => void;
  }) => (
    <button type="button" onClick={onSelect}>
      {children}
    </button>
  ),
  CommandList: ({ children }: { children: ReactNode }) => <div>{children}</div>,
}));

vi.mock("@/stores/centralSkillsStore", () => ({
  useCentralSkillsStore: vi.fn(),
}));

vi.mock("@/stores/collectionStore", () => ({
  useCollectionStore: vi.fn(),
}));

vi.mock("@/stores/platformStore", () => ({
  usePlatformStore: vi.fn(),
}));

const mockUseCentralSkillsStore = vi.mocked(useCentralSkillsStore);
const mockUseCollectionStore = vi.mocked(useCollectionStore);
const mockUsePlatformStore = vi.mocked(usePlatformStore);

const mockLoadCollections = vi.fn();
const mockLoadCentralSkills = vi.fn();

type CollectionMockState = {
  collections: Array<{
    id: string;
    name: string;
    description: string | null;
  }>;
  hasLoaded: boolean;
  isLoading: boolean;
  error: string | null;
  loadCollections: typeof mockLoadCollections;
};

type CentralMockState = {
  skills: Array<{
    id: string;
    name: string;
    description: string | null;
  }>;
  hasLoaded: boolean;
  isLoading: boolean;
  error: string | null;
  loadCentralSkills: typeof mockLoadCentralSkills;
};

let collectionState: CollectionMockState;
let centralState: CentralMockState;

function LocationProbe() {
  const location = useLocation();
  return (
    <div
      data-testid="location"
      data-pathname={location.pathname}
      data-state={JSON.stringify(location.state ?? null)}
    >
      {location.pathname}
    </div>
  );
}

function dialogUi(open: boolean, onOpenChange: (open: boolean) => void, onAction: () => void) {
  return (
    <MemoryRouter initialEntries={["/central"]}>
      <GlobalSearchDialog open={open} onOpenChange={onOpenChange} onAction={onAction} />
      <LocationProbe />
    </MemoryRouter>
  );
}

function renderDialog(open = true) {
  const onOpenChange = vi.fn();
  const onAction = vi.fn();
  const view = render(dialogUi(open, onOpenChange, onAction));
  return { ...view, onOpenChange, onAction };
}

function defaultPlatformState() {
  return {
    agents: [
      {
        id: "codex",
        display_name: "Codex CLI",
        category: "coding",
        global_skills_dir: "/Users/test/.agents/skills/",
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
    ],
    categoryVisibility: {
      coding: true,
      lobster: true,
    },
  };
}

function bindStoreMocks() {
  mockUseCollectionStore.mockImplementation((selector?: unknown) => {
    if (typeof selector === "function") return selector(collectionState);
    return collectionState;
  });
  Object.assign(mockUseCollectionStore, {
    getState: () => collectionState,
  });

  mockUseCentralSkillsStore.mockImplementation((selector?: unknown) => {
    if (typeof selector === "function") return selector(centralState);
    return centralState;
  });
  Object.assign(mockUseCentralSkillsStore, {
    getState: () => centralState,
  });

  mockUsePlatformStore.mockImplementation((selector?: unknown) => {
    const state = defaultPlatformState();
    if (typeof selector === "function") return selector(state);
    return state;
  });
}

describe("GlobalSearchDialog", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    collectionState = {
      collections: [],
      hasLoaded: true,
      isLoading: false,
      error: null,
      loadCollections: mockLoadCollections,
    };
    centralState = {
      skills: [],
      hasLoaded: true,
      isLoading: false,
      error: null,
      loadCentralSkills: mockLoadCentralSkills,
    };
    bindStoreMocks();
  });

  it("routes Universal search results to the Universal platform page", () => {
    const { onOpenChange } = renderDialog();

    fireEvent.click(screen.getByRole("button", { name: /Universal/i }));

    expect(screen.getByTestId("location")).toHaveTextContent("/platform/universal-agents");
    expect(onOpenChange).toHaveBeenCalledWith(false);
  });

  it("routes Dashboard action results to the Dashboard page", () => {
    const { onOpenChange } = renderDialog();

    fireEvent.click(screen.getByRole("button", { name: /Dashboard/i }));

    expect(screen.getByTestId("location")).toHaveTextContent("/dashboard");
    expect(onOpenChange).toHaveBeenCalledWith(false);
  });

  it("routes collection results to /collections with collectionContext state", () => {
    collectionState.collections = [
      {
        id: "col-search",
        name: "Search Target Collection",
        description: null,
      },
    ];
    const { onOpenChange } = renderDialog();

    fireEvent.click(
      screen.getByRole("button", { name: /Search Target Collection/i }),
    );

    const probe = screen.getByTestId("location");
    expect(probe).toHaveTextContent("/collections");
    expect(probe.getAttribute("data-pathname")).toBe("/collections");
    expect(JSON.parse(probe.getAttribute("data-state") ?? "null")).toEqual({
      collectionContext: { collectionId: "col-search" },
    });
    expect(onOpenChange).toHaveBeenCalledWith(false);
    expect(probe).not.toHaveTextContent("/collection/col-search");
  });

  it("loads collections and Central once on a cold-start open", async () => {
    collectionState.hasLoaded = false;
    centralState.hasLoaded = false;

    renderDialog();

    await waitFor(
      () => {
        expect(mockLoadCollections).toHaveBeenCalledTimes(1);
        expect(mockLoadCentralSkills).toHaveBeenCalledTimes(1);
      },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
  });

  it("does not reload after a successful empty load when the dialog is reopened", async () => {
    collectionState.hasLoaded = true;
    collectionState.collections = [];
    centralState.hasLoaded = true;
    centralState.skills = [];

    const onOpenChange = vi.fn();
    const onAction = vi.fn();
    const { rerender } = render(dialogUi(true, onOpenChange, onAction));

    expect(mockLoadCollections).not.toHaveBeenCalled();
    expect(mockLoadCentralSkills).not.toHaveBeenCalled();

    rerender(dialogUi(false, onOpenChange, onAction));
    rerender(dialogUi(true, onOpenChange, onAction));

    await waitFor(
      () => {
        expect(mockLoadCollections).not.toHaveBeenCalled();
        expect(mockLoadCentralSkills).not.toHaveBeenCalled();
      },
      { timeout: ASYNC_UI_TIMEOUT_MS },
    );
  });

  it("shows a localized load error and retry instead of no-results when a source fails", async () => {
    collectionState.hasLoaded = false;
    collectionState.error = "DB error";
    centralState.hasLoaded = true;

    const { onOpenChange, onAction, rerender } = renderDialog();
    const dialog = await screen.findByRole("dialog", {}, { timeout: ASYNC_UI_TIMEOUT_MS });
    const collectionsGroup = within(dialog).getByRole("heading", {
      name: "技能集合",
    }).parentElement;
    expect(collectionsGroup).not.toBeNull();
    const group = within(collectionsGroup as HTMLElement);

    expect(group.getByRole("alert")).toHaveTextContent("加载失败：DB error");
    expect(group.getByRole("button", { name: "重试" })).toBeInTheDocument();
    expect(screen.queryByText("未找到结果。")).not.toBeInTheDocument();

    mockLoadCollections.mockClear();
    fireEvent.click(group.getByRole("button", { name: "重试" }));
    expect(mockLoadCollections).toHaveBeenCalledTimes(1);

    collectionState.hasLoaded = true;
    collectionState.error = null;
    collectionState.collections = [
      { id: "col-retry", name: "Retry Collection", description: null },
    ];
    rerender(dialogUi(true, onOpenChange, onAction));

    expect(screen.getByRole("button", { name: /Retry Collection/i })).toBeInTheDocument();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(screen.queryByText("未找到结果。")).not.toBeInTheDocument();
  });

  it("shows loaded-empty rather than loading or error when both sources loaded empty", () => {
    collectionState.hasLoaded = true;
    collectionState.collections = [];
    centralState.hasLoaded = true;
    centralState.skills = [];

    renderDialog();

    expect(screen.queryByRole("status")).not.toBeInTheDocument();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
    expect(screen.getAllByText("暂无内容。").length).toBeGreaterThanOrEqual(1);
    expect(screen.queryByText("未找到结果。")).not.toBeInTheDocument();
  });
});
