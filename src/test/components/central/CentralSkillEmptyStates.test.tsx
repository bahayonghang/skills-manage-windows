import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { CentralSkillEmptyState, CentralSkillFirstVisitEmptyState } from "@/components/central/CentralSkillEmptyStates";

describe("CentralSkillEmptyStates probe", () => {
  it("renders loading empty state", () => {
    render(<CentralSkillEmptyState message="正在加载技能..." />);
    expect(screen.getByText("正在加载技能...")).toBeInTheDocument();
  });
  it("renders first visit state", () => {
    render(<MemoryRouter><CentralSkillFirstVisitEmptyState /></MemoryRouter>);
    expect(screen.getByText(/欢迎使用 SkillPort/)).toBeInTheDocument();
  });
});
