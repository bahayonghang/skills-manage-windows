import {
  GitHubImportProgressPayload,
  GitHubRepoImportResult,
  GitHubRepoPreview,
  GitHubRepoRef,
  GitHubSkillImportSelection,
  MarketplaceSkill,
  SkillRegistry,
  SkillsShFileEntry,
  SkillsShSkill,
} from "@/types";

export interface GitHubImportState {
  isPreviewLoading: boolean;
  isImporting: boolean;
  preview: GitHubRepoPreview | null;
  importResult: GitHubRepoImportResult | null;
  previewedRepoUrl: string | null;
  previewedBranch: string | null;
  error: string | null;
  importProgress: GitHubImportProgressPayload | null;
  importStartedAt: number | null;
  skillMarkdown: Record<string, SkillMarkdownEntry>;
  aiSummaries: Record<string, GitHubImportAiSummaryEntry>;
}

export interface SkillMarkdownEntry {
  status: "loading" | "ready" | "error";
  content?: string;
  error?: string;
}

export interface GitHubImportAiSummaryEntry {
  summary: string | null;
  isLoading: boolean;
  isStreaming: boolean;
  error: string | null;
}

export interface MarketplaceState {
  registries: SkillRegistry[];
  skills: MarketplaceSkill[];
  selectedRegistryId: string | null;
  searchQuery: string;
  isLoading: boolean;
  isSyncing: boolean;
  installingIds: Set<string>;
  error: string | null;
  githubImport: GitHubImportState;
  skillsShResults: SkillsShSkill[];
  skillsShQuery: string;
  isSkillsShLoading: boolean;
  skillsShError: string | null;

  loadRegistries: () => Promise<void>;
  selectRegistry: (id: string) => void;
  setSearchQuery: (query: string) => void;
  syncRegistry: (registryId: string, forceRefresh?: boolean) => Promise<void>;
  loadSkills: (registryId: string, query?: string) => Promise<void>;
  loadPreviewSkills: (registryId: string) => Promise<MarketplaceSkill[]>;
  installSkill: (skillId: string) => Promise<void>;
  /** Re-run AI explanation for a marketplace preview. Used by preview dialogs/drawers. */
  triggerSkillExplanation: (
    skillId: string,
    content: string,
    lang: string,
  ) => Promise<void>;
  addRegistry: (
    name: string,
    sourceType: string,
    url: string,
  ) => Promise<SkillRegistry>;
  removeRegistry: (registryId: string) => Promise<void>;
  getNormalizedRegistryIdentity: (url: string) => string | null;
  findDuplicateRegistry: (url: string) => SkillRegistry | null;
  searchSkillsSh: (query: string) => Promise<SkillsShSkill[]>;
  resolveSkillsShUrl: (source: string, skillId: string) => Promise<string>;
  browseSkillsShDirectory: (
    source: string,
    skillId: string,
  ) => Promise<SkillsShFileEntry[]>;
  readSkillsShFile: (source: string, filePath: string) => Promise<string>;
  installFromSkillsSh: (source: string, skillId: string) => Promise<string>;
  previewGitHubRepoSkills: (
    repoUrl: string,
    branch?: string | null,
  ) => Promise<GitHubRepoPreview>;
  previewGitHubRepoImport: (
    repoUrl: string,
    branch?: string | null,
  ) => Promise<GitHubRepoPreview>;
  /**
   * Import the skills confirmed in a preview snapshot. `previewId` defaults to
   * the snapshot currently held in the store; without one the action rejects
   * instead of letting the backend re-resolve the branch.
   */
  importGitHubRepoSkills: (
    repoUrl: string,
    selections: GitHubSkillImportSelection[],
    previewId?: string | null,
  ) => Promise<GitHubRepoImportResult>;
  installGitHubPreviewSkill: (
    repoUrl: string,
    sourcePath: string,
  ) => Promise<GitHubRepoImportResult>;
  fetchGitHubSkillMarkdown: (
    repo: GitHubRepoRef,
    sourcePath: string,
  ) => Promise<void>;
  generateGitHubImportAiSummary: (
    sourcePath: string,
    skillName: string,
    content: string,
    lang: string,
    refresh?: boolean,
  ) => Promise<void>;
  resetGitHubImport: () => void;
  resetForTargetChange: () => void;
}

export type MarketplaceStoreSet = (
  partial:
    | Partial<MarketplaceState>
    | ((state: MarketplaceState) => Partial<MarketplaceState>),
) => void;

export type MarketplaceStoreGet = () => MarketplaceState;

export interface MarketplaceStoreContext {
  set: MarketplaceStoreSet;
  get: MarketplaceStoreGet;
  getGeneration: () => number;
  bumpGeneration: () => void;
}
