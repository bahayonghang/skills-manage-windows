import type {
  BatchDeleteCentralSkillRequest,
  BatchDeleteCentralSkillResult,
  CentralSkillUpdateState,
  GitHubRepoImportResult,
  GitHubRepoRef,
  GitHubSkillImportSelection,
  GitHubSkillPreview,
} from "@/types";

export interface CentralRemoteAddedSkill {
  repositoryId: string;
  repo: GitHubRepoRef;
  preview: GitHubSkillPreview;
}

export interface CentralRemoteMissingSkill {
  state: CentralSkillUpdateState;
  repositoryId?: string | null;
  repositoryName: string;
  repo?: GitHubRepoRef | null;
}

export interface CentralRepositorySyncSummary {
  repositoryId: string;
  name: string;
  checked: number;
  updateAvailable: number;
  remoteMissing: number;
  unsupported: number;
  failed: number;
  remoteAdded: number;
  skippedRemoteAdded: number;
}

export interface CentralRepositorySyncFailure {
  repositoryId: string;
  name?: string | null;
  error: string;
}

export interface CentralRepositorySyncPreview {
  states: CentralSkillUpdateState[];
  remoteAdded: CentralRemoteAddedSkill[];
  skippedRemoteAdded: CentralRemoteAddedSkill[];
  remoteMissing: CentralRemoteMissingSkill[];
  repositories: CentralRepositorySyncSummary[];
  failedRepositories: CentralRepositorySyncFailure[];
}

/**
 * Central repository sync confirms additions from its own verified inventory,
 * not from a wizard preview snapshot, so it carries no `previewId`.
 */
export interface CentralRepositoryAddedSkillSelection {
  repositoryId: string;
  selections: GitHubSkillImportSelection[];
}

export interface CentralRepositoryAdditionSkipRequest {
  repositoryId: string;
  sourcePath: string;
  skillId: string;
  skillName: string;
}

export interface CentralRepositoryAdditionUnskipRequest {
  repositoryId: string;
  sourcePath: string;
}

export interface CentralRepositorySyncDecisions {
  keepSkillIds: string[];
  deleteRequests: BatchDeleteCentralSkillRequest[];
  additions: CentralRepositoryAddedSkillSelection[];
  skipAdditions: CentralRepositoryAdditionSkipRequest[];
  unskipAdditions: CentralRepositoryAdditionUnskipRequest[];
}

export interface CentralRepositorySyncApplyResult {
  keptSkillIds: string[];
  deleteResult: BatchDeleteCentralSkillResult;
  importResults: GitHubRepoImportResult[];
  skippedAdditions: CentralRepositoryAdditionSkipRequest[];
  unskippedAdditions: CentralRepositoryAdditionUnskipRequest[];
  failedRepositories: CentralRepositorySyncFailure[];
  states: CentralSkillUpdateState[];
}
