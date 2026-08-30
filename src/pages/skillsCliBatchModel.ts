import type {
  SkillsCliGlobalSkill,
  SkillsCliInstallKind,
  SkillsCliInstallTarget,
  SkillsCliPlacement,
  SkillsCliPlacementState,
  SkillsCliRemovePlan,
} from "@/types";

export type SkillsCliExportScope = "all" | "selected";

export interface PlacementMutationOutcome {
  succeeded: Array<{ skillName: string; agentId?: string }>;
  failed: Array<{ skillName: string; agentId?: string; errorCode: string }>;
  skipped: Array<{ skillName: string; agentId: string; reasonCode: string }>;
}

export interface PlacementPartitionItem {
  skillName: string;
  agentId: string;
  placement: SkillsCliPlacement;
}

export interface PlacementPartition {
  allowed: PlacementPartitionItem[];
  skipped: Array<{ skillName: string; agentId: string; reasonCode: string }>;
}

export interface RemovalImpact {
  skillNames: string[];
  ownedContentCount: number;
  managedLinkCount: number;
  retainedDirectCopies: Array<{
    skillName: string;
    agentId: string;
    displayName: string;
  }>;
  conflicts: Array<{
    skillName: string;
    agentId: string;
    displayName: string;
    reasonCode: string;
  }>;
  confirmable: boolean;
}

export interface SkillsCliLinkTargetSummary {
  agentId: string;
  displayName: string;
  linkableCount: number;
  managedCount: number;
  directCopyCount: number;
  blockedCount: number;
}

export interface SkillsCliExportPlacement {
  agentId: string;
  displayName: string;
  state: SkillsCliPlacementState;
}

export interface SkillsCliExportSkillV1 {
  name: string;
  source: string | null;
  sourceType: string | null;
  sourceUrl: string | null;
  installKind: SkillsCliInstallKind;
  canonicalPath: string | null;
  folderHash: string | null;
  installedAt: string | null;
  updatedAt: string | null;
  placements: SkillsCliExportPlacement[];
}

export interface SkillsCliExportV1 {
  schemaVersion: 1;
  exportedAt: string;
  scope: SkillsCliExportScope;
  skillCount: number;
  skills: SkillsCliExportSkillV1[];
}

export const SKILLS_CLI_SKIP_ALREADY_LINKED = "skills_cli.already_linked";
export const SKILLS_CLI_SKIP_NOT_LINKED = "skills_cli.not_linked";
export const SKILLS_CLI_SKIP_NO_PLACEMENT = "skills_cli.placement_unavailable";
export const SKILLS_CLI_SKIP_NOT_OWNED = "skills_cli.skill_not_owned";

export type CleanupGroup = "stale" | "platformUnavailable";

export interface CleanupReason {
  platform: string;
  reasonCode: string;
}

export interface CleanupCandidate {
  name: string;
  group: CleanupGroup;
  reasons: readonly CleanupReason[];
}

export type CleanupReasonCode =
  | "canonical_missing"
  | "platform_unsupported"
  | "platform_not_detected"
  | "platform_disabled";

export function allPlacementsUnavailable(
  placements: readonly SkillsCliPlacement[],
): boolean {
  if (placements.length === 0) {
    return false;
  }
  return placements.every((placement) => placement.state === "unavailable");
}

function isCleanupReasonCode(value: string): value is CleanupReasonCode {
  return (
    value === "canonical_missing" ||
    value === "platform_unsupported" ||
    value === "platform_not_detected" ||
    value === "platform_disabled"
  );
}

export function cleanupReasonI18nKey(reasonCode: string): string {
  if (!isCleanupReasonCode(reasonCode)) {
    return "skillsCli.cleanup.reason.unknown";
  }
  switch (reasonCode) {
    case "canonical_missing":
      return "skillsCli.cleanup.reason.canonical_missing";
    case "platform_unsupported":
      return "skillsCli.cleanup.reason.platform_unsupported";
    case "platform_not_detected":
      return "skillsCli.cleanup.reason.platform_not_detected";
    case "platform_disabled":
      return "skillsCli.cleanup.reason.platform_disabled";
    default: {
      const _exhaustive: never = reasonCode;
      return _exhaustive;
    }
  }
}

export function deriveCleanupCandidates(
  skills: readonly SkillsCliGlobalSkill[],
): readonly CleanupCandidate[] {
  const candidates: CleanupCandidate[] = [];
  for (const skill of skills) {
    if (!allPlacementsUnavailable(skill.placements)) {
      continue;
    }
    const reasons = skill.placements.map((placement) => ({
      platform: placement.displayName,
      reasonCode: placement.reasonCode ?? "",
    }));
    const stale = skill.placements.some(
      (placement) => placement.reasonCode === "canonical_missing",
    );
    candidates.push({
      name: skill.name,
      group: stale ? "stale" : "platformUnavailable",
      reasons,
    });
  }
  return candidates;
}

export function defaultCleanupSelectedNames(
  candidates: readonly CleanupCandidate[],
): string[] {
  return candidates
    .filter((candidate) => candidate.group === "stale")
    .map((candidate) => candidate.name);
}

export function emptyPlacementOutcome(): PlacementMutationOutcome {
  return { succeeded: [], failed: [], skipped: [] };
}

export function reconcileSelectedNames(
  selected: ReadonlySet<string>,
  existingNames: Iterable<string>,
): Set<string> {
  const existing = new Set(existingNames);
  const next = new Set<string>();
  for (const name of selected) {
    if (existing.has(name)) {
      next.add(name);
    }
  }
  return next;
}

export function isGroupFullySelected(
  selected: ReadonlySet<string>,
  skillNames: readonly string[],
): boolean {
  return (
    skillNames.length > 0 && skillNames.every((name) => selected.has(name))
  );
}

export function toggleGroupSkillSelection(
  selected: ReadonlySet<string>,
  skillNames: readonly string[],
): Set<string> {
  const next = new Set(selected);
  const deselect = isGroupFullySelected(next, skillNames);
  for (const name of skillNames) {
    if (deselect) {
      next.delete(name);
    } else {
      next.add(name);
    }
  }
  return next;
}

export function selectedSkillsInStoreOrder(
  skills: readonly SkillsCliGlobalSkill[],
  selectedNames: ReadonlySet<string>,
): SkillsCliGlobalSkill[] {
  return skills.filter((skill) => selectedNames.has(skill.name));
}

function skipReasonForLink(state: SkillsCliPlacementState): string | null {
  switch (state) {
    case "missing":
      return null;
    case "managed_link":
      return SKILLS_CLI_SKIP_ALREADY_LINKED;
    case "direct_copy":
      return "skills_cli.direct_copy_not_toggleable";
    case "conflict":
      return "skills_cli.placement_conflict";
    case "unavailable":
      return "skills_cli.placement_unavailable";
    default: {
      const _exhaustive: never = state;
      return _exhaustive;
    }
  }
}

function skipReasonForUnlink(state: SkillsCliPlacementState): string | null {
  switch (state) {
    case "managed_link":
      return null;
    case "missing":
      return SKILLS_CLI_SKIP_NOT_LINKED;
    case "direct_copy":
      return "skills_cli.direct_copy_not_toggleable";
    case "conflict":
      return "skills_cli.placement_conflict";
    case "unavailable":
      return "skills_cli.placement_unavailable";
    default: {
      const _exhaustive: never = state;
      return _exhaustive;
    }
  }
}

function skillByName(
  skills: readonly SkillsCliGlobalSkill[],
  skillName: string,
): SkillsCliGlobalSkill | undefined {
  return skills.find((skill) => skill.name === skillName);
}

export function partitionLinkBatch(
  skills: readonly SkillsCliGlobalSkill[],
  skillNames: readonly string[],
  agentId: string,
): PlacementPartition {
  const allowed: PlacementPartitionItem[] = [];
  const skipped: PlacementPartition["skipped"] = [];
  for (const skillName of skillNames) {
    const skill = skillByName(skills, skillName);
    if (!skill) {
      skipped.push({
        skillName,
        agentId,
        reasonCode: SKILLS_CLI_SKIP_NOT_OWNED,
      });
      continue;
    }
    const placement = skill.placements.find((item) => item.agentId === agentId);
    if (!placement) {
      skipped.push({
        skillName,
        agentId,
        reasonCode: SKILLS_CLI_SKIP_NO_PLACEMENT,
      });
      continue;
    }
    const reasonCode = skipReasonForLink(placement.state);
    if (reasonCode) {
      skipped.push({ skillName, agentId, reasonCode });
      continue;
    }
    allowed.push({ skillName, agentId, placement });
  }
  return { allowed, skipped };
}

export function partitionUnlinkBatch(
  skills: readonly SkillsCliGlobalSkill[],
  skillNames: readonly string[],
): PlacementPartition {
  const allowed: PlacementPartitionItem[] = [];
  const skipped: PlacementPartition["skipped"] = [];
  for (const skillName of skillNames) {
    const skill = skillByName(skills, skillName);
    if (!skill) {
      skipped.push({
        skillName,
        agentId: "",
        reasonCode: SKILLS_CLI_SKIP_NOT_OWNED,
      });
      continue;
    }
    if (skill.placements.length === 0) {
      skipped.push({
        skillName,
        agentId: "",
        reasonCode: SKILLS_CLI_SKIP_NOT_LINKED,
      });
      continue;
    }
    for (const placement of skill.placements) {
      const reasonCode = skipReasonForUnlink(placement.state);
      if (reasonCode) {
        skipped.push({
          skillName,
          agentId: placement.agentId,
          reasonCode,
        });
        continue;
      }
      allowed.push({
        skillName,
        agentId: placement.agentId,
        placement,
      });
    }
  }
  return { allowed, skipped };
}

export function partitionUnlinkBatchForAgent(
  skills: readonly SkillsCliGlobalSkill[],
  skillNames: readonly string[],
  agentId: string,
): PlacementPartition {
  const allowed: PlacementPartitionItem[] = [];
  const skipped: PlacementPartition["skipped"] = [];
  for (const skillName of skillNames) {
    const skill = skillByName(skills, skillName);
    if (!skill) {
      skipped.push({
        skillName,
        agentId,
        reasonCode: SKILLS_CLI_SKIP_NOT_OWNED,
      });
      continue;
    }
    const placement = skill.placements.find((item) => item.agentId === agentId);
    if (!placement) {
      skipped.push({
        skillName,
        agentId,
        reasonCode: SKILLS_CLI_SKIP_NO_PLACEMENT,
      });
      continue;
    }
    const reasonCode = skipReasonForUnlink(placement.state);
    if (reasonCode) {
      skipped.push({ skillName, agentId, reasonCode });
      continue;
    }
    allowed.push({ skillName, agentId, placement });
  }
  return { allowed, skipped };
}

export function summarizeLinkTargets(
  skills: readonly SkillsCliGlobalSkill[],
  selectedNames: ReadonlySet<string>,
  targets: readonly SkillsCliInstallTarget[],
): SkillsCliLinkTargetSummary[] {
  const selected = selectedSkillsInStoreOrder(skills, selectedNames);
  return targets.map((target) => {
    let linkableCount = 0;
    let managedCount = 0;
    let directCopyCount = 0;
    let blockedCount = 0;
    for (const skill of selected) {
      const placement = skill.placements.find(
        (item) => item.agentId === target.id,
      );
      if (!placement) {
        blockedCount += 1;
        continue;
      }
      switch (placement.state) {
        case "missing":
          linkableCount += 1;
          break;
        case "managed_link":
          managedCount += 1;
          break;
        case "direct_copy":
          directCopyCount += 1;
          break;
        case "conflict":
        case "unavailable":
          blockedCount += 1;
          break;
        default: {
          const _exhaustive: never = placement.state;
          void _exhaustive;
          blockedCount += 1;
        }
      }
    }
    return {
      agentId: target.id,
      displayName: target.displayName,
      linkableCount,
      managedCount,
      directCopyCount,
      blockedCount,
    };
  });
}

export function selectedHasManagedLink(
  skills: readonly SkillsCliGlobalSkill[],
  selectedNames: ReadonlySet<string>,
): boolean {
  return selectedSkillsInStoreOrder(skills, selectedNames).some((skill) =>
    skill.placements.some((placement) => placement.state === "managed_link"),
  );
}

export function aggregateRemovalImpact(
  plans: readonly SkillsCliRemovePlan[],
): RemovalImpact {
  const skillNames = plans.map((plan) => plan.skillName);
  const retainedDirectCopies = plans.flatMap((plan) =>
    plan.retainedDirectCopies.map((item) => ({
      skillName: plan.skillName,
      agentId: item.agentId,
      displayName: item.displayName,
    })),
  );
  const conflicts = plans.flatMap((plan) =>
    plan.conflicts.map((item) => ({
      skillName: plan.skillName,
      agentId: item.agentId,
      displayName: item.displayName,
      reasonCode: item.reasonCode,
    })),
  );
  const ownedContentCount = plans.filter((plan) => plan.ownedCanonical).length;
  const managedLinkCount = plans.reduce(
    (total, plan) => total + plan.managedPlacements.length,
    0,
  );
  return {
    skillNames,
    ownedContentCount,
    managedLinkCount,
    retainedDirectCopies,
    conflicts,
    confirmable:
      plans.length > 0 &&
      plans.every((plan) => plan.confirmable) &&
      conflicts.length === 0,
  };
}

export function orderedExportPlacements(
  placements: readonly SkillsCliPlacement[],
  targetIds: readonly string[],
): SkillsCliExportPlacement[] {
  const byId = new Map(
    placements.map((placement) => [placement.agentId, placement]),
  );
  const seen = new Set<string>();
  const ordered: SkillsCliExportPlacement[] = [];
  for (const agentId of targetIds) {
    const placement = byId.get(agentId);
    if (!placement) {
      continue;
    }
    ordered.push(toExportPlacement(placement));
    seen.add(agentId);
  }
  for (const placement of placements) {
    if (seen.has(placement.agentId)) {
      continue;
    }
    ordered.push(toExportPlacement(placement));
  }
  return ordered;
}

function toExportPlacement(
  placement: SkillsCliPlacement,
): SkillsCliExportPlacement {
  return {
    agentId: placement.agentId,
    displayName: placement.displayName,
    state: placement.state,
  };
}

export function toExportSkillV1(
  skill: SkillsCliGlobalSkill,
  targetIds: readonly string[],
): SkillsCliExportSkillV1 {
  return {
    name: skill.name,
    source: skill.source,
    sourceType: skill.sourceType,
    sourceUrl: skill.sourceUrl,
    installKind: skill.installKind,
    canonicalPath: skill.canonicalPath,
    folderHash: skill.folderHash,
    installedAt: skill.installedAt,
    updatedAt: skill.updatedAt,
    placements: orderedExportPlacements(skill.placements, targetIds),
  };
}

export function buildSkillsCliExportV1(
  skills: readonly SkillsCliGlobalSkill[],
  scope: SkillsCliExportScope,
  now: Date,
  targetIds: readonly string[],
): SkillsCliExportV1 {
  const exported = skills.map((skill) => toExportSkillV1(skill, targetIds));
  return {
    schemaVersion: 1,
    exportedAt: now.toISOString(),
    scope,
    skillCount: exported.length,
    skills: exported,
  };
}

export function stringifySkillsCliExportV1(envelope: SkillsCliExportV1): string {
  return `${JSON.stringify(envelope, null, 2)}\n`;
}

export function skillsCliExportFileName(
  scope: SkillsCliExportScope,
  now: Date,
): string {
  const day = now.toISOString().slice(0, 10);
  return scope === "all"
    ? `skillport-skills-cli-all-${day}.json`
    : `skillport-skills-cli-selected-${day}.json`;
}
