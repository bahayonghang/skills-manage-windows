import { describe, expect, it } from "vitest";

import {
  matchSkillAgainstFilters,
  matchesGlob,
  parseCentralQuery,
  tokenizeCentralQuery,
  type CentralQueryContext,
} from "@/lib/centralSearchQuery";
import type { SkillWithLinks } from "@/types";

const baseSkill: SkillWithLinks = {
  id: "s1",
  name: "test-skill",
  description: "demo",
  file_path: "/x",
  is_central: true,
  scanned_at: "2026-04-17T00:00:00.000Z",
  created_at: "2026-04-17T00:00:00.000Z",
  updated_at: "2026-04-17T00:00:00.000Z",
  linked_agents: ["claude-code"],
  shared_root_agents: [],
  tags: [],
};

const emptyCtx: CentralQueryContext = {
  updateStatuses: {},
  aiReviewSkillIds: new Set(),
};

function skill(overrides: Partial<SkillWithLinks>): SkillWithLinks {
  return { ...baseSkill, ...overrides };
}

describe("tokenizeCentralQuery", () => {
  it("splits on whitespace", () => {
    expect(tokenizeCentralQuery("a b c")).toEqual(["a", "b", "c"]);
  });

  it("preserves quoted phrases", () => {
    expect(tokenizeCentralQuery('tag:"hello world" foo')).toEqual([
      "tag:hello world",
      "foo",
    ]);
  });

  it("supports escaped quotes inside quoted phrases", () => {
    expect(tokenizeCentralQuery('"a \\"b\\" c"')).toEqual(['a "b" c']);
  });

  it("returns [] for empty input", () => {
    expect(tokenizeCentralQuery("")).toEqual([]);
    expect(tokenizeCentralQuery("   ")).toEqual([]);
  });
});

describe("parseCentralQuery", () => {
  it("extracts a tag filter", () => {
    const ast = parseCentralQuery("tag:editor");
    expect(ast.filters).toEqual([{ kind: "tag", value: "editor", negated: false }]);
    expect(ast.freeText).toBe("");
  });

  it("supports negated tag filter", () => {
    const ast = parseCentralQuery("-tag:wip");
    expect(ast.filters).toEqual([{ kind: "tag", value: "wip", negated: true }]);
  });

  it("treats lone -word as free text, not negation", () => {
    const ast = parseCentralQuery("-word");
    expect(ast.filters).toEqual([]);
    expect(ast.freeText).toBe("-word");
  });

  it("is case-insensitive on keys (D1)", () => {
    const ast = parseCentralQuery("TAG:foo Repo:Bar SOURCE:GITHUB");
    expect(ast.filters).toEqual([
      { kind: "tag", value: "foo", negated: false },
      { kind: "repo", value: "Bar", negated: false },
      { kind: "source", value: "github", negated: false },
    ]);
  });

  it("preserves value casing (D1)", () => {
    const ast = parseCentralQuery("tag:Editor");
    expect(ast.filters[0]).toMatchObject({ value: "Editor" });
  });

  it("collects free text after stripping filters", () => {
    const ast = parseCentralQuery("paper tag:writing repo:anthropics/* word");
    expect(ast.freeText).toBe("paper word");
    expect(ast.filters).toHaveLength(2);
  });

  it("handles has: enum values", () => {
    const ast = parseCentralQuery("has:update has:no-tag has:ai-review");
    expect(ast.filters.map((f) => f.kind)).toEqual(["has", "has", "has"]);
  });

  it("rejects unknown has: value", () => {
    const ast = parseCentralQuery("has:bogus");
    expect(ast.invalid).toEqual(["has:bogus"]);
    expect(ast.filters).toHaveLength(0);
  });

  it("treats unknown keys as free text instead of dropping them", () => {
    const ast = parseCentralQuery("color:red");
    expect(ast.filters).toEqual([]);
    expect(ast.freeText).toBe("color:red");
  });

  it("parses time operators", () => {
    const ast = parseCentralQuery("created:>2026-01-01 updated:<7d");
    expect(ast.filters).toEqual([
      { kind: "created", op: ">", value: "2026-01-01", negated: false },
      { kind: "updated", op: "<", value: "7d", negated: false },
    ]);
  });

  it("supports quoted phrase as filter value", () => {
    const ast = parseCentralQuery('tag:"code review"');
    expect(ast.filters).toEqual([
      { kind: "tag", value: "code review", negated: false },
    ]);
  });
});

describe("matchesGlob", () => {
  it("matches exact strings", () => {
    expect(matchesGlob("anthropics/skills", "anthropics/skills")).toBe(true);
  });

  it("matches with single wildcard", () => {
    expect(matchesGlob("anthropics/skills", "anthropics/*")).toBe(true);
    expect(matchesGlob("bahayonghang/x", "anthropics/*")).toBe(false);
  });

  it("ignores case", () => {
    expect(matchesGlob("Anthropics/Skills", "anthropics/*")).toBe(true);
  });
});

describe("matchSkillAgainstFilters", () => {
  it("matches tag by name (case-insensitive)", () => {
    const s = skill({
      tags: [
        {
          id: "t1",
          name: "Editor",
          is_builtin: false,
          created_at: "",
          updated_at: "",
        },
      ],
    });
    const ast = parseCentralQuery("tag:editor");
    expect(matchSkillAgainstFilters(s, ast, emptyCtx)).toBe(true);
  });

  it("excludes when tag is negated", () => {
    const s = skill({
      tags: [
        {
          id: "t1",
          name: "wip",
          is_builtin: false,
          created_at: "",
          updated_at: "",
        },
      ],
    });
    const ast = parseCentralQuery("-tag:wip");
    expect(matchSkillAgainstFilters(s, ast, emptyCtx)).toBe(false);
  });

  it("matches repo with owner/name pattern", () => {
    const s = skill({
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
    });
    expect(
      matchSkillAgainstFilters(s, parseCentralQuery("repo:anthropics/skills"), emptyCtx)
    ).toBe(true);
    expect(
      matchSkillAgainstFilters(s, parseCentralQuery("repo:anthropics/*"), emptyCtx)
    ).toBe(true);
    expect(
      matchSkillAgainstFilters(s, parseCentralQuery("repo:other/*"), emptyCtx)
    ).toBe(false);
  });

  it("matches owner", () => {
    const s = skill({
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
    });
    expect(
      matchSkillAgainstFilters(s, parseCentralQuery("owner:anthropics"), emptyCtx)
    ).toBe(true);
    expect(matchSkillAgainstFilters(s, parseCentralQuery("owner:tw93"), emptyCtx)).toBe(false);
  });

  it("matches source:github / source:local", () => {
    const github = skill({
      repository: {
        id: "r1",
        name: "skills",
        source_type: "github",
        owner: "x",
        repo: "y",
        branch: null,
        url: null,
        pinned: false,
        is_unknown: false,
        created_at: "",
        updated_at: "",
      },
    });
    const local = skill({ is_source_unknown: true });
    expect(matchSkillAgainstFilters(github, parseCentralQuery("source:github"), emptyCtx)).toBe(
      true
    );
    expect(matchSkillAgainstFilters(github, parseCentralQuery("source:local"), emptyCtx)).toBe(
      false
    );
    expect(matchSkillAgainstFilters(local, parseCentralQuery("source:local"), emptyCtx)).toBe(
      true
    );
  });

  it("matches has:update via context", () => {
    const s = skill({});
    const ctx: CentralQueryContext = {
      updateStatuses: {
        s1: {
          skill_id: "s1",
          source_type: "github",
          status: "update_available",
        },
      },
      aiReviewSkillIds: new Set(),
    };
    expect(matchSkillAgainstFilters(s, parseCentralQuery("has:update"), ctx)).toBe(true);
    expect(matchSkillAgainstFilters(s, parseCentralQuery("has:update"), emptyCtx)).toBe(false);
  });

  it("matches has:no-tag for empty / uncategorized only tags", () => {
    expect(
      matchSkillAgainstFilters(skill({ tags: [] }), parseCentralQuery("has:no-tag"), emptyCtx)
    ).toBe(true);
    expect(
      matchSkillAgainstFilters(
        skill({
          tags: [
            {
              id: "t1",
              name: "real",
              is_builtin: false,
              created_at: "",
              updated_at: "",
            },
          ],
        }),
        parseCentralQuery("has:no-tag"),
        emptyCtx
      )
    ).toBe(false);
  });

  it("matches has:ai-review via context set", () => {
    const ctx: CentralQueryContext = {
      updateStatuses: {},
      aiReviewSkillIds: new Set(["s1"]),
    };
    expect(matchSkillAgainstFilters(skill({}), parseCentralQuery("has:ai-review"), ctx)).toBe(
      true
    );
  });

  it("matches platform via linked_agents", () => {
    const s = skill({ linked_agents: ["claude-code", "codex-cli"] });
    expect(
      matchSkillAgainstFilters(s, parseCentralQuery("platform:claude-code"), emptyCtx)
    ).toBe(true);
    expect(matchSkillAgainstFilters(s, parseCentralQuery("platform:cursor"), emptyCtx)).toBe(
      false
    );
  });

  it("AND-joins multiple filters", () => {
    const s = skill({
      tags: [
        {
          id: "t1",
          name: "editor",
          is_builtin: false,
          created_at: "",
          updated_at: "",
        },
      ],
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
    });
    expect(
      matchSkillAgainstFilters(
        s,
        parseCentralQuery("tag:editor owner:anthropics"),
        emptyCtx
      )
    ).toBe(true);
    expect(
      matchSkillAgainstFilters(s, parseCentralQuery("tag:editor owner:other"), emptyCtx)
    ).toBe(false);
  });

  it("matches updated:<7d", () => {
    const recent = skill({
      updated_at: new Date(Date.now() - 24 * 3_600_000).toISOString(),
    });
    const stale = skill({
      updated_at: new Date(Date.now() - 30 * 86_400_000).toISOString(),
    });
    expect(matchSkillAgainstFilters(recent, parseCentralQuery("updated:<7d"), emptyCtx)).toBe(
      true
    );
    expect(matchSkillAgainstFilters(stale, parseCentralQuery("updated:<7d"), emptyCtx)).toBe(
      false
    );
  });

  it("matches created:>2025-01-01", () => {
    const after = skill({ created_at: "2026-04-17T00:00:00.000Z" });
    const before = skill({ created_at: "2024-08-08T00:00:00.000Z" });
    expect(
      matchSkillAgainstFilters(after, parseCentralQuery("created:>2025-01-01"), emptyCtx)
    ).toBe(true);
    expect(
      matchSkillAgainstFilters(before, parseCentralQuery("created:>2025-01-01"), emptyCtx)
    ).toBe(false);
  });

  it("returns true when ast has no filters", () => {
    expect(matchSkillAgainstFilters(skill({}), parseCentralQuery(""), emptyCtx)).toBe(true);
    expect(
      matchSkillAgainstFilters(skill({}), parseCentralQuery("free text only"), emptyCtx)
    ).toBe(true);
  });
});
