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

export interface CentralRepositorySyncSummary {
  repositoryId: string;
  name: string;
  checked: number;
  updateAvailable: number;
  remoteMissing: number;
  unsupported: number;
  failed: number;
  remoteAdded: number;
}

export interface CentralRepositorySyncFailure {
  repositoryId: string;
  name?: string | null;
  error: string;
}

export interface CentralRepositorySyncPreview {
  states: CentralSkillUpdateState[];
  remoteAdded: CentralRemoteAddedSkill[];
  remoteMissing: CentralSkillUpdateState[];
  repositories: CentralRepositorySyncSummary[];
  failedRepositories: CentralRepositorySyncFailure[];
}

export interface CentralRepositoryAddedSkillSelection {
  repositoryId: string;
  selections: GitHubSkillImportSelection[];
  previewWorkspaceId?: string | null;
}

export interface CentralRepositorySyncDecisions {
  keepSkillIds: string[];
  deleteRequests: BatchDeleteCentralSkillRequest[];
  additions: CentralRepositoryAddedSkillSelection[];
}

export interface CentralRepositorySyncApplyResult {
  keptSkillIds: string[];
  deleteResult: BatchDeleteCentralSkillResult;
  importResults: GitHubRepoImportResult[];
  failedRepositories: CentralRepositorySyncFailure[];
  states: CentralSkillUpdateState[];
}
