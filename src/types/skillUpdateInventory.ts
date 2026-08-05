import type {
  BatchDeleteCentralSkillRequest,
  CentralSkillUpdateState,
} from "@/types";
import type {
  CentralRepositoryAddedSkillSelection,
  CentralRepositoryAdditionSkipRequest,
  CentralRepositoryAdditionUnskipRequest,
} from "@/types/centralRepositorySync";

export type SkillRefreshScopeKind =
  "all" | "skills" | "repositories" | "platform";
export type SkillRefreshMode = "regular" | "sync";
export type SkillRefreshCachePolicy = "use_fresh" | "bypass";

export type SkillUpdateInventoryProgressStatus =
  | "started"
  | "repository_started"
  | "repository_completed"
  | "repository_failed"
  | "finalizing";

export interface SkillUpdateInventoryProgressPayload {
  operationId: string;
  status: SkillUpdateInventoryProgressStatus;
  total: number;
  completed: number;
  repositoryKey?: string | null;
  repositoryName?: string | null;
}

export interface ActiveRefreshRepository {
  key: string;
  name: string;
}

export interface SkillUpdateInventoryRefreshProgress {
  operationId: string;
  phase: "preparing" | "checking" | "finalizing";
  total: number;
  completed: number;
  activeRepositories: ActiveRefreshRepository[];
}

export interface SkillRefreshScope {
  kind: SkillRefreshScopeKind;
  mode?: SkillRefreshMode;
  cachePolicy?: SkillRefreshCachePolicy;
  skillIds?: string[] | null;
  repositoryIds?: string[] | null;
  agentIds?: string[] | null;
}

export interface SkillRefreshContext {
  /** 当前明确选中的、可同步 GitHub repository ids。 */
  repositoryIds: string[];
  /** 当前结果列表中可见的 Central skill ids。 */
  skillIds: string[];
  /** 当前平台入口对应的 agent ids。 */
  agentIds: string[];
}

export interface UpdatableSkill {
  state: CentralSkillUpdateState;
  repositoryId?: string | null;
  diagnostics?: SkillUpdateDiagnostic | null;
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
  diagnostics?: SkillUpdateDiagnostic | null;
}

export type UnsupportedSkillReasonCode =
  | "unknown_source"
  | "unsupported_source_type"
  | "missing_source_path"
  | "unsupported_source";

export interface UnsupportedSkill {
  skillId: string;
  reasonCode: UnsupportedSkillReasonCode;
}

export interface PlatformDuplicateGroup {
  agentId: string;
  skillId: string;
  skillName: string;
  writablePaths: string[];
  pluginPaths: string[];
}

export interface DeletedPlatformCopyGroup {
  agentId: string;
  skillId: string;
  skillName: string;
  writablePaths: string[];
}

export interface OrphanSkillEntry {
  skillId: string;
  brokenPath: string;
}

/**
 * What the Failed tab may offer on a row: `retryable` re-checks the repository,
 * `decision_required` re-checks it in incremental mode so the skill lands in the
 * removal bucket, `unknown` (inventories stored before this field) offers none.
 */
export type FailedRepositoryRetry =
  "retryable" | "decision_required" | "unknown";

export interface FailedRepository {
  repositoryId: string;
  error: string;
  /** Stable backend code for localizable reasons; absent on older inventories. */
  errorCode?: string | null;
  retry?: FailedRepositoryRetry;
  diagnostics?: SkillUpdateDiagnostic | null;
}

export interface SkillUpdateDiagnostic {
  sourceUrl?: string | null;
  ref?: string | null;
  sourcePath?: string | null;
  localHash?: string | null;
  baselineHash?: string | null;
  remoteHash?: string | null;
  localVersion?: string | null;
  remoteVersion?: string | null;
  cachePolicy: SkillRefreshCachePolicy;
  cacheHit: boolean;
  snapshotFetchedAt?: string | null;
}

export interface SkillUpdateInventory {
  updatable: UpdatableSkill[];
  remoteAdded: RemoteAddedSkill[];
  remoteMissing: RemoteMissingSkill[];
  /** Read-only classification for skills without a queryable remote source. */
  unsupported?: UnsupportedSkill[];
  platformDuplicates: PlatformDuplicateGroup[];
  deletedPlatformCopies: DeletedPlatformCopyGroup[];
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

export interface DeletedPlatformCopyRemoval {
  agentId: string;
  skillId: string;
  paths: string[];
}

export interface SkillUpdateDecisions {
  allowedAgentIds?: string[] | null;
  updates: string[];
  keepMissing: string[];
  deleteMissing: BatchDeleteCentralSkillRequest[];
  importAdditions: CentralRepositoryAddedSkillSelection[];
  skipAdditions: CentralRepositoryAdditionSkipRequest[];
  unskipAdditions: CentralRepositoryAdditionUnskipRequest[];
  removePlatformDuplicates: PlatformDuplicateRemoval[];
  removeDeletedPlatformCopies: DeletedPlatformCopyRemoval[];
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
  removedDeletedPlatformCopyPaths: string[];
  failures: SkillUpdateApplyFailure[];
}

export interface ForceSkillUpdateRequest {
  skillIds: string[];
  refreshCopyInstallations?: boolean;
}

export interface ForceSkillUpdateSuccess {
  skillId: string;
  repositoryId?: string | null;
  sourcePath?: string | null;
  localHash?: string | null;
  remoteHash?: string | null;
  bytesChanged: boolean;
  copyInstallationsRefreshed: boolean;
}

export interface ForceSkillUpdateSkip {
  skillId: string;
  reason: string;
}

export interface ForceSkillUpdateFailure {
  skillId: string;
  repositoryId?: string | null;
  sourcePath?: string | null;
  error: string;
}

export interface ForceSkillUpdateResult {
  overwritten: ForceSkillUpdateSuccess[];
  skipped: ForceSkillUpdateSkip[];
  failed: ForceSkillUpdateFailure[];
}

export interface ForceRepositoryMirrorRequest {
  repositoryIds: string[];
  deleteMissing: boolean;
  importAdded: boolean;
  overwriteTracked: boolean;
  removeCopyInstallationsForDeleted: boolean;
}

export interface ForceRepositoryMirrorResult {
  overwritten: ForceSkillUpdateSuccess[];
  imported: Array<{
    sourcePath: string;
    originalSkillId: string;
    importedSkillId: string;
    skillName: string;
    targetDirectory: string;
    resolution: "overwrite" | "skip" | "rename";
  }>;
  deleted: {
    succeeded: Array<{
      skill_id?: string;
      skillId?: string;
      removed_central_path?: string;
      removedCentralPath?: string;
      removed_agent_ids?: string[];
      removedAgentIds?: string[];
      retained_agent_ids?: string[];
      retainedAgentIds?: string[];
    }>;
    failed: Array<{ skill_id?: string; skillId?: string; error: string }>;
  };
  skipped: ForceSkillUpdateSkip[];
  failedRepositories: FailedRepository[];
  failedItems: ForceSkillUpdateFailure[];
}
