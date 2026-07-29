import { render, screen } from "@testing-library/react";
import type { TFunction } from "i18next";
import { afterEach, describe, expect, it, vi } from "vitest";

import { CentralSearchBar } from "@/components/central/CentralSearchBar";
import type { CentralQueryAst } from "@/types";
import { setNavigatorPlatform } from "@/test/support/testPlatform";

const emptyQueryAst: CentralQueryAst = {
  freeText: "",
  filters: [],
  invalid: [],
};

const t = ((key: string) => key) as TFunction;

let restorePlatform: (() => void) | undefined;

function renderSearchBar() {
  render(
    <CentralSearchBar
      t={t}
      value=""
      onChange={vi.fn()}
      queryAst={emptyQueryAst}
      onRemoveFilter={vi.fn()}
      onClear={vi.fn()}
      onOpenPalette={vi.fn()}
    />,
  );
}

function setPlatform(platform: string) {
  restorePlatform?.();
  restorePlatform = setNavigatorPlatform(platform);
}

describe("CentralSearchBar shortcut hint", () => {
  afterEach(() => {
    restorePlatform?.();
    restorePlatform = undefined;
  });

  it("renders Ctrl+K on non-macOS", () => {
    setPlatform("Win32");

    renderSearchBar();

    const button = screen.getByTestId("central-open-palette");
    expect(button).toHaveTextContent("CtrlK");
    expect(button).not.toHaveTextContent("⌘K");
    expect(screen.getByLabelText("Ctrl+K")).toBeInTheDocument();
  });

  it("renders Command+K on macOS", () => {
    setPlatform("MacIntel");

    renderSearchBar();

    const button = screen.getByTestId("central-open-palette");
    expect(button).toHaveTextContent("⌘K");
    expect(screen.getByLabelText("Command+K")).toBeInTheDocument();
  });
});
