import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { SkillDetailFileTree } from "../components/skill/SkillDetailFileTree";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string) =>
      (
        {
          "detail.fileTreeTitle": "File tree",
          "detail.fileTreeLoading": "Loading file tree...",
          "detail.fileTreeEmpty": "No files available for this skill.",
          "common.expand": "Expand",
          "common.collapse": "Collapse",
        } as Record<string, string>
      )[key] ?? key,
  }),
}));

const nestedEntries = [
  {
    name: "examples",
    path: "/skill/examples",
    file_type: "dir",
    children: [
      {
        name: "demo.md",
        path: "/skill/examples/demo.md",
        file_type: "file",
        children: [],
      },
    ],
  },
  {
    name: "SKILL.md",
    path: "/skill/SKILL.md",
    file_type: "file",
    children: [],
  },
];

describe("SkillDetailFileTree", () => {
  it("renders loading and empty states", () => {
    const { rerender } = render(
      <SkillDetailFileTree entries={[]} isLoading onOpenPath={vi.fn()} />
    );

    expect(screen.getByText("Loading file tree...")).toBeInTheDocument();

    rerender(<SkillDetailFileTree entries={[]} isLoading={false} onOpenPath={vi.fn()} />);
    expect(screen.getByText("No files available for this skill.")).toBeInTheDocument();
  });

  it("opens a file path when clicking a node", () => {
    const onOpenPath = vi.fn();
    render(
      <SkillDetailFileTree entries={nestedEntries} isLoading={false} onOpenPath={onOpenPath} />
    );

    fireEvent.click(screen.getByRole("button", { name: "SKILL.md" }));

    expect(onOpenPath).toHaveBeenCalledWith("/skill/SKILL.md");
  });

  it("toggles nested directories", () => {
    render(
      <SkillDetailFileTree entries={nestedEntries} isLoading={false} onOpenPath={vi.fn()} />
    );

    expect(screen.getByText("demo.md")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Collapse" }));
    expect(screen.queryByText("demo.md")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Expand" }));
    expect(screen.getByText("demo.md")).toBeInTheDocument();
  });
});
