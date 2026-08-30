import type { TFunction } from "i18next";

import type {
  BatchDeleteCentralSkillPreviewResult,
  BatchDeleteCentralSkillRequest,
  DeleteCentralSkillPreview,
  DuplicateResolution,
} from "@/types";
import type {
  CentralRemoteAddedSkill,
  CentralRemoteMissingSkill,
  CentralRepositorySyncPreview,
} from "@/types/centralRepositorySync";

export type RepositorySyncTab = "pending" | "skipped" | "missing" | "failed";
export type MissingDecision = "keep" | "delete";

export type AddedDecision = {
  selected: boolean;
  resolution: DuplicateResolution;
  renamedSkillId: string;
  deleteExisting: boolean;
};

export type SkippedDecision = {
  action: "keep" | "import" | "rename" | "unskip" | "delete";
  renamedSkillId: string;
};

export const REPOSITORY_SYNC_TABS: readonly RepositorySyncTab[] = [
  "pending",
  "skipped",
  "missing",
  "failed",
] as const;

const SKILL_ID_PATTERN = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;

export function uniqueIds(ids: string[]): string[] {
  return Array.from(new Set(ids));
}

export function copyKey(skillId: string, agentId: string): string {
  return `${skillId}\0${agentId}`;
}

export function addedKey(repositoryId: string, sourcePath: string): string {
  return `${repositoryId}\0${sourcePath}`;
}

export function missingSkillId(item: CentralRemoteMissingSkill): string {
  return item.state.skill_id;
}

export function conflictSkillId(item: CentralRemoteAddedSkill): string | null {
  return item.preview.conflict?.existingSkillId?.trim() || null;
}

export function defaultRepositorySyncTab(preview: CentralRepositorySyncPreview | null): RepositorySyncTab {
  if (!preview) return "pending";
  if (preview.remoteAdded.length > 0) return "pending";
  if (preview.skippedRemoteAdded.length > 0) return "skipped";
  if (preview.remoteMissing.length > 0) return "missing";
  return "failed";
}

export function collectRepositorySyncDeletePreviewSkillIds(
  preview: CentralRepositorySyncPreview
): string[] {
  return uniqueIds([
    ...preview.remoteMissing.map((item) => item.state.skill_id),
    ...preview.remoteAdded.map(conflictSkillId).filter((id): id is string => Boolean(id)),
    ...preview.skippedRemoteAdded.map(conflictSkillId).filter((id): id is string => Boolean(id)),
  ]);
}

export function buildDeletePreviewMaps(deletePreview: BatchDeleteCentralSkillPreviewResult | null) {
  return {
    previewBySkillId: new Map((deletePreview?.previews ?? []).map((item) => [item.skill_id, item])),
    failedBySkillId: new Map((deletePreview?.failed ?? []).map((item) => [item.skill_id, item.error])),
  };
}

export function validateSkillId(value: string, t: TFunction): string | null {
  const trimmed = value.trim();
  if (!trimmed) return t("central.repositorySyncRenameRequired");
  if (!SKILL_ID_PATTERN.test(trimmed)) return t("central.repositorySyncRenameInvalid");
  return null;
}

export function deleteRequestFromPreview(
  item: DeleteCentralSkillPreview,
  selectedCopyKeys: ReadonlySet<string>
): BatchDeleteCentralSkillRequest {
  return {
    skill_id: item.skill_id,
    remove_agent_ids: uniqueIds(
      item.copy_installations
        .map((installation) => installation.agent_id)
        .filter((agentId) => selectedCopyKeys.has(copyKey(item.skill_id, agentId)))
    ),
    force: false,
  };
}

export function mergeDeleteRequest(
  requests: Map<string, BatchDeleteCentralSkillRequest>,
  next: BatchDeleteCentralSkillRequest
) {
  const existing = requests.get(next.skill_id);
  requests.set(next.skill_id, {
    skill_id: next.skill_id,
    remove_agent_ids: uniqueIds([...(existing?.remove_agent_ids ?? []), ...next.remove_agent_ids]),
    force: Boolean(existing?.force || next.force),
  });
}
