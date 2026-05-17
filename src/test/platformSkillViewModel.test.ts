import { describe, expect, it } from "vitest";

import {
  derivePlatformSkillRows,
  type PlatformSkillGroupLabels,
} from "@/lib/platformSkillViewModel";
import type { ScannedSkill, SkillRepository } from "@/types";

const labels: PlatformSkillGroupLabels = {
  all: "All skills",
  localSource: "Local / user source",
  pluginSource: "Plugin source",
  unknownSource: "Unknown source",
};

function repo(overrides: Partial<SkillRepository> & { id: string; name: string }): SkillRepository {
  return {
    source_type: "github",
    owner: "owner",
    repo: "repo",
    branch: "main",
    url: "https://github.com/owner/repo",
    is_unknown: false,
    created_at: "2026-01-01T00:00:00Z",
    updated_at: "2026-01-01T00:00:00Z",
    ...overrides,
  };
}

function skill(overrides: Partial<ScannedSkill> & { id: string; name: string }): ScannedSkill {
  return {
    description: `${overrides.name} description`,
    file_path: `/skills/${overrides.id}/SKILL.md`,
    dir_path: `/skills/${overrides.id}`,
    link_type: "copy",
    is_central: false,
    scanned_at: "2026-01-01T00:00:00Z",
    ...overrides,
  };
}

describe("platformSkillViewModel", () => {
  it("combines source filtering, expanded search fields, and name sorting", () => {
    const rows = derivePlatformSkillRows({
      skills: [
        skill({
          id: "beta",
          name: "Beta",
          repository: repo({ id: "repo-beta", name: "tools/beta" }),
        }),
        skill({
          id: "alpha",
          name: "Alpha",
          source_kind: "plugin",
          source_root: "/tmp/.codex/plugins/cache/openai/example/1.0.0",
        }),
      ],
      searchQuery: "openai/example",
      sourceFilter: "plugin",
      sort: { field: "name", direction: "asc" },
      groupBy: "none",
      labels,
    });

    expect(rows.sourceFilteredSkills.map((row) => row.id)).toEqual(["alpha"]);
    expect(rows.filteredSkills.map((row) => row.id)).toEqual(["alpha"]);
    expect(rows.sortedSkills.map((row) => row.id)).toEqual(["alpha"]);
  });

  it("sorts installed and updated timestamps with name tie-breakers", () => {
    const skills = [
      skill({
        id: "bravo",
        name: "Bravo",
        installed_at: "2026-01-02T00:00:00Z",
        updated_at: "2026-01-05T00:00:00Z",
      }),
      skill({
        id: "alpha",
        name: "Alpha",
        created_at: "2026-01-02T00:00:00Z",
        updated_at: "2026-01-05T00:00:00Z",
      }),
      skill({
        id: "charlie",
        name: "Charlie",
        installed_at: "2026-01-04T00:00:00Z",
        updated_at: "2026-01-03T00:00:00Z",
      }),
    ];

    expect(
      derivePlatformSkillRows({
        skills,
        searchQuery: "",
        sourceFilter: "all",
        sort: { field: "installedAt", direction: "asc" },
        groupBy: "none",
        labels,
      }).sortedSkills.map((row) => row.name)
    ).toEqual(["Alpha", "Bravo", "Charlie"]);

    expect(
      derivePlatformSkillRows({
        skills,
        searchQuery: "",
        sourceFilter: "all",
        sort: { field: "updatedAt", direction: "desc" },
        groupBy: "none",
        labels,
      }).sortedSkills.map((row) => row.name)
    ).toEqual(["Alpha", "Bravo", "Charlie"]);
  });

  it("groups by central repository first, plugin source next, and local fallback last", () => {
    const rows = derivePlatformSkillRows({
      skills: [
        skill({
          id: "local",
          name: "Local",
          source_kind: "user",
          source_root: "/tmp/.claude/skills",
        }),
        skill({
          id: "plugin",
          name: "Plugin",
          source_kind: "plugin",
          source_root: "/tmp/.claude/plugins/cache/publisher/plugin-a/1.0.0",
          is_read_only: true,
        }),
        skill({
          id: "repo",
          name: "Repo",
          repository: repo({ id: "github-owner-repo-main", name: "owner/repo" }),
        }),
      ],
      searchQuery: "",
      sourceFilter: "all",
      sort: { field: "repository", direction: "asc" },
      groupBy: "repository",
      labels,
    });

    expect(rows.groups.map((group) => group.label)).toEqual([
      "owner/repo",
      "publisher/plugin-a",
      "Local / user source",
    ]);
    expect(rows.groups.map((group) => group.skills.map((row) => row.id))).toEqual([
      ["repo"],
      ["plugin"],
      ["local"],
    ]);
  });
});
