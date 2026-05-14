import { AgentWithStatus } from "@/types";

export type PlatformCategoryKey = "coding" | "lobster";

export interface PlatformCategoryVisibility {
  coding: boolean;
  lobster: boolean;
}

export const PLATFORM_CATEGORY_VISIBILITY_SETTING_KEY = "platform_category_visibility";

export const DEFAULT_ENABLED_PLATFORM_IDS = [
  "claude-code",
  "codex",
  "gemini-cli",
  "opencode",
  "kiro",
] as const;

const DEFAULT_ENABLED_PLATFORM_ID_SET = new Set<string>(DEFAULT_ENABLED_PLATFORM_IDS);

export const DEFAULT_PLATFORM_CATEGORY_VISIBILITY: PlatformCategoryVisibility = {
  coding: true,
  lobster: false,
};

export function getPlatformCategoryKey(category: string): PlatformCategoryKey {
  return category === "lobster" ? "lobster" : "coding";
}

export function isDefaultEnabledPlatform(agentId: string): boolean {
  return DEFAULT_ENABLED_PLATFORM_ID_SET.has(agentId);
}

export function derivePlatformCategoryVisibility(
  agents: AgentWithStatus[]
): PlatformCategoryVisibility {
  if (agents.length === 0) {
    return DEFAULT_PLATFORM_CATEGORY_VISIBILITY;
  }

  return {
    coding: agents.some(
      (agent) =>
        agent.id !== "central" &&
        getPlatformCategoryKey(agent.category) === "coding" &&
        agent.is_enabled
    ),
    lobster: agents.some(
      (agent) =>
        agent.id !== "central" &&
        getPlatformCategoryKey(agent.category) === "lobster" &&
        agent.is_enabled
    ),
  };
}

export function normalizePlatformCategoryVisibility(
  value: Partial<PlatformCategoryVisibility> | null | undefined,
  fallback = DEFAULT_PLATFORM_CATEGORY_VISIBILITY
): PlatformCategoryVisibility {
  return {
    coding: value?.coding ?? fallback.coding,
    lobster: value?.lobster ?? fallback.lobster,
  };
}

export function ensureAtLeastOnePlatformCategoryVisible(
  value: PlatformCategoryVisibility,
  fallback = DEFAULT_PLATFORM_CATEGORY_VISIBILITY,
  agents: AgentWithStatus[] = []
): PlatformCategoryVisibility {
  if (value.coding || value.lobster || agents.length === 0) {
    return value;
  }

  if (fallback.coding || fallback.lobster) {
    return fallback;
  }

  return DEFAULT_PLATFORM_CATEGORY_VISIBILITY;
}

export function resolvePlatformCategoryVisibility(
  value: string | null | undefined,
  agents: AgentWithStatus[]
): PlatformCategoryVisibility {
  const fallback = derivePlatformCategoryVisibility(agents);

  if (!value) {
    return fallback;
  }

  try {
    const parsed = JSON.parse(value) as Partial<PlatformCategoryVisibility>;
    return ensureAtLeastOnePlatformCategoryVisible(
      normalizePlatformCategoryVisibility(parsed, fallback),
      fallback,
      agents
    );
  } catch {
    return fallback;
  }
}

export function parsePlatformCategoryVisibility(
  value: string | null | undefined
): PlatformCategoryVisibility {
  return resolvePlatformCategoryVisibility(value, []);
}

export function isPlatformAgentVisible(
  agent: AgentWithStatus,
  categoryVisibility: PlatformCategoryVisibility
): boolean {
  if (agent.id === "central") {
    return true;
  }

  if (!agent.is_enabled) {
    return false;
  }

  return categoryVisibility[getPlatformCategoryKey(agent.category)];
}

export function filterVisiblePlatformAgents(
  agents: AgentWithStatus[],
  categoryVisibility: PlatformCategoryVisibility
): AgentWithStatus[] {
  const visibleAgents = agents.filter(
    (agent) =>
      agent.id !== "central" && isPlatformAgentVisible(agent, categoryVisibility)
  );

  if (visibleAgents.length > 0 || categoryVisibility.coding || categoryVisibility.lobster) {
    return visibleAgents;
  }

  return agents.filter(
    (agent) =>
      agent.id !== "central" && isPlatformAgentVisible(agent, DEFAULT_PLATFORM_CATEGORY_VISIBILITY)
  );
}

export function sortPlatformVisibilityAgents(
  agents: AgentWithStatus[]
): AgentWithStatus[] {
  return [...agents].sort((left, right) => {
    if (left.is_enabled !== right.is_enabled) {
      return left.is_enabled ? -1 : 1;
    }

    const leftDefaultRank = DEFAULT_ENABLED_PLATFORM_IDS.indexOf(
      left.id as (typeof DEFAULT_ENABLED_PLATFORM_IDS)[number]
    );
    const rightDefaultRank = DEFAULT_ENABLED_PLATFORM_IDS.indexOf(
      right.id as (typeof DEFAULT_ENABLED_PLATFORM_IDS)[number]
    );

    if (leftDefaultRank !== -1 || rightDefaultRank !== -1) {
      if (leftDefaultRank === -1) {
        return 1;
      }

      if (rightDefaultRank === -1) {
        return -1;
      }

      return leftDefaultRank - rightDefaultRank;
    }

    return left.display_name.localeCompare(right.display_name, undefined, {
      sensitivity: "base",
    });
  });
}
