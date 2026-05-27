import { describe, expect, it } from "vitest";

import {
  buildDecisions,
  buildInitialState,
  countDecisionSelections,
} from "@/components/central/updateCenter/decisionAggregation";
import {
  buildRefreshScope,
  coerceRefreshScopeKind,
  isRefreshScopeEnabled,
} from "@/lib/updateCenterRefreshScope";
import type {
  SkillUpdateInventory,
  RemoteMissingSkill,
} from "@/types/skillUpdateInventory";

function emptyInventory(
  overrides: Partial<SkillUpdateInventory> = {},
): SkillUpdateInventory {
  return {
    updatable: [],
    remoteAdded: [],
    remoteMissing: [],
    platformDuplicates: [],
    orphans: [],
    failedRepositories: [],
    generatedAt: "2026-05-23T00:00:00.000Z",
    ...overrides,
  };
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
});

describe("updateCenter refresh scope", () => {
  it("builds repository and current-result payloads from opener context", () => {
    const context = {
      repositoryIds: ["github:owner-repo-main"],
      skillIds: ["a", "b"],
    };

    expect(buildRefreshScope("repositories", context)).toEqual({
      kind: "repositories",
      repositoryIds: ["github:owner-repo-main"],
    });
    expect(buildRefreshScope("skills", context)).toEqual({
      kind: "skills",
      skillIds: ["a", "b"],
    });
  });

  it("disables empty scoped options and coerces invalid scope back to all", () => {
    const empty = { repositoryIds: [], skillIds: [] };

    expect(isRefreshScopeEnabled("repositories", empty)).toBe(false);
    expect(isRefreshScopeEnabled("skills", empty)).toBe(false);
    expect(coerceRefreshScopeKind("repositories", empty)).toBe("all");
    expect(buildRefreshScope("skills", empty)).toEqual({ kind: "all" });
  });
});
