import { fireEvent, render, screen } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { MemoryRouter, useLocation } from "react-router-dom";

import { GlobalSearchDialog } from "@/components/layout/GlobalSearchDialog";
import { useCentralSkillsStore } from "@/stores/centralSkillsStore";
import { useCollectionStore } from "@/stores/collectionStore";
import { useDiscoverStore } from "@/stores/discoverStore";
import { usePlatformStore } from "@/stores/platformStore";

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
  }) => (open ? <div>{children}</div> : null),
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

vi.mock("@/stores/discoverStore", () => ({
  useDiscoverStore: vi.fn(),
}));

vi.mock("@/stores/collectionStore", () => ({
  useCollectionStore: vi.fn(),
}));

vi.mock("@/stores/platformStore", () => ({
  usePlatformStore: vi.fn(),
}));

const mockUseCentralSkillsStore = vi.mocked(useCentralSkillsStore);
const mockUseDiscoverStore = vi.mocked(useDiscoverStore);
const mockUseCollectionStore = vi.mocked(useCollectionStore);
const mockUsePlatformStore = vi.mocked(usePlatformStore);

function LocationProbe() {
  const location = useLocation();
  return <div data-testid="location">{location.pathname}</div>;
}

function renderDialog() {
  const onOpenChange = vi.fn();
  const onAction = vi.fn();

  render(
    <MemoryRouter initialEntries={["/central"]}>
      <GlobalSearchDialog open onOpenChange={onOpenChange} onAction={onAction} />
      <LocationProbe />
    </MemoryRouter>
  );

  return { onOpenChange, onAction };
}

describe("GlobalSearchDialog", () => {
  beforeEach(() => {
    vi.clearAllMocks();

    mockUseCentralSkillsStore.mockImplementation((selector?: unknown) => {
      const state = { skills: [] };
      if (typeof selector === "function") return selector(state);
      return state;
    });
    mockUseDiscoverStore.mockImplementation((selector?: unknown) => {
      const state = { discoveredProjects: [] };
      if (typeof selector === "function") return selector(state);
      return state;
    });
    mockUseCollectionStore.mockImplementation((selector?: unknown) => {
      const state = { collections: [] };
      if (typeof selector === "function") return selector(state);
      return state;
    });
    mockUsePlatformStore.mockImplementation((selector?: unknown) => {
      const state = {
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
      if (typeof selector === "function") return selector(state);
      return state;
    });
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
});
