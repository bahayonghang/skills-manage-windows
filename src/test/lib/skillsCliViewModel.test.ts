import { describe, expect, it } from "vitest";

import {
  SKILLS_CLI_CONTENT_CONTAINER_CLASS,
  SKILLS_CLI_FOUR_COLUMN_MIN_PX,
  SKILLS_CLI_GRID_CLASS,
  SKILLS_CLI_THREE_COLUMN_MIN_PX,
  bucketSkillsCli,
  closeSkillsCliSurface,
  deriveSkillsCliCounts,
  deriveSkillsCliLayoutBands,
  enabledTargetIdSet,
  filterSkillsCli,
  openSkillsCliDetail,
  openSkillsCliInstall,
  openSkillsCliUninstall,
  openSkillsCliUpdate,
} from "@/pages/skillsCliViewModel";
import type {
  SkillsCliGlobalSkill,
  SkillsCliInstallTarget,
  SkillsCliPlacement,
  SkillsCliPlacementState,
} from "@/types";

const targets: SkillsCliInstallTarget[] = [
  {
    id: "cursor",
    displayName: "Cursor",
    iconName: null,
    cliAgent: "cursor",
    isEnabled: true,
    defaultSelected: true,
  },
  {
    id: "amp",
    displayName: "Amp",
    iconName: null,
    cliAgent: "amp",
    isEnabled: true,
    defaultSelected: true,
  },
  {
    id: "codex",
    displayName: "Codex",
    iconName: null,
    cliAgent: "codex",
    isEnabled: false,
    defaultSelected: false,
  },
];

function placement(
  agentId: string,
  state: SkillsCliPlacementState,
  displayName = agentId,
): SkillsCliPlacement {
  return {
    agentId,
    displayName,
    targetPath: `/tmp/${agentId}/skills/x`,
    state,
    managedLinkKind: state === "managed_link" ? "windows_junction" : null,
    reasonCode: null,
  };
}

function skill(overrides: Partial<SkillsCliGlobalSkill>): SkillsCliGlobalSkill {
  return {
    name: "skill",
    path: "/tmp/skill",
    installKind: "canonical",
    scope: "global",
    agents: [],
    source: "owner/repo",
    sourceUrl: "https://github.com/owner/repo",
    sourceType: "github",
    sourceTypeBucket: "github",
    canonicalPath: "/tmp/canonical/skill",
    folderHash: null,
    installedAt: null,
    updatedAt: null,
    placements: [],
    ...overrides,
  };
}

const mixedSkills: SkillsCliGlobalSkill[] = [
  skill({
    name: "linked-only",
    source: "alpha/one",
    canonicalPath: "/canonical/linked-only",
    placements: [placement("cursor", "managed_link", "Cursor")],
  }),
  skill({
    name: "copy-skill",
    source: "alpha/one",
    path: "/copy/copy-skill",
    canonicalPath: null,
    placements: [placement("cursor", "direct_copy", "Cursor")],
  }),
  skill({
    name: "unlinked-skill",
    source: "beta/two",
    canonicalPath: "/canonical/unlinked-skill",
    placements: [
      placement("amp", "missing", "Amp"),
      placement("codex", "unavailable", "Codex"),
    ],
  }),
  skill({
    name: "conflict-skill",
    source: null,
    path: "/tmp/conflict-skill",
    canonicalPath: "/canonical/conflict-skill",
    placements: [placement("amp", "conflict", "Amp")],
  }),
  skill({
    name: "mixed-skill",
    source: "Gamma/Three",
    canonicalPath: "/canonical/mixed-skill",
    placements: [
      placement("cursor", "managed_link", "Cursor"),
      placement("amp", "missing", "Amp"),
      placement("codex", "direct_copy", "Codex"),
    ],
  }),
];

const enabledIds = enabledTargetIdSet(targets);

describe("deriveSkillsCliCounts", () => {
  it("counts installed, managed_link linked, enabled-missing unlinked, and distinct sources", () => {
    const counts = deriveSkillsCliCounts(mixedSkills, enabledIds);
    expect(counts.installed).toBe(5);
    expect(counts.linked).toBe(2);
    expect(counts.unlinked).toBe(2);
    expect(counts.repositories).toBe(3);
  });

  it("does not treat direct_copy as linked or derive unlinked from installed-linked", () => {
    const copyOnly = [
      skill({
        name: "copy",
        source: "a/b",
        placements: [placement("cursor", "direct_copy")],
      }),
    ];
    const counts = deriveSkillsCliCounts(copyOnly, enabledIds);
    expect(counts.installed).toBe(1);
    expect(counts.linked).toBe(0);
    expect(counts.unlinked).toBe(0);
    expect(counts.unlinked).not.toBe(counts.installed - counts.linked);
  });

  it("ignores missing on disabled targets and conflict/unavailable for unlinked", () => {
    const skills = [
      skill({
        name: "disabled-missing",
        placements: [placement("codex", "missing")],
      }),
      skill({
        name: "conflict",
        placements: [placement("cursor", "conflict")],
      }),
      skill({
        name: "unavailable",
        placements: [placement("amp", "unavailable")],
      }),
    ];
    const counts = deriveSkillsCliCounts(skills, enabledIds);
    expect(counts.unlinked).toBe(0);
    expect(counts.linked).toBe(0);
  });
});

describe("filterSkillsCli", () => {
  it("matches name, source label, and canonical path case-insensitively", () => {
    expect(
      filterSkillsCli(mixedSkills, { query: "LINKED-ONLY", platformFilter: null, unlinkedOnly: false }, enabledIds).map(
        (item) => item.name,
      ),
    ).toEqual(["linked-only"]);
    expect(
      filterSkillsCli(mixedSkills, { query: "beta/two", platformFilter: null, unlinkedOnly: false }, enabledIds).map(
        (item) => item.name,
      ),
    ).toEqual(["unlinked-skill"]);
    expect(
      filterSkillsCli(
        mixedSkills,
        { query: "/CANONICAL/MIXED-SKILL", platformFilter: null, unlinkedOnly: false },
        enabledIds,
      ).map((item) => item.name),
    ).toEqual(["mixed-skill"]);
  });

  it("matches a platform chip only on managed_link or direct_copy for that target", () => {
    const cursor = filterSkillsCli(
      mixedSkills,
      { query: "", platformFilter: "cursor", unlinkedOnly: false },
      enabledIds,
    ).map((item) => item.name);
    expect(cursor).toEqual(["linked-only", "copy-skill", "mixed-skill"]);
    const amp = filterSkillsCli(
      mixedSkills,
      { query: "", platformFilter: "amp", unlinkedOnly: false },
      enabledIds,
    ).map((item) => item.name);
    expect(amp).toEqual([]);
  });

  it("keeps Unlinked only on enabled missing and excludes copy, conflict, and unavailable", () => {
    const names = filterSkillsCli(
      mixedSkills,
      { query: "", platformFilter: null, unlinkedOnly: true },
      enabledIds,
    ).map((item) => item.name);
    expect(names).toEqual(["unlinked-skill", "mixed-skill"]);
    expect(names).not.toContain("copy-skill");
    expect(names).not.toContain("conflict-skill");
  });

  it("stacks query, platform chip, and Unlinked only", () => {
    const names = filterSkillsCli(
      mixedSkills,
      { query: "mixed", platformFilter: "cursor", unlinkedOnly: true },
      enabledIds,
    ).map((item) => item.name);
    expect(names).toEqual(["mixed-skill"]);
  });
});

describe("bucketSkillsCli", () => {
  it("groups by repository with first-seen order and unknown last", () => {
    const buckets = bucketSkillsCli(mixedSkills, "repo", targets, enabledIds);
    expect(buckets.map((bucket) => bucket.id)).toEqual([
      "repo:alpha/one",
      "repo:beta/two",
      "repo:Gamma/Three",
      "repo:unknown",
    ]);
    expect(buckets[buckets.length - 1]?.labelKey).toBe("skillsCli.buckets.unknown");
  });

  it("groups by platform with multi-bucket membership and a stable unlinked bucket", () => {
    const buckets = bucketSkillsCli(mixedSkills, "platform", targets, enabledIds);
    expect(buckets.map((bucket) => bucket.id)).toEqual([
      "platform:cursor",
      "platform:codex",
      "platform:unlinked",
    ]);
    expect(buckets.find((bucket) => bucket.id === "platform:cursor")?.skills.map((item) => item.name)).toEqual([
      "linked-only",
      "copy-skill",
      "mixed-skill",
    ]);
    expect(buckets.find((bucket) => bucket.id === "platform:unlinked")?.skills.map((item) => item.name)).toEqual([
      "unlinked-skill",
      "mixed-skill",
    ]);
    expect(buckets.some((bucket) => bucket.id === "platform:amp")).toBe(false);
  });

  it("groups by status with linked, unlinked, and copy-or-conflict buckets", () => {
    const buckets = bucketSkillsCli(mixedSkills, "status", targets, enabledIds);
    expect(buckets.map((bucket) => bucket.id)).toEqual([
      "status:linked",
      "status:unlinked",
      "status:copy-or-conflict",
    ]);
    expect(buckets.find((bucket) => bucket.id === "status:linked")?.skills.map((item) => item.name)).toEqual([
      "linked-only",
      "mixed-skill",
    ]);
    expect(buckets.find((bucket) => bucket.id === "status:copy-or-conflict")?.skills.map((item) => item.name)).toEqual([
      "copy-skill",
      "conflict-skill",
      "mixed-skill",
    ]);
  });

  it("uses a single stable all bucket for none grouping and drops empty buckets", () => {
    const buckets = bucketSkillsCli(mixedSkills, "none", targets, enabledIds);
    expect(buckets.map((bucket) => bucket.id)).toEqual(["none:all"]);
    expect(bucketSkillsCli([], "none", targets, enabledIds)).toEqual([]);
  });
});

describe("deriveSkillsCliLayoutBands", () => {
  it("uses shared 719/720 drawer and 899/900 plus 1179/1180 grid boundaries", () => {
    expect(deriveSkillsCliLayoutBands(719)).toEqual({
      grid: "twoColumns",
      drawer: "fullWidth",
    });
    expect(deriveSkillsCliLayoutBands(720)).toEqual({
      grid: "twoColumns",
      drawer: "fixed460",
    });
    expect(deriveSkillsCliLayoutBands(899)).toEqual({
      grid: "twoColumns",
      drawer: "fixed460",
    });
    expect(deriveSkillsCliLayoutBands(900)).toEqual({
      grid: "threeColumns",
      drawer: "fixed460",
    });
    expect(deriveSkillsCliLayoutBands(1179)).toEqual({
      grid: "threeColumns",
      drawer: "fixed460",
    });
    expect(deriveSkillsCliLayoutBands(1180)).toEqual({
      grid: "fourColumns",
      drawer: "fixed460",
    });
    expect(SKILLS_CLI_THREE_COLUMN_MIN_PX).toBe(900);
    expect(SKILLS_CLI_FOUR_COLUMN_MIN_PX).toBe(1180);
  });

  it("exports the named container grid contract classes", () => {
    expect(SKILLS_CLI_CONTENT_CONTAINER_CLASS).toContain("@container/skills-cli");
    expect(SKILLS_CLI_GRID_CLASS).toContain("grid-cols-2");
    expect(SKILLS_CLI_GRID_CLASS).toContain("@min-[900px]/skills-cli:grid-cols-3");
    expect(SKILLS_CLI_GRID_CLASS).toContain("@min-[1180px]/skills-cli:grid-cols-4");
    expect(SKILLS_CLI_GRID_CLASS).not.toMatch(/(?:^|\s)(?:md|lg|xl|min-\[[0-9]+px\]):grid-cols-/);
  });
});

describe("surface helpers", () => {
  it("opens detail with null focus by default and links focus only when requested", () => {
    expect(openSkillsCliDetail("demo", null)).toEqual({
      kind: "detail",
      skillName: "demo",
      focus: null,
    });
    expect(openSkillsCliDetail("demo", "links")).toEqual({
      kind: "detail",
      skillName: "demo",
      focus: "links",
    });
  });

  it("stores uninstall names and resets the whole surface on close", () => {
    expect(openSkillsCliUninstall(["a", "b"])).toEqual({
      kind: "uninstall",
      skillNames: ["a", "b"],
    });
    expect(closeSkillsCliSurface()).toBeNull();
  });

  it("opens install and update as distinct surface kinds", () => {
    expect(openSkillsCliInstall()).toEqual({ kind: "install" });
    expect(openSkillsCliUpdate()).toEqual({ kind: "update" });
  });
});
