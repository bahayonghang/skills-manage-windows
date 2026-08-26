import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";

import { InventoryCensus } from "@/components/skillsCli/InventoryCensus";
import type { SkillsCliGlobalSkill, SkillsCliInstallTarget } from "@/types";

const targets: SkillsCliInstallTarget[] = [
  {
    id: "cursor",
    displayName: "Cursor",
    iconName: null,
    cliAgent: "cursor",
    isEnabled: true,
    defaultSelected: true,
  },
  {
    id: "amp",
    displayName: "Amp",
    iconName: null,
    cliAgent: "amp",
    isEnabled: false,
    defaultSelected: false,
  },
];

function skill(overrides: Partial<SkillsCliGlobalSkill>): SkillsCliGlobalSkill {
  return {
    name: "skill",
    path: null,
    installKind: "canonical",
    scope: "global",
    agents: [],
    source: null,
    sourceUrl: null,
    sourceType: null,
    sourceTypeBucket: "unknown",
    canonicalPath: null,
    folderHash: null,
    installedAt: null,
    updatedAt: null,
    placements: [],
    ...overrides,
  };
}

function chartTitles(container: HTMLElement): string[] {
  return Array.from(container.querySelectorAll("svg[role='img'] title")).map(
    (node) => node.textContent ?? "",
  );
}

describe("InventoryCensus", () => {
  it("renders KPI counts for installed, linked, and source kinds", () => {
    render(
      <InventoryCensus
        targets={targets}
        skills={[
          skill({ name: "a", agents: ["Cursor"], sourceTypeBucket: "github" }),
          skill({ name: "b", agents: ["Amp"], sourceTypeBucket: "github" }),
          skill({ name: "c", agents: [], sourceTypeBucket: "unknown" }),
        ]}
      />,
    );

    const strip = screen.getByLabelText("库存统计 KPI");
    expect(strip).toHaveTextContent("已装技能");
    expect(strip).toHaveTextContent("3");
    expect(strip).toHaveTextContent("已链接平台");
    expect(strip).toHaveTextContent("2");
    expect(strip).toHaveTextContent("来源种类");
    expect(strip).toHaveTextContent("2");
  });

  it("keeps zero-count platform buckets with a titled bar", () => {
    const { container } = render(
      <InventoryCensus
        targets={targets}
        skills={[skill({ name: "a", agents: ["Cursor"] })]}
      />,
    );

    const titles = chartTitles(container);
    expect(titles).toContain("Cursor：1");
    expect(titles).toContain("Amp：0");
  });

  it("charts accessible roles, labels, and titles for donut and bars", () => {
    const { container } = render(
      <InventoryCensus
        targets={targets}
        skills={[
          skill({ name: "a", sourceTypeBucket: "github" }),
          skill({ name: "b", sourceTypeBucket: "github" }),
          skill({ name: "c", sourceTypeBucket: "unknown" }),
        ]}
      />,
    );

    const charts = container.querySelectorAll("svg[role='img']");
    expect(charts).toHaveLength(2);
    for (const chart of charts) {
      expect(chart.getAttribute("aria-label")).toMatch(/\S/);
    }
    const titles = chartTitles(container);
    expect(titles).toContain("GitHub：2（67%）");
    expect(titles).toContain("未知：1（33%）");
    expect(container.querySelectorAll("circle")).toHaveLength(2);
    expect(container.querySelectorAll("rect")).toHaveLength(2);
    expect(container.querySelectorAll("title")).toHaveLength(4);
  });

  it("renders an empty census placeholder without charts", () => {
    render(<InventoryCensus targets={targets} skills={[]} />);

    expect(screen.getByTestId("skills-cli-census-empty")).toBeInTheDocument();
    expect(document.querySelector("svg[role='img']")).toBeNull();
  });
});
