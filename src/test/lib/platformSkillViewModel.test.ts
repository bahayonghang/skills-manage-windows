import { describe, expect, it } from "vitest";

import {
  derivePlatformOriginNav,
  derivePlatformSkillRows,
  getPlatformOriginRepoKey,
  getPlatformSkillOrigin,
  type PlatformSkillGroupLabels,
} from "@/lib/platformSkillViewModel";
import type { ScannedSkill, SkillRepository } from "@/types";

const labels: PlatformSkillGroupLabels = {
  all: "All skills",
  localSource: "Local / user source",
  pluginSource: "Plugin source",
  unknownSource: "Unknown source",
};

function repo(
  overrides: Partial<SkillRepository> & { id: string; name: string },
): SkillRepository {
  return {
    source_type: "github",
    owner: "owner",
    repo: "repo",
    branch: "main",
    url: "https://github.com/owner/repo",
    pinned: false,
    is_unknown: false,
    created_at: "2026-01-01T00:00:00Z",
    updated_at: "2026-01-01T00:00:00Z",
    ...overrides,
  };
}

function skill(
  overrides: Partial<ScannedSkill> & { id: string; name: string },
): ScannedSkill {
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
      originFilter: { kind: "all" },
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
        originFilter: { kind: "all" },
        sort: { field: "installedAt", direction: "asc" },
        groupBy: "none",
        labels,
      }).sortedSkills.map((row) => row.name),
    ).toEqual(["Alpha", "Bravo", "Charlie"]);

    expect(
      derivePlatformSkillRows({
        skills,
        searchQuery: "",
        sourceFilter: "all",
        originFilter: { kind: "all" },
        sort: { field: "updatedAt", direction: "desc" },
        groupBy: "none",
        labels,
      }).sortedSkills.map((row) => row.name),
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
          repository: repo({
            id: "github-owner-repo-main",
            name: "owner/repo",
          }),
        }),
      ],
      searchQuery: "",
      sourceFilter: "all",
      originFilter: { kind: "all" },
      sort: { field: "repository", direction: "asc" },
      groupBy: "repository",
      labels,
    });

    expect(rows.groups.map((group) => group.label)).toEqual([
      "owner/repo",
      "publisher/plugin-a",
      "Local / user source",
    ]);
    expect(
      rows.groups.map((group) => group.skills.map((row) => row.id)),
    ).toEqual([["repo"], ["plugin"], ["local"]]);
  });

  it("sorts by 30-day call count with missing counts treated as zero", () => {
    const skills = [
      skill({ id: "unused-beta", name: "Unused Beta" }),
      skill({ id: "heavy", name: "Heavy Usage" }),
      skill({ id: "unused-alpha", name: "Unused Alpha" }),
      skill({ id: "light", name: "Light Usage" }),
    ];

    const ascRows = derivePlatformSkillRows({
      skills,
      searchQuery: "",
      sourceFilter: "all",
      originFilter: { kind: "all" },
      sort: { field: "callCount", direction: "asc" },
      groupBy: "none",
      labels,
      usageCounts: {
        "Heavy Usage": 12,
        "Light Usage": 2,
      },
    });

    expect(ascRows.sortedSkills.map((row) => row.name)).toEqual([
      "Unused Alpha",
      "Unused Beta",
      "Light Usage",
      "Heavy Usage",
    ]);

    const descRows = derivePlatformSkillRows({
      skills,
      searchQuery: "",
      sourceFilter: "all",
      originFilter: { kind: "all" },
      sort: { field: "callCount", direction: "desc" },
      groupBy: "none",
      labels,
      usageCounts: {
        "Heavy Usage": 12,
        "Light Usage": 2,
      },
    });

    expect(descRows.sortedSkills.map((row) => row.name)).toEqual([
      "Heavy Usage",
      "Light Usage",
      "Unused Alpha",
      "Unused Beta",
    ]);
  });

  describe("getPlatformSkillOrigin", () => {
    it("classifies symlink rows as central and everything else as standalone", () => {
      expect(
        getPlatformSkillOrigin(
          skill({ id: "a", name: "A", link_type: "symlink" }),
        ),
      ).toBe("central");
      expect(
        getPlatformSkillOrigin(
          skill({ id: "b", name: "B", link_type: "copy" }),
        ),
      ).toBe("standalone");
      expect(
        getPlatformSkillOrigin(
          skill({ id: "c", name: "C", link_type: "native" }),
        ),
      ).toBe("standalone");
    });
  });

  describe("getPlatformOriginRepoKey", () => {
    it("keys assigned repositories by id and treats unknown or missing repositories as unassigned", () => {
      expect(
        getPlatformOriginRepoKey(
          skill({
            id: "a",
            name: "A",
            repository: repo({ id: "repo-a", name: "tools/alpha" }),
          }),
        ),
      ).toBe("repo:repo-a");
      expect(
        getPlatformOriginRepoKey(
          skill({
            id: "b",
            name: "B",
            repository: repo({
              id: "unknown",
              name: "Unknown",
              is_unknown: true,
            }),
          }),
        ),
      ).toBe("unassigned");
      expect(getPlatformOriginRepoKey(skill({ id: "c", name: "C" }))).toBe(
        "unassigned",
      );
    });
  });

  describe("derivePlatformOriginNav", () => {
    it("conserves counts, sorts repo buckets by label, and routes unknown/missing repos to unassigned", () => {
      const nav = derivePlatformOriginNav([
        skill({
          id: "bravo-1",
          name: "Bravo One",
          link_type: "symlink",
          repository: repo({ id: "repo-bravo", name: "tools/bravo" }),
        }),
        skill({
          id: "alpha-1",
          name: "Alpha One",
          link_type: "symlink",
          repository: repo({ id: "repo-alpha", name: "tools/alpha" }),
        }),
        skill({
          id: "alpha-2",
          name: "Alpha Two",
          link_type: "symlink",
          repository: repo({ id: "repo-alpha", name: "tools/alpha" }),
        }),
        skill({
          id: "unknown-repo",
          name: "Unknown Repo",
          link_type: "symlink",
          repository: repo({
            id: "unknown",
            name: "Unknown",
            is_unknown: true,
          }),
        }),
        skill({ id: "no-repo", name: "No Repo", link_type: "symlink" }),
        skill({ id: "copied", name: "Copied", link_type: "copy" }),
        skill({ id: "native", name: "Native", link_type: "native" }),
      ]);

      expect(nav.total).toBe(7);
      expect(nav.centralCount).toBe(5);
      expect(nav.standaloneCount).toBe(2);
      expect(nav.centralCount + nav.standaloneCount).toBe(nav.total);
      expect(nav.repos).toEqual([
        { key: "repo:repo-alpha", label: "tools/alpha", count: 2 },
        { key: "repo:repo-bravo", label: "tools/bravo", count: 1 },
      ]);
      expect(nav.unassignedCentralCount).toBe(2);
      expect(
        nav.repos.reduce((sum, bucket) => sum + bucket.count, 0) +
          nav.unassignedCentralCount,
      ).toBe(nav.centralCount);
    });

    it("does not count standalone rows with an assigned repository into repo buckets", () => {
      const nav = derivePlatformOriginNav([
        skill({
          id: "handoff",
          name: "Handoff Copy",
          link_type: "copy",
          repository: repo({ id: "repo-alpha", name: "tools/alpha" }),
        }),
      ]);

      expect(nav.total).toBe(1);
      expect(nav.centralCount).toBe(0);
      expect(nav.standaloneCount).toBe(1);
      expect(nav.repos).toEqual([]);
      expect(nav.unassignedCentralCount).toBe(0);
    });

    it("falls back to owner/repo when the repository has no name", () => {
      const nav = derivePlatformOriginNav([
        skill({
          id: "unnamed",
          name: "Unnamed Repo Skill",
          link_type: "symlink",
          repository: repo({
            id: "repo-unnamed",
            name: "",
            owner: "acme",
            repo: "widgets",
          }),
        }),
      ]);

      expect(nav.repos).toEqual([
        { key: "repo:repo-unnamed", label: "acme/widgets", count: 1 },
      ]);
    });
  });

  describe("originFilter pipeline", () => {
    const originSkills = [
      skill({
        id: "alpha-central",
        name: "Alpha Central",
        link_type: "symlink",
        repository: repo({ id: "repo-a", name: "tools/alpha" }),
      }),
      skill({
        id: "unassigned-central",
        name: "Unassigned Central",
        link_type: "symlink",
      }),
      skill({
        id: "standalone-copy",
        name: "Standalone Copy",
        link_type: "copy",
      }),
    ];

    function rowsFor(
      originFilter: Parameters<
        typeof derivePlatformSkillRows
      >[0]["originFilter"],
    ) {
      return derivePlatformSkillRows({
        skills: originSkills,
        searchQuery: "",
        sourceFilter: "all",
        originFilter,
        sort: { field: "name", direction: "asc" },
        groupBy: "none",
        labels,
      });
    }

    it("filters rows for each origin filter branch", () => {
      expect(
        rowsFor({ kind: "all" }).originFilteredSkills.map((row) => row.id),
      ).toEqual(["alpha-central", "unassigned-central", "standalone-copy"]);
      expect(
        rowsFor({ kind: "standalone" }).originFilteredSkills.map(
          (row) => row.id,
        ),
      ).toEqual(["standalone-copy"]);
      expect(
        rowsFor({ kind: "central" }).originFilteredSkills.map((row) => row.id),
      ).toEqual(["alpha-central", "unassigned-central"]);
      expect(
        rowsFor({
          kind: "central",
          repoKey: "repo:repo-a",
        }).originFilteredSkills.map((row) => row.id),
      ).toEqual(["alpha-central"]);
      expect(
        rowsFor({
          kind: "central",
          repoKey: "unassigned",
        }).originFilteredSkills.map((row) => row.id),
      ).toEqual(["unassigned-central"]);
    });

    it("applies the origin filter after the source tab filter and before search", () => {
      const rows = derivePlatformSkillRows({
        skills: [
          skill({
            id: "user-central",
            name: "User Central Match",
            link_type: "symlink",
            source_kind: "user",
          }),
          skill({
            id: "user-standalone-match",
            name: "User Standalone Match",
            link_type: "copy",
            source_kind: "user",
          }),
          skill({
            id: "user-standalone-other",
            name: "User Standalone Other",
            link_type: "copy",
            source_kind: "user",
          }),
          skill({
            id: "plugin-standalone",
            name: "Plugin Standalone Match",
            link_type: "copy",
            source_kind: "plugin",
          }),
        ],
        searchQuery: "match",
        sourceFilter: "user",
        originFilter: { kind: "standalone" },
        sort: { field: "name", direction: "asc" },
        groupBy: "none",
        labels,
      });

      // sourceFilteredSkills 只做 tab 过滤，不含 origin 过滤（导航计数口径）
      expect(rows.sourceFilteredSkills.map((row) => row.id)).toEqual([
        "user-central",
        "user-standalone-match",
        "user-standalone-other",
      ]);
      // originFilteredSkills 在搜索之前（导航空态口径）
      expect(rows.originFilteredSkills.map((row) => row.id)).toEqual([
        "user-standalone-match",
        "user-standalone-other",
      ]);
      // filteredSkills 叠加搜索
      expect(rows.filteredSkills.map((row) => row.id)).toEqual([
        "user-standalone-match",
      ]);
    });
  });
});
