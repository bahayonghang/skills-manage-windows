import { render, screen } from "@testing-library/react";
import type { ButtonHTMLAttributes, ReactNode } from "react";
import { MemoryRouter } from "react-router-dom";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { TopBar } from "@/components/layout/TopBar";
import { usePlatformStore } from "@/stores/platformStore";
import { useThemeStore } from "@/stores/themeStore";
import { setNavigatorPlatform } from "./testPlatform";

vi.mock("@/components/layout/TargetQuickSwitcher", () => ({
  TargetQuickSwitcher: () => <div data-testid="target-switcher" />,
}));

vi.mock("@/components/ui/button", () => ({
  Button: ({
    children,
    size: _size,
    variant: _variant,
    ...props
  }: {
    children: ReactNode;
    size?: string;
    variant?: string;
  } & ButtonHTMLAttributes<HTMLButtonElement>) => <button {...props}>{children}</button>,
}));

vi.mock("@/stores/platformStore", () => ({
  usePlatformStore: vi.fn(),
}));

vi.mock("@/stores/themeStore", () => ({
  useThemeStore: vi.fn(),
}));

const mockUsePlatformStore = vi.mocked(usePlatformStore);
const mockUseThemeStore = vi.mocked(useThemeStore);

let restorePlatform: (() => void) | undefined;

function setPlatform(platform: string) {
  restorePlatform?.();
  restorePlatform = setNavigatorPlatform(platform);
}

function renderTopBar() {
  render(
    <MemoryRouter initialEntries={["/central"]}>
      <TopBar onSearchClick={vi.fn()} />
    </MemoryRouter>,
  );
}

describe("TopBar shortcut hint", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockUsePlatformStore.mockImplementation((selector?: unknown) => {
      const state = {
        agents: [],
        skillsByAgent: { central: 3 },
        categoryVisibility: undefined,
      };
      if (typeof selector === "function") return selector(state);
      return state;
    });
    mockUseThemeStore.mockImplementation((selector?: unknown) => {
      const state = {
        flavor: "mocha",
        setFlavor: vi.fn(),
      };
      if (typeof selector === "function") return selector(state);
      return state;
    });
  });

  afterEach(() => {
    restorePlatform?.();
    restorePlatform = undefined;
  });

  it("renders Ctrl+K on non-macOS", () => {
    setPlatform("Win32");

    renderTopBar();

    expect(screen.getByLabelText("Ctrl+K")).toHaveTextContent("CtrlK");
    expect(screen.queryByLabelText("Command+K")).not.toBeInTheDocument();
  });

  it("renders Command+K on macOS", () => {
    setPlatform("MacIntel");

    renderTopBar();

    expect(screen.getByLabelText("Command+K")).toHaveTextContent("⌘K");
  });
});
