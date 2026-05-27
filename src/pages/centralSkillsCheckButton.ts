import type { TFunction } from "i18next";

import type { SkillRepositoryWithStats, SkillWithLinks } from "@/types";

type CheckButtonScope = "selected" | "current-results" | "all";
type CheckButtonMode = "skill" | "repository-sync";

export interface CentralSkillsCheckButtonInput {
  currentViewSkills: SkillWithLinks[];
  hasCurrentFilters: boolean;
  hasNonRepositoryFilters: boolean;
  repositories: SkillRepositoryWithStats[];
  selectedSkillIds: string[];
  selectedRepoIds: string[];
  sortedSkills: SkillWithLinks[];
  t: TFunction;
  totalSkillCount: number;
}

export interface CentralSkillsCheckButtonState {
  label: string;
  mode: CheckButtonMode;
  repositoryIds?: string[];
  scopedSkillIds?: string[];
  targetSkillIds: string[];
}

export function getCentralSkillsCheckButtonState({
  currentViewSkills,
  hasCurrentFilters,
  hasNonRepositoryFilters,
  repositories,
  selectedSkillIds,
  selectedRepoIds,
  sortedSkills,
  t,
  totalSkillCount,
}: CentralSkillsCheckButtonInput): CentralSkillsCheckButtonState {
  const scope = getCheckButtonScope({ hasCurrentFilters, selectedSkillIds });
  const targetSkillIds = getCheckButtonTargetSkillIds({
    currentViewSkills,
    scope,
    selectedSkillIds,
    sortedSkills,
  });
  const syncableRepoIds = getSyncableRepositoryIds(repositories);
  const isSingleRepoScope =
    scope === "current-results" &&
    selectedRepoIds.length === 1 &&
    !hasNonRepositoryFilters;
  const repositoryIds =
    scope === "all" ? syncableRepoIds : isSingleRepoScope ? selectedRepoIds : [];
  const mode: CheckButtonMode = repositoryIds.length > 0 ? "repository-sync" : "skill";
  const selectedRepository = repositories.find((repo) => repo.id === selectedRepoIds[0]);

  return {
    label: getCheckButtonLabel({
      scope,
      t,
      targetSkillIds,
      totalSkillCount,
      repositoryName: isSingleRepoScope ? selectedRepository?.name : undefined,
    }),
    mode,
    repositoryIds,
    scopedSkillIds: scope === "all" ? undefined : targetSkillIds,
    targetSkillIds,
  };
}

function getCheckButtonScope({
  hasCurrentFilters,
  selectedSkillIds,
}: {
  hasCurrentFilters: boolean;
  selectedSkillIds: string[];
}): CheckButtonScope {
  if (selectedSkillIds.length > 0) return "selected";
  if (hasCurrentFilters) return "current-results";
  return "all";
}

function getCheckButtonTargetSkillIds({
  currentViewSkills,
  scope,
  selectedSkillIds,
  sortedSkills,
}: {
  currentViewSkills: SkillWithLinks[];
  scope: CheckButtonScope;
  selectedSkillIds: string[];
  sortedSkills: SkillWithLinks[];
}): string[] {
  if (scope === "selected") return selectedSkillIds;
  if (scope === "current-results") return currentViewSkills.map((skill) => skill.id);
  return sortedSkills.map((skill) => skill.id);
}

function getSyncableRepositoryIds(repositories: SkillRepositoryWithStats[]): string[] {
  return repositories
    .filter((repo) => repo.source_type === "github" && !repo.is_unknown)
    .map((repo) => repo.id);
}

function getCheckButtonLabel({
  repositoryName,
  scope,
  t,
  targetSkillIds,
  totalSkillCount,
}: {
  repositoryName?: string;
  scope: CheckButtonScope;
  t: TFunction;
  targetSkillIds: string[];
  totalSkillCount: number;
}): string {
  if (scope === "selected") {
    return t("central.checkUpdatesSelected", { count: targetSkillIds.length });
  }
  if (scope === "current-results") {
    if (repositoryName) {
      return t("central.checkUpdatesRepository", {
        repo: repositoryName,
        count: targetSkillIds.length,
      });
    }
    return t("central.checkUpdatesCurrentResults", { count: targetSkillIds.length });
  }
  return t("central.checkUpdatesAll", { count: totalSkillCount });
}
