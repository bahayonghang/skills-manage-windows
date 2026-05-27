import { render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { RemoteAddedTabPanel } from "@/components/central/updateCenter/RemoteAddedTabPanel";
import type { SkillConflictSourceInfo } from "@/lib/centralConflictSource";
import type { RemoteAddedSkill } from "@/types/skillUpdateInventory";

const remoteAdded: RemoteAddedSkill[] = [
  {
    repositoryId: "github-openai-skills-main",
    sourcePath: "skills/conflicting",
    skillId: "conflicting-skill",
    skillName: "Conflicting Remote",
    conflictExistingSkillId: "conflicting-skill",
  },
];

const existingSkillSources = new Map<string, SkillConflictSourceInfo>([
  [
    "conflicting-skill",
    {
      skillId: "conflicting-skill",
      skillName: "Local Conflicting Skill",
      repositoryLabel: "anthropic/skills",
      sourcePath: "skills/conflicting",
    },
  ],
]);

describe("RemoteAddedTabPanel", () => {
  it("shows remote and existing source details for conflicts", () => {
    render(
      <RemoteAddedTabPanel
        items={remoteAdded}
        state={{}}
        existingSkillSources={existingSkillSources}
        repositoryLabels={new Map([["github-openai-skills-main", "openai/skills"]])}
        onChange={vi.fn()}
      />
    );

    const row = screen.getByText("Conflicting Remote").closest("article")!;
    expect(
      within(row).getByText(
        "冲突：远端 openai/skills/skills/conflicting ↔ 已有 Local Conflicting Skill（anthropic/skills/skills/conflicting）"
      )
    ).toBeInTheDocument();
  });
});
