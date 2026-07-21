import React from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { SkillDetailModalShell } from "@/components/skill/SkillDetailModalShell";

function TestHarness({
  initialOpen = true,
  titleId,
  maxWidth,
  maxHeight,
  headerActions,
}: {
  initialOpen?: boolean;
  titleId?: string;
  maxWidth?: string;
  maxHeight?: string;
  headerActions?: React.ReactNode;
}) {
  const [open, setOpen] = React.useState(initialOpen);
  const triggerRef = React.useRef<HTMLButtonElement>(null);

  return (
    <>
      <button ref={triggerRef} onClick={() => setOpen(true)}>
        Open modal
      </button>
      <SkillDetailModalShell
        open={open}
        onOpenChange={setOpen}
        returnFocusRef={triggerRef}
        titleId={titleId}
        maxWidth={maxWidth}
        maxHeight={maxHeight}
        headerActions={headerActions}
      >
        <h2 id="test-title">Test Title</h2>
        <p>Modal content</p>
      </SkillDetailModalShell>
    </>
  );
}

describe("SkillDetailModalShell", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("renders dialog with role='dialog' and aria-modal='true' when open=true", async () => {
    render(<TestHarness titleId="test-title" />);

    const dialog = await screen.findByTestId("skill-detail-modal");
    expect(dialog).toHaveAttribute("role", "dialog");
    expect(dialog).toHaveAttribute("aria-modal", "true");
  });

  it("does not render dialog when open=false", () => {
    render(<TestHarness initialOpen={false} />);
    expect(screen.queryByTestId("skill-detail-modal")).not.toBeInTheDocument();
  });

  it("aria-labelledby correctly associates with titleId", async () => {
    render(<TestHarness titleId="test-title" />);

    const dialog = await screen.findByTestId("skill-detail-modal");
    expect(dialog).toHaveAttribute("aria-labelledby", "test-title");
  });

  it("close button exists and triggers onOpenChange(false) on click", async () => {
    render(<TestHarness titleId="test-title" />);

    const closeButton = await screen.findByRole("button", { name: /关闭/i });
    expect(closeButton).toBeInTheDocument();

    fireEvent.click(closeButton);

    await waitFor(() => {
      expect(screen.queryByTestId("skill-detail-modal")).not.toBeInTheDocument();
    });
  });

  it("overlay click triggers onOpenChange(false)", async () => {
    render(<TestHarness titleId="test-title" />);

    const overlay = await screen.findByTestId("skill-detail-modal-overlay");
    expect(overlay).toBeInTheDocument();

    fireEvent.click(overlay);

    await waitFor(() => {
      expect(screen.queryByTestId("skill-detail-modal")).not.toBeInTheDocument();
    });
  });

  it("Escape key triggers onOpenChange(false)", async () => {
    render(<TestHarness titleId="test-title" />);

    const dialog = await screen.findByTestId("skill-detail-modal");
    fireEvent.keyDown(dialog, { key: "Escape" });

    await waitFor(() => {
      expect(screen.queryByTestId("skill-detail-modal")).not.toBeInTheDocument();
    });
  });

  it("custom maxWidth/maxHeight set CSS variables correctly", async () => {
    render(<TestHarness titleId="test-title" maxWidth="900px" maxHeight="600px" />);

    const dialog = await screen.findByTestId("skill-detail-modal");
    const style = dialog.style;
    expect(style.getPropertyValue("--modal-max-w")).toBe("900px");
    expect(style.getPropertyValue("--modal-max-h")).toBe("600px");
  });

  it("default CSS variable values when props not provided", async () => {
    render(<TestHarness titleId="test-title" />);

    const dialog = await screen.findByTestId("skill-detail-modal");
    const style = dialog.style;
    expect(style.getPropertyValue("--modal-max-w")).toBe("1200px");
    expect(style.getPropertyValue("--modal-max-h")).toBe("800px");
  });

  it("headerActions slot renders provided content", async () => {
    render(
      <TestHarness
        titleId="test-title"
        headerActions={<button data-testid="custom-action">Install</button>}
      />
    );

    expect(await screen.findByTestId("custom-action")).toBeInTheDocument();
    expect(screen.getByTestId("custom-action")).toHaveTextContent("Install");
  });
});
