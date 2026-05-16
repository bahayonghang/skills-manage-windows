import type { TFunction } from "i18next";

import type { SkillWithLinks } from "@/types";

type CheckButtonScope = "selected" | "current-results" | "all";

export interface CentralSkillsCheckButtonInput {
  currentViewSkills: SkillWithLinks[];
  hasCurrentFilters: boolean;
  selectedSkillIds: string[];
  sortedSkills: SkillWithLinks[];
  t: TFunction;
  totalSkillCount: number;
}

export interface CentralSkillsCheckButtonState {
  label: string;
  scopedSkillIds?: string[];
  targetSkillIds: string[];
}

export function getCentralSkillsCheckButtonState({
  currentViewSkills,
  hasCurrentFilters,
  selectedSkillIds,
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

  return {
    label: getCheckButtonLabel({ scope, t, targetSkillIds, totalSkillCount }),
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

function getCheckButtonLabel({
  scope,
  t,
  targetSkillIds,
  totalSkillCount,
}: {
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
  return t("central.checkUpdatesAll", { count: totalSkillCount });
}
