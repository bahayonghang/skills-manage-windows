import { describe, expect, it } from "vitest";
import { render, screen, within } from "@testing-library/react";
import { SkillMarkdownRenderer } from "@/components/skill/SkillMarkdownRenderer";

const markdownFixture = `# Darwin Skill

Intro paragraph with an [external link](https://github.com/alchaincyf/darwin-skill).

> Quote block for the optimization philosophy.

## Rubric

| Dimension | Weight |
| --- | --- |
| Structure | 60 |
| Effectiveness | 40 |

\`\`\`ts
console.log("skill");
\`\`\`
`;

describe("SkillMarkdownRenderer", () => {
  it("renders detail variant with document label and markdown structure wrappers", () => {
    const { container } = render(
      <SkillMarkdownRenderer content={markdownFixture} variant="detail" />
    );

    expect(screen.getByText("SKILL.md")).toBeInTheDocument();

    const surface = container.querySelector('[data-skill-markdown-variant="detail"]');
    expect(surface).not.toBeNull();

    const tableWrapper = container.querySelector('[data-skill-markdown-table="true"]');
    expect(tableWrapper).not.toBeNull();
    expect(within(tableWrapper as HTMLElement).getByText("Dimension")).toBeInTheDocument();

    const blockquote = container.querySelector('[data-skill-markdown-callout="true"]');
    expect(blockquote).not.toBeNull();
    expect(within(blockquote as HTMLElement).getByText(/optimization philosophy/i)).toBeInTheDocument();

    const pre = container.querySelector('[data-skill-markdown-pre="true"]');
    expect(pre).not.toBeNull();
    expect(within(pre as HTMLElement).getByText('console.log("skill");')).toBeInTheDocument();

    const link = screen.getByRole("link", { name: /external link/i });
    expect(link).toHaveAttribute("target", "_blank");
    expect(link).toHaveAttribute("rel", "noreferrer");
  });

  it("renders compact variant without the detail label", () => {
    const { container } = render(
      <SkillMarkdownRenderer content={"# Compact\n\nBody."} variant="compact" />
    );

    expect(screen.queryByText("SKILL.md")).not.toBeInTheDocument();
    expect(
      container.querySelector('[data-skill-markdown-variant="compact"]')
    ).not.toBeNull();
    expect(screen.getByRole("heading", { name: "Compact" })).toBeInTheDocument();
  });
});
