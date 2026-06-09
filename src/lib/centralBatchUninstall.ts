import type { BatchUninstallSkillRequest, SkillWithLinks } from "@/types";

export type CentralBatchUninstallSkipReason =
  | "no_platform_installs"
  | "shared_root_only";

export interface CentralBatchUninstallSkippedSkill {
  skillId: string;
  skillName: string;
  reason: CentralBatchUninstallSkipReason;
}

export interface CentralBatchUninstallSharedRootLink {
  skillId: string;
  skillName: string;
  agentId: string;
}

export interface CentralBatchUninstallAgentGroup {
  agentId: string;
  requests: BatchUninstallSkillRequest[];
}

export interface CentralBatchUninstallPreview {
  selectedSkillIds: string[];
  selectedSkills: SkillWithLinks[];
  groups: CentralBatchUninstallAgentGroup[];
  skippedSkills: CentralBatchUninstallSkippedSkill[];
  sharedRootLinks: CentralBatchUninstallSharedRootLink[];
  totals: {
    selectedSkillCount: number;
    removableInstallCount: number;
    removablePlatformCount: number;
    skippedSkillCount: number;
    sharedRootInstallCount: number;
  };
}

export interface CentralBatchUninstallSuccess {
  skill_id: string;
  agent_id: string;
}

export interface CentralBatchUninstallFailure {
  skill_id: string;
  agent_id: string;
  error: string;
}

export interface CentralBatchUninstallApplyResult {
  succeeded: CentralBatchUninstallSuccess[];
  failed: CentralBatchUninstallFailure[];
  skipped: CentralBatchUninstallSkippedSkill[];
  sharedRootLinks: CentralBatchUninstallSharedRootLink[];
}

function uniqueIds(ids: readonly string[] | undefined): string[] {
  return Array.from(new Set((ids ?? []).filter(Boolean)));
}

export function createCentralBatchUninstallPreview(
  selectedSkillIds: readonly string[],
  skills: readonly SkillWithLinks[],
): CentralBatchUninstallPreview {
  const skillById = new Map(skills.map((skill) => [skill.id, skill]));
  const uniqueSelectedSkillIds = uniqueIds([...selectedSkillIds]);
  const groupsByAgent = new Map<string, Map<string, BatchUninstallSkillRequest>>();
  const selectedSkills: SkillWithLinks[] = [];
  const skippedSkills: CentralBatchUninstallSkippedSkill[] = [];
  const sharedRootLinks: CentralBatchUninstallSharedRootLink[] = [];

  for (const skillId of uniqueSelectedSkillIds) {
    const skill = skillById.get(skillId);
    if (!skill) continue;

    selectedSkills.push(skill);

    const sharedRootAgentIds = new Set(uniqueIds(skill.shared_root_agents));
    const linkedAgentIds = uniqueIds(skill.linked_agents);
    const removableAgentIds = linkedAgentIds.filter(
      (agentId) => agentId !== "central" && !sharedRootAgentIds.has(agentId),
    );

    for (const agentId of linkedAgentIds) {
      if (agentId === "central" || !sharedRootAgentIds.has(agentId)) {
        continue;
      }
      sharedRootLinks.push({
        skillId: skill.id,
        skillName: skill.name,
        agentId,
      });
    }

    if (removableAgentIds.length === 0) {
      skippedSkills.push({
        skillId: skill.id,
        skillName: skill.name,
        reason:
          sharedRootLinks.some((link) => link.skillId === skill.id)
            ? "shared_root_only"
            : "no_platform_installs",
      });
      continue;
    }

    for (const agentId of removableAgentIds) {
      const requestsBySkill =
        groupsByAgent.get(agentId) ??
        new Map<string, BatchUninstallSkillRequest>();
      requestsBySkill.set(skill.id, { skill_id: skill.id });
      groupsByAgent.set(agentId, requestsBySkill);
    }
  }

  const groups = Array.from(groupsByAgent, ([agentId, requestsBySkill]) => ({
    agentId,
    requests: Array.from(requestsBySkill.values()),
  }));

  const removableInstallCount = groups.reduce(
    (count, group) => count + group.requests.length,
    0,
  );

  return {
    selectedSkillIds: selectedSkills.map((skill) => skill.id),
    selectedSkills,
    groups,
    skippedSkills,
    sharedRootLinks,
    totals: {
      selectedSkillCount: selectedSkills.length,
      removableInstallCount,
      removablePlatformCount: groups.length,
      skippedSkillCount: skippedSkills.length,
      sharedRootInstallCount: sharedRootLinks.length,
    },
  };
}
