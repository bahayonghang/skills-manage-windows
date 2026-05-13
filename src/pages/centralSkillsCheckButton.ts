import type { TFunction } from "i18next";

import type { SkillRepositoryWithStats, SkillWithLinks } from "@/types";

type CheckButtonScope = "selected" | "current-results" | "repository" | "all";

export interface CentralSkillsCheckButtonInput {
  currentViewSkills: SkillWithLinks[];
  repositories: SkillRepositoryWithStats[];
  repositoryFilter: string;
  selectedSkillIds: string[];
  sortedSkills: SkillWithLinks[];
  t: TFunction;
  totalSkillCount: number;
  v2Enabled: boolean;
  v2HasCurrentFilters: boolean;
}

export interface CentralSkillsCheckButtonState {
  label: string;
  scopedSkillIds?: string[];
  targetSkillIds: string[];
}

export function getCentralSkillsCheckButtonState({
  currentViewSkills,
  repositories,
  repositoryFilter,
  selectedSkillIds,
  sortedSkills,
  t,
  totalSkillCount,
  v2Enabled,
  v2HasCurrentFilters,
}: CentralSkillsCheckButtonInput): CentralSkillsCheckButtonState {
  const scope = getCheckButtonScope({
    repositoryFilter,
    selectedSkillIds,
    v2Enabled,
    v2HasCurrentFilters,
  });
  const targetSkillIds = getCheckButtonTargetSkillIds({
    currentViewSkills,
    repositoryFilter,
    scope,
    selectedSkillIds,
    sortedSkills,
  });

  return {
    label: getCheckButtonLabel({
      repositories,
      repositoryFilter,
      scope,
      t,
      targetSkillIds,
      totalSkillCount,
    }),
    scopedSkillIds: scope === "all" ? undefined : targetSkillIds,
    targetSkillIds,
  };
}

function getCheckButtonScope({
  repositoryFilter,
  selectedSkillIds,
  v2Enabled,
  v2HasCurrentFilters,
}: {
  repositoryFilter: string;
  selectedSkillIds: string[];
  v2Enabled: boolean;
  v2HasCurrentFilters: boolean;
}): CheckButtonScope {
  if (selectedSkillIds.length > 0) return "selected";
  if (v2Enabled && v2HasCurrentFilters) return "current-results";
  if (repositoryFilter !== "all") return "repository";
  return "all";
}

function getCheckButtonTargetSkillIds({
  currentViewSkills,
  repositoryFilter,
  scope,
  selectedSkillIds,
  sortedSkills,
}: {
  currentViewSkills: SkillWithLinks[];
  repositoryFilter: string;
  scope: CheckButtonScope;
  selectedSkillIds: string[];
  sortedSkills: SkillWithLinks[];
}): string[] {
  if (scope === "selected") return selectedSkillIds;
  if (scope === "current-results") return currentViewSkills.map((skill) => skill.id);
  if (scope === "repository") {
    return sortedSkills
      .filter((skill) => (skill.repository?.id ?? null) === repositoryFilter)
      .map((skill) => skill.id);
  }
  return sortedSkills.map((skill) => skill.id);
}

function getCheckButtonLabel({
  repositories,
  repositoryFilter,
  scope,
  t,
  targetSkillIds,
  totalSkillCount,
}: {
  repositories: SkillRepositoryWithStats[];
  repositoryFilter: string;
  scope: CheckButtonScope;
  t: TFunction;
  targetSkillIds: string[];
  totalSkillCount: number;
}): string {
  if (scope === "selected") {
    return t("central.checkUpdatesSelected", { count: targetSkillIds.length });
  }
  if (scope === "current-results") {
    return t("central.checkUpdatesCurrentResults", { count: targetSkillIds.length });
  }
  if (scope === "repository") {
    const repositoryName =
      repositories.find((repo) => repo.id === repositoryFilter)?.name ?? "";
    return t("central.checkUpdatesRepository", {
      repo: repositoryName,
      count: targetSkillIds.length,
    });
  }
  return t("central.checkUpdatesAll", { count: totalSkillCount });
}
