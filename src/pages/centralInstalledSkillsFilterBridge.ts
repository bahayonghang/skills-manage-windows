import { useMemo, useState } from "react";

import {
  matchesInstalledSkillsFilter,
  type InstalledSkillsFilterValue,
} from "@/lib/centralInstalledFilters";
import type { PlatformTarget } from "@/lib/platformTargetGroups";
import type { SkillWithLinks } from "@/types";

function filterInstalledSkills(
  skills: SkillWithLinks[],
  value: InstalledSkillsFilterValue,
  availableInstallAgents: readonly PlatformTarget[]
): SkillWithLinks[] {
  if (value === "all") {
    return skills;
  }

  return skills.filter((skill) =>
    matchesInstalledSkillsFilter(skill, value, availableInstallAgents)
  );
}

export function useCentralInstalledSkillsFilterBridge({
  availableInstallAgents,
  currentViewSkills,
  filteredSkills,
  selectedSkillIds,
  setIsBatchInstallDialogOpen,
  setSelectedSkillIds,
  v2FilteredSkills,
}: {
  availableInstallAgents: readonly PlatformTarget[];
  currentViewSkills: SkillWithLinks[];
  filteredSkills: SkillWithLinks[];
  selectedSkillIds: string[];
  setIsBatchInstallDialogOpen: (open: boolean) => void;
  setSelectedSkillIds: (skillIds: string[]) => void;
  v2FilteredSkills: SkillWithLinks[];
}) {
  const [installedSkillsFilter, setInstalledSkillsFilter] =
    useState<InstalledSkillsFilterValue>("all");
  const isInstalledSkillsFilterActive = installedSkillsFilter !== "all";
  const visibleCurrentViewSkills = useMemo(
    () =>
      filterInstalledSkills(
        currentViewSkills,
        installedSkillsFilter,
        availableInstallAgents
      ),
    [availableInstallAgents, currentViewSkills, installedSkillsFilter]
  );
  const installedSkillCount = useMemo(
    () =>
      filterInstalledSkills(currentViewSkills, "installed", availableInstallAgents)
        .length,
    [availableInstallAgents, currentViewSkills]
  );
  const installedSkillsFilterProps = {
    availableInstallAgents,
    filteredCount: visibleCurrentViewSkills.length,
    installedCount: installedSkillCount,
    selectedCount: selectedSkillIds.length,
    value: installedSkillsFilter,
    onChange: (value: InstalledSkillsFilterValue) => {
      setInstalledSkillsFilter(value);
      setSelectedSkillIds([]);
    },
    onClear: () => {
      setInstalledSkillsFilter("all");
      setSelectedSkillIds([]);
    },
    onInstallSelected: () => setIsBatchInstallDialogOpen(true),
    onSelectFiltered: () => {
      setSelectedSkillIds(visibleCurrentViewSkills.map((skill) => skill.id));
    },
  };

  return {
    installedSkillsFilter,
    installedSkillsFilterProps,
    isInstalledSkillsFilterActive,
    visibleCurrentViewSkills,
    visibleFilteredSkills: filterInstalledSkills(
      filteredSkills,
      installedSkillsFilter,
      availableInstallAgents
    ),
    visibleV2FilteredSkills: filterInstalledSkills(
      v2FilteredSkills,
      installedSkillsFilter,
      availableInstallAgents
    ),
  };
}
