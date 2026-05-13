import { describe, expect, it } from "vitest";

import {
  computeFacetCounts,
  type FacetCountsContext,
  type FacetSelections,
} from "@/lib/centralFacetCounts";
import type {
  SkillRepositoryWithStats,
  SkillTag,
  SkillWithLinks,
} from "@/types";

function repo(
  id: string,
  overrides: Partial<SkillRepositoryWithStats> = {}
): SkillRepositoryWithStats {
  return {
    id,
    name: id,
    source_type: "github",
    is_unknown: false,
    skill_count: 0,
    unknown_skill_count: 0,
    created_at: "",
    updated_at: "",
    ...overrides,
  };
}

function tag(id: string): SkillTag {
  return { id, name: id, is_builtin: false, created_at: "", updated_at: "" };
}

function skill(
  id: string,
  overrides: Partial<SkillWithLinks> = {}
): SkillWithLinks {
  return {
    id,
    name: id,
    file_path: `/${id}`,
    is_central: true,
    scanned_at: "2026-04-17T00:00:00.000Z",
    linked_agents: [],
    shared_root_agents: [],
    tags: [],
    ...overrides,
  };
}

const noSelections: FacetSelections = { repositories: [], tags: [] };
const emptyCtx: FacetCountsContext = {
  updateStatuses: {},
  aiReviewSkillIds: new Set(),
};

describe("computeFacetCounts", () => {
  it("counts skills per repository", () => {
    const repoA = repo("rA");
    const repoB = repo("rB");
    const repos = [repoA, repoB];

    const skills = [
      skill("s1", { repository: repoA }),
      skill("s2", { repository: repoA }),
      skill("s3", { repository: repoB }),
    ];

    const counts = computeFacetCounts(skills, repos, [], noSelections, emptyCtx);
    expect(counts.repositories.rA).toBe(2);
    expect(counts.repositories.rB).toBe(1);
    expect(counts.repositories.all).toBe(3);
    expect(counts.repositories.unassigned).toBe(0);
  });

  it("counts unassigned (is_source_unknown)", () => {
    const skills = [
      skill("s1", { is_source_unknown: true }),
      skill("s2", { repository: repo("rA") }),
    ];
    const counts = computeFacetCounts(skills, [repo("rA")], [], noSelections, emptyCtx);
    expect(counts.repositories.unassigned).toBe(1);
    expect(counts.repositories.rA).toBe(1);
  });

  it("counts skills per tag", () => {
    const skills = [
      skill("s1", { tags: [tag("editor")] }),
      skill("s2", { tags: [tag("editor"), tag("writing")] }),
      skill("s3", { tags: [tag("writing")] }),
    ];
    const counts = computeFacetCounts(
      skills,
      [],
      [tag("editor"), tag("writing")],
      noSelections,
      emptyCtx
    );
    expect(counts.tags.editor).toBe(2);
    expect(counts.tags.writing).toBe(2);
  });

  it("smart views: all / uncategorized / updates / aiReview", () => {
    const skills = [
      skill("s1", { tags: [] }),
      skill("s2", { tags: [tag("editor")] }),
      skill("s3", { tags: [] }),
    ];
    const ctx: FacetCountsContext = {
      updateStatuses: {
        s1: { skill_id: "s1", source_type: "github", status: "update_available" },
      },
      aiReviewSkillIds: new Set(["s2"]),
    };
    const counts = computeFacetCounts(skills, [], [tag("editor")], noSelections, ctx);
    expect(counts.smartViews.all).toBe(3);
    expect(counts.smartViews.uncategorized).toBe(2);
    expect(counts.smartViews.updates).toBe(1);
    expect(counts.smartViews.aiReview).toBe(1);
  });

  it("dependent counts: tag count is repo-aware", () => {
    const repoA = repo("rA");
    const repoB = repo("rB");

    const skills = [
      skill("s1", { repository: repoA, tags: [tag("editor")] }),
      skill("s2", { repository: repoA, tags: [tag("writing")] }),
      skill("s3", { repository: repoB, tags: [tag("editor")] }),
    ];

    // 选了 repo:rA：tag 计数应反映"在 rA 下"
    const counts = computeFacetCounts(
      skills,
      [repoA, repoB],
      [tag("editor"), tag("writing")],
      { repositories: ["rA"], tags: [] },
      emptyCtx
    );
    expect(counts.tags.editor).toBe(1); // 仅 s1
    expect(counts.tags.writing).toBe(1); // 仅 s2

    // 仓库计数不受 tag 选择影响
    expect(counts.repositories.rA).toBe(2);
    expect(counts.repositories.rB).toBe(1);
  });

  it("dependent counts: repo count is tag-aware", () => {
    const repoA = repo("rA");
    const repoB = repo("rB");

    const skills = [
      skill("s1", { repository: repoA, tags: [tag("editor")] }),
      skill("s2", { repository: repoA, tags: [tag("writing")] }),
      skill("s3", { repository: repoB, tags: [tag("editor")] }),
    ];

    const counts = computeFacetCounts(
      skills,
      [repoA, repoB],
      [tag("editor"), tag("writing")],
      { repositories: [], tags: ["editor"] },
      emptyCtx
    );
    expect(counts.repositories.rA).toBe(1); // 仅 s1（s2 不带 editor）
    expect(counts.repositories.rB).toBe(1); // s3
  });

  it("returns zero counts for empty inputs", () => {
    const counts = computeFacetCounts([], [], [], noSelections, emptyCtx);
    expect(counts.repositories.all).toBe(0);
    expect(counts.smartViews.all).toBe(0);
  });
});
