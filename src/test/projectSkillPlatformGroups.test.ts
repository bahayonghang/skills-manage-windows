import { describe, expect, it } from "vitest";

import { groupProjectSkillsByPlatform } from "../lib/projectSkillPlatformGroups";
import { getPlatformTargetGroups } from "../lib/platformTargetGroups";
import type { AgentWithStatus, ProjectSkill } from "../types";

const baseAgent = {
  category: "coding",
  global_skills_dir: "~/.agent/skills",
  is_detected: true,
  is_builtin: true,
  is_enabled: true,
} satisfies Omit<AgentWithStatus, "id" | "display_name">;

function agent(id: string, displayName: string): AgentWithStatus {
  return {
    ...baseAgent,
    id,
    display_name: displayName,
  };
}

function projectSkill(
  skillId: string,
  agentId: string,
  agentDisplayName: string
): ProjectSkill {
  return {
    projectId: "project-1",
    skillId,
    name: skillId,
    description: null,
    filePath: `D:/demo/${agentId}/${skillId}/SKILL.md`,
    sourceOrigin: "central",
    agentId,
    agentDisplayName,
    installedPath: `D:/demo/${agentId}/${skillId}`,
    linkType: "symlink",
    symlinkTarget: `C:/Users/demo/.agents/skills/${skillId}`,
  };
}

describe("groupProjectSkillsByPlatform", () => {
  it("groups project skills by Sidebar platform targets and order", () => {
    const targets = getPlatformTargetGroups(
      [
        agent("codex", "Codex CLI"),
        agent("claude-code", "Claude Code"),
        agent("kiro", "Kiro"),
        agent("central", "Central Skills"),
      ],
      { coding: true, lobster: true }
    );

    const groups = groupProjectSkillsByPlatform(
      [
        projectSkill("kiro-helper", "kiro", "Kiro"),
        projectSkill("universal-helper", "codex", "Codex CLI"),
        projectSkill("claude-helper", "claude-code", "Claude Code"),
      ],
      targets
    );

    expect(groups.map((group) => group.id)).toEqual([
      "universal-agents",
      "claude-code",
      "kiro",
    ]);
    expect(groups[0].rawAgentIds).toEqual(["codex"]);
    expect(groups[0].skills.map((skill) => skill.skillId)).toEqual([
      "universal-helper",
    ]);
  });

  it("folds every Universal member skill into one Universal group", () => {
    const targets = getPlatformTargetGroups(
      [
        agent("codex", "Codex CLI"),
        agent("cursor", "Cursor"),
        agent("opencode", "OpenCode"),
      ],
      { coding: true, lobster: true }
    );

    const groups = groupProjectSkillsByPlatform(
      [
        projectSkill("codex-skill", "codex", "Codex CLI"),
        projectSkill("cursor-skill", "cursor", "Cursor"),
        projectSkill("opencode-skill", "opencode", "OpenCode"),
      ],
      targets
    );

    expect(groups).toHaveLength(1);
    expect(groups[0].id).toBe("universal-agents");
    expect(groups[0].rawAgentIds).toEqual(["codex", "cursor", "opencode"]);
  });

  it("folds the Universal representative and legacy raw member ids into Universal", () => {
    const targets = getPlatformTargetGroups(
      [
        agent("codex", "Codex CLI"),
        agent("opencode", "OpenCode"),
        agent("claude-code", "Claude Code"),
      ],
      { coding: true, lobster: true }
    );

    const groups = groupProjectSkillsByPlatform(
      [
        projectSkill("canonical-agents-skill", "codex", "Codex CLI"),
        projectSkill("legacy-opencode-skill", "opencode", "OpenCode"),
      ],
      targets
    );

    expect(groups).toHaveLength(1);
    expect(groups[0].id).toBe("universal-agents");
    expect(groups[0].rawAgentIds).toEqual(["codex", "opencode"]);
    expect(groups[0].skills.map((skill) => skill.skillId)).toEqual([
      "canonical-agents-skill",
      "legacy-opencode-skill",
    ]);
  });

  it("keeps hidden or unknown platform skills visible in a fallback group", () => {
    const targets = getPlatformTargetGroups(
      [agent("claude-code", "Claude Code")],
      { coding: true, lobster: true }
    );

    const groups = groupProjectSkillsByPlatform(
      [projectSkill("hidden-skill", "disabled-agent", "Disabled Agent")],
      targets
    );

    expect(groups).toHaveLength(1);
    expect(groups[0].target).toBeNull();
    expect(groups[0].rawAgentIds).toEqual(["disabled-agent"]);
    expect(groups[0].skills[0].skillId).toBe("hidden-skill");
  });
});
