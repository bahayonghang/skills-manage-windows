import { describe, expect, it } from "vitest";

import { createCentralBatchUninstallPreview } from "@/lib/centralBatchUninstall";
import type { SkillWithLinks } from "@/types";

function skill(
  id: string,
  linkedAgents: string[],
  sharedRootAgents: string[] = [],
): SkillWithLinks {
  return {
    id,
    name: id,
    description: `${id} description`,
    file_path: `~/.skillsmanage/skills/${id}/SKILL.md`,
    canonical_path: `~/.skillsmanage/skills/${id}`,
    is_central: true,
    scanned_at: "2026-06-09T00:00:00Z",
    linked_agents: linkedAgents,
    shared_root_agents: sharedRootAgents,
  };
}

describe("createCentralBatchUninstallPreview", () => {
  it("groups removable linked agents and excludes central/shared-root agents", () => {
    const preview = createCentralBatchUninstallPreview(
      ["frontend-design", "code-reviewer"],
      [
        skill(
          "frontend-design",
          ["central", "codex", "cursor", "cursor"],
          ["cursor"],
        ),
        skill("code-reviewer", ["claude-code"], ["claude-code"]),
      ],
    );

    expect(preview.groups).toEqual([
      {
        agentId: "codex",
        requests: [{ skill_id: "frontend-design" }],
      },
    ]);
    expect(preview.sharedRootLinks).toEqual([
      {
        skillId: "frontend-design",
        skillName: "frontend-design",
        agentId: "cursor",
      },
      {
        skillId: "code-reviewer",
        skillName: "code-reviewer",
        agentId: "claude-code",
      },
    ]);
    expect(preview.skippedSkills).toEqual([
      {
        skillId: "code-reviewer",
        skillName: "code-reviewer",
        reason: "shared_root_only",
      },
    ]);
    expect(preview.totals).toMatchObject({
      removableInstallCount: 1,
      removablePlatformCount: 1,
      skippedSkillCount: 1,
      sharedRootInstallCount: 2,
    });
  });

  it("treats selected skills without removable platform installs as skipped no-ops", () => {
    const preview = createCentralBatchUninstallPreview(
      ["missing", "local-only", "central-only"],
      [
        skill("local-only", []),
        skill("central-only", ["central"]),
      ],
    );

    expect(preview.selectedSkillIds).toEqual(["local-only", "central-only"]);
    expect(preview.groups).toEqual([]);
    expect(preview.skippedSkills).toEqual([
      {
        skillId: "local-only",
        skillName: "local-only",
        reason: "no_platform_installs",
      },
      {
        skillId: "central-only",
        skillName: "central-only",
        reason: "no_platform_installs",
      },
    ]);
    expect(preview.totals.removableInstallCount).toBe(0);
  });

  it("dedupes selected skill ids and duplicate linked agent ids", () => {
    const preview = createCentralBatchUninstallPreview(
      ["frontend-design", "frontend-design"],
      [skill("frontend-design", ["codex", "codex", "claude-code"])],
    );

    expect(preview.selectedSkillIds).toEqual(["frontend-design"]);
    expect(preview.groups).toEqual([
      {
        agentId: "codex",
        requests: [{ skill_id: "frontend-design" }],
      },
      {
        agentId: "claude-code",
        requests: [{ skill_id: "frontend-design" }],
      },
    ]);
    expect(preview.totals.removableInstallCount).toBe(2);
  });
});
