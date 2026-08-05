import { describe, expect, it } from "vitest";

import {
  buildDeletedPlatformCopyCleanupDecisions,
  buildDecisions,
  buildInitialState,
  countDeletedPlatformCopyPaths,
  countDecisionSelections,
  countsFromInventory,
  summarizeDecisionSelections,
} from "@/components/central/updateCenter/decisionAggregation";
import {
  buildRefreshScope,
  coerceRefreshScopeKind,
  isRefreshScopeEnabled,
} from "@/lib/updateCenterRefreshScope";
import { preferredUpdateCenterTab } from "@/pages/centralUpdateCheckMode";
import type {
  SkillUpdateInventory,
  RemoteMissingSkill,
} from "@/types/skillUpdateInventory";

function emptyInventory(
  overrides: Partial<SkillUpdateInventory> = {},
): SkillUpdateInventory {
  const base: SkillUpdateInventory = {
    updatable: [],
    remoteAdded: [],
    remoteMissing: [],
    platformDuplicates: [],
    deletedPlatformCopies: [],
    orphans: [],
    failedRepositories: [],
    generatedAt: "2026-05-23T00:00:00.000Z",
  };
  return { ...base, ...overrides };
}

function remoteMissing(skillId: string): RemoteMissingSkill {
  return {
    repositoryId: "github:owner-repo-main",
    state: {
      skill_id: skillId,
      source_type: "github",
      source_url: "https://github.com/owner/repo",
      ref: "main",
      source_path: `skills/${skillId}`,
      status: "remote_missing",
      error: "Repository path is gone",
    },
  };
}

describe("updateCenter decision aggregation", () => {
  it("counts unsupported skills without making them applyable", () => {
    const inventory = emptyInventory({
      unsupported: [
        { skillId: "local-only", reasonCode: "unknown_source" },
        { skillId: "missing-path", reasonCode: "missing_source_path" },
      ],
    });
    const decisions = buildInitialState(inventory);

    expect(countsFromInventory(inventory).unsupported).toBe(2);
    expect(countDecisionSelections(decisions, inventory)).toBe(0);
    expect(preferredUpdateCenterTab(inventory)).toBe("unsupported");
  });

  it("treats default keep for remote-missing rows as an applyable detach decision", () => {
    const inventory = emptyInventory({
      remoteMissing: [remoteMissing("keep-local")],
    });
    const decisions = buildInitialState(inventory);

    expect(countDecisionSelections(decisions, inventory)).toBe(1);
    expect(buildDecisions(decisions, inventory)).toMatchObject({
      keepMissing: ["keep-local"],
      deleteMissing: [],
    });
  });

  it("keeps delete remote-missing rows applyable and sends delete payload", () => {
    const inventory = emptyInventory({
      remoteMissing: [remoteMissing("delete-local")],
    });
    const decisions = buildInitialState(inventory);
    decisions.missing["delete-local"] = {
      decision: "delete",
      removeAgentIds: ["codex"],
    };

    expect(countDecisionSelections(decisions, inventory)).toBe(1);
    expect(buildDecisions(decisions, inventory)).toMatchObject({
      keepMissing: [],
      deleteMissing: [
        { skill_id: "delete-local", remove_agent_ids: ["codex"] },
      ],
    });
  });

  it("does not default global platform leftovers to selected", () => {
    const inventory = emptyInventory({
      deletedPlatformCopies: [
        {
          agentId: "claude-code",
          skillId: "removed-skill",
          skillName: "removed-skill",
          writablePaths: [
            "~/.claude/skills/removed-skill",
            "~/.claude/skills/removed-skill-copy",
          ],
        },
      ],
    });
    const decisions = buildInitialState(inventory);

    expect(countDecisionSelections(decisions, inventory)).toBe(0);
    expect(buildDecisions(decisions, inventory)).toMatchObject({
      removeDeletedPlatformCopies: [],
    });
  });

  it("defaults current-platform leftovers to selected and counts selected paths", () => {
    const inventory = emptyInventory({
      deletedPlatformCopies: [
        {
          agentId: "claude-code",
          skillId: "removed-skill",
          skillName: "removed-skill",
          writablePaths: [
            "~/.claude/skills/removed-skill",
            "~/.claude/skills/removed-skill-copy",
          ],
        },
      ],
    });
    const decisions = buildInitialState(inventory, "platform");

    expect(countDecisionSelections(decisions, inventory)).toBe(2);
    expect(summarizeDecisionSelections(decisions, inventory)).toMatchObject({
      deletedPlatformCopies: 2,
    });
    expect(buildDecisions(decisions, inventory)).toMatchObject({
      removeDeletedPlatformCopies: [
        {
          agentId: "claude-code",
          skillId: "removed-skill",
          paths: [
            "~/.claude/skills/removed-skill",
            "~/.claude/skills/removed-skill-copy",
          ],
        },
      ],
    });
  });

  it("builds cleanup-only decisions for all platform leftover paths", () => {
    const inventory = emptyInventory({
      updatable: [
        {
          state: {
            skill_id: "has-update",
            source_type: "github",
            source_url: "https://github.com/example/repo",
            ref: "main",
            source_path: "skills/has-update",
            status: "update_available",
            error: null,
          },
        },
      ],
      platformDuplicates: [
        {
          agentId: "codex",
          skillId: "dup-skill",
          skillName: "dup-skill",
          writablePaths: ["~/.agents/skills/dup-skill"],
          pluginPaths: ["~/.codex/plugins/cache/plugin/skills/dup-skill"],
        },
      ],
      deletedPlatformCopies: [
        {
          agentId: "codex",
          skillId: "removed-skill",
          skillName: "removed-skill",
          writablePaths: [
            "~/.agents/skills/removed-skill",
            "~/.agents/skills/removed-skill-copy",
            "~/.agents/skills/removed-skill",
          ],
        },
        {
          agentId: "amp",
          skillId: "old-skill",
          skillName: "old-skill",
          writablePaths: ["~/.amp/skills/old-skill"],
        },
      ],
    });

    expect(countDeletedPlatformCopyPaths(inventory)).toBe(3);
    expect(buildDeletedPlatformCopyCleanupDecisions(inventory, ["codex"]))
      .toEqual({
        allowedAgentIds: ["codex"],
        updates: [],
        keepMissing: [],
        deleteMissing: [],
        importAdditions: [],
        skipAdditions: [],
        unskipAdditions: [],
        removePlatformDuplicates: [],
        removeDeletedPlatformCopies: [
          {
            agentId: "codex",
            skillId: "removed-skill",
            paths: [
              "~/.agents/skills/removed-skill",
              "~/.agents/skills/removed-skill-copy",
            ],
          },
          {
            agentId: "amp",
            skillId: "old-skill",
            paths: ["~/.amp/skills/old-skill"],
          },
        ],
      });
  });

  it("only defaults duplicate cleanup paths inside current-platform scope", () => {
    const inventory = emptyInventory({
      platformDuplicates: [
        {
          agentId: "codex",
          skillId: "dup-skill",
          skillName: "dup-skill",
          writablePaths: ["~/.agents/skills/dup-skill"],
          pluginPaths: ["~/.codex/plugins/cache/plugin/skills/dup-skill"],
        },
      ],
    });

    expect(countDecisionSelections(buildInitialState(inventory), inventory)).toBe(0);

    const platformDecisions = buildInitialState(inventory, "platform");
    expect(countDecisionSelections(platformDecisions, inventory)).toBe(1);
    expect(buildDecisions(platformDecisions, inventory, ["codex"])).toMatchObject({
      allowedAgentIds: ["codex"],
      removePlatformDuplicates: [
        {
          agentId: "codex",
          skillId: "dup-skill",
          paths: ["~/.agents/skills/dup-skill"],
        },
      ],
    });
  });
});

describe("updateCenter refresh scope", () => {
  it("builds repository and current-result payloads from opener context", () => {
    const context = {
      repositoryIds: ["github:owner-repo-main"],
      skillIds: ["a", "b"],
    };

    expect(buildRefreshScope("repositories", context)).toEqual({
      kind: "repositories",
      mode: "sync",
      repositoryIds: ["github:owner-repo-main"],
    });
    expect(buildRefreshScope("skills", context, "regular")).toEqual({
      kind: "skills",
      mode: "regular",
      skillIds: ["a", "b"],
    });
  });

  it("builds current-platform payloads from opener context", () => {
    const context = {
      repositoryIds: [],
      skillIds: ["a"],
      agentIds: [" codex ", "codex", "amp"],
    };

    expect(buildRefreshScope("platform", context)).toEqual({
      kind: "platform",
      mode: "sync",
      agentIds: ["codex", "amp"],
    });
  });

  it("disables empty scoped options and coerces invalid scope back to all", () => {
    const empty = { repositoryIds: [], skillIds: [] };

    expect(isRefreshScopeEnabled("repositories", empty)).toBe(false);
    expect(isRefreshScopeEnabled("skills", empty)).toBe(false);
    expect(isRefreshScopeEnabled("platform", empty)).toBe(false);
    expect(coerceRefreshScopeKind("repositories", empty)).toBe("all");
    expect(buildRefreshScope("skills", empty)).toEqual({ kind: "all", mode: "sync" });
  });
});
