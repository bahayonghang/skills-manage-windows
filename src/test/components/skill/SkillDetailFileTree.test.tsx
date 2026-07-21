import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { SkillDetailFileTree } from "@/components/skill/SkillDetailFileTree";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, values?: Record<string, string>) => {
      const translated = (
        {
          "detail.fileTreeTitle": "File tree",
          "detail.fileTreeLoading": "Loading file tree...",
          "detail.fileTreeEmpty": "No files available for this skill.",
          "detail.expandDirectory": "Expand {{name}}",
          "detail.collapseDirectory": "Collapse {{name}}",
          "detail.openTreePath": "Open {{name}}",
        } as Record<string, string>
      )[key] ?? key;
      return translated.replace("{{name}}", values?.name ?? "");
    },
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
    expect(screen.getByRole("region", { name: "File tree" })).toHaveAttribute(
      "aria-busy",
      "true"
    );

    rerender(<SkillDetailFileTree entries={[]} isLoading={false} onOpenPath={vi.fn()} />);
    expect(screen.getByText("No files available for this skill.")).toBeInTheDocument();
  });

  it("opens a file path when clicking a node", () => {
    const onOpenPath = vi.fn();
    render(
      <SkillDetailFileTree entries={nestedEntries} isLoading={false} onOpenPath={onOpenPath} />
    );

    fireEvent.click(screen.getByRole("button", { name: "Open SKILL.md" }));

    expect(onOpenPath).toHaveBeenCalledWith("/skill/SKILL.md");
  });

  it("starts top-level directories collapsed and exposes named disclosure state", () => {
    render(
      <SkillDetailFileTree entries={nestedEntries} isLoading={false} onOpenPath={vi.fn()} />
    );

    const disclosure = screen.getByRole("button", { name: "Expand examples" });
    expect(disclosure).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByText("demo.md")).not.toBeInTheDocument();

    fireEvent.click(disclosure);
    expect(screen.getByText("demo.md")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Collapse examples" })).toHaveAttribute(
      "aria-expanded",
      "true"
    );
    expect(screen.getByRole("button", { name: "Open examples" })).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "Collapse examples" }));
    expect(screen.queryByText("demo.md")).not.toBeInTheDocument();
  });

  it("classifies representative file families with a neutral fallback", () => {
    const entries = [
      ["README.md", "docs"],
      ["settings.json", "data"],
      ["component.tsx", "web"],
      ["worker.py", "python"],
      ["lib.rs", "rust"],
      ["preview.png", "image"],
      [".env.local", "config"],
      ["worker.test.ts", "test"],
      ["shortcut", "symlink"],
      ["archive.bin", "unknown"],
    ].map(([name, kind]) => ({
      name,
      path: `/skill/${name}`,
      file_type: kind === "symlink" ? "symlink" : "file",
      children: [],
      kind,
    }));

    render(
      <SkillDetailFileTree entries={entries} isLoading={false} onOpenPath={vi.fn()} />
    );

    for (const entry of entries) {
      expect(screen.getByTestId(`file-tree-entry-${entry.name}`)).toHaveAttribute(
        "data-file-kind",
        entry.kind
      );
    }
  });
});
