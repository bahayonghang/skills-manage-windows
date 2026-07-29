import { describe, expect, it } from "vitest";

import {
  coerceToArray,
  matchesRepositoryFilter,
  matchesTagFilter,
  type TagFilterContext,
} from "@/lib/centralFilters";
import type { SkillTag, SkillWithLinks } from "@/types";

const baseSkill: SkillWithLinks = {
  id: "s1",
  name: "demo",
  file_path: "/x",
  is_central: true,
  scanned_at: "2026-04-17T00:00:00.000Z",
  linked_agents: [],
  shared_root_agents: [],
  tags: [],
};

function tag(id: string, name: string = id): SkillTag {
  return { id, name, is_builtin: false, created_at: "", updated_at: "" };
}

const emptyCtx: TagFilterContext = {
  updateStatuses: {},
  aiReviewSkillIds: new Set(),
};

describe("coerceToArray", () => {
  it("undefined -> []", () => {
    expect(coerceToArray(undefined)).toEqual([]);
  });

  it("empty string -> []", () => {
    expect(coerceToArray("")).toEqual([]);
  });

  it("string -> [string]", () => {
    expect(coerceToArray("a")).toEqual(["a"]);
  });

  it("array -> array (filtered)", () => {
    expect(coerceToArray(["a", "", "b"])).toEqual(["a", "b"]);
  });
});

describe("matchesRepositoryFilter", () => {
  const repoSkill: SkillWithLinks = {
    ...baseSkill,
    repository: {
      id: "r1",
      name: "skills",
      source_type: "github",
      owner: "anthropics",
      repo: "skills",
      branch: null,
      url: null,
      pinned: false,
      is_unknown: false,
      created_at: "",
      updated_at: "",
    },
  };
  const unknownSkill: SkillWithLinks = {
    ...baseSkill,
    is_source_unknown: true,
  };

  it("all matches everything (single)", () => {
    expect(matchesRepositoryFilter(repoSkill, "all")).toBe(true);
    expect(matchesRepositoryFilter(unknownSkill, "all")).toBe(true);
  });

  it("empty / undefined matches everything", () => {
    expect(matchesRepositoryFilter(repoSkill, undefined)).toBe(true);
    expect(matchesRepositoryFilter(repoSkill, [])).toBe(true);
  });

  it("matches by repository.id", () => {
    expect(matchesRepositoryFilter(repoSkill, "r1")).toBe(true);
    expect(matchesRepositoryFilter(repoSkill, "r2")).toBe(false);
  });

  it("unassigned matches is_source_unknown", () => {
    expect(matchesRepositoryFilter(unknownSkill, "unassigned")).toBe(true);
    expect(matchesRepositoryFilter(repoSkill, "unassigned")).toBe(false);
  });

  it("array OR semantics", () => {
    expect(matchesRepositoryFilter(repoSkill, ["r1", "r2"])).toBe(true);
    expect(matchesRepositoryFilter(repoSkill, ["r2", "r3"])).toBe(false);
    expect(matchesRepositoryFilter(unknownSkill, ["unassigned", "r9"])).toBe(true);
  });

  it("array containing 'all' short-circuits to true", () => {
    expect(matchesRepositoryFilter(repoSkill, ["all", "r2"])).toBe(true);
  });
});

describe("matchesTagFilter", () => {
  it("all matches everything", () => {
    const s = { ...baseSkill, tags: [tag("editor")] };
    expect(matchesTagFilter(s, "all", emptyCtx)).toBe(true);
  });

  it("matches by tag id", () => {
    const s = { ...baseSkill, tags: [tag("editor"), tag("writing")] };
    expect(matchesTagFilter(s, "editor", emptyCtx)).toBe(true);
    expect(matchesTagFilter(s, "missing", emptyCtx)).toBe(false);
  });

  it("uncategorized matches empty tag list", () => {
    expect(matchesTagFilter({ ...baseSkill, tags: [] }, "uncategorized", emptyCtx)).toBe(true);
    expect(
      matchesTagFilter(
        { ...baseSkill, tags: [tag("editor")] },
        "uncategorized",
        emptyCtx
      )
    ).toBe(false);
  });

  it("updates matches update_available status from ctx", () => {
    const s = { ...baseSkill };
    const ctx: TagFilterContext = {
      updateStatuses: {
        s1: { skill_id: "s1", source_type: "github", status: "update_available" },
      },
      aiReviewSkillIds: new Set(),
    };
    expect(matchesTagFilter(s, "updates", ctx)).toBe(true);
    expect(matchesTagFilter(s, "updates", emptyCtx)).toBe(false);
  });

  it("ai-review matches ctx set", () => {
    const ctx: TagFilterContext = {
      updateStatuses: {},
      aiReviewSkillIds: new Set(["s1"]),
    };
    expect(matchesTagFilter(baseSkill, "ai-review", ctx)).toBe(true);
    expect(matchesTagFilter(baseSkill, "ai-review", emptyCtx)).toBe(false);
  });

  it("array OR semantics across special and id values", () => {
    const s = { ...baseSkill, tags: [tag("editor")] };
    expect(matchesTagFilter(s, ["editor", "uncategorized"], emptyCtx)).toBe(true);
    expect(matchesTagFilter(s, ["missing", "uncategorized"], emptyCtx)).toBe(false);
  });
});
