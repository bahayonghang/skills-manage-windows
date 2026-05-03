import {
  AgentWithStatus,
  AiTagJob,
  BatchDeleteCentralSkillPreviewResult,
  BatchDeleteCentralSkillRequest,
  BatchDeleteCentralSkillResult,
  BatchInstallResult,
  CentralBatchInstallResult,
  CentralSkillUpdateJob,
  CentralSkillUpdateResult,
  CentralSkillUpdateState,
  DeleteSkillRepositoryPreview,
  DeleteSkillRepositoryResult,
  SkillportStateImportPreview,
  SkillportStateImportResolution,
  SkillportStateImportResult,
  SkillDetail,
  SkillAiTagReview,
  SkillRepository,
  SkillRepositoryWithStats,
  SkillTag,
  SkillTagSuggestionResult,
  SkillWithLinks,
} from "@/types";

export interface CentralSkillsState {
  skills: SkillWithLinks[];
  agents: AgentWithStatus[];
  repositories: SkillRepositoryWithStats[];
  tags: SkillTag[];
  aiTagReviews: SkillAiTagReview[];
  aiTagJob: AiTagJob;
  updateStatuses: Record<string, CentralSkillUpdateState>;
  updateJob: CentralSkillUpdateJob;
  aiTaggingAvailable: boolean;
  isLoading: boolean;
  isInstalling: boolean;
  isDeleting: boolean;
  isMetadataUpdating: boolean;
  isSuggestingTags: boolean;
  isCheckingUpdates: boolean;
  updatingSkillIds: string[];
  /** Agent ID currently being toggled (null = idle). */
  togglingAgentId: string | null;
  error: string | null;

  // Actions
  loadCentralSkills: () => Promise<void>;
  installSkill: (
    skillId: string,
    agentIds: string[],
    method: string,
    projectPath?: string | null
  ) => Promise<BatchInstallResult>;
  batchInstallSkills: (
    skillIds: string[],
    agentIds: string[],
    method: string,
    projectPath?: string | null
  ) => Promise<CentralBatchInstallResult>;
  loadDeletePreview: (skillId: string) => Promise<SkillDetail>;
  loadBatchDeletePreview: (skillIds: string[]) => Promise<BatchDeleteCentralSkillPreviewResult>;
  loadRepositoryDeletePreview: (repositoryId: string) => Promise<DeleteSkillRepositoryPreview>;
  deleteCentralSkill: (skillId: string, removeAgentIds: string[]) => Promise<void>;
  deleteCentralSkills: (
    requests: BatchDeleteCentralSkillRequest[]
  ) => Promise<BatchDeleteCentralSkillResult>;
  deleteSkillRepository: (
    repositoryId: string,
    requests: BatchDeleteCentralSkillRequest[]
  ) => Promise<DeleteSkillRepositoryResult>;
  togglePlatformLink: (skillId: string, agentId: string) => Promise<void>;
  createRepository: (name: string) => Promise<SkillRepository>;
  assignSkillsToRepository: (skillIds: string[], repositoryId: string) => Promise<void>;
  createTag: (name: string) => Promise<SkillTag>;
  assignSkillTags: (skillIds: string[], tagIds: string[]) => Promise<void>;
  bulkSuggestSkillTags: (skillIds: string[]) => Promise<SkillTagSuggestionResult[]>;
  checkSkillUpdates: (skillIds?: string[]) => Promise<CentralSkillUpdateState[]>;
  updateSkills: (skillIds: string[]) => Promise<CentralSkillUpdateResult>;
  keepRemoteMissingSkills: (skillIds: string[]) => Promise<string[]>;
  cancelAiTagJob: () => Promise<void>;
  loadAiTagReviews: () => Promise<void>;
  acceptAiTagReview: (skillId: string, tagIds: string[]) => Promise<void>;
  skipAiTagReview: (skillId: string) => Promise<void>;
  subscribeAiTagProgress: () => Promise<() => void>;
  subscribeUpdateProgress: () => Promise<() => void>;
  exportSkillportState: () => Promise<string>;
  previewSkillportStateImport: (json: string) => Promise<SkillportStateImportPreview>;
  importSkillportState: (
    json: string,
    resolutions: SkillportStateImportResolution[]
  ) => Promise<SkillportStateImportResult>;
  resetForTargetChange: () => void;
}


export type CentralStoreSet = (
  partial:
    | Partial<CentralSkillsState>
    | ((state: CentralSkillsState) => Partial<CentralSkillsState>)
) => void;

export type CentralStoreGet = () => CentralSkillsState;

export interface CentralStoreContext {
  set: CentralStoreSet;
  get: CentralStoreGet;
  getGeneration: () => number;
  bumpGeneration: () => void;
}
