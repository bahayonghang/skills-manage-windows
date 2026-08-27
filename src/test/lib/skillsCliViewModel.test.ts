import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { describe, expect, it, vi } from "vitest";

import en from "@/i18n/locales/en.json";
import zh from "@/i18n/locales/zh.json";
import {
  aggregateRemovalImpact,
  allPlacementsUnavailable,
  buildSkillsCliExportV1,
  defaultCleanupSelectedNames,
  deriveCleanupCandidates,
  partitionLinkBatch,
  partitionUnlinkBatch,
  partitionUnlinkBatchForAgent,
  reconcileSelectedNames,
  selectedSkillsInStoreOrder,
  skillsCliExportFileName,
  stringifySkillsCliExportV1,
  summarizeLinkTargets,
} from "@/pages/skillsCliBatchModel";
import { exportSkillsCliInventory } from "@/pages/skillsCliExport";
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
  groupSkillNamesByRepositoryKey,
  openSkillsCliCleanup,
  openSkillsCliDetail,
  openSkillsCliInstall,
  openSkillsCliUninstall,
  openSkillsCliUpdate,
  argvPreviewForSelection,
  isUpdateApplyEnabled,
  isUpdateReinstallEnabled,
  actionableUpdateSkillNames,
  pendingUpdateCountForSkills,
  skillsCliUpdateStatuses,
  visibleUpdateStatus,
} from "@/pages/skillsCliViewModel";
import type {
  SkillsCliGlobalSkill,
  SkillsCliInstallTarget,
  SkillsCliPlacement,
  SkillsCliPlacementState,
  SkillsCliUpdateInventory,
  SkillsCliUpdateSkillRow,
} from "@/types";
import { EMPTY_SKILLS_CLI_UPDATE_INVENTORY } from "@/types";

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
  reasonCode: string | null = null,
): SkillsCliPlacement {
  return {
    agentId,
    displayName,
    targetPath: `/tmp/${agentId}/skills/x`,
    state,
    managedLinkKind: state === "managed_link" ? "windows_junction" : null,
    reasonCode,
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
    expect(deriveSkillsCliLayoutBands(720).grid).toBe("twoColumns");
    expect(deriveSkillsCliLayoutBands(1000).grid).toBe("threeColumns");
    expect(deriveSkillsCliLayoutBands(1280).grid).toBe("fourColumns");
    expect(SKILLS_CLI_THREE_COLUMN_MIN_PX).toBe(900);
    expect(SKILLS_CLI_FOUR_COLUMN_MIN_PX).toBe(1180);
    expect(SKILLS_CLI_GRID_CLASS).toContain("gap-3");
  });

  it("exports the named container grid contract classes", () => {
    expect(SKILLS_CLI_CONTENT_CONTAINER_CLASS).toContain("@container/skills-cli");
    expect(SKILLS_CLI_GRID_CLASS).toContain("grid-cols-2");
    expect(SKILLS_CLI_GRID_CLASS).toContain("@min-[900px]/skills-cli:grid-cols-3");
    expect(SKILLS_CLI_GRID_CLASS).toContain("@min-[1180px]/skills-cli:grid-cols-4");
    expect(SKILLS_CLI_GRID_CLASS).not.toMatch(/(?:^|\s)(?:md|lg|xl|min-\[[0-9]+px\]):grid-cols-/);
  });
});

describe("cleanup candidate grouping", () => {
  it("puts canonical_missing skills in the stale group and selects them by default", () => {
    const stale = skill({
      name: "ghost",
      placements: [
        placement("cursor", "unavailable", "Cursor", "canonical_missing"),
        placement("amp", "unavailable", "Amp", "canonical_missing"),
      ],
    });
    const candidates = deriveCleanupCandidates([stale]);
    expect(allPlacementsUnavailable(stale.placements)).toBe(true);
    expect(candidates).toEqual([
      {
        name: "ghost",
        group: "stale",
        reasons: [
          { platform: "Cursor", reasonCode: "canonical_missing" },
          { platform: "Amp", reasonCode: "canonical_missing" },
        ],
      },
    ]);
    expect(defaultCleanupSelectedNames(candidates)).toEqual(["ghost"]);
  });

  it("puts healthy skills that are only platform-unavailable in the unchecked group", () => {
    const healthy = skill({
      name: "healthy",
      placements: [
        placement("cursor", "unavailable", "Cursor", "platform_not_detected"),
        placement("amp", "unavailable", "Amp", "platform_disabled"),
      ],
    });
    const candidates = deriveCleanupCandidates([healthy]);
    expect(candidates).toHaveLength(1);
    expect(candidates[0]?.group).toBe("platformUnavailable");
    expect(candidates[0]?.reasons).toEqual([
      { platform: "Cursor", reasonCode: "platform_not_detected" },
      { platform: "Amp", reasonCode: "platform_disabled" },
    ]);
    expect(defaultCleanupSelectedNames(candidates)).toEqual([]);
  });

  it("keeps mixed platform reasons in a single platformUnavailable group", () => {
    const mixed = skill({
      name: "mixed-unavailable",
      placements: [
        placement("cursor", "unavailable", "Cursor", "platform_not_detected"),
        placement("amp", "unavailable", "Amp", "platform_disabled"),
      ],
    });
    const linked = skill({
      name: "still-linked",
      placements: [placement("cursor", "managed_link", "Cursor")],
    });
    const missingBadge = skill({
      name: "has-missing",
      placements: [
        placement("cursor", "unavailable", "Cursor", "platform_disabled"),
        placement("amp", "missing", "Amp"),
      ],
    });
    const candidates = deriveCleanupCandidates([mixed, linked, missingBadge]);
    expect(candidates.map((item) => item.name)).toEqual(["mixed-unavailable"]);
    expect(candidates[0]?.group).toBe("platformUnavailable");
    expect(candidates[0]?.reasons).toHaveLength(2);
    expect(allPlacementsUnavailable(missingBadge.placements)).toBe(false);
  });

  it("puts platform_unsupported skills in the unchecked platformUnavailable group", () => {
    const unsupported = skill({
      name: "unsupported-only",
      placements: [
        placement("cursor", "unavailable", "Cursor", "platform_unsupported"),
      ],
    });
    const candidates = deriveCleanupCandidates([unsupported]);
    expect(candidates).toEqual([
      {
        name: "unsupported-only",
        group: "platformUnavailable",
        reasons: [
          { platform: "Cursor", reasonCode: "platform_unsupported" },
        ],
      },
    ]);
    expect(defaultCleanupSelectedNames(candidates)).toEqual([]);
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
    expect(
      openSkillsCliUpdate({
        repositoryKey: "owner/repo@main",
        skillNames: ["demo"],
      }),
    ).toEqual({
      kind: "update",
      repositoryKey: "owner/repo@main",
      skillNames: ["demo"],
    });
    expect(openSkillsCliCleanup()).toEqual({ kind: "cleanup" });
  });
});

describe("i18n parity for batch-actions strings", () => {
  it("keeps en/zh keys aligned for batch, export, uninstall impact, and new backend codes", () => {
    function keysOf(value: unknown, prefix = ""): string[] {
      if (!value || typeof value !== "object" || Array.isArray(value)) {
        return prefix ? [prefix] : [];
      }
      return Object.entries(value as Record<string, unknown>).flatMap(([key, child]) =>
        keysOf(child, prefix ? `${prefix}.${key}` : key),
      );
    }
    expect(keysOf(en.skillsCli.batch)).toEqual(keysOf(zh.skillsCli.batch));
    expect(keysOf(en.skillsCli.export)).toEqual(keysOf(zh.skillsCli.export));
    expect(keysOf(en.skillsCli.uninstallImpact)).toEqual(
      keysOf(zh.skillsCli.uninstallImpact),
    );
    expect(keysOf(en.skillsCli.cleanup)).toEqual(keysOf(zh.skillsCli.cleanup));
    expect(keysOf(en.skillsCli.updates)).toEqual(keysOf(zh.skillsCli.updates));
    expect(keysOf(en.backendErrors.skills_cli)).toEqual(
      keysOf(zh.backendErrors.skills_cli),
    );
  });
});

describe("batch-actions source contracts", () => {
  it("does not add window Escape listeners or sonner/invoke in the new surfaces", () => {
    const root = process.cwd();
    const files = [
      "src/pages/SkillsCliView.tsx",
      "src/components/skillsCli/SkillsCliBatchBar.tsx",
      "src/components/skillsCli/SkillsCliUninstallDialog.tsx",
      "src/components/skillsCli/SkillsCliCleanupDialog.tsx",
      "src/pages/skillsCliExport.ts",
      "src/components/skillsCli/SkillsCliUpdateDrawer.tsx",
    ];
    const joined = files.map((file) => readFileSync(resolve(root, file), "utf8")).join("\n");
    expect(joined).not.toMatch(/window\.addEventListener\(\s*['"]keydown['"]/);
    expect(joined).not.toMatch(/document\.addEventListener\(\s*['"]keydown['"]/);
    expect(joined).not.toMatch(/from ["']sonner["']/);
    expect(joined).not.toMatch(/from ["']@\/lib\/ipc["']/);
    expect(joined).not.toMatch(/#(?:[0-9a-fA-F]{3,8})\b/);
    const view = readFileSync(resolve(root, "src/pages/SkillsCliView.tsx"), "utf8");
    expect((view.match(/const \[selectedCardNames/g) ?? []).length).toBe(1);
    expect(joined).not.toMatch(/\binvoke\s*\(/);
    expect(view).not.toMatch(/\bruntimeBlocked\b/);
    const denseRow = readFileSync(
      resolve(root, "src/components/skill/SkillCardDenseRow.tsx"),
      "utf8",
    );
    expect(denseRow).toContain("allPlacementsUnavailable");
    expect(denseRow).not.toMatch(
      /placements\.every\(\s*\((?:placement|item)\)\s*=>\s*(?:placement|item)\.state\s*===\s*["']unavailable["']/,
    );
  });
});

describe("selection reconcile", () => {
  it("intersects selected names with the current inventory and keeps store order for export", () => {
    expect(
      [...reconcileSelectedNames(new Set(["gone", "copy-skill", "linked-only"]), mixedSkills.map((item) => item.name))],
    ).toEqual(["copy-skill", "linked-only"]);
    expect(
      selectedSkillsInStoreOrder(mixedSkills, new Set(["mixed-skill", "linked-only"])).map(
        (item) => item.name,
      ),
    ).toEqual(["linked-only", "mixed-skill"]);
  });
});

describe("placement mutation partition", () => {
  it("allows link only for missing and records localized skip reasons without inventing IPC targets", () => {
    const { allowed, skipped } = partitionLinkBatch(
      mixedSkills,
      ["unlinked-skill", "copy-skill", "conflict-skill", "linked-only", "mixed-skill"],
      "cursor",
    );
    expect(allowed.map((item) => item.skillName)).toEqual([]);
    expect(skipped.map((item) => ({ name: item.skillName, reason: item.reasonCode }))).toEqual([
      { name: "unlinked-skill", reason: "skills_cli.placement_unavailable" },
      { name: "copy-skill", reason: "skills_cli.direct_copy_not_toggleable" },
      { name: "conflict-skill", reason: "skills_cli.placement_unavailable" },
      { name: "linked-only", reason: "skills_cli.already_linked" },
      { name: "mixed-skill", reason: "skills_cli.already_linked" },
    ]);
    const amp = partitionLinkBatch(mixedSkills, ["unlinked-skill", "mixed-skill", "copy-skill"], "amp");
    expect(amp.allowed.map((item) => item.skillName)).toEqual([
      "unlinked-skill",
      "mixed-skill",
    ]);
    expect(amp.skipped).toEqual([
      {
        skillName: "copy-skill",
        agentId: "amp",
        reasonCode: "skills_cli.placement_unavailable",
      },
    ]);
  });

  it("allows unlink only for managed_link and keeps direct_copy/conflict/unavailable as skips", () => {
    const { allowed, skipped } = partitionUnlinkBatch(mixedSkills, [
      "linked-only",
      "copy-skill",
      "conflict-skill",
      "unlinked-skill",
      "mixed-skill",
    ]);
    expect(allowed.map((item) => ({ name: item.skillName, agentId: item.agentId }))).toEqual([
      { name: "linked-only", agentId: "cursor" },
      { name: "mixed-skill", agentId: "cursor" },
    ]);
    expect(
      skipped.some(
        (item) =>
          item.skillName === "copy-skill" &&
          item.reasonCode === "skills_cli.direct_copy_not_toggleable",
      ),
    ).toBe(true);
    expect(
      skipped.some(
        (item) =>
          item.skillName === "conflict-skill" &&
          item.reasonCode === "skills_cli.placement_conflict",
      ),
    ).toBe(true);
    expect(
      skipped.some(
        (item) =>
          item.skillName === "unlinked-skill" &&
          item.reasonCode === "skills_cli.not_linked",
      ),
    ).toBe(true);
  });

  it("unlinks one agent only and skips other states without creating IPC targets", () => {
    const cursor = partitionUnlinkBatchForAgent(
      mixedSkills,
      ["mixed-skill", "copy-skill", "unlinked-skill"],
      "cursor",
    );
    expect(cursor.allowed.map((item) => item.skillName)).toEqual(["mixed-skill"]);
    expect(
      cursor.skipped.map((item) => ({
        name: item.skillName,
        reason: item.reasonCode,
      })),
    ).toEqual([
      { name: "copy-skill", reason: "skills_cli.direct_copy_not_toggleable" },
      { name: "unlinked-skill", reason: "skills_cli.placement_unavailable" },
    ]);
    const amp = partitionUnlinkBatchForAgent(
      mixedSkills,
      ["mixed-skill", "copy-skill"],
      "amp",
    );
    expect(amp.allowed).toEqual([]);
    expect(
      amp.skipped.map((item) => ({ name: item.skillName, reason: item.reasonCode })),
    ).toEqual([
      { name: "mixed-skill", reason: "skills_cli.not_linked" },
      { name: "copy-skill", reason: "skills_cli.placement_unavailable" },
    ]);
  });
});

describe("removal impact", () => {
  it("counts owned canonicals and managed links, retains direct copies, and blocks on conflict", () => {
    const impact = aggregateRemovalImpact([
      {
        skillName: "owned-skill",
        ownedCanonical: true,
        managedPlacements: [
          { agentId: "cursor", displayName: "Cursor" },
          { agentId: "amp", displayName: "Amp" },
        ],
        retainedDirectCopies: [{ agentId: "codex", displayName: "Codex" }],
        conflicts: [],
        confirmable: true,
      },
      {
        skillName: "copy-only",
        ownedCanonical: false,
        managedPlacements: [],
        retainedDirectCopies: [{ agentId: "cursor", displayName: "Cursor" }],
        conflicts: [],
        confirmable: true,
      },
    ]);
    expect(impact.ownedContentCount).toBe(1);
    expect(impact.ownedContentCount).not.toBe(impact.skillNames.length);
    expect(impact.managedLinkCount).toBe(2);
    expect(impact.retainedDirectCopies).toEqual([
      { skillName: "owned-skill", agentId: "codex", displayName: "Codex" },
      { skillName: "copy-only", agentId: "cursor", displayName: "Cursor" },
    ]);
    expect(impact.confirmable).toBe(true);

    const blocked = aggregateRemovalImpact([
      {
        skillName: "blocked",
        ownedCanonical: true,
        managedPlacements: [],
        retainedDirectCopies: [],
        conflicts: [
          {
            agentId: "amp",
            displayName: "Amp",
            reasonCode: "skills_cli.placement_conflict",
          },
        ],
        confirmable: false,
      },
    ]);
    expect(blocked.confirmable).toBe(false);
    expect(blocked.conflicts).toHaveLength(1);
  });
});

describe("export v1 envelope", () => {
  const now = new Date("2026-08-26T15:04:05.000Z");

  it("emits the exact v1 whitelist, target order, trailing newline, and scoped filenames", () => {
    const envelope = buildSkillsCliExportV1(
      [mixedSkills[4]!, mixedSkills[0]!],
      "selected",
      now,
      ["cursor", "amp", "codex"],
    );
    expect(envelope).toEqual({
      schemaVersion: 1,
      exportedAt: "2026-08-26T15:04:05.000Z",
      scope: "selected",
      skillCount: 2,
      skills: [
        {
          name: "mixed-skill",
          source: "Gamma/Three",
          sourceType: "github",
          sourceUrl: "https://github.com/owner/repo",
          installKind: "canonical",
          canonicalPath: "/canonical/mixed-skill",
          folderHash: null,
          installedAt: null,
          updatedAt: null,
          placements: [
            {
              agentId: "cursor",
              displayName: "Cursor",
              state: "managed_link",
            },
            { agentId: "amp", displayName: "Amp", state: "missing" },
            { agentId: "codex", displayName: "Codex", state: "direct_copy" },
          ],
        },
        {
          name: "linked-only",
          source: "alpha/one",
          sourceType: "github",
          sourceUrl: "https://github.com/owner/repo",
          installKind: "canonical",
          canonicalPath: "/canonical/linked-only",
          folderHash: null,
          installedAt: null,
          updatedAt: null,
          placements: [
            {
              agentId: "cursor",
              displayName: "Cursor",
              state: "managed_link",
            },
          ],
        },
      ],
    });
    const json = stringifySkillsCliExportV1(envelope);
    expect(json.endsWith("\n")).toBe(true);
    expect(json).not.toContain("targetPath");
    expect(json).not.toContain("managedLinkKind");
    expect(json).not.toContain("reasonCode");
    const parsed = JSON.parse(json) as { skills: Array<Record<string, unknown>> };
    expect(Object.keys(parsed)).toEqual([
      "schemaVersion",
      "exportedAt",
      "scope",
      "skillCount",
      "skills",
    ]);
    expect(Object.keys(parsed.skills[0] ?? {})).toEqual([
      "name",
      "source",
      "sourceType",
      "sourceUrl",
      "installKind",
      "canonicalPath",
      "folderHash",
      "installedAt",
      "updatedAt",
      "placements",
    ]);
    expect(Object.keys((parsed.skills[0]?.placements as object[])[0] ?? {})).toEqual([
      "agentId",
      "displayName",
      "state",
    ]);
    expect(skillsCliExportFileName("all", now)).toBe(
      "skillport-skills-cli-all-2026-08-26.json",
    );
    expect(skillsCliExportFileName("selected", now)).toBe(
      "skillport-skills-cli-selected-2026-08-26.json",
    );
  });

  it("cancels silently when the save dialog returns null and otherwise writes the v1 JSON", async () => {
    const exportInventory = vi.fn();
    const cancelled = await exportSkillsCliInventory({
      scope: "all",
      skills: mixedSkills,
      targets,
      now,
      save: async () => null,
      exportInventory,
    });
    expect(cancelled).toBe("cancelled");
    expect(exportInventory).not.toHaveBeenCalled();

    const saved = await exportSkillsCliInventory({
      scope: "selected",
      skills: mixedSkills,
      selectedNames: new Set(["mixed-skill"]),
      targets,
      now,
      save: async ({ defaultPath }) => {
        expect(defaultPath).toBe("skillport-skills-cli-selected-2026-08-26.json");
        return "D:/tmp/export.json";
      },
      exportInventory,
    });
    expect(saved).toBe("saved");
    expect(exportInventory).toHaveBeenCalledTimes(1);
    const payload = exportInventory.mock.calls[0]?.[0] as {
      path: string;
      json: string;
    };
    expect(payload.path).toBe("D:/tmp/export.json");
    expect(payload.json.endsWith("\n")).toBe(true);
    expect(JSON.parse(payload.json).skillCount).toBe(1);
    expect(JSON.parse(payload.json).scope).toBe("selected");
  });
});

describe("link target summaries", () => {
  it("counts missing/managed/copy/blocked buckets per target", () => {
    const summaries = summarizeLinkTargets(
      mixedSkills,
      new Set(["linked-only", "copy-skill", "unlinked-skill", "conflict-skill", "mixed-skill"]),
      targets,
    );
    expect(summaries.find((item) => item.agentId === "cursor")).toEqual({
      agentId: "cursor",
      displayName: "Cursor",
      linkableCount: 0,
      managedCount: 2,
      directCopyCount: 1,
      blockedCount: 2,
    });
    expect(summaries.find((item) => item.agentId === "amp")?.linkableCount).toBe(2);
  });
});

function updateRow(
  overrides: Partial<SkillsCliUpdateSkillRow> = {},
): SkillsCliUpdateSkillRow {
  return {
    skillName: "demo",
    repositoryKey: "owner/repo@main",
    normalizedSource: "https://github.com/owner/repo",
    skillPath: "demo",
    status: "update_available",
    installedRevisionSha: "aaa",
    observedRevisionSha: "bbb",
    pendingRevisionSha: "bbb",
    installedLocalDigest: "sha256-v1:a",
    observedUpstreamDigest: "sha256-v1:b",
    pendingUpstreamDigest: "sha256-v1:b",
    isStale: false,
    lastErrorCode: null,
    changeSummary: ["SKILL.md"],
    blockers: [],
    argvPreview: [
      "refresh",
      "owned-canonical",
      "from-pinned-github-snapshot",
      "demo",
    ],
    ...overrides,
  };
}

describe("Skills CLI update view-model", () => {
  it("groups selected names by repositoryKey in first-seen order", () => {
    const inventory = {
      ...EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
      skills: [
        updateRow({ skillName: "alpha", repositoryKey: "owner/alpha@main" }),
        updateRow({ skillName: "beta", repositoryKey: "owner/beta@main" }),
        updateRow({ skillName: "gamma", repositoryKey: "owner/alpha@main" }),
        updateRow({ skillName: "plain", repositoryKey: null }),
      ],
    };
    expect(
      groupSkillNamesByRepositoryKey(
        ["alpha", "beta", "gamma", "plain", "missing"],
        inventory,
      ),
    ).toEqual([
      { repositoryKey: "owner/alpha@main", skillNames: ["alpha", "gamma"] },
      { repositoryKey: "owner/beta@main", skillNames: ["beta"] },
    ]);
  });

  it("exposes the nine update statuses", () => {
    expect(skillsCliUpdateStatuses()).toEqual([
      "not_checked",
      "checking",
      "current",
      "update_available",
      "local_modified",
      "baseline_required",
      "unsupported",
      "rate_limited",
      "failed",
    ]);
  });

  it("keeps pending counts after failed or rate-limited repository rows", () => {
    const inventory: SkillsCliUpdateInventory = {
      ...EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
      skills: [
        updateRow({ status: "failed", pendingRevisionSha: "bbb" }),
        updateRow({
          skillName: "other",
          status: "rate_limited",
          pendingRevisionSha: "ccc",
        }),
      ],
    };
    expect(
      pendingUpdateCountForSkills(
        [skill({ name: "demo" }), skill({ name: "other" })],
        inventory,
      ),
    ).toBe(2);
    expect(visibleUpdateStatus(updateRow({ status: "failed" }), false, null)).toBe(
      "failed",
    );
    expect(
      visibleUpdateStatus(updateRow({ status: "current" }), true, "owner/repo@main"),
    ).toBe("checking");
  });

  it("disables apply for stale, recovery, and topology blockers", () => {
    const demo = skill({ name: "demo" });
    expect(isUpdateApplyEnabled(updateRow({ isStale: true }), demo, false)).toBe(
      false,
    );
    expect(isUpdateApplyEnabled(updateRow(), demo, true)).toBe(false);
    expect(
      isUpdateApplyEnabled(
        updateRow(),
        skill({
          name: "demo",
          placements: [placement("cursor", "direct_copy")],
        }),
        false,
      ),
    ).toBe(false);
    expect(isUpdateApplyEnabled(updateRow(), demo, false)).toBe(true);
    expect(
      isUpdateApplyEnabled(
        updateRow({
          status: "baseline_required",
          pendingRevisionSha: "bbb",
        }),
        demo,
        false,
      ),
    ).toBe(false);
    expect(
      isUpdateReinstallEnabled(
        updateRow({
          status: "baseline_required",
          pendingRevisionSha: "bbb",
        }),
        demo,
        false,
      ),
    ).toBe(true);
    expect(
      actionableUpdateSkillNames(
        [demo, skill({ name: "other" })],
        {
          ...EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
          skills: [
            updateRow(),
            updateRow({
              skillName: "other",
              status: "baseline_required",
              pendingRevisionSha: "ccc",
            }),
            updateRow({
              skillName: "failed-skill",
              status: "failed",
              pendingRevisionSha: "ddd",
              isStale: true,
            }),
          ],
        },
        false,
      ),
    ).toEqual(["demo"]);
  });

  it("uses backend argv preview and never adds force or keep-links", () => {
    const preview = argvPreviewForSelection(
      {
        ...EMPTY_SKILLS_CLI_UPDATE_INVENTORY,
        skills: [
          updateRow({
            argvPreview: [
              "refresh",
              "owned-canonical",
              "from-pinned-github-snapshot",
              "--force",
              "demo",
            ],
          }),
        ],
      },
      ["demo"],
    );
    expect(preview).toEqual([
      "refresh",
      "owned-canonical",
      "from-pinned-github-snapshot",
      "demo",
    ]);
    expect(preview).not.toContain("--force");
    expect(preview).not.toContain("--keep-links");
  });
});

