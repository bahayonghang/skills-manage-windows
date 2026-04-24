import type { AgentWithStatus } from "@/types";
import { arePathsEquivalent, compactHomePath } from "@/lib/path";
import {
  filterVisiblePlatformAgents,
  type PlatformCategoryVisibility,
} from "@/lib/platformVisibility";

export const UNIVERSAL_PLATFORM_TARGET_ID = "universal-agents";

const UNIVERSAL_AGENT_ID_ORDER = [
  "amp",
  "antigravity",
  "cline",
  "codex",
  "cursor",
  "deep-agents",
  "firebender",
  "gemini-cli",
  "copilot",
  "kimi-code-cli",
  "opencode",
  "warp",
] as const;

const UNIVERSAL_INSTALL_AGENT_ORDER = [
  "codex",
  "opencode",
  "gemini-cli",
  "cursor",
  "amp",
] as const;

export interface PlatformTargetGroup extends AgentWithStatus {
  is_virtual_group: true;
  member_agents: AgentWithStatus[];
  install_agent_id: string;
}

export type PlatformTarget = AgentWithStatus | PlatformTargetGroup;

function getCentralAgent(agents: AgentWithStatus[]): AgentWithStatus | undefined {
  return agents.find((agent) => agent.id === "central");
}

function universalAgentRank(agent: AgentWithStatus): number {
  const rank = UNIVERSAL_AGENT_ID_ORDER.indexOf(
    agent.id as (typeof UNIVERSAL_AGENT_ID_ORDER)[number]
  );
  return rank === -1 ? Number.MAX_SAFE_INTEGER : rank;
}

function sortUniversalMembers(agents: AgentWithStatus[]): AgentWithStatus[] {
  return [...agents].sort((left, right) => {
    const rankDiff = universalAgentRank(left) - universalAgentRank(right);
    if (rankDiff !== 0) {
      return rankDiff;
    }

    return left.display_name.localeCompare(right.display_name, undefined, {
      sensitivity: "base",
    });
  });
}

function isSharedCentralRoot(
  agent: AgentWithStatus,
  centralAgent: AgentWithStatus | undefined
): boolean {
  return (
    agent.id !== "central" &&
    Boolean(centralAgent) &&
    arePathsEquivalent(agent.global_skills_dir, centralAgent?.global_skills_dir)
  );
}

function selectUniversalInstallAgent(
  visibleMembers: AgentWithStatus[],
  allMembers: AgentWithStatus[]
): AgentWithStatus {
  for (const preferredId of UNIVERSAL_INSTALL_AGENT_ORDER) {
    const visibleMatch = visibleMembers.find((agent) => agent.id === preferredId);
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
  agent: PlatformTarget
): agent is PlatformTargetGroup {
  return (
    agent.id === UNIVERSAL_PLATFORM_TARGET_ID ||
    Boolean((agent as PlatformTargetGroup).is_virtual_group)
  );
}

export function getUniversalPlatformMembers(
  agents: AgentWithStatus[]
): AgentWithStatus[] {
  const centralAgent = getCentralAgent(agents);
  if (!centralAgent) {
    return [];
  }

  return sortUniversalMembers(
    agents.filter((agent) => isSharedCentralRoot(agent, centralAgent))
  );
}

export function createPlatformTargetGroups(
  visibleAgents: AgentWithStatus[],
  allAgents: AgentWithStatus[] = visibleAgents
): PlatformTarget[] {
  const centralAgent = getCentralAgent(allAgents);
  const allUniversalMembers = getUniversalPlatformMembers(allAgents);

  if (!centralAgent || allUniversalMembers.length === 0) {
    return visibleAgents.filter((agent) => agent.id !== "central");
  }

  const visibleMemberIds = new Set(
    visibleAgents
      .filter((agent) => isSharedCentralRoot(agent, centralAgent))
      .map((agent) => agent.id)
  );

  if (visibleMemberIds.size === 0) {
    return visibleAgents.filter((agent) => agent.id !== "central");
  }

  const visibleMembers = allUniversalMembers.filter((agent) =>
    visibleMemberIds.has(agent.id)
  );
  const installAgent = selectUniversalInstallAgent(
    visibleMembers,
    allUniversalMembers
  );
  const standaloneAgents = visibleAgents.filter(
    (agent) => agent.id !== "central" && !visibleMemberIds.has(agent.id)
  );
  const universalGroup: PlatformTargetGroup = {
    ...installAgent,
    id: UNIVERSAL_PLATFORM_TARGET_ID,
    display_name: "Universal",
    category: "coding",
    global_skills_dir: centralAgent.global_skills_dir,
    project_skills_dir: centralAgent.project_skills_dir,
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

export function getPlatformTargetGroups(
  agents: AgentWithStatus[],
  categoryVisibility: PlatformCategoryVisibility
): PlatformTarget[] {
  return createPlatformTargetGroups(
    filterVisiblePlatformAgents(agents, categoryVisibility),
    agents
  );
}

export function getPlatformTargetMemberIds(
  agent: PlatformTarget
): string[] {
  if (!isUniversalPlatformTarget(agent)) {
    return [agent.id];
  }

  return agent.member_agents.map((member) => member.id);
}

export function getPlatformTargetInstallAgentIds(
  agent: PlatformTarget
): string[] {
  if (!isUniversalPlatformTarget(agent)) {
    return [agent.id];
  }

  return [agent.install_agent_id];
}

export function getPlatformTargetMemberNames(
  agent: PlatformTarget
): string[] {
  if (!isUniversalPlatformTarget(agent)) {
    return [agent.display_name];
  }

  return agent.member_agents.map((member) => member.display_name);
}

export function getPlatformTargetPathHint(
  agent: PlatformTarget
): string {
  return compactHomePath(agent.global_skills_dir);
}
