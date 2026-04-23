import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { InlineConfirmAction } from "@/components/ui/inline-confirm-action";

describe("InlineConfirmAction", () => {
  it("arms confirmation on first click and confirms on second click", async () => {
    const onConfirm = vi.fn().mockResolvedValue(undefined);

    render(
      <InlineConfirmAction
        idleAriaLabel="Remove item"
        confirmLabel="Confirm delete"
        onConfirm={onConfirm}
        icon={<span>x</span>}
      />
    );

    fireEvent.click(screen.getByRole("button", { name: "Remove item" }));

    expect(onConfirm).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: "Confirm delete" })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Confirm delete" }));

    await waitFor(() => {
      expect(onConfirm).toHaveBeenCalledTimes(1);
    });
  });

  it("closes the confirmation state when clicking outside", async () => {
    render(
      <div>
        <InlineConfirmAction
          idleAriaLabel="Remove item"
          confirmLabel="Confirm delete"
          onConfirm={vi.fn()}
          icon={<span>x</span>}
        />
        <button type="button">Outside</button>
      </div>
    );

    fireEvent.click(screen.getByRole("button", { name: "Remove item" }));
    expect(screen.getByRole("button", { name: "Confirm delete" })).toBeInTheDocument();

    fireEvent.pointerDown(screen.getByRole("button", { name: "Outside" }));

    await waitFor(() => {
      expect(
        screen.queryByRole("button", { name: "Confirm delete" })
      ).not.toBeInTheDocument();
    });
    expect(screen.getByRole("button", { name: "Remove item" })).toBeInTheDocument();
  });
});
