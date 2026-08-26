import { createRef } from "react";
import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";

import {
  SKILLS_CLI_INSTALL_SURFACE_AVAILABLE,
  SkillsCliInstallMount,
  renderSkillsCliInstallMount,
} from "@/components/skillsCli/SkillsCliInstallMount";

describe("SkillsCliInstallMount", () => {
  it("starts unavailable and renders null", () => {
    expect(SKILLS_CLI_INSTALL_SURFACE_AVAILABLE).toBe(false);
    const { container } = render(
      <SkillsCliInstallMount
        open
        onOpenChange={vi.fn()}
        returnFocusRef={createRef<HTMLElement | null>()}
        contentWidthPx={1180}
      />,
    );
    expect(container).toBeEmptyDOMElement();
    expect(screen.queryByTestId("skills-cli-install-mount")).not.toBeInTheDocument();
  });

  it("passes open, close, return-focus, and content width through when available", () => {
    const onOpenChange = vi.fn();
    const returnFocusRef = createRef<HTMLButtonElement | null>();
    const { rerender } = render(
      <>
        {renderSkillsCliInstallMount(
          {
            open: true,
            onOpenChange,
            returnFocusRef,
            contentWidthPx: 900,
          },
          true,
        )}
      </>,
    );
    const mount = screen.getByTestId("skills-cli-install-mount");
    expect(mount).toHaveAttribute("data-open", "true");
    expect(mount).toHaveAttribute("data-content-width", "900");
    expect(mount).toHaveAttribute("data-has-return-focus", "true");

    rerender(
      <>
        {renderSkillsCliInstallMount(
          {
            open: false,
            onOpenChange,
            returnFocusRef,
            contentWidthPx: null,
          },
          true,
        )}
      </>,
    );
    expect(screen.getByTestId("skills-cli-install-mount")).toHaveAttribute(
      "data-open",
      "false",
    );
    expect(screen.getByTestId("skills-cli-install-mount")).toHaveAttribute(
      "data-content-width",
      "",
    );
  });
});
