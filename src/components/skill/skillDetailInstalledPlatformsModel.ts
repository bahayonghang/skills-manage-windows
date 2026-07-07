import { flattenPlatformTargets } from "@/lib/platformTargetGroups";
import type { AgentWithStatus, SkillInstallation } from "@/types";

export interface InstalledPlatformRow {
  agentId: string;
  displayName: string;
  installedPath: string;
  linkType: string;
  isRemovable: boolean;
}

export interface InstalledPlatformUninstallRequest {
  agentId: string;
  displayName: string;
}

export function buildInstalledPlatformRows(
  targetAgents: AgentWithStatus[],
  installationMap: Map<string, SkillInstallation>,
  sharedRootAgentIds: Set<string>
): InstalledPlatformRow[] {
  const agentById = new Map(
    flattenPlatformTargets(targetAgents).map((agent) => [agent.id, agent])
  );

  return Array.from(installationMap.values())
    .filter((installation) => installation.agent_id !== "central")
    .map((installation) => {
      const agent = agentById.get(installation.agent_id);
      return {
        agentId: installation.agent_id,
        displayName: agent?.display_name ?? installation.agent_id,
        installedPath: installation.installed_path,
        linkType: installation.link_type,
        isRemovable: !sharedRootAgentIds.has(installation.agent_id),
      };
    })
    .sort((left, right) =>
      left.displayName.localeCompare(right.displayName, undefined, {
        sensitivity: "base",
      })
    );
}
