import type { ProjectSkill } from "@/types";
import {
  getPlatformTargetMemberIds,
  type PlatformTarget,
} from "@/lib/platformTargetGroups";

export const PROJECT_SKILL_FALLBACK_GROUP_ID = "project-skill-hidden-platforms";

export interface ProjectSkillPlatformGroup {
  id: string;
  target: PlatformTarget | null;
  skills: ProjectSkill[];
  rawAgentIds: string[];
}

function sortProjectSkills(skills: ProjectSkill[]): ProjectSkill[] {
  return [...skills].sort((left, right) => {
    const nameDiff = left.name.localeCompare(right.name, undefined, {
      sensitivity: "base",
    });
    if (nameDiff !== 0) return nameDiff;

    const originDiff = left.sourceOrigin.localeCompare(right.sourceOrigin);
    if (originDiff !== 0) return originDiff;

    return left.agentDisplayName.localeCompare(right.agentDisplayName, undefined, {
      sensitivity: "base",
    });
  });
}

function uniqueAgentIds(skills: readonly ProjectSkill[]): string[] {
  return Array.from(new Set(skills.map((skill) => skill.agentId)));
}

export function groupProjectSkillsByPlatform(
  skills: readonly ProjectSkill[],
  targets: readonly PlatformTarget[]
): ProjectSkillPlatformGroup[] {
  const remaining = new Set(skills.map((_, index) => index));
  const groups: ProjectSkillPlatformGroup[] = [];

  for (const target of targets) {
    const memberIds = new Set(getPlatformTargetMemberIds(target));
    const targetSkills: ProjectSkill[] = [];

    for (const index of Array.from(remaining)) {
      const skill = skills[index];
      if (memberIds.has(skill.agentId)) {
        targetSkills.push(skill);
        remaining.delete(index);
      }
    }

    if (targetSkills.length > 0) {
      groups.push({
        id: target.id,
        target,
        skills: sortProjectSkills(targetSkills),
        rawAgentIds: uniqueAgentIds(targetSkills),
      });
    }
  }

  if (remaining.size > 0) {
    const fallbackSkills = sortProjectSkills(
      Array.from(remaining).map((index) => skills[index])
    );
    groups.push({
      id: PROJECT_SKILL_FALLBACK_GROUP_ID,
      target: null,
      skills: fallbackSkills,
      rawAgentIds: uniqueAgentIds(fallbackSkills),
    });
  }

  return groups;
}
