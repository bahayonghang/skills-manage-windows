import type { AgentWithStatus } from "@/types";
import {
  createPlatformTargetGroups,
  getPlatformTargetMemberIds,
  isUniversalPlatformTarget,
} from "@/lib/platformTargetGroups";

export interface PlatformCleanupGroup {
  id: string;
  label: string;
  agentIds: string[];
  detail: string | null;
}

function uniqueValues(values: string[]): string[] {
  return Array.from(new Set(values.filter(Boolean)));
}

function agentNameById(agents: AgentWithStatus[], agentId: string): string {
  return agents.find((agent) => agent.id === agentId)?.display_name ?? agentId;
}

export function groupPlatformAgentIds(
  agents: AgentWithStatus[],
  agentIds: string[],
  universalLabel = "Universal"
): PlatformCleanupGroup[] {
  const requestedIds = new Set(uniqueValues(agentIds).filter((agentId) => agentId !== "central"));
  if (requestedIds.size === 0) {
    return [];
  }

  const targets = createPlatformTargetGroups(
    agents.filter((agent) => agent.id !== "central"),
    agents
  );
  const groupedIds = new Set<string>();
  const groups: PlatformCleanupGroup[] = [];

  for (const target of targets) {
    const memberIds = getPlatformTargetMemberIds(target).filter((agentId) =>
      requestedIds.has(agentId)
    );
    if (memberIds.length === 0) {
      continue;
    }

    memberIds.forEach((agentId) => groupedIds.add(agentId));

    if (isUniversalPlatformTarget(target)) {
      const memberNames = target.member_agents
        .filter((member) => requestedIds.has(member.id))
        .map((member) => member.display_name);
      groups.push({
        id: target.id,
        label: universalLabel,
        agentIds: memberIds,
        detail: memberNames.join(", "),
      });
      continue;
    }

    groups.push({
      id: target.id,
      label: target.display_name,
      agentIds: memberIds,
      detail: target.global_skills_dir,
    });
  }

  for (const agentId of requestedIds) {
    if (groupedIds.has(agentId)) {
      continue;
    }
    groups.push({
      id: agentId,
      label: agentNameById(agents, agentId),
      agentIds: [agentId],
      detail: agents.find((agent) => agent.id === agentId)?.global_skills_dir ?? null,
    });
  }

  return groups;
}
