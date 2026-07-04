import type { AgentWithStatus } from "@/types";
import type { TFunction } from "i18next";
import { getPlatformPathHint as formatPlatformPathHint } from "@/lib/path";
import {
  UNIVERSAL_AGENT_ID_ORDER,
  UNIVERSAL_INSTALL_AGENT_ORDER,
  UNIVERSAL_PROJECT_AGENT_ID_ORDER,
} from "@/lib/platformRegistry";
import {
  filterVisiblePlatformAgents,
  type PlatformCategoryVisibility,
} from "@/lib/platformVisibility";

export const UNIVERSAL_PLATFORM_TARGET_ID = "universal-agents";

export interface PlatformTargetGroup extends AgentWithStatus {
  is_virtual_group: true;
  member_agents: AgentWithStatus[];
  install_agent_id: string;
}

export type PlatformTarget = AgentWithStatus | PlatformTargetGroup;

function universalAgentRank(
  agent: AgentWithStatus,
  order: readonly string[],
): number {
  const rank = order.findIndex((id) => id === agent.id);
  return rank === -1 ? Number.MAX_SAFE_INTEGER : rank;
}

function sortUniversalMembers(
  agents: AgentWithStatus[],
  order: readonly string[],
): AgentWithStatus[] {
  return [...agents].sort((left, right) => {
    const rankDiff =
      universalAgentRank(left, order) - universalAgentRank(right, order);
    if (rankDiff !== 0) {
      return rankDiff;
    }

    return left.display_name.localeCompare(right.display_name, undefined, {
      sensitivity: "base",
    });
  });
}

function isUniversalAgent(agent: AgentWithStatus): boolean {
  return UNIVERSAL_AGENT_ID_ORDER.includes(agent.id);
}

function isUniversalProjectAgent(agent: AgentWithStatus): boolean {
  return UNIVERSAL_PROJECT_AGENT_ID_ORDER.includes(agent.id);
}

function selectUniversalInstallAgent(
  visibleMembers: AgentWithStatus[],
  allMembers: AgentWithStatus[],
): AgentWithStatus {
  for (const preferredId of UNIVERSAL_INSTALL_AGENT_ORDER) {
    const visibleMatch = visibleMembers.find(
      (agent) => agent.id === preferredId,
    );
    if (visibleMatch) {
      return visibleMatch;
    }

    const match = allMembers.find((agent) => agent.id === preferredId);
    if (match) {
      return match;
    }
  }

  return visibleMembers[0] ?? allMembers[0];
}

export function isUniversalPlatformTarget(
  agent: PlatformTarget,
): agent is PlatformTargetGroup {
  return (
    agent.id === UNIVERSAL_PLATFORM_TARGET_ID ||
    Boolean((agent as PlatformTargetGroup).is_virtual_group)
  );
}

export function getUniversalPlatformMembers(
  agents: AgentWithStatus[],
): AgentWithStatus[] {
  return sortUniversalMembers(
    agents.filter(isUniversalAgent),
    UNIVERSAL_AGENT_ID_ORDER,
  );
}

function createPlatformTargetGroupsForScope(
  visibleAgents: AgentWithStatus[],
  allAgents: AgentWithStatus[],
  isUniversalMember: (agent: AgentWithStatus) => boolean,
  universalOrder: readonly string[],
): PlatformTarget[] {
  const allUniversalMembers = sortUniversalMembers(
    allAgents.filter(isUniversalMember),
    universalOrder,
  );

  if (allUniversalMembers.length === 0) {
    return visibleAgents.filter((agent) => agent.id !== "central");
  }

  const visibleMemberIds = new Set(
    visibleAgents.filter(isUniversalMember).map((agent) => agent.id),
  );

  if (visibleMemberIds.size === 0) {
    return visibleAgents.filter((agent) => agent.id !== "central");
  }

  const visibleMembers = allUniversalMembers.filter((agent) =>
    visibleMemberIds.has(agent.id),
  );
  const installAgent = selectUniversalInstallAgent(
    visibleMembers,
    allUniversalMembers,
  );
  const standaloneAgents = visibleAgents.filter(
    (agent) => agent.id !== "central" && !visibleMemberIds.has(agent.id),
  );
  const universalGroup: PlatformTargetGroup = {
    ...installAgent,
    id: UNIVERSAL_PLATFORM_TARGET_ID,
    display_name: "Universal",
    category: "coding",
    global_skills_dir: installAgent.global_skills_dir,
    project_skills_dir: installAgent.project_skills_dir,
    icon_name: "universal-agents",
    is_detected: allUniversalMembers.some((agent) => agent.is_detected),
    is_builtin: true,
    is_enabled: visibleMembers.some((agent) => agent.is_enabled),
    is_virtual_group: true,
    member_agents: allUniversalMembers,
    install_agent_id: installAgent.id,
  };

  return [universalGroup, ...standaloneAgents];
}

export function createPlatformTargetGroups(
  visibleAgents: AgentWithStatus[],
  allAgents: AgentWithStatus[] = visibleAgents,
): PlatformTarget[] {
  return createPlatformTargetGroupsForScope(
    visibleAgents,
    allAgents,
    isUniversalAgent,
    UNIVERSAL_AGENT_ID_ORDER,
  );
}

export function createProjectPlatformTargetGroups(
  visibleAgents: AgentWithStatus[],
  allAgents: AgentWithStatus[] = visibleAgents,
): PlatformTarget[] {
  return createPlatformTargetGroupsForScope(
    visibleAgents,
    allAgents,
    isUniversalProjectAgent,
    UNIVERSAL_PROJECT_AGENT_ID_ORDER,
  );
}

export function getPlatformTargetGroups(
  agents: AgentWithStatus[],
  categoryVisibility: PlatformCategoryVisibility,
): PlatformTarget[] {
  return createPlatformTargetGroups(
    filterVisiblePlatformAgents(agents, categoryVisibility),
    agents,
  );
}

export function getProjectPlatformTargetGroups(
  agents: AgentWithStatus[],
  categoryVisibility: PlatformCategoryVisibility,
): PlatformTarget[] {
  return createProjectPlatformTargetGroups(
    filterVisiblePlatformAgents(agents, categoryVisibility),
    agents,
  );
}

export function flattenPlatformTargets(
  targets: readonly PlatformTarget[],
): AgentWithStatus[] {
  const byId = new Map<string, AgentWithStatus>();
  for (const target of targets) {
    if (isUniversalPlatformTarget(target)) {
      for (const member of target.member_agents) {
        byId.set(member.id, member);
      }
    } else {
      byId.set(target.id, target);
    }
  }

  return Array.from(byId.values());
}

export function getPlatformTargetMemberIds(agent: PlatformTarget): string[] {
  if (!isUniversalPlatformTarget(agent)) {
    return [agent.id];
  }

  return agent.member_agents.map((member) => member.id);
}

export function getPlatformTargetInstallAgentIds(
  agent: PlatformTarget,
): string[] {
  if (!isUniversalPlatformTarget(agent)) {
    return [agent.id];
  }

  return [agent.install_agent_id];
}

export function hasProjectSkillPattern(agent: PlatformTarget): boolean {
  if (isUniversalPlatformTarget(agent)) {
    return agent.member_agents.some(isUniversalProjectAgent);
  }

  if (isUniversalProjectAgent(agent)) {
    return true;
  }

  if (agent.project_skills_dir?.trim()) {
    return true;
  }

  const normalized = agent.global_skills_dir.replace(/\\/g, "/");
  return (
    normalized.startsWith("~/") || /\/\.[^/]+\/skills\/?$/.test(normalized)
  );
}

export function getPlatformTargetMemberNames(agent: PlatformTarget): string[] {
  if (!isUniversalPlatformTarget(agent)) {
    return [agent.display_name];
  }

  return agent.member_agents.map((member) => member.display_name);
}

export function getPlatformTargetPathHint(agent: PlatformTarget): string {
  return formatPlatformPathHint(agent.global_skills_dir);
}

export function getPlatformTargetLabel(
  target: PlatformTarget,
  t: TFunction,
  variant: "full" | "short",
): string {
  if (!isUniversalPlatformTarget(target)) {
    return target.display_name;
  }

  return variant === "short"
    ? t("platformTargets.universalShortLabel")
    : t("platformTargets.universalLabel");
}

export function getPlatformTargetTitleHint(target: PlatformTarget): string {
  return isUniversalPlatformTarget(target)
    ? getPlatformTargetMemberNames(target).join(", ")
    : target.global_skills_dir;
}

export function getPlatformTargetCountAgentId(target: PlatformTarget): string {
  return isUniversalPlatformTarget(target)
    ? target.install_agent_id
    : target.id;
}
