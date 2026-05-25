import { useEffect, useMemo, useState } from "react";

import {
  getPlatformSkillRowKey,
  isPlatformBatchSelectableSkill,
} from "@/lib/platformBatchActions";
import type { ScannedSkill } from "@/types";

interface UsePlatformSkillSelectionOptions {
  agentId?: string;
  filteredSkills: ScannedSkill[];
}

export function usePlatformSkillSelection({
  agentId,
  filteredSkills,
}: UsePlatformSkillSelectionOptions) {
  const [selectedSkillKeys, setSelectedSkillKeys] = useState<Set<string>>(new Set());

  const selectableFilteredSkills = useMemo(
    () => filteredSkills.filter(isPlatformBatchSelectableSkill),
    [filteredSkills]
  );
  const selectableRowKeys = useMemo(
    () => new Set(selectableFilteredSkills.map((skill) => getPlatformSkillRowKey(skill))),
    [selectableFilteredSkills]
  );
  const selectedPlatformSkills = useMemo(
    () =>
      selectableFilteredSkills.filter((skill) =>
        selectedSkillKeys.has(getPlatformSkillRowKey(skill))
      ),
    [selectableFilteredSkills, selectedSkillKeys]
  );

  useEffect(() => {
    setSelectedSkillKeys(new Set());
  }, [agentId]);

  useEffect(() => {
    setSelectedSkillKeys((current) => {
      if (current.size === 0) {
        return current;
      }

      const next = new Set(
        Array.from(current).filter((skillKey) => selectableRowKeys.has(skillKey))
      );
      return next.size === current.size ? current : next;
    });
  }, [selectableRowKeys]);

  function toggleSelectedSkill(skill: ScannedSkill) {
    if (!isPlatformBatchSelectableSkill(skill)) return;

    const rowKey = getPlatformSkillRowKey(skill);
    setSelectedSkillKeys((current) => {
      const next = new Set(current);
      if (next.has(rowKey)) {
        next.delete(rowKey);
      } else {
        next.add(rowKey);
      }
      return next;
    });
  }

  function selectCurrentResults() {
    setSelectedSkillKeys(
      new Set(selectableFilteredSkills.map((skill) => getPlatformSkillRowKey(skill)))
    );
  }

  function clearSelectedSkills() {
    setSelectedSkillKeys(new Set());
  }

  return {
    selectedSkillKeys,
    setSelectedSkillKeys,
    selectableFilteredSkills,
    selectedPlatformSkills,
    toggleSelectedSkill,
    selectCurrentResults,
    clearSelectedSkills,
  };
}
