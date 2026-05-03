import { describe, expect, it } from "vitest";

import type { AgentWithStatus } from "../types";
import {
  getPlatformTargetGroups,
  getPlatformTargetInstallAgentIds,
  getPlatformTargetMemberIds,
  hasProjectSkillPattern,
  isUniversalPlatformTarget,
} from "../lib/platformTargetGroups";

const baseAgent = {
  category: "coding",
  is_detected: true,
  is_builtin: true,
  is_enabled: true,
} satisfies Pick<
  AgentWithStatus,
  "category" | "is_detected" | "is_builtin" | "is_enabled"
>;

function agent(
  id: string,
  displayName: string,
  path: string,
  enabled = true
): AgentWithStatus {
  return {
    ...baseAgent,
    id,
    display_name: displayName,
    global_skills_dir: path,
    project_skills_dir: undefined,
    is_enabled: enabled,
  };
}

describe("platformTargetGroups", () => {
  it("folds known Universal agents into one Universal target", () => {
    const agents = [
      agent("codex", "Codex CLI", "C:\\Users\\lyh\\.agents\\skills\\"),
      agent("cursor", "Cursor", "c:/users/lyh/.agents/skills"),
      agent("claude-code", "Claude Code", "C:\\Users\\lyh\\.claude\\skills"),
      agent("central", "Central Skills", "C:\\Users\\lyh\\.skillsmanage\\skills"),
    ];

    const groups = getPlatformTargetGroups(agents, {
      coding: true,
      lobster: true,
    });

    expect(groups.map((group) => group.id)).toEqual([
      "universal-agents",
      "claude-code",
    ]);
    expect(isUniversalPlatformTarget(groups[0])).toBe(true);
    expect(getPlatformTargetMemberIds(groups[0])).toEqual(["codex", "cursor"]);
    expect(getPlatformTargetInstallAgentIds(groups[0])).toEqual(["codex"]);
  });

  it("keeps independent platforms unchanged", () => {
    const agents = [
      agent("codex", "Codex CLI", "~/.agents/skills"),
      agent("claude-code", "Claude Code", "~/.claude/skills"),
      agent("kiro", "Kiro", "~/.kiro/skills"),
      agent("central", "Central Skills", "~/.skillsmanage/skills"),
    ];

    const groups = getPlatformTargetGroups(agents, {
      coding: true,
      lobster: true,
    });

    expect(groups.map((group) => group.id)).toEqual([
      "universal-agents",
      "claude-code",
      "kiro",
    ]);
  });

  it("hides the Universal target when every member is hidden", () => {
    const agents = [
      agent("codex", "Codex CLI", "~/.agents/skills", false),
      agent("cursor", "Cursor", "~/.agents/skills", false),
      agent("claude-code", "Claude Code", "~/.claude/skills"),
      agent("central", "Central Skills", "~/.skillsmanage/skills"),
    ];

    const groups = getPlatformTargetGroups(agents, {
      coding: true,
      lobster: true,
    });

    expect(groups.map((group) => group.id)).toEqual(["claude-code"]);
  });

  it("requires an explicit project pattern before reporting project install support", () => {
    const builtinWithoutProjectPattern: AgentWithStatus = {
      ...agent("claude-code", "Claude Code", "~/.claude/skills"),
      project_skills_dir: undefined,
    };

    expect(hasProjectSkillPattern(builtinWithoutProjectPattern)).toBe(true);

    const absoluteCustomAgent: AgentWithStatus = {
      ...agent("custom-app", "Custom App", "D:\\Tools\\CustomApp\\skills"),
      is_builtin: false,
      project_skills_dir: undefined,
    };

    expect(hasProjectSkillPattern(absoluteCustomAgent)).toBe(false);
  });
});
