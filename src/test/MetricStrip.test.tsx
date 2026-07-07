import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import "@/i18n";
import { MetricStrip } from "@/components/dashboard/sections/MetricStrip";

function renderStrip() {
  const onNavigate = vi.fn();
  render(
    <MetricStrip
      onNavigate={onNavigate}
      centralTotal={12}
      aiReviewCount={3}
      sourceRepositoryCount={5}
      collectionCount={2}
    />,
  );
  const strip = screen.getByRole("region", {
    name: /Library metrics|技能库指标/,
  });
  return { onNavigate, strip };
}

describe("MetricStrip", () => {
  it("renders four compact stat buttons inside a single strip", () => {
    const { strip } = renderStrip();

    const buttons = within(strip).getAllByRole("button");
    expect(buttons).toHaveLength(4);
    for (const button of buttons) {
      expect(button).toHaveClass("focus-ring");
    }

    expect(screen.getByTestId("dashboard-metric-central")).toHaveTextContent(
      "12",
    );
    expect(screen.getByTestId("dashboard-metric-ai-stat")).toHaveTextContent(
      "3",
    );
    expect(
      screen.getByTestId("dashboard-metric-collections"),
    ).toHaveTextContent("2");
  });

  it("navigates to the mapped route for each stat", () => {
    const { onNavigate, strip } = renderStrip();

    fireEvent.click(screen.getByTestId("dashboard-metric-central"));
    expect(onNavigate).toHaveBeenLastCalledWith("/central");

    fireEvent.click(screen.getByTestId("dashboard-metric-ai-stat"));
    expect(onNavigate).toHaveBeenLastCalledWith("/central?filter=ai-review");

    // The sources item has no testId (existing behavior); it is the third button.
    fireEvent.click(within(strip).getAllByRole("button")[2]);
    expect(onNavigate).toHaveBeenLastCalledWith("/marketplace");

    fireEvent.click(screen.getByTestId("dashboard-metric-collections"));
    expect(onNavigate).toHaveBeenLastCalledWith("/collections");

    expect(onNavigate).toHaveBeenCalledTimes(4);
  });
});
