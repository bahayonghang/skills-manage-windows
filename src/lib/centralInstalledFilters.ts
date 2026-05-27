import {
  getPlatformTargetMemberIds,
  type PlatformTarget,
} from "@/lib/platformTargetGroups";
import type { SkillWithLinks } from "@/types";

export type InstalledSkillsFilterValue = "all" | "installed" | `platform:${string}`;

export function platformInstalledFilterValue(agentId: string): InstalledSkillsFilterValue {
  return `platform:${agentId}`;
}

export function getInstalledFilterPlatformId(
  value: InstalledSkillsFilterValue
): string | null {
  return value.startsWith("platform:") ? value.slice("platform:".length) : null;
}

export function getSkillInstalledAgentIds(skill: SkillWithLinks): Set<string> {
  return new Set([...(skill.linked_agents ?? []), ...(skill.shared_root_agents ?? [])]);
}

export function getSkillInstalledPlatformCount(skill: SkillWithLinks): number {
  return getSkillInstalledAgentIds(skill).size;
}

export function selectMostUniversalSkills(skills: readonly SkillWithLinks[]): SkillWithLinks[] {
  const maxCount = skills.reduce(
    (max, skill) => Math.max(max, getSkillInstalledPlatformCount(skill)),
    0
  );
  if (maxCount <= 0) return [];
  return skills.filter((skill) => getSkillInstalledPlatformCount(skill) === maxCount);
}

export function matchesInstalledSkillsFilter(
  skill: SkillWithLinks,
  filter: InstalledSkillsFilterValue,
  platformTargets: readonly PlatformTarget[]
): boolean {
  if (filter === "all") return true;

  const installedAgentIds = getSkillInstalledAgentIds(skill);
  if (filter === "installed") return installedAgentIds.size > 0;

  const platformId = getInstalledFilterPlatformId(filter);
  if (!platformId) return true;

  const target = platformTargets.find((agent) => agent.id === platformId);
  const targetAgentIds = target ? getPlatformTargetMemberIds(target) : [platformId];
  return targetAgentIds.some((agentId) => installedAgentIds.has(agentId));
}
