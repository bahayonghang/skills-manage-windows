import type {
  SkillsCliGlobalSkill,
  SkillsCliInstallTarget,
  SkillsCliPlacement,
} from "@/types";

export type SkillsCliGroupBy = "repo" | "platform" | "status" | "none";
export type SkillsCliLayoutBand = "twoColumns" | "threeColumns" | "fourColumns";
export type SkillsCliDrawerBand = "fullWidth" | "fixed460";

export type SkillsCliActiveSurface =
  | null
  | { kind: "install" }
  | { kind: "detail"; skillName: string; focus: null | "links" }
  | { kind: "update" }
  | { kind: "uninstall"; skillNames: readonly string[] };

export interface SkillsCliCounts {
  installed: number;
  linked: number;
  unlinked: number;
  repositories: number;
}

export interface SkillsCliBucket {
  id: string;
  labelKey: string;
  labelValue?: string;
  skillCount: number;
  managedLinkCount: number;
  skills: SkillsCliGlobalSkill[];
}

export interface SkillsCliFilterState {
  query: string;
  platformFilter: string | null;
  unlinkedOnly: boolean;
}

export const SKILLS_CLI_DRAWER_BAND_MIN_PX = 720;
export const SKILLS_CLI_THREE_COLUMN_MIN_PX = 900;
export const SKILLS_CLI_FOUR_COLUMN_MIN_PX = 1180;
export const SKILLS_CLI_SKELETON_COUNT = 12;

export const SKILLS_CLI_CONTENT_CONTAINER_CLASS = "@container/skills-cli min-w-0";
export const SKILLS_CLI_GRID_CLASS =
  "grid grid-cols-2 gap-3 @min-[900px]/skills-cli:grid-cols-3 @min-[1180px]/skills-cli:grid-cols-4";

const SELECTION_EMPTY_ENVELOPE =
  "skills_cli.selection_empty:Select at least one skill and one platform.";

export const SKILLS_CLI_SELECTION_EMPTY_ENVELOPE = SELECTION_EMPTY_ENVELOPE;

function caseFold(value: string): string {
  return value.trim().toLocaleLowerCase("en-US");
}

function sourceIdentity(skill: SkillsCliGlobalSkill): string {
  return (skill.source ?? "").trim();
}

function canonicalPathOf(skill: SkillsCliGlobalSkill): string {
  return skill.canonicalPath ?? skill.path ?? "";
}

export function enabledTargetIdSet(
  targets: readonly SkillsCliInstallTarget[],
): ReadonlySet<string> {
  return new Set(
    targets.filter((target) => target.isEnabled).map((target) => target.id),
  );
}

export function skillHasManagedLink(skill: SkillsCliGlobalSkill): boolean {
  return skill.placements.some((placement) => placement.state === "managed_link");
}

export function skillHasEnabledMissing(
  skill: SkillsCliGlobalSkill,
  enabledTargetIds: ReadonlySet<string>,
): boolean {
  return skill.placements.some(
    (placement) =>
      placement.state === "missing" && enabledTargetIds.has(placement.agentId),
  );
}

export function skillHasCopyOrConflict(skill: SkillsCliGlobalSkill): boolean {
  return skill.placements.some(
    (placement) =>
      placement.state === "direct_copy" || placement.state === "conflict",
  );
}

function placementOnPlatform(
  placements: readonly SkillsCliPlacement[],
  agentId: string,
): boolean {
  return placements.some(
    (placement) =>
      placement.agentId === agentId &&
      (placement.state === "managed_link" || placement.state === "direct_copy"),
  );
}

function managedLinkCountOf(skills: readonly SkillsCliGlobalSkill[]): number {
  return skills.reduce(
    (total, skill) =>
      total +
      skill.placements.filter((placement) => placement.state === "managed_link")
        .length,
    0,
  );
}

function toBucket(
  id: string,
  labelKey: string,
  skills: SkillsCliGlobalSkill[],
  labelValue?: string,
): SkillsCliBucket {
  return {
    id,
    labelKey,
    labelValue,
    skillCount: skills.length,
    managedLinkCount: managedLinkCountOf(skills),
    skills,
  };
}

export function deriveSkillsCliCounts(
  skills: readonly SkillsCliGlobalSkill[],
  enabledTargetIds: ReadonlySet<string>,
): SkillsCliCounts {
  const repositories = new Set<string>();
  let linked = 0;
  let unlinked = 0;
  for (const skill of skills) {
    const identity = sourceIdentity(skill);
    if (identity) {
      repositories.add(identity);
    }
    if (skillHasManagedLink(skill)) {
      linked += 1;
    }
    if (skillHasEnabledMissing(skill, enabledTargetIds)) {
      unlinked += 1;
    }
  }
  return {
    installed: skills.length,
    linked,
    unlinked,
    repositories: repositories.size,
  };
}

export function filterSkillsCli(
  skills: readonly SkillsCliGlobalSkill[],
  filters: SkillsCliFilterState,
  enabledTargetIds: ReadonlySet<string>,
): SkillsCliGlobalSkill[] {
  const query = caseFold(filters.query);
  return skills.filter((skill) => {
    if (query) {
      const haystack = [
        skill.name,
        skill.source ?? "",
        canonicalPathOf(skill),
      ];
      if (!haystack.some((field) => caseFold(field).includes(query))) {
        return false;
      }
    }
    if (filters.platformFilter) {
      if (!placementOnPlatform(skill.placements, filters.platformFilter)) {
        return false;
      }
    }
    if (filters.unlinkedOnly) {
      if (!skillHasEnabledMissing(skill, enabledTargetIds)) {
        return false;
      }
    }
    return true;
  });
}

export function bucketSkillsCli(
  skills: readonly SkillsCliGlobalSkill[],
  groupBy: SkillsCliGroupBy,
  targets: readonly SkillsCliInstallTarget[],
  enabledTargetIds: ReadonlySet<string>,
): SkillsCliBucket[] {
  switch (groupBy) {
    case "none":
      return skills.length === 0
        ? []
        : [toBucket("none:all", "skillsCli.buckets.all", [...skills])];
    case "repo":
      return bucketByRepo(skills);
    case "platform":
      return bucketByPlatform(skills, targets, enabledTargetIds);
    case "status":
      return bucketByStatus(skills, enabledTargetIds);
    default: {
      const _exhaustive: never = groupBy;
      return _exhaustive;
    }
  }
}

function bucketByRepo(skills: readonly SkillsCliGlobalSkill[]): SkillsCliBucket[] {
  const known = new Map<string, SkillsCliGlobalSkill[]>();
  const knownOrder: string[] = [];
  const unknown: SkillsCliGlobalSkill[] = [];
  for (const skill of skills) {
    const identity = sourceIdentity(skill);
    if (!identity) {
      unknown.push(skill);
      continue;
    }
    const existing = known.get(identity);
    if (existing) {
      existing.push(skill);
    } else {
      known.set(identity, [skill]);
      knownOrder.push(identity);
    }
  }
  const buckets = knownOrder.map((identity) =>
    toBucket(`repo:${identity}`, "skillsCli.buckets.named", known.get(identity) ?? [], identity),
  );
  if (unknown.length > 0) {
    buckets.push(toBucket("repo:unknown", "skillsCli.buckets.unknown", unknown));
  }
  return buckets;
}

function bucketByPlatform(
  skills: readonly SkillsCliGlobalSkill[],
  targets: readonly SkillsCliInstallTarget[],
  enabledTargetIds: ReadonlySet<string>,
): SkillsCliBucket[] {
  const buckets: SkillsCliBucket[] = [];
  for (const target of targets) {
    const members = skills.filter((skill) =>
      placementOnPlatform(skill.placements, target.id),
    );
    if (members.length === 0) {
      continue;
    }
    buckets.push(
      toBucket(
        `platform:${target.id}`,
        "skillsCli.buckets.named",
        members,
        target.displayName,
      ),
    );
  }
  const unlinked = skills.filter((skill) =>
    skillHasEnabledMissing(skill, enabledTargetIds),
  );
  if (unlinked.length > 0) {
    buckets.push(
      toBucket("platform:unlinked", "skillsCli.buckets.unlinked", unlinked),
    );
  }
  return buckets;
}

function bucketByStatus(
  skills: readonly SkillsCliGlobalSkill[],
  enabledTargetIds: ReadonlySet<string>,
): SkillsCliBucket[] {
  const buckets: SkillsCliBucket[] = [];
  const linked = skills.filter(skillHasManagedLink);
  if (linked.length > 0) {
    buckets.push(toBucket("status:linked", "skillsCli.buckets.linked", linked));
  }
  const unlinked = skills.filter((skill) =>
    skillHasEnabledMissing(skill, enabledTargetIds),
  );
  if (unlinked.length > 0) {
    buckets.push(
      toBucket("status:unlinked", "skillsCli.buckets.unlinked", unlinked),
    );
  }
  const copyOrConflict = skills.filter(skillHasCopyOrConflict);
  if (copyOrConflict.length > 0) {
    buckets.push(
      toBucket(
        "status:copy-or-conflict",
        "skillsCli.buckets.copyOrConflict",
        copyOrConflict,
      ),
    );
  }
  return buckets;
}

export function deriveSkillsCliLayoutBands(contentWidthPx: number | null): {
  grid: SkillsCliLayoutBand;
  drawer: SkillsCliDrawerBand;
} {
  if (contentWidthPx == null) {
    return { grid: "twoColumns", drawer: "fullWidth" };
  }
  const grid: SkillsCliLayoutBand =
    contentWidthPx >= SKILLS_CLI_FOUR_COLUMN_MIN_PX
      ? "fourColumns"
      : contentWidthPx >= SKILLS_CLI_THREE_COLUMN_MIN_PX
        ? "threeColumns"
        : "twoColumns";
  const drawer: SkillsCliDrawerBand =
    contentWidthPx >= SKILLS_CLI_DRAWER_BAND_MIN_PX ? "fixed460" : "fullWidth";
  return { grid, drawer };
}

export function openSkillsCliInstall(): SkillsCliActiveSurface {
  return { kind: "install" };
}

export function openSkillsCliDetail(
  skillName: string,
  focus: null | "links",
): SkillsCliActiveSurface {
  return { kind: "detail", skillName, focus };
}

export function openSkillsCliUpdate(): SkillsCliActiveSurface {
  return { kind: "update" };
}

export function openSkillsCliUninstall(
  skillNames: readonly string[],
): SkillsCliActiveSurface {
  return { kind: "uninstall", skillNames };
}

export function closeSkillsCliSurface(): SkillsCliActiveSurface {
  return null;
}
