import { beforeEach, describe, expect, it, vi, type Mock } from "vitest";

vi.mock("@/lib/ipc", () => ({
  invoke: vi.fn(),
  listen: vi.fn(),
  isTauriRuntime: vi.fn(() => true),
}));

import { invoke, listen, isTauriRuntime } from "@/lib/ipc";

import { useMarketplaceStore } from "@/stores/marketplaceStore";

const mockInvoke = vi.mocked(invoke) as unknown as Mock;
const mockListen = vi.mocked(listen);
const mockIsTauriRuntime = vi.mocked(isTauriRuntime);

describe("marketplaceStore", () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    mockListen.mockReset();
    mockIsTauriRuntime.mockReset();
    mockIsTauriRuntime.mockReturnValue(true);
    mockListen.mockResolvedValue(vi.fn());
    useMarketplaceStore.setState({
      registries: [],
      skills: [],
      selectedRegistryId: null,
      searchQuery: "",
      isLoading: false,
      isSyncing: false,
      installingIds: new Set(),
      error: null,
      githubImport: {
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
      },
      skillsShResults: [],
      skillsShQuery: "",
      isSkillsShLoading: false,
      skillsShError: null,
    });
  });

  it("uses cached sync by default and refreshes registry metadata", async () => {
    const skills = [
      {
        id: "skill-1",
        registry_id: "reg-1",
        name: "Skill One",
        description: "cached",
        download_url: "https://example.com/skill-1",
        is_installed: false,
        synced_at: "2026-04-16T00:00:00Z",
        cache_updated_at: "2026-04-16T00:00:00Z",
      },
    ];
    const registries = [
      {
        id: "reg-1",
        name: "Repo",
        source_type: "github",
        url: "https://github.com/acme/repo",
        is_builtin: false,
        is_enabled: true,
        last_synced: "2026-04-16T00:00:00Z",
        last_attempted_sync: "2026-04-16T00:00:00Z",
        last_sync_status: "success",
        last_sync_error: null,
        cache_updated_at: "2026-04-16T00:00:00Z",
        cache_expires_at: null,
        etag: null,
        last_modified: null,
        created_at: "2026-04-15T00:00:00Z",
      },
    ];

    mockInvoke.mockResolvedValueOnce(skills).mockResolvedValueOnce(registries);

    await useMarketplaceStore.getState().syncRegistry("reg-1");

    expect(mockInvoke).toHaveBeenNthCalledWith(1, "sync_registry", {
      registryId: "reg-1",
    });
    expect(mockInvoke).toHaveBeenNthCalledWith(2, "list_registries");
    expect(useMarketplaceStore.getState().skills).toEqual(skills);
    expect(useMarketplaceStore.getState().registries).toEqual(registries);
    expect(useMarketplaceStore.getState().isSyncing).toBe(false);
  });

  it("force refreshes via sync_registry_with_options and preserves refreshed metadata", async () => {
    const freshSkills = [
      {
        id: "skill-1",
        registry_id: "reg-1",
        name: "Skill One Fresh",
        description: "fresh",
        download_url: "https://example.com/skill-1",
        is_installed: false,
        synced_at: "2026-04-16T01:00:00Z",
        cache_updated_at: "2026-04-16T01:00:00Z",
      },
    ];
    const refreshedRegistries = [
      {
        id: "reg-1",
        name: "Repo",
        source_type: "github",
        url: "https://github.com/acme/repo",
        is_builtin: false,
        is_enabled: true,
        last_synced: "2026-04-16T01:00:00Z",
        last_attempted_sync: "2026-04-16T01:00:00Z",
        last_sync_status: "success",
        last_sync_error: null,
        cache_updated_at: "2026-04-16T01:00:00Z",
        cache_expires_at: null,
        etag: '"etag"',
        last_modified: null,
        created_at: "2026-04-15T00:00:00Z",
      },
    ];

    mockInvoke
      .mockResolvedValueOnce(freshSkills)
      .mockResolvedValueOnce(refreshedRegistries);

    await useMarketplaceStore.getState().syncRegistry("reg-1", true);

    expect(mockInvoke).toHaveBeenNthCalledWith(
      1,
      "sync_registry_with_options",
      {
        registryId: "reg-1",
        options: { forceRefresh: true },
      },
    );
    expect(mockInvoke).toHaveBeenNthCalledWith(2, "list_registries");
    expect(useMarketplaceStore.getState().skills).toEqual(freshSkills);
    expect(useMarketplaceStore.getState().registries[0]?.last_synced).toBe(
      "2026-04-16T01:00:00Z",
    );
  });

  it("keeps last successful cached skills visible when force refresh fails", async () => {
    useMarketplaceStore.setState({
      skills: [
        {
          id: "skill-1",
          registry_id: "reg-1",
          name: "Cached Skill",
          description: "cached",
          download_url: "https://example.com/skill-1",
          is_installed: false,
          synced_at: "2026-04-16T00:00:00Z",
          cache_updated_at: "2026-04-16T00:00:00Z",
        },
      ],
      registries: [
        {
          id: "reg-1",
          name: "Repo",
          source_type: "github",
          url: "https://github.com/acme/repo",
          is_builtin: false,
          is_enabled: true,
          last_synced: "2026-04-16T00:00:00Z",
          last_attempted_sync: "2026-04-16T00:00:00Z",
          last_sync_status: "success",
          last_sync_error: null,
          cache_updated_at: "2026-04-16T00:00:00Z",
          cache_expires_at: null,
          etag: null,
          last_modified: null,
          created_at: "2026-04-15T00:00:00Z",
        },
      ],
    });

    mockInvoke
      .mockRejectedValueOnce(new Error("network down"))
      .mockResolvedValueOnce([
        {
          id: "reg-1",
          name: "Repo",
          source_type: "github",
          url: "https://github.com/acme/repo",
          is_builtin: false,
          is_enabled: true,
          last_synced: "2026-04-16T00:00:00Z",
          last_attempted_sync: "2026-04-16T02:00:00Z",
          last_sync_status: "error",
          last_sync_error: "network down",
          cache_updated_at: "2026-04-16T00:00:00Z",
          cache_expires_at: null,
          etag: null,
          last_modified: null,
          created_at: "2026-04-15T00:00:00Z",
        },
      ]);

    await expect(
      useMarketplaceStore.getState().syncRegistry("reg-1", true),
    ).rejects.toThrow("network down");

    expect(useMarketplaceStore.getState().skills[0]?.name).toBe("Cached Skill");
    expect(useMarketplaceStore.getState().registries[0]?.last_sync_status).toBe(
      "error",
    );
    expect(useMarketplaceStore.getState().error).toContain("network down");
    expect(useMarketplaceStore.getState().isSyncing).toBe(false);
  });

  it("normalizes GitHub identities when checking duplicate registries", () => {
    useMarketplaceStore.setState({
      registries: [
        {
          id: "official-1",
          name: "Official Repo",
          source_type: "github",
          url: "https://github.com/Anthropics/Skills",
          normalized_url: "github:anthropics/skills",
          is_builtin: true,
          is_enabled: true,
          last_synced: null,
          created_at: "2026-04-15T00:00:00Z",
        },
      ],
    });

    const duplicate = useMarketplaceStore
      .getState()
      .findDuplicateRegistry("https://github.com/anthropics/skills.git/");

    expect(duplicate?.id).toBe("official-1");
  });

  it("blocks addRegistry when normalized identity already exists", async () => {
    useMarketplaceStore.setState({
      registries: [
        {
          id: "official-1",
          name: "Official Repo",
          source_type: "github",
          url: "https://github.com/anthropics/skills",
          normalized_url: "github:anthropics/skills",
          is_builtin: true,
          is_enabled: true,
          last_synced: null,
          created_at: "2026-04-15T00:00:00Z",
        },
      ],
    });

    await expect(
      useMarketplaceStore
        .getState()
        .addRegistry(
          "Skills",
          "github",
          "https://github.com/Anthropics/skills",
        ),
    ).rejects.toThrow("DUPLICATE_REGISTRY:");

    expect(mockInvoke).not.toHaveBeenCalled();
  });

  it("stores github repo preview results before import", async () => {
    const preview = {
      repo: {
        owner: "anthropics",
        repo: "skills",
        branch: "main",
        normalizedUrl: "https://github.com/anthropics/skills",
      },
      skills: [
        {
          sourcePath: "skills/research/SKILL.md",
          skillId: "research",
          skillName: "research",
          description: "Research helper",
          rootDirectory: "skills",
          skillDirectoryName: "research",
          downloadUrl: "https://example.com/research",
          conflict: null,
        },
      ],
      previewId: "github-preview-1",
      resolvedCommitSha: "1111111111111111111111111111111111111111",
      snapshotDigest: `sha256-v1:${"e".repeat(64)}`,
      expiresAt: new Date(Date.now() + 30 * 60 * 1000).toISOString(),
    };

    mockInvoke.mockResolvedValueOnce(preview);

    await expect(
      useMarketplaceStore
        .getState()
        .previewGitHubRepoImport("https://github.com/anthropics/skills"),
    ).resolves.toEqual(preview);

    expect(mockInvoke).toHaveBeenCalledWith("preview_github_repo_import", {
      repoUrl: "https://github.com/anthropics/skills",
      branch: null,
    });
    expect(useMarketplaceStore.getState().githubImport.preview).toEqual(
      preview,
    );
    expect(useMarketplaceStore.getState().githubImport.preview?.previewId).toBe(
      "github-preview-1",
    );
    expect(useMarketplaceStore.getState().githubImport.previewedRepoUrl).toBe(
      "https://github.com/anthropics/skills",
    );
    expect(useMarketplaceStore.getState().githubImport.isPreviewLoading).toBe(
      false,
    );
  });

  it("normalizes an explicit branch and reuses the preview-associated value on import", async () => {
    const repoUrl = "https://github.com/anthropics/skills";
    const repo = {
      owner: "anthropics",
      repo: "skills",
      branch: "dev",
      normalizedUrl: repoUrl,
    };
    const preview = {
      repo,
      skills: [],
      previewId: "github-preview-dev",
      resolvedCommitSha: "1111111111111111111111111111111111111111",
      snapshotDigest: `sha256-v1:${"e".repeat(64)}`,
      expiresAt: new Date(Date.now() + 30 * 60 * 1000).toISOString(),
    };
    const result = { repo, importedSkills: [], skippedSkills: [] };
    mockInvoke.mockResolvedValueOnce(preview).mockResolvedValueOnce(result);

    await useMarketplaceStore
      .getState()
      .previewGitHubRepoImport(repoUrl, "  dev  ");
    expect(mockInvoke).toHaveBeenNthCalledWith(
      1,
      "preview_github_repo_import",
      { repoUrl, branch: "dev" },
    );
    expect(useMarketplaceStore.getState().githubImport.previewedBranch).toBe(
      "dev",
    );

    const selections = [
      {
        sourcePath: "skills/dev-skill",
        resolution: "overwrite" as const,
        renamedSkillId: null,
      },
    ];
    await useMarketplaceStore
      .getState()
      .importGitHubRepoSkills(repoUrl, selections);
    expect(mockInvoke).toHaveBeenNthCalledWith(
      2,
      "import_github_repo_skills",
      {
        previewId: "github-preview-dev",
        repoUrl,
        branch: "dev",
        selections,
      },
    );
  });

  it("fetches github markdown with repository identity instead of a renderer URL", async () => {
    const repo = {
      owner: "anthropics",
      repo: "skills",
      branch: "main",
      normalizedUrl: "https://github.com/anthropics/skills",
    };
    useMarketplaceStore.setState((state) => ({
      githubImport: {
        ...state.githubImport,
        preview: {
          repo,
          skills: [],
          previewId: "github-preview-markdown",
          resolvedCommitSha: "1111111111111111111111111111111111111111",
          snapshotDigest: `sha256-v1:${"e".repeat(64)}`,
          expiresAt: new Date(Date.now() + 30 * 60 * 1000).toISOString(),
        },
      },
    }));
    mockInvoke.mockResolvedValueOnce("# Research helper");

    await useMarketplaceStore
      .getState()
      .fetchGitHubSkillMarkdown(repo, "skills/research");

    expect(mockInvoke).toHaveBeenCalledWith("fetch_github_skill_markdown", {
      previewId: "github-preview-markdown",
      repo,
      sourcePath: "skills/research",
    });
    expect(mockInvoke.mock.calls[0]?.[1]).not.toHaveProperty("downloadUrl");
    expect(mockInvoke.mock.calls[0]?.[1]).not.toHaveProperty(
      "previewWorkspaceId",
    );
    expect(
      useMarketplaceStore.getState().githubImport.skillMarkdown[
        "skills/research"
      ],
    ).toEqual({ status: "ready", content: "# Research helper" });
  });

  it("reports a friendly desktop-only error when preview is triggered outside Tauri", async () => {
    mockIsTauriRuntime.mockReturnValue(false);

    await expect(
      useMarketplaceStore
        .getState()
        .previewGitHubRepoImport("https://github.com/anthropics/skills"),
    ).rejects.toThrow(
      "Desktop-only feature: GitHub repo preview is available in the Tauri app.",
    );

    expect(mockInvoke).not.toHaveBeenCalled();
    expect(useMarketplaceStore.getState().githubImport.error).toContain(
      "Desktop-only feature",
    );
    expect(useMarketplaceStore.getState().githubImport.previewedRepoUrl).toBe(
      "https://github.com/anthropics/skills",
    );
  });

  it("stores github repo import results", async () => {
    const result = {
      repo: {
        owner: "dorukardahan",
        repo: "twitterapi-io-skill",
        branch: "main",
        normalizedUrl: "https://github.com/dorukardahan/twitterapi-io-skill",
      },
      importedSkills: [
        {
          sourcePath: "twitterapi-io-skill/SKILL.md",
          originalSkillId: "twitterapi-io",
          importedSkillId: "twitterapi-io",
          skillName: "twitterapi-io",
          targetDirectory: "/Users/test/.skillsmanage/skills/twitterapi-io",
          resolution: "overwrite",
        },
      ],
      skippedSkills: [],
    };

    useMarketplaceStore.setState((state) => ({
      githubImport: {
        ...state.githubImport,
        preview: {
          repo: result.repo,
          skills: [],
          previewId: "github-preview-import",
          resolvedCommitSha: "1111111111111111111111111111111111111111",
          snapshotDigest: `sha256-v1:${"e".repeat(64)}`,
          expiresAt: new Date(Date.now() + 30 * 60 * 1000).toISOString(),
        },
      },
    }));
    mockInvoke.mockResolvedValueOnce(result);

    await expect(
      useMarketplaceStore
        .getState()
        .importGitHubRepoSkills(
          "https://github.com/dorukardahan/twitterapi-io-skill",
          [
            {
              sourcePath: "twitterapi-io-skill/SKILL.md",
              resolution: "overwrite",
              renamedSkillId: null,
            },
          ],
        ),
    ).resolves.toEqual(result);

    expect(mockInvoke).toHaveBeenCalledWith("import_github_repo_skills", {
      previewId: "github-preview-import",
      repoUrl: "https://github.com/dorukardahan/twitterapi-io-skill",
      branch: null,
      selections: [
        {
          sourcePath: "twitterapi-io-skill/SKILL.md",
          resolution: "overwrite",
          renamedSkillId: null,
        },
      ],
    });
    expect(useMarketplaceStore.getState().githubImport.importResult).toEqual(
      result,
    );
    expect(useMarketplaceStore.getState().githubImport.isImporting).toBe(false);
    // The backend consumed the snapshot, so the store drops the dead token.
    expect(useMarketplaceStore.getState().githubImport.preview).toBeNull();
  });

  it("rejects an import that has no preview snapshot instead of letting the backend re-resolve the branch", async () => {
    await expect(
      useMarketplaceStore
        .getState()
        .importGitHubRepoSkills(
          "https://github.com/dorukardahan/twitterapi-io-skill",
          [
            {
              sourcePath: "twitterapi-io-skill/SKILL.md",
              resolution: "overwrite",
              renamedSkillId: null,
            },
          ],
        ),
    ).rejects.toThrow();

    expect(
      mockInvoke.mock.calls.some(
        ([command]) => command === "import_github_repo_skills",
      ),
    ).toBe(false);
    expect(useMarketplaceStore.getState().githubImport.isImporting).toBe(false);
  });

  it("keeps the preview snapshot after a failed import so the same token can retry", async () => {
    const repo = {
      owner: "openai",
      repo: "skills",
      branch: "main",
      normalizedUrl: "https://github.com/openai/skills",
    };
    const preview = {
      repo,
      skills: [],
      previewId: "github-preview-retry",
      resolvedCommitSha: "2222222222222222222222222222222222222222",
      snapshotDigest: `sha256-v1:${"f".repeat(64)}`,
      expiresAt: new Date(Date.now() + 30 * 60 * 1000).toISOString(),
    };
    useMarketplaceStore.setState((state) => ({
      githubImport: { ...state.githubImport, preview },
    }));
    mockInvoke.mockRejectedValueOnce(
      "github_import.preview_integrity:GitHub preview snapshot content changed after preview.",
    );

    await expect(
      useMarketplaceStore
        .getState()
        .importGitHubRepoSkills("https://github.com/openai/skills", [
          {
            sourcePath: "skills/openai-docs",
            resolution: "overwrite",
            renamedSkillId: null,
          },
        ]),
    ).rejects.toBeDefined();

    expect(useMarketplaceStore.getState().githubImport.preview).toEqual(
      preview,
    );
    expect(useMarketplaceStore.getState().githubImport.error).toContain(
      "github_import.preview_integrity",
    );
    expect(useMarketplaceStore.getState().githubImport.isImporting).toBe(false);
  });

  it("installs a direct github preview through a fresh backend preview workspace", async () => {
    const repoUrl = "https://github.com/anthropics/skills";
    const sourcePath = "skills/research/SKILL.md";
    const repo = {
      owner: "anthropics",
      repo: "skills",
      branch: "main",
      normalizedUrl: repoUrl,
    };
    const preview = {
      repo,
      skills: [
        {
          sourcePath,
          skillId: "research",
          skillName: "research",
          description: "Research helper",
          rootDirectory: "skills",
          skillDirectoryName: "research",
          downloadUrl: "https://example.com/research",
          conflict: null,
        },
      ],
      previewId: "github-preview-install",
      resolvedCommitSha: "1111111111111111111111111111111111111111",
      snapshotDigest: `sha256-v1:${"e".repeat(64)}`,
      expiresAt: new Date(Date.now() + 30 * 60 * 1000).toISOString(),
    };
    const result = {
      repo,
      importedSkills: [],
      skippedSkills: [],
    };
    mockInvoke.mockResolvedValueOnce(preview).mockResolvedValueOnce(result);

    await expect(
      useMarketplaceStore
        .getState()
        .installGitHubPreviewSkill(repoUrl, sourcePath),
    ).resolves.toEqual(result);

    expect(mockInvoke).toHaveBeenNthCalledWith(
      1,
      "preview_github_repo_import",
      {
        repoUrl,
        branch: null,
      },
    );
    expect(mockInvoke).toHaveBeenNthCalledWith(2, "import_github_repo_skills", {
      previewId: "github-preview-install",
      repoUrl,
      branch: null,
      selections: [
        {
          sourcePath,
          resolution: "overwrite",
          renamedSkillId: null,
        },
      ],
    });
  });

  it("discards the fresh preview workspace when the selected candidate disappeared", async () => {
    const repoUrl = "https://github.com/anthropics/skills";
    const sourcePath = "skills/research/SKILL.md";
    mockInvoke
      .mockResolvedValueOnce({
        repo: {
          owner: "anthropics",
          repo: "skills",
          branch: "main",
          normalizedUrl: repoUrl,
        },
        skills: [],
        previewId: "github-preview-disappeared",
        resolvedCommitSha: "1111111111111111111111111111111111111111",
        snapshotDigest: `sha256-v1:${"e".repeat(64)}`,
        expiresAt: new Date(Date.now() + 30 * 60 * 1000).toISOString(),
      })
      .mockResolvedValueOnce(undefined);

    await expect(
      useMarketplaceStore
        .getState()
        .installGitHubPreviewSkill(repoUrl, sourcePath),
    ).rejects.toThrow(sourcePath);

    expect(mockInvoke).toHaveBeenNthCalledWith(
      1,
      "preview_github_repo_import",
      {
        repoUrl,
        branch: null,
      },
    );
    expect(mockInvoke).toHaveBeenNthCalledWith(
      2,
      "discard_github_repo_preview_snapshot",
      { previewId: "github-preview-disappeared" },
    );
    expect(
      mockInvoke.mock.calls.some(
        ([command]) => command === "import_github_repo_skills",
      ),
    ).toBe(false);
  });

  it("passes the saved preview snapshot id when importing ssh previews", async () => {
    const result = {
      repo: {
        owner: "openai",
        repo: "skills",
        branch: "main",
        normalizedUrl: "https://github.com/openai/skills",
      },
      importedSkills: [],
      skippedSkills: [],
    };
    useMarketplaceStore.setState((state) => ({
      githubImport: {
        ...state.githubImport,
        preview: {
          repo: result.repo,
          skills: [],
          previewId: "github-preview-ssh",
          resolvedCommitSha: "1111111111111111111111111111111111111111",
          snapshotDigest: `sha256-v1:${"e".repeat(64)}`,
          expiresAt: new Date(Date.now() + 30 * 60 * 1000).toISOString(),
        },
      },
    }));
    mockInvoke.mockResolvedValueOnce(result);

    await useMarketplaceStore
      .getState()
      .importGitHubRepoSkills("https://github.com/openai/skills", [
        {
          sourcePath: "skills/openai-docs",
          resolution: "overwrite",
          renamedSkillId: null,
        },
      ]);

    expect(mockInvoke).toHaveBeenCalledWith("import_github_repo_skills", {
      previewId: "github-preview-ssh",
      repoUrl: "https://github.com/openai/skills",
      branch: null,
      selections: [
        {
          sourcePath: "skills/openai-docs",
          resolution: "overwrite",
          renamedSkillId: null,
        },
      ],
    });
  });

  it("discards an old preview snapshot before re-previewing", async () => {
    const preview = {
      repo: {
        owner: "openai",
        repo: "skills",
        branch: "main",
        normalizedUrl: "https://github.com/openai/skills",
      },
      skills: [],
      previewId: "github-preview-new",
      resolvedCommitSha: "1111111111111111111111111111111111111111",
      snapshotDigest: `sha256-v1:${"e".repeat(64)}`,
      expiresAt: new Date(Date.now() + 30 * 60 * 1000).toISOString(),
    };
    useMarketplaceStore.setState((state) => ({
      githubImport: {
        ...state.githubImport,
        preview: {
          repo: preview.repo,
          skills: [],
          previewId: "github-preview-old",
          resolvedCommitSha: "1111111111111111111111111111111111111111",
          snapshotDigest: `sha256-v1:${"e".repeat(64)}`,
          expiresAt: new Date(Date.now() + 30 * 60 * 1000).toISOString(),
        },
      },
    }));
    mockInvoke.mockResolvedValueOnce(undefined).mockResolvedValueOnce(preview);

    await useMarketplaceStore
      .getState()
      .previewGitHubRepoImport("https://github.com/openai/skills");

    expect(mockInvoke).toHaveBeenNthCalledWith(
      1,
      "discard_github_repo_preview_snapshot",
      { previewId: "github-preview-old" },
    );
    expect(mockInvoke).toHaveBeenNthCalledWith(
      2,
      "preview_github_repo_import",
      {
        repoUrl: "https://github.com/openai/skills",
        branch: null,
      },
    );
    expect(useMarketplaceStore.getState().githubImport.preview?.previewId).toBe(
      "github-preview-new",
    );
  });

  it("discards the preview snapshot when resetting import state", async () => {
    useMarketplaceStore.setState((state) => ({
      githubImport: {
        ...state.githubImport,
        preview: {
          repo: {
            owner: "openai",
            repo: "skills",
            branch: "main",
            normalizedUrl: "https://github.com/openai/skills",
          },
          skills: [],
          previewId: "github-preview-close",
          resolvedCommitSha: "1111111111111111111111111111111111111111",
          snapshotDigest: `sha256-v1:${"e".repeat(64)}`,
          expiresAt: new Date(Date.now() + 30 * 60 * 1000).toISOString(),
        },
      },
    }));

    useMarketplaceStore.getState().resetGitHubImport();

    await vi.waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith(
        "discard_github_repo_preview_snapshot",
        { previewId: "github-preview-close" },
      );
    });
    expect(useMarketplaceStore.getState().githubImport.preview).toBeNull();
  });

  it("reports a friendly desktop-only error when import is triggered outside Tauri", async () => {
    mockIsTauriRuntime.mockReturnValue(false);

    await expect(
      useMarketplaceStore
        .getState()
        .importGitHubRepoSkills(
          "https://github.com/dorukardahan/twitterapi-io-skill",
          [
            {
              sourcePath: "twitterapi-io-skill/SKILL.md",
              resolution: "overwrite",
              renamedSkillId: null,
            },
          ],
        ),
    ).rejects.toThrow(
      "Desktop-only feature: GitHub repo import is available in the Tauri app.",
    );

    expect(mockInvoke).not.toHaveBeenCalled();
    expect(useMarketplaceStore.getState().githubImport.error).toContain(
      "Desktop-only feature",
    );
    expect(useMarketplaceStore.getState().githubImport.isImporting).toBe(false);
  });

  it("tracks github import progress events while importing", async () => {
    const result = {
      repo: {
        owner: "dorukardahan",
        repo: "twitterapi-io-skill",
        branch: "main",
        normalizedUrl: "https://github.com/dorukardahan/twitterapi-io-skill",
      },
      importedSkills: [],
      skippedSkills: [],
    };

    type GitHubImportProgressHandler = (event: {
      payload: {
        phase: "writing";
        currentSkill: string;
        currentPath: string;
        completedFiles: number;
        totalFiles: number;
        completedBytes: number;
        totalBytes: number;
      };
    }) => void;

    let progressHandler: GitHubImportProgressHandler | null = null;

    mockListen.mockImplementation(async (eventName, handler) => {
      if (eventName === "github-import:progress") {
        progressHandler = handler as GitHubImportProgressHandler;
      }
      return () => undefined;
    });

    useMarketplaceStore.setState((state) => ({
      githubImport: {
        ...state.githubImport,
        preview: {
          repo: result.repo,
          skills: [],
          previewId: "github-preview-progress",
          resolvedCommitSha: "1111111111111111111111111111111111111111",
          snapshotDigest: `sha256-v1:${"e".repeat(64)}`,
          expiresAt: new Date(Date.now() + 30 * 60 * 1000).toISOString(),
        },
      },
    }));

    type ResolveImport = (value: typeof result) => void;
    let resolveImport: ResolveImport | null = null;
    mockInvoke.mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          resolveImport = resolve as ResolveImport;
        }),
    );

    const importPromise = useMarketplaceStore
      .getState()
      .importGitHubRepoSkills(
        "https://github.com/dorukardahan/twitterapi-io-skill",
        [
          {
            sourcePath: "twitterapi-io-skill/SKILL.md",
            resolution: "overwrite",
            renamedSkillId: null,
          },
        ],
      );

    expect(
      useMarketplaceStore.getState().githubImport.importStartedAt,
    ).not.toBeNull();
    await vi.waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith("import_github_repo_skills", {
        previewId: "github-preview-progress",
        repoUrl: "https://github.com/dorukardahan/twitterapi-io-skill",
        branch: null,
        selections: [
          {
            sourcePath: "twitterapi-io-skill/SKILL.md",
            resolution: "overwrite",
            renamedSkillId: null,
          },
        ],
      });
      expect(resolveImport).not.toBeNull();
    });

    if (!progressHandler) {
      throw new Error(
        "Expected github import progress handler to be registered",
      );
    }
    const progressHandlerFn = progressHandler as GitHubImportProgressHandler;

    progressHandlerFn({
      payload: {
        phase: "writing",
        currentSkill: "twitterapi-io-skill/SKILL.md",
        currentPath: "SKILL.md",
        completedFiles: 1,
        totalFiles: 4,
        completedBytes: 128,
        totalBytes: 512,
      },
    });

    expect(useMarketplaceStore.getState().githubImport.importProgress).toEqual({
      phase: "writing",
      currentSkill: "twitterapi-io-skill/SKILL.md",
      currentPath: "SKILL.md",
      completedFiles: 1,
      totalFiles: 4,
      completedBytes: 128,
      totalBytes: 512,
    });
    expect(
      useMarketplaceStore.getState().githubImport.importStartedAt,
    ).not.toBeNull();

    if (!resolveImport) {
      throw new Error("Expected github import promise resolver to be captured");
    }
    const resolveImportFn = resolveImport as ResolveImport;

    resolveImportFn(result);
    await expect(importPromise).resolves.toEqual(result);
    expect(
      useMarketplaceStore.getState().githubImport.importProgress,
    ).toBeNull();
    expect(
      useMarketplaceStore.getState().githubImport.importStartedAt,
    ).toBeNull();
  });

  it("searches skills.sh and stores results", async () => {
    const results = [
      {
        id: "anthropics/skills/webapp-testing",
        skill_id: "webapp-testing",
        name: "webapp-testing",
        source: "anthropics/skills",
        installs: 68897,
        stars: 1234,
      },
    ];
    mockInvoke.mockResolvedValueOnce(results);

    await expect(
      useMarketplaceStore.getState().searchSkillsSh(" webapp "),
    ).resolves.toEqual(results);

    expect(mockInvoke).toHaveBeenCalledWith("search_skills_sh", {
      query: "webapp",
      limit: 30,
    });
    expect(useMarketplaceStore.getState().skillsShResults).toEqual(results);
    expect(useMarketplaceStore.getState().isSkillsShLoading).toBe(false);
  });

  it("installs a skills.sh skill with a stable source/id loading key", async () => {
    mockInvoke.mockResolvedValueOnce("webapp-testing");

    await expect(
      useMarketplaceStore
        .getState()
        .installFromSkillsSh("anthropics/skills", "webapp-testing"),
    ).resolves.toBe("webapp-testing");

    expect(mockInvoke).toHaveBeenCalledWith("install_from_skills_sh", {
      source: "anthropics/skills",
      skillId: "webapp-testing",
    });
    expect(
      useMarketplaceStore
        .getState()
        .installingIds.has("skills.sh:anthropics/skills:webapp-testing"),
    ).toBe(false);
  });

  it("routes skills.sh detail file commands through Tauri", async () => {
    mockInvoke
      .mockResolvedValueOnce(
        "https://raw.githubusercontent.com/anthropics/skills/main/webapp-testing/SKILL.md",
      )
      .mockResolvedValueOnce([
        { name: "SKILL.md", path: "webapp-testing/SKILL.md", is_dir: false },
      ])
      .mockResolvedValueOnce("# webapp");

    await expect(
      useMarketplaceStore
        .getState()
        .resolveSkillsShUrl("anthropics/skills", "webapp-testing"),
    ).resolves.toContain("raw.githubusercontent.com");
    await expect(
      useMarketplaceStore
        .getState()
        .browseSkillsShDirectory("anthropics/skills", "webapp-testing"),
    ).resolves.toHaveLength(1);
    await expect(
      useMarketplaceStore
        .getState()
        .readSkillsShFile("anthropics/skills", "webapp-testing/SKILL.md"),
    ).resolves.toBe("# webapp");

    expect(mockInvoke).toHaveBeenNthCalledWith(1, "resolve_skills_sh_url", {
      source: "anthropics/skills",
      skillId: "webapp-testing",
    });
    expect(mockInvoke).toHaveBeenNthCalledWith(
      2,
      "browse_skills_sh_directory",
      {
        source: "anthropics/skills",
        skillId: "webapp-testing",
      },
    );
    expect(mockInvoke).toHaveBeenNthCalledWith(3, "read_skills_sh_file", {
      source: "anthropics/skills",
      filePath: "webapp-testing/SKILL.md",
    });
  });
});
