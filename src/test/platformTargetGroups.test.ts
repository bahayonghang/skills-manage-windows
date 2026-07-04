import { describe, expect, it } from "vitest";
import type { TFunction } from "i18next";

import type { AgentWithStatus } from "../types";
import {
  getProjectPlatformTargetGroups,
  getPlatformTargetCountAgentId,
  getPlatformTargetGroups,
  getPlatformTargetInstallAgentIds,
  getPlatformTargetLabel,
  getPlatformTargetMemberIds,
  getPlatformTargetTitleHint,
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
  enabled = true,
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
      agent(
        "central",
        "Central Skills",
        "C:\\Users\\lyh\\.skillsmanage\\skills",
      ),
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
      {
        ...agent("grok", "Grok", "~/.grok/skills"),
        project_skills_dir: ".grok/skills",
      },
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
      "grok",
      "claude-code",
      "kiro",
    ]);
    expect(isUniversalPlatformTarget(groups[1])).toBe(false);
    expect(hasProjectSkillPattern(groups[1])).toBe(true);
  });

  it("keeps Antigravity as a standalone global install target", () => {
    const agents = [
      agent("codex", "Codex CLI", "~/.agents/skills"),
      agent("antigravity", "Antigravity", "~/.gemini/antigravity/skills"),
      agent(
        "antigravity-cli",
        "Antigravity CLI",
        "~/.gemini/antigravity-cli/skills",
      ),
      agent("gemini-cli", "Gemini CLI (legacy)", "~/.gemini/skills", false),
      agent("central", "Central Skills", "~/.skillsmanage/skills"),
    ];

    const groups = getPlatformTargetGroups(agents, {
      coding: true,
      lobster: true,
    });

    expect(groups.map((group) => group.id)).toEqual([
      "universal-agents",
      "antigravity",
      "antigravity-cli",
    ]);
    expect(getPlatformTargetMemberIds(groups[0])).toEqual(["codex"]);
    expect(isUniversalPlatformTarget(groups[1])).toBe(false);
    expect(hasProjectSkillPattern(groups[1])).toBe(true);
    expect(isUniversalPlatformTarget(groups[2])).toBe(false);
    expect(hasProjectSkillPattern(groups[2])).toBe(true);
  });

  it("folds Antigravity into the project Universal target", () => {
    const agents = [
      agent("codex", "Codex CLI", "~/.agents/skills"),
      {
        ...agent("grok", "Grok", "~/.grok/skills"),
        project_skills_dir: ".grok/skills",
      },
      agent("antigravity", "Antigravity", "~/.gemini/antigravity/skills"),
      agent(
        "antigravity-cli",
        "Antigravity CLI",
        "~/.gemini/antigravity-cli/skills",
      ),
      agent("claude-code", "Claude Code", "~/.claude/skills"),
      agent("central", "Central Skills", "~/.skillsmanage/skills"),
    ];

    const groups = getProjectPlatformTargetGroups(agents, {
      coding: true,
      lobster: true,
    });

    expect(groups.map((group) => group.id)).toEqual([
      "universal-agents",
      "grok",
      "claude-code",
    ]);
    expect(getPlatformTargetMemberIds(groups[0])).toEqual([
      "antigravity",
      "antigravity-cli",
      "codex",
    ]);
    expect(getPlatformTargetInstallAgentIds(groups[0])).toEqual(["codex"]);
    expect(isUniversalPlatformTarget(groups[1])).toBe(false);
    expect(hasProjectSkillPattern(groups[1])).toBe(true);
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

  describe("display helpers", () => {
    const t = ((key: string) => key) as TFunction;

    function buildTargets() {
      const agents = [
        agent("codex", "Codex CLI", "~/.agents/skills"),
        agent("cursor", "Cursor", "~/.agents/skills"),
        agent("claude-code", "Claude Code", "~/.claude/skills"),
        agent("central", "Central Skills", "~/.skillsmanage/skills"),
      ];

      const groups = getPlatformTargetGroups(agents, {
        coding: true,
        lobster: true,
      });

      return { universal: groups[0], plain: groups[1] };
    }

    it("labels the universal group with i18n keys and plain agents with display_name", () => {
      const { universal, plain } = buildTargets();

      expect(getPlatformTargetLabel(universal, t, "full")).toBe(
        "platformTargets.universalLabel",
      );
      expect(getPlatformTargetLabel(universal, t, "short")).toBe(
        "platformTargets.universalShortLabel",
      );
      expect(getPlatformTargetLabel(plain, t, "full")).toBe("Claude Code");
      expect(getPlatformTargetLabel(plain, t, "short")).toBe("Claude Code");
    });

    it("hints the universal group with member names and plain agents with skills dir", () => {
      const { universal, plain } = buildTargets();

      expect(getPlatformTargetTitleHint(universal)).toBe("Codex CLI, Cursor");
      expect(getPlatformTargetTitleHint(plain)).toBe("~/.claude/skills");
    });

    it("counts the universal group by its install agent and plain agents by id", () => {
      const { universal, plain } = buildTargets();

      expect(getPlatformTargetCountAgentId(universal)).toBe("codex");
      expect(getPlatformTargetCountAgentId(plain)).toBe("claude-code");
    });
  });
});
