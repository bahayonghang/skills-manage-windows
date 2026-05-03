import {
  GitHubImportProgressPayload,
  GitHubRepoImportResult,
  GitHubRepoPreview,
  GitHubSkillImportSelection,
  MarketplaceSkill,
  SkillRegistry,
} from "@/types";

export interface GitHubImportState {
  isPreviewLoading: boolean;
  isImporting: boolean;
  preview: GitHubRepoPreview | null;
  importResult: GitHubRepoImportResult | null;
  previewedRepoUrl: string | null;
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

  loadRegistries: () => Promise<void>;
  selectRegistry: (id: string) => void;
  setSearchQuery: (query: string) => void;
  syncRegistry: (registryId: string, forceRefresh?: boolean) => Promise<void>;
  loadSkills: (registryId: string, query?: string) => Promise<void>;
  loadPreviewSkills: (registryId: string) => Promise<MarketplaceSkill[]>;
  installSkill: (skillId: string) => Promise<void>;
  /** Re-run AI explanation for a marketplace preview. Used by preview dialogs/drawers. */
  triggerSkillExplanation: (skillId: string, content: string, lang: string) => Promise<void>;
  addRegistry: (name: string, sourceType: string, url: string) => Promise<SkillRegistry>;
  removeRegistry: (registryId: string) => Promise<void>;
  getNormalizedRegistryIdentity: (url: string) => string | null;
  findDuplicateRegistry: (url: string) => SkillRegistry | null;
  previewGitHubRepoSkills: (repoUrl: string) => Promise<GitHubRepoPreview>;
  previewGitHubRepoImport: (repoUrl: string) => Promise<GitHubRepoPreview>;
  importGitHubRepoSkills: (
    repoUrl: string,
    selections: GitHubSkillImportSelection[],
    previewWorkspaceId?: string | null,
  ) => Promise<GitHubRepoImportResult>;
  fetchGitHubSkillMarkdown: (sourcePath: string, downloadUrl: string) => Promise<void>;
  generateGitHubImportAiSummary: (
    sourcePath: string,
    skillName: string,
    content: string,
    lang: string,
    refresh?: boolean
  ) => Promise<void>;
  resetGitHubImport: () => void;
  resetForTargetChange: () => void;
}


export type MarketplaceStoreSet = (
  partial:
    | Partial<MarketplaceState>
    | ((state: MarketplaceState) => Partial<MarketplaceState>)
) => void;

export type MarketplaceStoreGet = () => MarketplaceState;

export interface MarketplaceStoreContext {
  set: MarketplaceStoreSet;
  get: MarketplaceStoreGet;
  getGeneration: () => number;
  bumpGeneration: () => void;
}
