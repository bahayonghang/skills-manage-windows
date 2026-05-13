import { useMemo } from "react";

import type { PlatformCategoryVisibility } from "@/lib/platformVisibility";
import {
  getPlatformTargetGroups,
  type PlatformTarget,
} from "@/lib/platformTargetGroups";
import { buildSearchText, normalizeSearchQuery } from "@/lib/search";
import type { AgentWithStatus, DiscoveredProject, DiscoveredSkill } from "@/types";

export interface DiscoverContext {
  projectPath?: string;
  skillSearch?: string;
}

export interface ScrollRestorationState {
  key?: string;
  scrollTop?: number;
}

export interface DiscoverViewModel {
  displayedSkills: DiscoveredSkill[];
  filteredProjectList: DiscoveredProject[];
  normalizedProjectQuery: string;
  normalizedSkillQuery: string;
  platformAgents: PlatformTarget[];
  selectedProject: DiscoveredProject | null;
  selectedProjectMatchesFilter: boolean;
}

export function useDiscoverViewModel({
  agents,
  categoryVisibility,
  deferredSkillSearch,
  discoveredProjects,
  projectPath,
  projectSearch,
  skillSearch,
}: {
  agents: AgentWithStatus[];
  categoryVisibility: PlatformCategoryVisibility;
  deferredSkillSearch: string;
  discoveredProjects: DiscoveredProject[];
  projectPath: string | undefined;
  projectSearch: string;
  skillSearch: string;
}): DiscoverViewModel {
  const normalizedProjectQuery = useMemo(
    () => normalizeSearchQuery(projectSearch),
    [projectSearch]
  );

  const filteredProjectList = useMemo(() => {
    if (!normalizedProjectQuery) return discoveredProjects;
    return discoveredProjects.filter(
      (project) =>
        project.project_name.toLowerCase().includes(normalizedProjectQuery) ||
        project.project_path.toLowerCase().includes(normalizedProjectQuery)
    );
  }, [discoveredProjects, normalizedProjectQuery]);

  const selectedProject = useMemo(() => {
    if (!projectPath) return null;
    const decodedProjectPath = decodeURIComponent(projectPath);
    return (
      discoveredProjects.find(
        (project) => project.project_path === decodedProjectPath
      ) ?? null
    );
  }, [discoveredProjects, projectPath]);

  const effectiveSkillSearch =
    selectedProject && selectedProject.skills.length > 80
      ? deferredSkillSearch
      : skillSearch;

  const normalizedSkillQuery = useMemo(
    () => normalizeSearchQuery(effectiveSkillSearch),
    [effectiveSkillSearch]
  );

  const selectedProjectSkillEntries = useMemo(
    () =>
      (selectedProject?.skills ?? []).map((skill) => ({
        skill,
        searchText: buildSearchText([skill.name, skill.description]),
      })),
    [selectedProject]
  );

  const selectedProjectMatchesFilter = useMemo(() => {
    if (!selectedProject) return true;
    if (!normalizedProjectQuery) return true;
    return (
      selectedProject.project_name.toLowerCase().includes(normalizedProjectQuery) ||
      selectedProject.project_path.toLowerCase().includes(normalizedProjectQuery)
    );
  }, [selectedProject, normalizedProjectQuery]);

  const displayedSkills = useMemo(() => {
    if (!selectedProject) return [];
    if (!normalizedSkillQuery) return selectedProject.skills;
    return selectedProjectSkillEntries
      .filter(({ searchText }) => searchText.includes(normalizedSkillQuery))
      .map(({ skill }) => skill);
  }, [normalizedSkillQuery, selectedProject, selectedProjectSkillEntries]);

  const platformAgents = useMemo(
    () => getPlatformTargetGroups(agents, categoryVisibility),
    [agents, categoryVisibility]
  );

  return {
    displayedSkills,
    filteredProjectList,
    normalizedProjectQuery,
    normalizedSkillQuery,
    platformAgents,
    selectedProject,
    selectedProjectMatchesFilter,
  };
}
