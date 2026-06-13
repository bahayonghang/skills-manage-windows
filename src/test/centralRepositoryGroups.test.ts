import { describe, expect, it } from "vitest";

import {
  filterRepositorySectionsForSearch,
  groupRepositoriesForSidebar,
} from "@/lib/centralRepositoryGroups";
import type { SkillRepositoryWithStats } from "@/types";

function repo(
  id: string,
  overrides: Partial<SkillRepositoryWithStats> = {}
): SkillRepositoryWithStats {
  return {
    id,
    name: id,
    source_type: "github",
    is_unknown: false,
    pinned: false,
    skill_count: 0,
    unknown_skill_count: 0,
    created_at: "",
    updated_at: "",
    ...overrides,
  };
}

describe("groupRepositoriesForSidebar", () => {
  it("returns empty array for no repositories", () => {
    expect(groupRepositoriesForSidebar([])).toEqual([]);
  });

  it("groups GitHub repos by owner alphabetically", () => {
    const repos = [
      repo("r1", { owner: "tw93", repo: "skills", name: "tw93/skills", skill_count: 8 }),
      repo("r2", {
        owner: "anthropics",
        repo: "skills",
        name: "anthropics/skills",
        skill_count: 17,
      }),
      repo("r3", {
        owner: "anthropics",
        repo: "tools",
        name: "anthropics/tools",
        skill_count: 3,
      }),
    ];

    const sections = groupRepositoriesForSidebar(repos);
    expect(sections).toHaveLength(1);
    expect(sections[0].kind).toBe("github");
    expect(sections[0].groups.map((g) => (g.kind === "owner" ? g.owner : g.groupId))).toEqual([
      "anthropics",
      "tw93",
    ]);

    const anthropicsGroup = sections[0].groups[0];
    if (anthropicsGroup.kind !== "owner") throw new Error("expected owner");
    expect(anthropicsGroup.repositories.map((r) => r.name)).toEqual([
      "anthropics/skills",
      "anthropics/tools",
    ]);
    expect(anthropicsGroup.totalSkillCount).toBe(20);
  });

  it("sorts pinned repositories before unpinned repositories within a group", () => {
    const repos = [
      repo("r1", { owner: "openai", repo: "alpha", name: "openai/alpha" }),
      repo("r2", { owner: "openai", repo: "zeta", name: "openai/zeta", pinned: true }),
      repo("r3", { owner: "openai", repo: "beta", name: "openai/beta" }),
    ];

    const sections = groupRepositoriesForSidebar(repos);
    const group = sections[0].groups[0];
    if (group.kind !== "owner") throw new Error("expected owner");
    expect(group.repositories.map((r) => r.name)).toEqual([
      "openai/zeta",
      "openai/alpha",
      "openai/beta",
    ]);
  });

  it("falls back to github-no-owner group when owner is missing", () => {
    const repos = [
      repo("r1", { name: "loose-github", repo: "x", owner: "" }),
    ];
    const sections = groupRepositoriesForSidebar(repos);
    expect(sections).toHaveLength(1);
    expect(sections[0].groups[0]).toMatchObject({ kind: "flat", groupId: "github-no-owner" });
  });

  it("places non-github repos in local section", () => {
    const repos = [
      repo("r1", { source_type: "local", owner: "", repo: "", name: "本地 / 未来源" }),
    ];
    const sections = groupRepositoriesForSidebar(repos);
    expect(sections).toHaveLength(1);
    expect(sections[0].kind).toBe("local");
    expect(sections[0].groups[0].repositories[0].name).toBe("本地 / 未来源");
  });

  it("omits the system unknown repository when it has no skills", () => {
    const sections = groupRepositoriesForSidebar([
      repo("local-unknown", {
        source_type: "local",
        name: "本地 / 未知来源",
        is_unknown: true,
        skill_count: 0,
        unknown_skill_count: 0,
      }),
    ]);

    expect(sections).toEqual([]);
  });

  it("keeps the system unknown repository when unassigned skills exist", () => {
    const sections = groupRepositoriesForSidebar([
      repo("local-unknown", {
        source_type: "local",
        name: "本地 / 未知来源",
        is_unknown: true,
        skill_count: 0,
        unknown_skill_count: 2,
      }),
    ]);

    expect(sections).toHaveLength(1);
    expect(sections[0].kind).toBe("local");
    expect(sections[0].groups[0].repositories[0].id).toBe("local-unknown");
  });

  it("github section appears before local section", () => {
    const repos = [
      repo("r1", { source_type: "local", name: "local-1" }),
      repo("r2", { owner: "anthropics", repo: "skills", name: "anthropics/skills" }),
    ];
    const sections = groupRepositoriesForSidebar(repos);
    expect(sections.map((s) => s.kind)).toEqual(["github", "local"]);
  });

  it("sums skill_count to totalSkillCount; uses unknown_skill_count for is_unknown repos", () => {
    const repos = [
      repo("r1", {
        owner: "anthropics",
        repo: "a",
        name: "a",
        skill_count: 5,
      }),
      repo("r2", {
        owner: "anthropics",
        repo: "b",
        name: "b",
        skill_count: 3,
      }),
      repo("r3", {
        is_unknown: true,
        owner: "anthropics",
        repo: "c",
        name: "c",
        skill_count: 0,
        unknown_skill_count: 2,
      }),
    ];
    const sections = groupRepositoriesForSidebar(repos);
    expect(sections[0].totalSkillCount).toBe(10);
  });
});

describe("filterRepositorySectionsForSearch", () => {
  it("returns the grouped repository tree when query is empty", () => {
    const sections = groupRepositoriesForSidebar([
      repo("r1", { owner: "openai", repo: "skills", name: "openai/skills" }),
    ]);

    expect(filterRepositorySectionsForSearch(sections, "  ")).toEqual(sections);
  });

  it("keeps all repositories for a matching owner group", () => {
    const sections = groupRepositoriesForSidebar([
      repo("r1", { owner: "openai", repo: "alpha", name: "openai/alpha" }),
      repo("r2", {
        owner: "openai",
        repo: "zeta",
        name: "openai/zeta",
        pinned: true,
      }),
      repo("r3", {
        owner: "anthropics",
        repo: "skills",
        name: "anthropics/skills",
      }),
    ]);

    const filtered = filterRepositorySectionsForSearch(sections, "OpenAI");

    expect(filtered).toHaveLength(1);
    const group = filtered[0].groups[0];
    if (group.kind !== "owner") throw new Error("expected owner");
    expect(group.owner).toBe("openai");
    expect(group.repositories.map((r) => r.name)).toEqual([
      "openai/zeta",
      "openai/alpha",
    ]);
  });

  it("keeps only matching repository rows when the owner does not match", () => {
    const sections = groupRepositoriesForSidebar([
      repo("r1", { owner: "openai", repo: "alpha", name: "openai/alpha" }),
      repo("r2", {
        owner: "openai",
        repo: "agent-tools",
        name: "openai/agent-tools",
        url: "https://github.com/openai/agent-tools",
      }),
      repo("r3", {
        owner: "anthropics",
        repo: "skills",
        name: "anthropics/skills",
      }),
    ]);

    const filtered = filterRepositorySectionsForSearch(sections, "agent-tools");

    expect(filtered).toHaveLength(1);
    const group = filtered[0].groups[0];
    if (group.kind !== "owner") throw new Error("expected owner");
    expect(group.owner).toBe("openai");
    expect(group.repositories.map((r) => r.name)).toEqual(["openai/agent-tools"]);
  });

  it("matches local repositories by display name and id", () => {
    const sections = groupRepositoriesForSidebar([
      repo("local-tools", {
        source_type: "local",
        owner: "",
        repo: "",
        name: "Local Tooling",
      }),
      repo("local-archive", {
        source_type: "local",
        owner: "",
        repo: "",
        name: "Archived",
      }),
    ]);

    const byName = filterRepositorySectionsForSearch(sections, "tooling");
    expect(byName[0].kind).toBe("local");
    expect(byName[0].groups[0].repositories.map((r) => r.id)).toEqual(["local-tools"]);

    const byId = filterRepositorySectionsForSearch(sections, "local-archive");
    expect(byId[0].groups[0].repositories.map((r) => r.id)).toEqual(["local-archive"]);
  });

  it("returns no sections when no repository matches", () => {
    const sections = groupRepositoriesForSidebar([
      repo("r1", { owner: "openai", repo: "skills", name: "openai/skills" }),
    ]);

    expect(filterRepositorySectionsForSearch(sections, "not-present")).toEqual([]);
  });

  it("does not reveal unknown repositories hidden by the grouping step", () => {
    const sections = groupRepositoriesForSidebar([
      repo("local-unknown", {
        source_type: "local",
        name: "本地 / 未知来源",
        is_unknown: true,
        skill_count: 0,
        unknown_skill_count: 0,
      }),
    ]);

    expect(filterRepositorySectionsForSearch(sections, "unknown")).toEqual([]);
  });
});
