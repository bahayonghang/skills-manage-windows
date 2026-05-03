import type { AgentWithStatus } from "@/types";
import { compactHomePath } from "@/lib/path";
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

function isUniversalAgent(agent: AgentWithStatus): boolean {
  return UNIVERSAL_AGENT_ID_ORDER.includes(
    agent.id as (typeof UNIVERSAL_AGENT_ID_ORDER)[number]
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
  return sortUniversalMembers(agents.filter(isUniversalAgent));
}

export function createPlatformTargetGroups(
  visibleAgents: AgentWithStatus[],
  allAgents: AgentWithStatus[] = visibleAgents
): PlatformTarget[] {
  const allUniversalMembers = getUniversalPlatformMembers(allAgents);

  if (allUniversalMembers.length === 0) {
    return visibleAgents.filter((agent) => agent.id !== "central");
  }

  const visibleMemberIds = new Set(
    visibleAgents
      .filter(isUniversalAgent)
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

export function hasProjectSkillPattern(agent: PlatformTarget): boolean {
  if (agent.project_skills_dir?.trim()) {
    return true;
  }

  const normalized = agent.global_skills_dir.replace(/\\/g, "/");
  return normalized.startsWith("~/") || /\/\.[^/]+\/skills\/?$/.test(normalized);
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
