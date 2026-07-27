import type { BatchDeleteCentralSkillRequest } from "@/types";
import type {
  CentralRepositoryAdditionSkipRequest,
  CentralRepositoryAddedSkillSelection,
} from "@/types/centralRepositorySync";
import type { UpdateCenterTab } from "@/stores/updateCenterStore";
import type {
  DeletedPlatformCopyRemoval,
  PlatformDuplicateRemoval,
  SkillRefreshScopeKind,
  SkillUpdateDecisions,
  SkillUpdateInventory,
} from "@/types/skillUpdateInventory";

import type { DeletedPlatformCopyRowState } from "@/components/central/updateCenter/DeletedPlatformCopiesTabPanel";
import type { PlatformDuplicateRowState } from "@/components/central/updateCenter/PlatformDuplicatesTabPanel";
import type { RemoteAddedRowState } from "@/components/central/updateCenter/RemoteAddedTabPanel";
import type { RemoteMissingRowState } from "@/components/central/updateCenter/RemoteMissingTabPanel";
import type { UpdatableRowState } from "@/components/central/updateCenter/UpdatableTabPanel";
import {
  deletedPlatformCopyGroupKey,
  duplicateGroupKey,
  remoteAddedKey,
} from "@/components/central/updateCenter/keys";

export interface DecisionState {
  updatable: Record<string, UpdatableRowState>;
  added: Record<string, RemoteAddedRowState>;
  missing: Record<string, RemoteMissingRowState>;
  duplicates: Record<string, PlatformDuplicateRowState>;
  deletedPlatformCopies: Record<string, DeletedPlatformCopyRowState>;
}

export function emptyDecisionState(): DecisionState {
  return {
    updatable: {},
    added: {},
    missing: {},
    duplicates: {},
    deletedPlatformCopies: {},
  };
}

export function buildInitialState(
  inventory: SkillUpdateInventory | null,
  scopeKind: SkillRefreshScopeKind = "all",
): DecisionState {
  if (!inventory) return emptyDecisionState();
  const shouldSelectPlatformCleanup = scopeKind === "platform";
  const updatable: Record<string, UpdatableRowState> = {};
  for (const item of inventory.updatable) {
    updatable[item.state.skill_id] = { selected: true };
  }
  const added: Record<string, RemoteAddedRowState> = {};
  for (const item of inventory.remoteAdded) {
    const key = remoteAddedKey(item.repositoryId, item.sourcePath);
    const hasConflict = Boolean(item.conflictExistingSkillId);
    added[key] = {
      selected: true,
      resolution: hasConflict ? "skip" : "overwrite",
      renamedSkillId: item.skillId,
    };
  }
  const missing: Record<string, RemoteMissingRowState> = {};
  for (const item of inventory.remoteMissing) {
    missing[item.state.skill_id] = { decision: "keep", removeAgentIds: [] };
  }
  const duplicates: Record<string, PlatformDuplicateRowState> = {};
  for (const group of inventory.platformDuplicates) {
    duplicates[duplicateGroupKey(group)] = {
      selectedPaths: shouldSelectPlatformCleanup ? [...group.writablePaths] : [],
    };
  }
  const deletedPlatformCopies: Record<string, DeletedPlatformCopyRowState> = {};
  for (const group of inventory.deletedPlatformCopies ?? []) {
    deletedPlatformCopies[deletedPlatformCopyGroupKey(group)] = {
      selectedPaths: shouldSelectPlatformCleanup ? [...group.writablePaths] : [],
    };
  }
  return { updatable, added, missing, duplicates, deletedPlatformCopies };
}

export function countsFromInventory(
  inventory: SkillUpdateInventory | null,
): Record<UpdateCenterTab, number> {
  if (!inventory) {
    return {
      updatable: 0,
      added: 0,
      missing: 0,
      failed: 0,
      duplicates: 0,
      deletedPlatformCopies: 0,
      orphans: 0,
    };
  }
  return {
    updatable: inventory.updatable.length,
    added: inventory.remoteAdded.length,
    missing: inventory.remoteMissing.length,
    failed: inventory.failedRepositories.length,
    duplicates: inventory.platformDuplicates.length,
    deletedPlatformCopies: inventory.deletedPlatformCopies?.length ?? 0,
    orphans: inventory.orphans.length,
  };
}

export function countDecisionSelections(
  decisions: DecisionState,
  inventory: SkillUpdateInventory | null,
): number {
  if (!inventory) return 0;
  let count = 0;
  for (const item of inventory.updatable) {
    if (decisions.updatable[item.state.skill_id]?.selected) count += 1;
  }
  for (const item of inventory.remoteAdded) {
    const key = remoteAddedKey(item.repositoryId, item.sourcePath);
    if (decisions.added[key]?.selected) count += 1;
  }
  for (const item of inventory.remoteMissing) {
    const decision = decisions.missing[item.state.skill_id]?.decision;
    if (decision === "keep" || decision === "delete") count += 1;
  }
  for (const group of inventory.platformDuplicates) {
    const paths = decisions.duplicates[duplicateGroupKey(group)]?.selectedPaths ?? [];
    count += paths.length;
  }
  for (const group of inventory.deletedPlatformCopies ?? []) {
    const paths =
      decisions.deletedPlatformCopies[deletedPlatformCopyGroupKey(group)]
        ?.selectedPaths ?? [];
    count += paths.length;
  }
  return count;
}

export interface DecisionSelectionSummary {
  updatable: number;
  added: number;
  missing: number;
  duplicates: number;
  deletedPlatformCopies: number;
}

export function summarizeDecisionSelections(
  decisions: DecisionState,
  inventory: SkillUpdateInventory | null,
): DecisionSelectionSummary {
  if (!inventory) {
    return {
      updatable: 0,
      added: 0,
      missing: 0,
      duplicates: 0,
      deletedPlatformCopies: 0,
    };
  }
  let updatable = 0;
  let added = 0;
  let missing = 0;
  let duplicates = 0;
  let deletedPlatformCopies = 0;

  for (const item of inventory.updatable) {
    if (decisions.updatable[item.state.skill_id]?.selected) updatable += 1;
  }
  for (const item of inventory.remoteAdded) {
    const key = remoteAddedKey(item.repositoryId, item.sourcePath);
    if (decisions.added[key]?.selected) added += 1;
  }
  for (const item of inventory.remoteMissing) {
    const decision = decisions.missing[item.state.skill_id]?.decision;
    if (decision === "keep" || decision === "delete") missing += 1;
  }
  for (const group of inventory.platformDuplicates) {
    duplicates +=
      decisions.duplicates[duplicateGroupKey(group)]?.selectedPaths.length ?? 0;
  }
  for (const group of inventory.deletedPlatformCopies ?? []) {
    deletedPlatformCopies +=
      decisions.deletedPlatformCopies[deletedPlatformCopyGroupKey(group)]
        ?.selectedPaths.length ?? 0;
  }

  return { updatable, added, missing, duplicates, deletedPlatformCopies };
}

export function countDeletedPlatformCopyPaths(
  inventory: SkillUpdateInventory | null,
): number {
  if (!inventory) return 0;
  return (inventory.deletedPlatformCopies ?? []).reduce(
    (total, group) => total + new Set(group.writablePaths).size,
    0,
  );
}

export function buildDeletedPlatformCopyCleanupDecisions(
  inventory: SkillUpdateInventory,
  allowedAgentIds?: string[],
): SkillUpdateDecisions {
  return {
    allowedAgentIds:
      allowedAgentIds && allowedAgentIds.length > 0 ? allowedAgentIds : null,
    updates: [],
    keepMissing: [],
    deleteMissing: [],
    importAdditions: [],
    skipAdditions: [],
    unskipAdditions: [],
    removePlatformDuplicates: [],
    removeDeletedPlatformCopies: (inventory.deletedPlatformCopies ?? [])
      .map<DeletedPlatformCopyRemoval>((group) => ({
        agentId: group.agentId,
        skillId: group.skillId,
        paths: Array.from(new Set(group.writablePaths)),
      }))
      .filter((removal) => removal.paths.length > 0),
  };
}

export function buildDecisions(
  decisions: DecisionState,
  inventory: SkillUpdateInventory,
  allowedAgentIds?: string[],
): SkillUpdateDecisions {
  const updates: string[] = [];
  for (const item of inventory.updatable) {
    if (decisions.updatable[item.state.skill_id]?.selected) {
      updates.push(item.state.skill_id);
    }
  }

  const keepMissing: string[] = [];
  const deleteMissing: BatchDeleteCentralSkillRequest[] = [];
  for (const item of inventory.remoteMissing) {
    const state = decisions.missing[item.state.skill_id];
    if (state?.decision === "keep") {
      keepMissing.push(item.state.skill_id);
      continue;
    }
    if (state?.decision === "delete") {
      deleteMissing.push({
        skill_id: item.state.skill_id,
        remove_agent_ids: state.removeAgentIds,
      });
    }
  }

  const additionsByRepo = new Map<string, CentralRepositoryAddedSkillSelection>();
  const skipAdditions: CentralRepositoryAdditionSkipRequest[] = [];
  for (const item of inventory.remoteAdded) {
    const key = remoteAddedKey(item.repositoryId, item.sourcePath);
    const decision = decisions.added[key];
    if (!decision?.selected) continue;
    if (decision.resolution === "skip") {
      skipAdditions.push({
        repositoryId: item.repositoryId,
        sourcePath: item.sourcePath,
        skillId: item.skillId,
        skillName: item.skillName,
      });
      continue;
    }
    const entry = additionsByRepo.get(item.repositoryId) ?? {
      repositoryId: item.repositoryId,
      selections: [],
    };
    entry.selections.push({
      sourcePath: item.sourcePath,
      resolution: decision.resolution,
      renamedSkillId:
        decision.resolution === "rename"
          ? decision.renamedSkillId.trim() || item.skillId
          : null,
    });
    additionsByRepo.set(item.repositoryId, entry);
  }

  const removePlatformDuplicates: PlatformDuplicateRemoval[] = [];
  for (const group of inventory.platformDuplicates) {
    const paths = decisions.duplicates[duplicateGroupKey(group)]?.selectedPaths ?? [];
    if (paths.length === 0) continue;
    removePlatformDuplicates.push({
      agentId: group.agentId,
      skillId: group.skillId,
      paths,
    });
  }

  const removeDeletedPlatformCopies: DeletedPlatformCopyRemoval[] = [];
  for (const group of inventory.deletedPlatformCopies ?? []) {
    const paths =
      decisions.deletedPlatformCopies[deletedPlatformCopyGroupKey(group)]
        ?.selectedPaths ?? [];
    if (paths.length === 0) continue;
    removeDeletedPlatformCopies.push({
      agentId: group.agentId,
      skillId: group.skillId,
      paths,
    });
  }

  return {
    allowedAgentIds:
      allowedAgentIds && allowedAgentIds.length > 0 ? allowedAgentIds : null,
    updates,
    keepMissing,
    deleteMissing,
    importAdditions: Array.from(additionsByRepo.values()),
    skipAdditions,
    unskipAdditions: [],
    removePlatformDuplicates,
    removeDeletedPlatformCopies,
  };
}

export function inventorySignature(
  inventory: SkillUpdateInventory | null,
): string {
  if (!inventory) return "empty";
  const parts: string[] = [
    ...inventory.updatable.map((item) => `u:${item.state.skill_id}`),
    ...inventory.remoteAdded.map(
      (item) => `a:${item.repositoryId}:${item.sourcePath}`,
    ),
    ...inventory.remoteMissing.map((item) => `m:${item.state.skill_id}`),
    ...inventory.platformDuplicates.map(
      (group) => `d:${group.agentId}:${group.skillId}`,
    ),
    ...(inventory.deletedPlatformCopies ?? []).map(
      (group) => `x:${group.agentId}:${group.skillId}`,
    ),
  ];
  return parts.join("|");
}
