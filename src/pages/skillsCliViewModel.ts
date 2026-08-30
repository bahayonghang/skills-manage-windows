import type {
  SkillsCliGlobalSkill,
  SkillsCliInstallTarget,
  SkillsCliPlacement,
  SkillsCliUpdateInventory,
  SkillsCliUpdateSkillRow,
  SkillsCliUpdateStatus,
} from "@/types";

export type SkillsCliGroupBy = "repo" | "platform" | "status" | "none";
export type SkillsCliLayoutBand = "twoColumns" | "threeColumns" | "fourColumns";
export type SkillsCliDrawerBand = "fullWidth" | "fixed460";

export function skillsCliRemoteMutationLockReason(
  isLocal: boolean,
  translate: (key: string) => string,
): string | undefined {
  return isLocal ? undefined : translate("backendErrors.skills_cli.local_target_only");
}

export type SkillsCliActiveSurface =
  | null
  | { kind: "install" }
  | { kind: "detail"; skillName: string; focus: null | "links" }
  | { kind: "update"; repositoryKey: string; skillNames: readonly string[] }
  | { kind: "uninstall"; skillNames: readonly string[] }
  | { kind: "cleanup" };

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

export function openSkillsCliUpdate(input: {
  repositoryKey: string;
  skillNames: readonly string[];
}): SkillsCliActiveSurface {
  return {
    kind: "update",
    repositoryKey: input.repositoryKey,
    skillNames: input.skillNames,
  };
}

export function openSkillsCliUninstall(
  skillNames: readonly string[],
): SkillsCliActiveSurface {
  return { kind: "uninstall", skillNames };
}

export function openSkillsCliCleanup(): SkillsCliActiveSurface {
  return { kind: "cleanup" };
}

export function closeSkillsCliSurface(): SkillsCliActiveSurface {
  return null;
}

const NINE_STATES: readonly SkillsCliUpdateStatus[] = [
  "not_checked",
  "checking",
  "current",
  "update_available",
  "local_modified",
  "baseline_required",
  "unsupported",
  "rate_limited",
  "failed",
];

export function skillsCliUpdateStatuses(): readonly SkillsCliUpdateStatus[] {
  return NINE_STATES;
}

export function updateRowForSkill(
  inventory: SkillsCliUpdateInventory | null,
  skillName: string,
): SkillsCliUpdateSkillRow | null {
  return inventory?.skills.find((row) => row.skillName === skillName) ?? null;
}

export function visibleUpdateStatus(
  row: SkillsCliUpdateSkillRow | null,
  checking: boolean,
  currentRepositoryKey: string | null,
): SkillsCliUpdateStatus {
  if (
    checking &&
    row?.repositoryKey &&
    (currentRepositoryKey == null || row.repositoryKey === currentRepositoryKey)
  ) {
    return "checking";
  }
  return row?.status ?? "not_checked";
}

export function skillHasPendingUpdate(row: SkillsCliUpdateSkillRow | null): boolean {
  return Boolean(row?.pendingRevisionSha);
}

export function pendingUpdateCountForSkills(
  skills: readonly SkillsCliGlobalSkill[],
  inventory: SkillsCliUpdateInventory | null,
): number {
  return skills.filter((skill) =>
    skillHasPendingUpdate(updateRowForSkill(inventory, skill.name)),
  ).length;
}

export function skillHasTopologyBlocker(skill: SkillsCliGlobalSkill): boolean {
  return skill.placements.some(
    (placement) =>
      placement.state === "direct_copy" || placement.state === "conflict",
  );
}

function updateRowIsSelectable(
  row: SkillsCliUpdateSkillRow | null,
  skill: SkillsCliGlobalSkill | undefined,
  hasRecovery: boolean,
): boolean {
  if (!row || !skill || hasRecovery || row.isStale) {
    return false;
  }
  if (skillHasTopologyBlocker(skill)) {
    return false;
  }
  return Boolean(row.pendingRevisionSha);
}

export function isUpdateApplyEnabled(
  row: SkillsCliUpdateSkillRow | null,
  skill: SkillsCliGlobalSkill | undefined,
  hasRecovery: boolean,
): boolean {
  return (
    updateRowIsSelectable(row, skill, hasRecovery) &&
    row?.status === "update_available"
  );
}

export function isUpdateReinstallEnabled(
  row: SkillsCliUpdateSkillRow | null,
  skill: SkillsCliGlobalSkill | undefined,
  hasRecovery: boolean,
): boolean {
  return (
    updateRowIsSelectable(row, skill, hasRecovery) &&
    row?.status === "baseline_required"
  );
}

export function actionableUpdateSkillNames(
  skills: readonly SkillsCliGlobalSkill[],
  inventory: SkillsCliUpdateInventory | null,
  hasRecovery: boolean,
): string[] {
  return skills
    .filter((skill) =>
      isUpdateApplyEnabled(
        updateRowForSkill(inventory, skill.name),
        skill,
        hasRecovery,
      ),
    )
    .map((skill) => skill.name);
}

export function argvPreviewForSelection(
  inventory: SkillsCliUpdateInventory | null,
  skillNames: readonly string[],
): string[] {
  const forbidden = new Set(["--force", "--keep-links"]);
  const tokens: string[] = [];
  for (const name of skillNames) {
    const row = updateRowForSkill(inventory, name);
    for (const token of row?.argvPreview ?? []) {
      if (forbidden.has(token) || tokens.includes(token)) {
        continue;
      }
      tokens.push(token);
    }
  }
  return tokens;
}

export function repositoryKeyForSkills(
  skills: readonly SkillsCliGlobalSkill[],
  inventory: SkillsCliUpdateInventory | null,
): string | null {
  const keys = new Set<string>();
  for (const skill of skills) {
    const key = updateRowForSkill(inventory, skill.name)?.repositoryKey;
    if (key) {
      keys.add(key);
    }
  }
  if (keys.size !== 1) {
    return null;
  }
  const [key] = keys;
  return key ?? null;
}

export function groupSkillNamesByRepositoryKey(
  skillNames: readonly string[],
  inventory: SkillsCliUpdateInventory | null,
): Array<{ repositoryKey: string; skillNames: string[] }> {
  const groups = new Map<string, string[]>();
  const order: string[] = [];
  for (const name of skillNames) {
    const key = updateRowForSkill(inventory, name)?.repositoryKey;
    if (!key) {
      continue;
    }
    const existing = groups.get(key);
    if (!existing) {
      order.push(key);
      groups.set(key, [name]);
      continue;
    }
    existing.push(name);
  }
  return order.map((repositoryKey) => ({
    repositoryKey,
    skillNames: groups.get(repositoryKey) ?? [],
  }));
}

export function applySelectionsForNames(
  inventory: SkillsCliUpdateInventory | null,
  skillNames: readonly string[],
): Array<{
  skillName: string;
  skillPath: string;
  expectedInstalledRevision: string | null;
  expectedInstalledLocalDigest: string | null;
  expectedPendingRevision: string;
  expectedPendingDigest: string;
}> {
  const selections = [];
  for (const name of skillNames) {
    const row = updateRowForSkill(inventory, name);
    if (
      !row?.skillPath ||
      !row.pendingRevisionSha ||
      !row.pendingUpstreamDigest
    ) {
      continue;
    }
    selections.push({
      skillName: name,
      skillPath: row.skillPath,
      expectedInstalledRevision: row.installedRevisionSha,
      expectedInstalledLocalDigest: row.installedLocalDigest,
      expectedPendingRevision: row.pendingRevisionSha,
      expectedPendingDigest: row.pendingUpstreamDigest,
    });
  }
  return selections;
}

export function shortRevisionIdentity(sha: string | null | undefined): string {
  const trimmed = sha?.trim() ?? "";
  if (!trimmed) {
    return "";
  }
  return trimmed.length > 7 ? trimmed.slice(0, 7) : trimmed;
}

export interface SkillsCliUpdateDrawerRow {
  skillName: string;
  selected: boolean;
  status: SkillsCliUpdateStatus;
  installedRevision: string | null;
  observedRevision: string | null;
  changeSummary: string[];
  applyEnabled: boolean;
  reinstallEnabled: boolean;
  blockerCodes: string[];
}

export function buildUpdateDrawerRows(
  skills: readonly SkillsCliGlobalSkill[],
  inventory: SkillsCliUpdateInventory | null,
  repositoryKey: string,
  selectedNames: ReadonlySet<string>,
  hasRecovery: boolean,
): SkillsCliUpdateDrawerRow[] {
  const byName = new Map(skills.map((skill) => [skill.name, skill]));
  return (inventory?.skills ?? [])
    .filter(
      (row) => row.repositoryKey === repositoryKey && byName.has(row.skillName),
    )
    .map((row) => {
      const skill = byName.get(row.skillName);
      return {
        skillName: row.skillName,
        selected: selectedNames.has(row.skillName),
        status: row.status,
        installedRevision: row.installedRevisionSha,
        observedRevision: row.observedRevisionSha,
        changeSummary: row.changeSummary,
        applyEnabled: isUpdateApplyEnabled(row, skill, hasRecovery),
        reinstallEnabled: isUpdateReinstallEnabled(row, skill, hasRecovery),
        blockerCodes: [
          ...row.blockers.map((item) => item.code),
          ...(skill && skillHasTopologyBlocker(skill)
            ? ["skills_cli.update_topology_conflict"]
            : []),
        ],
      };
    });
}
