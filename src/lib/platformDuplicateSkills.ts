import type { ScannedSkill } from "@/types";

export interface DuplicatePlatformSkillGroup {
  skillId: string;
  name: string;
  writableRows: ScannedSkill[];
  pluginRows: ScannedSkill[];
  rows: ScannedSkill[];
}

export function getPlatformSkillRowKey(skill: ScannedSkill): string {
  return skill.row_id ?? `${skill.id}::${skill.dir_path}`;
}

export function isPluginReadOnlySkillRow(skill: ScannedSkill): boolean {
  return skill.source_kind === "plugin" || skill.is_read_only === true;
}

export function isWritablePlatformSkillRow(skill: ScannedSkill): boolean {
  return !isPluginReadOnlySkillRow(skill);
}

export function findDuplicatePlatformSkillGroups(
  skills: ScannedSkill[]
): DuplicatePlatformSkillGroup[] {
  const byId = new Map<string, ScannedSkill[]>();

  for (const skill of skills) {
    const group = byId.get(skill.id) ?? [];
    group.push(skill);
    byId.set(skill.id, group);
  }

  return Array.from(byId.entries())
    .map(([skillId, rows]) => {
      const writableRows = rows.filter(isWritablePlatformSkillRow);
      const pluginRows = rows.filter(isPluginReadOnlySkillRow);
      return {
        skillId,
        name: rows[0]?.name ?? skillId,
        writableRows,
        pluginRows,
        rows,
      };
    })
    .filter((group) => group.writableRows.length > 0 && group.pluginRows.length > 0)
    .sort((left, right) => left.name.localeCompare(right.name));
}
