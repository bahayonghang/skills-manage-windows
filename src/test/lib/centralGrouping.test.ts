import { describe, expect, it } from "vitest";

import { groupSkillsByMode, type GroupingContext } from "@/lib/centralGrouping";
import type {
  CentralSkillUpdateState,
  SkillRepository,
  SkillTag,
  SkillWithLinks,
} from "@/types";

function makeRepo(over: Partial<SkillRepository> & { id: string; name: string }): SkillRepository {
  return {
    id: over.id,
    name: over.name,
    source_type: over.source_type ?? "local",
    owner: over.owner,
    repo: over.repo,
    branch: over.branch,
    url: over.url,
    pinned: over.pinned ?? false,
    is_unknown: over.is_unknown ?? false,
    created_at: over.created_at ?? "2026-01-01T00:00:00Z",
    updated_at: over.updated_at ?? "2026-01-01T00:00:00Z",
  };
}

function makeTag(over: Partial<SkillTag> & { id: string; name: string }): SkillTag {
  return {
    id: over.id,
    name: over.name,
    description: over.description,
    color: over.color,
    is_builtin: over.is_builtin ?? false,
    created_at: over.created_at ?? "2026-01-01T00:00:00Z",
    updated_at: over.updated_at ?? "2026-01-01T00:00:00Z",
    group_id: over.group_id,
  };
}

function makeSkill(overrides: Partial<SkillWithLinks>): SkillWithLinks {
  return {
    id: overrides.id ?? "skill-1",
    name: overrides.name ?? "Skill",
    description: overrides.description,
    file_path: overrides.file_path ?? "/tmp/skill",
    is_central: overrides.is_central ?? true,
    source: overrides.source,
    scanned_at: overrides.scanned_at ?? "2026-01-01T00:00:00Z",
    repository: overrides.repository,
    source_path: overrides.source_path,
    tags: overrides.tags ?? [],
    linked_agents: overrides.linked_agents ?? [],
    shared_root_agents: overrides.shared_root_agents ?? [],
  };
}

function ctx(updateStatuses: Record<string, CentralSkillUpdateState> = {}): GroupingContext {
  return {
    updateStatuses,
    labels: {
      all: "All",
      uncategorized: "Uncategorized",
      unknownOwner: "Unknown owner",
      localRepos: "Local",
      statusUpToDate: "Up to date",
      statusNeedsUpdate: "Needs update",
      statusUnknown: "Status unknown",
    },
  };
}

describe("groupSkillsByMode", () => {
  it("mode=none returns a single group containing all skills", () => {
    const skills = [makeSkill({ id: "a" }), makeSkill({ id: "b" })];
    const groups = groupSkillsByMode(skills, "none", ctx());
    expect(groups).toHaveLength(1);
    expect(groups[0]?.key).toBe("__all__");
    expect(groups[0]?.skills.map((s) => s.id)).toEqual(["a", "b"]);
  });

  it("mode=repository buckets by repository_id and falls back to unassigned", () => {
    const skills = [
      makeSkill({
        id: "a",
        repository: makeRepo({ id: "r1", name: "Repo One" }),
      }),
      makeSkill({
        id: "b",
        repository: makeRepo({ id: "r2", name: "Repo Two" }),
      }),
      makeSkill({ id: "c" }),
    ];
    const groups = groupSkillsByMode(skills, "repository", ctx());
    const keys = groups.map((g) => g.key);
    expect(keys).toContain("repo:r1");
    expect(keys).toContain("repo:r2");
    expect(keys).toContain("repo:__unassigned__");
    // unassigned sinks to end
    expect(keys[keys.length - 1]).toBe("repo:__unassigned__");
  });

  it("mode=owner buckets github repos by owner and locals together", () => {
    const skills = [
      makeSkill({
        id: "a",
        repository: makeRepo({ id: "r1", name: "anthropic/x", source_type: "github", owner: "anthropic" }),
      }),
      makeSkill({
        id: "b",
        repository: makeRepo({ id: "r2", name: "anthropic/y", source_type: "github", owner: "anthropic" }),
      }),
      makeSkill({
        id: "c",
        repository: makeRepo({ id: "r3", name: "local-x" }),
      }),
    ];
    const groups = groupSkillsByMode(skills, "owner", ctx());
    const ownerGroup = groups.find((g) => g.key === "owner:gh:anthropic");
    const localGroup = groups.find((g) => g.key === "owner:local");
    expect(ownerGroup?.skills.map((s) => s.id)).toEqual(["a", "b"]);
    expect(localGroup?.skills.map((s) => s.id)).toEqual(["c"]);
    expect(ownerGroup?.label).toBe("anthropic");
  });

  it("mode=tag creates one group per tag and includes skill in each", () => {
    const skills = [
      makeSkill({
        id: "a",
        tags: [makeTag({ id: "t1", name: "Coding" }), makeTag({ id: "t2", name: "DevOps" })],
      }),
      makeSkill({ id: "b", tags: [makeTag({ id: "t2", name: "DevOps" })] }),
      makeSkill({ id: "c", tags: [] }),
    ];
    const groups = groupSkillsByMode(skills, "tag", ctx());
    const codingGroup = groups.find((g) => g.key === "tag:t1");
    const devopsGroup = groups.find((g) => g.key === "tag:t2");
    const uncatGroup = groups.find((g) => g.key === "__uncategorized__");
    expect(codingGroup?.skills.map((s) => s.id)).toEqual(["a"]);
    expect(devopsGroup?.skills.map((s) => s.id)).toEqual(["a", "b"]);
    expect(uncatGroup?.skills.map((s) => s.id)).toEqual(["c"]);
  });

  it("mode=status buckets by updateStatuses (needs_update first)", () => {
    const skills = [
      makeSkill({ id: "a" }),
      makeSkill({ id: "b" }),
      makeSkill({ id: "c" }),
    ];
    const statuses: Record<string, CentralSkillUpdateState> = {
      a: { skill_id: "a", source_type: "github", status: "update_available" } satisfies CentralSkillUpdateState,
      b: { skill_id: "b", source_type: "github", status: "up_to_date" } satisfies CentralSkillUpdateState,
    };
    const groups = groupSkillsByMode(skills, "status", ctx(statuses));
    expect(groups[0]?.key).toBe("status:update_available");
    expect(groups[0]?.skills.map((s) => s.id)).toEqual(["a"]);
    expect(groups.find((g) => g.key === "status:up_to_date")?.skills.map((s) => s.id)).toEqual(["b"]);
    expect(groups.find((g) => g.key === "status:unknown")?.skills.map((s) => s.id)).toEqual(["c"]);
  });

  it("preserves input order within groups", () => {
    const skills = [
      makeSkill({ id: "z" }),
      makeSkill({ id: "a" }),
      makeSkill({ id: "m" }),
    ];
    const groups = groupSkillsByMode(skills, "none", ctx());
    expect(groups[0]?.skills.map((s) => s.id)).toEqual(["z", "a", "m"]);
  });

  it("returns single group when input is empty", () => {
    const groups = groupSkillsByMode([], "repository", ctx());
    expect(groups).toEqual([]);
  });

  it("unknown mode falls back to single group", () => {
    const skills = [makeSkill({ id: "a" })];
    // @ts-expect-error 测试未知模式的 fallback 行为
    const groups = groupSkillsByMode(skills, "weird", ctx());
    expect(groups).toHaveLength(1);
    expect(groups[0]?.key).toBe("__all__");
  });
});
