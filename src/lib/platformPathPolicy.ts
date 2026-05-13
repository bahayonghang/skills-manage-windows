import type { AgentWithStatus, PlatformPathMap, ResolvedPlatformPaths } from "@/types";
import { joinPathForDisplay } from "@/lib/path";

export const BROWSER_PLATFORM_PATHS: PlatformPathMap = {
  "claude-code": {
    global_skills_dir: "~/.claude/skills/",
    project_skills_dir: ".claude/skills",
  },
  codex: {
    global_skills_dir: "~/.agents/skills/",
    project_skills_dir: ".codex/skills",
  },
  "gemini-cli": {
    global_skills_dir: "~/.agents/skills/",
    project_skills_dir: ".gemini/skills",
  },
  opencode: {
    global_skills_dir: "~/.agents/skills/",
    project_skills_dir: ".opencode/skills",
  },
  kiro: {
    global_skills_dir: "~/.kiro/skills/",
    project_skills_dir: ".kiro/skills",
  },
  cursor: {
    global_skills_dir: "~/.agents/skills/",
    project_skills_dir: ".cursor/skills",
  },
  openclaw: {
    global_skills_dir: "~/.openclaw/skills/",
    project_skills_dir: null,
  },
  central: {
    global_skills_dir: "~/.skillsmanage/skills/",
    project_skills_dir: null,
  },
};

export function collectPlatformPathsFromAgents(
  agents: AgentWithStatus[],
  platformPaths: PlatformPathMap = {}
): PlatformPathMap {
  return Object.fromEntries(
    agents.map((agent) => {
      const paths = platformPaths[agent.id];
      return [
        agent.id,
        {
          global_skills_dir: paths?.global_skills_dir ?? agent.global_skills_dir,
          project_skills_dir:
            paths?.project_skills_dir ?? agent.project_skills_dir ?? null,
        },
      ];
    })
  );
}

export function applyPlatformPathsToAgents(
  agents: AgentWithStatus[],
  platformPaths: PlatformPathMap = {}
): AgentWithStatus[] {
  return agents.map((agent) => {
    const paths = platformPaths[agent.id];
    if (!paths) return agent;
    return {
      ...agent,
      global_skills_dir: paths.global_skills_dir,
      project_skills_dir: paths.project_skills_dir ?? undefined,
    };
  });
}

export function getPlatformPathsForAgent(
  platformPaths: PlatformPathMap,
  agentId: string
): ResolvedPlatformPaths | undefined {
  return platformPaths[agentId];
}

export function getPlatformGlobalSkillsDir(
  platformPaths: PlatformPathMap,
  agentId: string
): string | undefined {
  return getPlatformPathsForAgent(platformPaths, agentId)?.global_skills_dir;
}

export function getPlatformSkillDir(
  platformPaths: PlatformPathMap,
  agentId: string,
  skillId: string
): string {
  const globalSkillsDir = getPlatformGlobalSkillsDir(platformPaths, agentId);
  return globalSkillsDir ? joinPathForDisplay(globalSkillsDir, skillId) : skillId;
}

export function getPlatformSkillFilePath(
  platformPaths: PlatformPathMap,
  agentId: string,
  skillId: string
): string {
  return joinPathForDisplay(
    getPlatformSkillDir(platformPaths, agentId, skillId),
    "SKILL.md"
  );
}
