import type {
  BatchDeleteCentralSkillRequest,
  CentralSkillUpdateState,
} from "@/types";
import type {
  CentralRepositoryAddedSkillSelection,
  CentralRepositoryAdditionSkipRequest,
  CentralRepositoryAdditionUnskipRequest,
} from "@/types/centralRepositorySync";

export type SkillRefreshScopeKind = "all" | "skills" | "repositories";

export interface SkillRefreshScope {
  kind: SkillRefreshScopeKind;
  skillIds?: string[] | null;
  repositoryIds?: string[] | null;
}

export interface SkillRefreshContext {
  /** 当前明确选中的、可同步 GitHub repository ids。 */
  repositoryIds: string[];
  /** 当前结果列表中可见的 Central skill ids。 */
  skillIds: string[];
}

export interface UpdatableSkill {
  state: CentralSkillUpdateState;
  repositoryId?: string | null;
}

export interface RemoteAddedSkill {
  repositoryId: string;
  sourcePath: string;
  skillId: string;
  skillName: string;
  conflictExistingSkillId?: string | null;
}

export interface RemoteMissingSkill {
  state: CentralSkillUpdateState;
  repositoryId?: string | null;
}

export interface PlatformDuplicateGroup {
  agentId: string;
  skillId: string;
  skillName: string;
  writablePaths: string[];
  pluginPaths: string[];
}

export interface OrphanSkillEntry {
  skillId: string;
  brokenPath: string;
}

export interface FailedRepository {
  repositoryId: string;
  error: string;
}

export interface SkillUpdateInventory {
  updatable: UpdatableSkill[];
  remoteAdded: RemoteAddedSkill[];
  remoteMissing: RemoteMissingSkill[];
  platformDuplicates: PlatformDuplicateGroup[];
  /** P2 始终空，保留位给未来的 broken symlink / 孤儿副本扫描。 */
  orphans: OrphanSkillEntry[];
  failedRepositories: FailedRepository[];
  generatedAt: string;
}

export interface PlatformDuplicateRemoval {
  agentId: string;
  skillId: string;
  paths: string[];
}

export interface SkillUpdateDecisions {
  updates: string[];
  keepMissing: string[];
  deleteMissing: BatchDeleteCentralSkillRequest[];
  importAdditions: CentralRepositoryAddedSkillSelection[];
  skipAdditions: CentralRepositoryAdditionSkipRequest[];
  unskipAdditions: CentralRepositoryAdditionUnskipRequest[];
  removePlatformDuplicates: PlatformDuplicateRemoval[];
}

export interface SkillUpdateApplyFailure {
  step: string;
  identifier: string;
  error: string;
}

export interface SkillUpdateApplyResult {
  updatedSkillIds: string[];
  keptMissingSkillIds: string[];
  deletedSkillIds: string[];
  importedSkillIds: string[];
  skippedAdditions: string[];
  unskippedAdditions: string[];
  removedPlatformDuplicatePaths: string[];
  failures: SkillUpdateApplyFailure[];
}
