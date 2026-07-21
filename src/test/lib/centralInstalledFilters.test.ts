import { describe, expect, it } from "vitest";

import { getSkillInstalledPlatformCount, selectMostUniversalSkills } from "@/lib/centralInstalledFilters";
import type { SkillWithLinks } from "@/types";


function centralInstalledFilterSkill(
  id: string,
  overrides: Partial<SkillWithLinks> = {}
): SkillWithLinks {
  return {
    id,
    name: id,
    description: "",
    file_path: `/central/${id}/SKILL.md`,
    canonical_path: `/central/${id}`,
    is_central: true,
    scanned_at: "2026-05-23T00:00:00Z",
    linked_agents: [],
    shared_root_agents: [],
    ...overrides,
  };
}

describe("central installed platform helpers", () => {
  it("counts linked and shared-root agents once", () => {
    const skill = centralInstalledFilterSkill("universal", {
      linked_agents: ["claude-code", "codex"],
      shared_root_agents: ["codex", "cursor"],
    });

    expect(getSkillInstalledPlatformCount(skill)).toBe(3);
  });

  it("selects only current skills with the highest non-zero installed platform count", () => {
    const alpha = centralInstalledFilterSkill("alpha", {
      linked_agents: ["codex"],
    });
    const beta = centralInstalledFilterSkill("beta", {
      linked_agents: ["codex", "claude-code"],
    });
    const gamma = centralInstalledFilterSkill("gamma", {
      linked_agents: ["cursor"],
      shared_root_agents: ["claude-code"],
    });
    const empty = centralInstalledFilterSkill("empty");

    expect(selectMostUniversalSkills([alpha, beta, gamma, empty]).map((skill) => skill.id))
      .toEqual(["beta", "gamma"]);
    expect(selectMostUniversalSkills([empty])).toEqual([]);
  });
});
