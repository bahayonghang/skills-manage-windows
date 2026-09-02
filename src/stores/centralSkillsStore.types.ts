import {
  AgentWithStatus,
  AiTagJob,
  BatchDeleteCentralSkillPreviewResult,
  BatchDeleteCentralSkillRequest,
  BatchDeleteCentralSkillResult,
  BatchInstallResult,
  CentralBatchInstallResult,
  CentralStoreLocationChangeResult,
  CentralStoreLocationPreview,
  CentralSkillUpdateJob,
  CentralSkillUpdateResult,
  CentralSkillUpdateState,
  DeleteCentralSkillPreview,
  DeleteSkillRepositoryPreview,
  DeleteSkillRepositoryResult,
  SkillportStateImportPreview,
  SkillportStateImportResolution,
  SkillportStateImportResult,
  SkillportStatePortabilityJob,
  SkillAiTagReview,
  SkillRepository,
  SkillRepositoryWithStats,
  SkillTag,
  SkillTagSuggestionResult,
  SkillWithLinks,
} from "@/types";
import type { ResetUnknownSourceSkillsPreview } from "@/types/resetUnknownSource";
import type {
  CentralRepositorySyncApplyResult,
  CentralRepositorySyncDecisions,
  CentralRepositorySyncPreview,
} from "@/types/centralRepositorySync";

export interface CentralSkillsState {
  skills: SkillWithLinks[];
  agents: AgentWithStatus[];
  repositories: SkillRepositoryWithStats[];
  tags: SkillTag[];
  aiTagReviews: SkillAiTagReview[];
  aiTagJob: AiTagJob;
  updateStatuses: Record<string, CentralSkillUpdateState>;
  updateJob: CentralSkillUpdateJob;
  portabilityJob: SkillportStatePortabilityJob;
  aiTaggingAvailable: boolean;
  isLoading: boolean;
  /**
   * True after at least one successful list load, including an empty array.
   * Array length must not be used as a substitute for this fact.
   */
  hasLoaded: boolean;
  /** 已有列表数据时的后台重取中（保留旧内容，不触发整页加载空态）。 */
  isRefreshingList: boolean;
  isInstalling: boolean;
  isDeleting: boolean;
  isMetadataUpdating: boolean;
  isSuggestingTags: boolean;
  isCheckingUpdates: boolean;
  updatingSkillIds: string[];
  /** Agent ID currently being toggled (null = idle). */
  togglingAgentId: string | null;
  /**
   * Mutation committed but a follow-up refresh failed. The last write is real;
   * the in-memory list may be stale until the next successful load/refresh.
   */
  requiresCentralReload: boolean;
  error: string | null;

  // Actions
  loadCentralSkills: (options?: { throwOnError?: boolean }) => Promise<void>;
  previewCentralStoreLocationChange: (
    targetPath: string,
  ) => Promise<CentralStoreLocationPreview>;
  applyCentralStoreLocationChange: (
    targetPath: string,
  ) => Promise<CentralStoreLocationChangeResult>;
  installSkill: (
    skillId: string,
    agentIds: string[],
    method: string,
    projectPath?: string | null,
  ) => Promise<BatchInstallResult>;
  batchInstallSkills: (
    skillIds: string[],
    agentIds: string[],
    method: string,
    projectPath?: string | null,
  ) => Promise<CentralBatchInstallResult>;
  loadDeletePreview: (skillId: string) => Promise<DeleteCentralSkillPreview>;
  loadBatchDeletePreview: (
    skillIds: string[],
  ) => Promise<BatchDeleteCentralSkillPreviewResult>;
  loadUnknownSourceResetPreview: () => Promise<ResetUnknownSourceSkillsPreview>;
  loadRepositoryDeletePreview: (
    repositoryId: string,
  ) => Promise<DeleteSkillRepositoryPreview>;
  deleteCentralSkill: (
    skillId: string,
    removeAgentIds: string[],
    force?: boolean,
  ) => Promise<void>;
  deleteCentralSkills: (
    requests: BatchDeleteCentralSkillRequest[],
  ) => Promise<BatchDeleteCentralSkillResult>;
  resetUnknownSourceSkills: (
    skillIds: string[],
    removeCopyAgentIds: string[],
  ) => Promise<BatchDeleteCentralSkillResult>;
  deleteSkillRepository: (
    repositoryId: string,
    requests: BatchDeleteCentralSkillRequest[],
  ) => Promise<DeleteSkillRepositoryResult>;
  togglePlatformLink: (skillId: string, agentId: string) => Promise<void>;
  createRepository: (name: string) => Promise<SkillRepository>;
  assignSkillsToRepository: (
    skillIds: string[],
    repositoryId: string,
  ) => Promise<void>;
  setRepositoryPinned: (repositoryId: string, pinned: boolean) => Promise<void>;
  createTag: (name: string) => Promise<SkillTag>;
  assignSkillTags: (skillIds: string[], tagIds: string[]) => Promise<void>;
  /** 解除单个 skill 的若干 tag 关联（卡上删标签用）。 */
  unassignSkillTags: (skillId: string, tagIds: string[]) => Promise<void>;
  bulkSuggestSkillTags: (
    skillIds: string[],
  ) => Promise<SkillTagSuggestionResult[]>;
  checkSkillUpdates: (
    skillIds?: string[],
  ) => Promise<CentralSkillUpdateState[]>;
  checkRepositorySync: (
    repositoryIds: string[],
    skillIds?: string[],
  ) => Promise<CentralRepositorySyncPreview>;
  applyRepositorySync: (
    decisions: CentralRepositorySyncDecisions,
  ) => Promise<CentralRepositorySyncApplyResult>;
  updateSkills: (skillIds: string[]) => Promise<CentralSkillUpdateResult>;
  cancelCentralUpdates: () => Promise<void>;
  keepRemoteMissingSkills: (skillIds: string[]) => Promise<string[]>;
  cancelAiTagJob: () => Promise<void>;
  loadAiTagReviews: () => Promise<void>;
  acceptAiTagReview: (skillId: string, tagIds: string[]) => Promise<void>;
  skipAiTagReview: (skillId: string) => Promise<void>;
  subscribeAiTagProgress: () => Promise<() => void>;
  subscribeUpdateProgress: () => Promise<() => void>;
  subscribePortabilityProgress: () => Promise<() => void>;
  cancelSkillportStatePortability: () => Promise<void>;
  exportSkillportState: () => Promise<string>;
  saveSkillportStateExport: (path: string, json: string) => Promise<void>;
  previewSkillportStateImport: (
    json: string,
  ) => Promise<SkillportStateImportPreview>;
  previewSkillportStateImportFile: (
    path: string,
  ) => Promise<{ json: string; preview: SkillportStateImportPreview }>;
  importSkillportState: (
    json: string,
    resolutions: SkillportStateImportResolution[],
  ) => Promise<SkillportStateImportResult>;
  resetForTargetChange: () => void;
}

export type CentralStoreSet = (
  partial:
    | Partial<CentralSkillsState>
    | ((state: CentralSkillsState) => Partial<CentralSkillsState>),
) => void;

export type CentralStoreGet = () => CentralSkillsState;

export interface CentralStoreContext {
  set: CentralStoreSet;
  get: CentralStoreGet;
  getGeneration: () => number;
  bumpGeneration: () => void;
}
