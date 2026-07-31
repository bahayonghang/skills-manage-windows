import type { GitHubImportState } from "./marketplaceStore.types";

export const createInitialGitHubImportState = (): GitHubImportState => ({
  isPreviewLoading: false,
  isImporting: false,
  preview: null,
  importResult: null,
  previewedRepoUrl: null,
  previewedBranch: null,
  error: null,
  importProgress: null,
  importStartedAt: null,
  skillMarkdown: {},
  aiSummaries: {},
});

export function createMarketplaceBaseState() {
  return {
    registries: [],
    skills: [],
    selectedRegistryId: null,
    searchQuery: "",
    isLoading: false,
    isSyncing: false,
    installingIds: new Set<string>(),
    error: null,
    githubImport: createInitialGitHubImportState(),
    skillsShResults: [],
    skillsShQuery: "",
    isSkillsShLoading: false,
    skillsShError: null,
  };
}

export function normalizeRegistryIdentity(url: string): string | null {
  const trimmed = url.trim();
  if (!trimmed) return null;

  const githubMatch = trimmed.match(
    /^(?:https?:\/\/)?(?:www\.)?github\.com\/([^/\s]+)\/([^/\s#?]+?)(?:\.git)?(?:\/)?$/i
  );
  if (githubMatch) {
    return `github:${githubMatch[1].toLowerCase()}/${githubMatch[2].toLowerCase()}`;
  }

  try {
    const parsed = new URL(trimmed.startsWith("http") ? trimmed : `https://${trimmed}`);
    const pathname = parsed.pathname.replace(/\/+$/, "");
    return `${parsed.hostname.toLowerCase()}${pathname.toLowerCase()}`;
  } catch {
    return trimmed.toLowerCase();
  }
}
