import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { Button } from "@/components/ui/button";

describe("Button", () => {
  it("renders button with text", () => {
    render(<Button>Click me</Button>);
    expect(screen.getByText("Click me")).toBeInTheDocument();
  });

  it("renders button with default variant", () => {
    render(<Button>Test</Button>);
    const button = screen.getByText("Test");
    expect(button).toBeInTheDocument();
  });
});
