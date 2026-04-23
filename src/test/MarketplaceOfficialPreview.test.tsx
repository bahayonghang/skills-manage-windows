import { beforeEach, describe, expect, it, vi } from "vitest";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import type { GitHubRepoPreview, MarketplaceSkill, SkillRegistry } from "@/types";
import { MarketplaceView } from "@/pages/MarketplaceView";

const mockInvoke = vi.fn();
const mockIsTauriRuntime = vi.fn(() => true);

vi.mock("@/lib/tauri", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
  isTauriRuntime: () => mockIsTauriRuntime(),
}));

vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

vi.mock("@/components/skill/UnifiedSkillCard", () => ({
  UnifiedSkillCard: () => null,
}));

vi.mock("@/components/marketplace/MarketplaceSkillDetailDrawer", () => ({
  MarketplaceSkillDetailDrawer: () => null,
}));

vi.mock("@/components/marketplace/GitHubRepoImportWizard", () => ({
  GitHubRepoImportWizard: () => null,
}));

function normalizeRegistryIdentity(url: string): string | null {
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

type StoreState = {
  registries: SkillRegistry[];
  installingIds: Set<string>;
  githubImport: {
    isPreviewLoading: boolean;
    isImporting: boolean;
    preview: GitHubRepoPreview | null;
    importResult: unknown | null;
    previewedRepoUrl: string | null;
    error: string | null;
  };
};

const storeState: StoreState = {
  registries: [],
  installingIds: new Set<string>(),
  githubImport: {
    isPreviewLoading: false,
    isImporting: false,
    preview: null,
    importResult: null,
    previewedRepoUrl: null,
    error: null,
  },
};

const mockLoadRegistries = vi.fn();
const mockLoadPreviewSkills = vi.fn<() => Promise<MarketplaceSkill[]>>();
const mockInstallSkill = vi.fn();
const mockPreviewGitHubRepoImport = vi.fn();
const mockImportGitHubRepoSkills = vi.fn();
const mockResetGitHubImport = vi.fn();
const mockRescan = vi.fn();
const mockLoadCentralSkills = vi.fn();
const mockInstallCentralSkill = vi.fn();
const mockGetSkillsByAgent = vi.fn();

vi.mock("@/stores/marketplaceStore", () => ({
  useMarketplaceStore: (
    selector: (state: Record<string, unknown>) => unknown
  ) =>
    selector({
      ...storeState,
      loadRegistries: mockLoadRegistries,
      loadPreviewSkills: mockLoadPreviewSkills,
      installSkill: mockInstallSkill,
      getNormalizedRegistryIdentity: normalizeRegistryIdentity,
      previewGitHubRepoImport: mockPreviewGitHubRepoImport,
      importGitHubRepoSkills: mockImportGitHubRepoSkills,
      resetGitHubImport: mockResetGitHubImport,
    }),
}));

vi.mock("@/stores/platformStore", () => ({
  usePlatformStore: (
    selector: (state: Record<string, unknown>) => unknown
  ) =>
    selector({
      rescan: mockRescan,
      agents: [],
    }),
}));

vi.mock("@/stores/centralSkillsStore", () => ({
  useCentralSkillsStore: (
    selector: (state: Record<string, unknown>) => unknown
  ) =>
    selector({
      skills: [],
      agents: [],
      loadCentralSkills: mockLoadCentralSkills,
      installSkill: mockInstallCentralSkill,
    }),
}));

vi.mock("@/stores/skillStore", () => ({
  useSkillStore: (
    selector: (state: Record<string, unknown>) => unknown
  ) =>
    selector({
      skillsByAgent: {},
      getSkillsByAgent: mockGetSkillsByAgent,
    }),
}));

function makeRegistry(overrides: Partial<SkillRegistry>): SkillRegistry {
  return {
    id: "registry-1",
    name: "Official Repo Cache",
    source_type: "github",
    url: "https://github.com/anthropics/knowledge-work-plugins",
    normalized_url: null,
    is_builtin: false,
    is_enabled: true,
    last_synced: "2026-04-21T00:00:00Z",
    last_attempted_sync: "2026-04-21T00:00:00Z",
    last_sync_status: "success",
    last_sync_error: null,
    cache_updated_at: "2026-04-21T00:00:00Z",
    cache_expires_at: "2026-04-22T00:00:00Z",
    etag: null,
    last_modified: null,
    created_at: "2026-04-20T00:00:00Z",
    ...overrides,
  };
}

function renderView() {
  return render(
    <MemoryRouter>
      <MarketplaceView />
    </MemoryRouter>
  );
}

async function openAnthropicRepo(repoPattern = /anthropics\/knowledge-work-plugins/i) {
  await act(async () => {
    fireEvent.click(screen.getByRole("button", { name: /Official Directory|官方源目录/i }));
  });
  const publisherButton = await screen.findByRole("button", { name: /Anthropic/i });
  await act(async () => {
    fireEvent.click(publisherButton);
  });
  const repoButton = await screen.findByRole("button", { name: repoPattern });
  await act(async () => {
    fireEvent.click(repoButton);
  });
}

async function toggleRepoPreview(repoPattern = /anthropics\/knowledge-work-plugins/i) {
  const repoButton = await screen.findByRole("button", { name: repoPattern });
  await act(async () => {
    fireEvent.click(repoButton);
  });
}

describe("Marketplace official preview", () => {
  beforeEach(() => {
    storeState.registries = [];
    storeState.installingIds = new Set<string>();
    storeState.githubImport = {
      isPreviewLoading: false,
      isImporting: false,
      preview: null,
      importResult: null,
      previewedRepoUrl: null,
      error: null,
    };

    mockInvoke.mockReset();
    mockIsTauriRuntime.mockReset();
    mockIsTauriRuntime.mockReturnValue(true);
    mockLoadRegistries.mockReset();
    mockLoadPreviewSkills.mockReset();
    mockInstallSkill.mockReset();
    mockPreviewGitHubRepoImport.mockReset();
    mockImportGitHubRepoSkills.mockReset();
    mockResetGitHubImport.mockReset();
    mockRescan.mockReset();
    mockLoadCentralSkills.mockReset();
    mockInstallCentralSkill.mockReset();
    mockGetSkillsByAgent.mockReset();
  });

  it("falls back to direct GitHub preview when the official repo is not registered locally", async () => {
    mockInvoke.mockResolvedValue({
      repo: {
        owner: "anthropics",
        repo: "knowledge-work-plugins",
        branch: "main",
        normalizedUrl: "https://github.com/anthropics/knowledge-work-plugins",
      },
      skills: [
        {
          sourcePath: "skills/research/SKILL.md",
          skillId: "research",
          skillName: "Research",
          description: "Direct GitHub preview skill",
          rootDirectory: "skills",
          skillDirectoryName: "research",
          downloadUrl: "https://example.com/research",
          conflict: null,
        },
      ],
    } satisfies GitHubRepoPreview);

    renderView();
    await openAnthropicRepo();

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("preview_github_repo_import", {
        repoUrl: "https://github.com/anthropics/knowledge-work-plugins",
      });
    });
    expect(mockLoadPreviewSkills).not.toHaveBeenCalled();
    expect(await screen.findByText("Research")).toBeInTheDocument();
    expect(screen.getByText("Direct GitHub preview skill")).toBeInTheDocument();
  });

  it("reuses the successful repo preview cache when the same repo is expanded again", async () => {
    mockInvoke.mockResolvedValue({
      repo: {
        owner: "anthropics",
        repo: "knowledge-work-plugins",
        branch: "main",
        normalizedUrl: "https://github.com/anthropics/knowledge-work-plugins",
      },
      skills: [
        {
          sourcePath: "skills/research/SKILL.md",
          skillId: "research",
          skillName: "Research",
          description: "Direct GitHub preview skill",
          rootDirectory: "skills",
          skillDirectoryName: "research",
          downloadUrl: "https://example.com/research",
          conflict: null,
        },
      ],
    } satisfies GitHubRepoPreview);

    renderView();
    await openAnthropicRepo();

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledTimes(1);
    });
    expect(await screen.findByText("Research")).toBeInTheDocument();

    await toggleRepoPreview();
    await waitFor(() => {
      expect(screen.queryByText("Research")).not.toBeInTheDocument();
    });

    await toggleRepoPreview();
    expect(await screen.findByText("Research")).toBeInTheDocument();
    expect(mockInvoke).toHaveBeenCalledTimes(1);
  });

  it("bypasses the preview cache when the user explicitly refreshes", async () => {
    mockInvoke.mockResolvedValue({
      repo: {
        owner: "anthropics",
        repo: "knowledge-work-plugins",
        branch: "main",
        normalizedUrl: "https://github.com/anthropics/knowledge-work-plugins",
      },
      skills: [
        {
          sourcePath: "skills/research/SKILL.md",
          skillId: "research",
          skillName: "Research",
          description: "Direct GitHub preview skill",
          rootDirectory: "skills",
          skillDirectoryName: "research",
          downloadUrl: "https://example.com/research",
          conflict: null,
        },
      ],
    } satisfies GitHubRepoPreview);

    renderView();
    await openAnthropicRepo();

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledTimes(1);
    });

    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /刷新预览|Refresh preview/i }));
    });

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledTimes(2);
    });
  });

  it("reuses cached preview when the registry URL matches after normalization", async () => {
    storeState.registries = [
      makeRegistry({
        id: "anthropic-cache",
        url: "https://github.com/anthropics/knowledge-work-plugins.git/",
      }),
    ];
    mockLoadPreviewSkills.mockResolvedValue([
      {
        id: "cached-skill-1",
        registry_id: "anthropic-cache",
        name: "Cached Preview Skill",
        description: "Loaded from local cache",
        download_url: "https://example.com/cached-skill-1",
        is_installed: false,
        synced_at: "2026-04-21T00:00:00Z",
        cache_updated_at: "2026-04-21T00:00:00Z",
      },
    ] satisfies MarketplaceSkill[]);

    renderView();
    await openAnthropicRepo();

    await waitFor(() => {
      expect(mockLoadPreviewSkills).toHaveBeenCalledWith("anthropic-cache");
    });
    expect(mockInvoke).not.toHaveBeenCalled();
    expect(await screen.findByText("Cached Preview Skill")).toBeInTheDocument();
    expect(screen.getByText("Loaded from local cache")).toBeInTheDocument();
  });
});
