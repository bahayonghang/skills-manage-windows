import {
  getPlatformTargetMemberNames,
  isUniversalPlatformTarget,
  type PlatformTarget,
} from "@/lib/platformTargetGroups";

export function matchesPlatformVisibilityQuery(
  agent: PlatformTarget,
  normalizedQuery: string
) {
  if (!normalizedQuery) return true;
  const tokens = [
    agent.id,
    agent.display_name,
    agent.global_skills_dir,
    agent.category,
    isUniversalPlatformTarget(agent) ? "Universal universal-agents" : "",
    ...getPlatformTargetMemberNames(agent),
    ...(isUniversalPlatformTarget(agent)
      ? agent.member_agents.map((member) => member.global_skills_dir)
      : []),
  ];
  return tokens.some((token) =>
    token.toLowerCase().includes(normalizedQuery)
  );
}
