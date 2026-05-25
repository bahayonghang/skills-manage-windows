import {
  getPlatformSkillRowKey,
  isWritablePlatformSkillRow,
} from "@/lib/platformDuplicateSkills";
import type {
  BatchUninstallSkillRequest,
  BatchUninstallSkillResult,
  ScannedSkill,
} from "@/types";

export { getPlatformSkillRowKey };

export function getPlatformSkillSummaryKeys(skill: ScannedSkill): string[] {
  return skill.row_id ? [skill.row_id, skill.id] : [skill.id];
}

export function getClaudeUserRowId(
  skill: ScannedSkill,
  agentId?: string
): string | undefined {
  return agentId === "claude-code" && skill.source_kind === "user" && !skill.is_read_only
    ? skill.row_id
    : undefined;
}

export function createPlatformBatchUninstallRequest(
  skill: ScannedSkill,
  agentId?: string
): BatchUninstallSkillRequest {
  const rowId = getClaudeUserRowId(skill, agentId);
  return rowId ? { skill_id: skill.id, row_id: rowId } : { skill_id: skill.id };
}

export function getBatchUninstallRequestActionKey(
  agentId: string,
  request: BatchUninstallSkillRequest
): string {
  return request.row_id ?? `${agentId}::${request.skill_id}`;
}

export function getPlatformSkillActionKey(
  skill: ScannedSkill,
  agentId?: string
): string {
  if (!agentId) {
    return getPlatformSkillRowKey(skill);
  }

  return getBatchUninstallRequestActionKey(
    agentId,
    createPlatformBatchUninstallRequest(skill, agentId)
  );
}

export function isPlatformBatchSelectableSkill(skill: ScannedSkill): boolean {
  return isWritablePlatformSkillRow(skill);
}

export function getFailedBatchUninstallActionKeys(
  agentId: string,
  result: BatchUninstallSkillResult
): Set<string> {
  return new Set(
    result.failed.map((item) => getBatchUninstallRequestActionKey(agentId, item))
  );
}
