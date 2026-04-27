import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import type {
  AgentWithStatus,
  GitHubRepoPreview,
  GitHubRepoImportResult,
  MarketplaceSkill,
  SkillRegistry,
} from "@/types";

const mockLoadRegistries = vi.fn();
const mockLoadPreviewSkills = vi.fn<() => Promise<MarketplaceSkill[]>>();
const mockGetNormalizedRegistryIdentity = vi.fn<(url: string) => string | null>();
const mockInstallSkill = vi.fn();
const mockPreviewGitHubRepoImport = vi.fn();
const mockImportGitHubRepoSkills = vi.fn();
const mockResetGitHubImport = vi.fn();
const mockRescan = vi.fn();
const mockLoadCentralSkills = vi.fn();
const mockInstallCentralSkill = vi.fn();
const mockGetSkillsByAgent = vi.fn();

const platformAgents: AgentWithStatus[] = [
  {
    id: "claude-code",
    display_name: "Claude Code",
    category: "coding",
    global_skills_dir: "~/.claude/skills/",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
  {
    id: "central",
    display_name: "Central Skills",
    category: "central",
    global_skills_dir: "~/.skillsmanage/skills/",
    is_detected: true,
    is_builtin: true,
    is_enabled: true,
  },
];

type StoreState = {
  registries: SkillRegistry[];
  installingIds: Set<string>;
  githubImport: {
    isPreviewLoading: boolean;
    isImporting: boolean;
    preview: GitHubRepoPreview | null;
    importResult: GitHubRepoImportResult | null;
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

function normalizeRegistryIdentity(url: string): string | null {
  const trimmed = url.trim();
  if (!trimmed) return null;
  const githubMatch = trimmed.match(
    /^(?:https?:\/\/)?(?:www\.)?github\.com\/([^/\s]+)\/([^/\s#?]+?)(?:\.git)?(?:\/)?$/i,
  );
  if (githubMatch) {
    return `github:${githubMatch[1].toLowerCase()}/${githubMatch[2].toLowerCase()}`;
  }
  return trimmed.toLowerCase();
}

function makeRegistry(id: string, url: string): SkillRegistry {
  return {
    id,
    name: id,
    source_type: "github",
    url,
    normalized_url: normalizeRegistryIdentity(url),
    is_builtin: true,
    is_enabled: true,
    last_synced: "2026-04-16T00:00:00Z",
    last_attempted_sync: "2026-04-16T00:10:00Z",
    last_sync_status: "success",
    last_sync_error: null,
    cache_updated_at: "2026-04-16T00:00:00Z",
    cache_expires_at: "2026-04-17T00:00:00Z",
    etag: null,
    last_modified: null,
    created_at: "2026-04-15T00:00:00Z",
  };
}

function makePreview(
  skills: GitHubRepoPreview["skills"],
  previewWorkspaceId?: string | null,
): GitHubRepoPreview {
  return {
    repo: {
      owner: "openai",
      repo: "skills",
      branch: "main",
      normalizedUrl: "https://github.com/openai/skills",
    },
    skills,
    previewWorkspaceId,
  };
}

vi.mock("@/components/skill/UnifiedSkillCard", () => ({
  UnifiedSkillCard: ({
    name,
    description,
    onDetail,
  }: {
    name: string;
    description?: string;
    onDetail?: () => void;
  }) => (
    <div>
      <button type="button" onClick={onDetail}>
        {name}
      </button>
      {description ? <div>{description}</div> : null}
    </div>
  ),
}));

vi.mock("@/components/central/InstallDialog", () => ({
  InstallDialog: () => null,
}));

vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

vi.mock("@/stores/marketplaceStore", () => ({
  useMarketplaceStore: (selector: (state: Record<string, unknown>) => unknown) =>
    selector({
      registries: storeState.registries,
      installingIds: storeState.installingIds,
      githubImport: storeState.githubImport,
      loadRegistries: mockLoadRegistries,
      loadPreviewSkills: mockLoadPreviewSkills,
      getNormalizedRegistryIdentity: mockGetNormalizedRegistryIdentity,
      installSkill: mockInstallSkill,
      previewGitHubRepoImport: mockPreviewGitHubRepoImport,
      importGitHubRepoSkills: mockImportGitHubRepoSkills,
      resetGitHubImport: mockResetGitHubImport,
    }),
}));

vi.mock("@/stores/platformStore", () => ({
  usePlatformStore: (selector: (state: Record<string, unknown>) => unknown) =>
    selector({
      rescan: mockRescan,
      agents: platformAgents,
    }),
}));

vi.mock("@/stores/centralSkillsStore", () => ({
  useCentralSkillsStore: (selector: (state: Record<string, unknown>) => unknown) =>
    selector({
      skills: [],
      agents: platformAgents,
      loadCentralSkills: mockLoadCentralSkills,
      installSkill: mockInstallCentralSkill,
    }),
}));

vi.mock("@/stores/skillStore", () => ({
  useSkillStore: (selector: (state: Record<string, unknown>) => unknown) =>
    selector({
      skillsByAgent: {},
      getSkillsByAgent: mockGetSkillsByAgent,
    }),
}));

import { MarketplaceView } from "@/pages/MarketplaceView";
import * as tauriBridge from "@/lib/tauri";
import { useTargetStore } from "@/stores/targetStore";

const defaultUpdateSshTargetPassword = useTargetStore.getState().updateSshTargetPassword;

describe("MarketplaceView", () => {
  beforeEach(() => {
    mockLoadRegistries.mockReset();
    mockLoadPreviewSkills.mockReset();
    mockGetNormalizedRegistryIdentity.mockReset();
    mockInstallSkill.mockReset();
    mockPreviewGitHubRepoImport.mockReset();
    mockImportGitHubRepoSkills.mockReset();
    mockResetGitHubImport.mockReset();
    mockRescan.mockReset();
    mockLoadCentralSkills.mockReset();
    mockInstallCentralSkill.mockReset();
    mockGetSkillsByAgent.mockReset();

    mockGetNormalizedRegistryIdentity.mockImplementation(normalizeRegistryIdentity);
    mockLoadPreviewSkills.mockResolvedValue([
      {
        id: "openai::knowledge-work-plugin",
        registry_id: "openai",
        name: "Knowledge Work Plugin",
        description: "Useful repo preview content",
        download_url: "https://example.com/openai/knowledge-work-plugin/SKILL.md",
        is_installed: false,
        synced_at: "2026-04-16T00:00:00Z",
        cache_updated_at: "2026-04-16T00:00:00Z",
      },
    ]);

    storeState.registries = [makeRegistry("openai", "https://github.com/openai/skills")];
    storeState.installingIds = new Set<string>();
    storeState.githubImport = {
      isPreviewLoading: false,
      isImporting: false,
      preview: null,
      importResult: null,
      previewedRepoUrl: null,
      error: null,
    };
    useTargetStore.setState({
      targets: [{ id: "local", kind: "local", label: "Local", isActive: true }],
      activeTarget: { id: "local", kind: "local", label: "Local", isActive: true },
      error: null,
      updateSshTargetPassword: defaultUpdateSshTargetPassword,
    });
  });

  function renderView() {
    return render(
      <MemoryRouter>
        <MarketplaceView />
      </MemoryRouter>,
    );
  }

  it("loads registries on mount", () => {
    renderView();
    expect(mockLoadRegistries).toHaveBeenCalledTimes(1);
  });

  it("shows recommended skills by default and filters them with search", () => {
    renderView();

    expect(screen.getByRole("button", { name: /Recommended|推荐/i })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "web-artifacts-builder" })).toBeInTheDocument();

    fireEvent.change(screen.getByPlaceholderText(/Search skills|搜索技能/i), {
      target: { value: "frontend-design" },
    });

    expect(screen.getByRole("button", { name: "frontend-design" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "web-artifacts-builder" })).not.toBeInTheDocument();
  });

  it("loads official directory preview skills from backend cache", async () => {
    renderView();

    fireEvent.click(screen.getByRole("button", { name: /Official Directory|官方源目录/i }));
    fireEvent.click(screen.getByRole("button", { name: /OpenAI/i }));
    fireEvent.click(screen.getByRole("button", { name: /Browse Skills|浏览 Skills/i }));

    await waitFor(() => {
      expect(mockLoadPreviewSkills).toHaveBeenCalledWith("openai");
    });
    expect(await screen.findByText("Knowledge Work Plugin")).toBeInTheDocument();
    expect(screen.getByText("Useful repo preview content")).toBeInTheDocument();
  });

  it("shows browser fallback copy when official preview runs without Tauri", async () => {
    const isTauriSpy = vi.spyOn(tauriBridge, "isTauriRuntime").mockReturnValue(false);

    renderView();

    fireEvent.click(screen.getByRole("button", { name: /Official Directory|官方源目录/i }));
    fireEvent.click(screen.getByRole("button", { name: /OpenAI/i }));
    fireEvent.click(screen.getByRole("button", { name: /Browse Skills|浏览 Skills/i }));

    expect(
      await screen.findByText(/Preview unavailable in browser mode|浏览器模式下暂不支持预览/i),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/desktop app|桌面应用/i),
    ).toBeInTheDocument();
    expect(mockLoadPreviewSkills).not.toHaveBeenCalled();

    isTauriSpy.mockRestore();
  });

  it("opens the GitHub import wizard from the marketplace CTA", async () => {
    renderView();

    fireEvent.click(screen.getByRole("button", { name: /Import GitHub repo|导入 GitHub 仓库/i }));

    expect(await screen.findByRole("dialog")).toBeInTheDocument();
    expect(screen.getByLabelText(/GitHub repository URL|GitHub 仓库 URL/i)).toBeInTheDocument();
  });

  it("renders the shared github preview workspace when preview data already exists", async () => {
    storeState.githubImport.preview = makePreview([
      {
        sourcePath: "skills/.curated/openai-docs",
        skillId: "openai-docs",
        skillName: "OpenAI Docs",
        description: "OpenAI docs skill description",
        rootDirectory: "skills/.curated",
        skillDirectoryName: "openai-docs",
        downloadUrl: "https://example.com/openai-docs/SKILL.md",
        conflict: null,
      },
    ]);

    renderView();
    fireEvent.click(screen.getByRole("button", { name: /Import GitHub repo|导入 GitHub 仓库/i }));

    expect(await screen.findByTestId("github-import-preview-workspace")).toBeInTheDocument();
    expect(screen.getByTestId("github-import-repo-toolbar")).toBeInTheDocument();
    expect(screen.getByTestId("github-import-summary-list")).toBeInTheDocument();
    expect(screen.getByTestId("github-import-detail-pane")).toBeInTheDocument();
  });

  it("shows the remote workspace hint for SSH previews", async () => {
    useTargetStore.setState({
      targets: [
        { id: "local", kind: "local", label: "Local", isActive: false },
        { id: "ssh-demo", kind: "ssh", label: "dckj", isActive: true },
      ],
      activeTarget: { id: "ssh-demo", kind: "ssh", label: "dckj", isActive: true },
    });
    storeState.githubImport.preview = makePreview(
      [
        {
          sourcePath: "skills/.curated/openai-docs",
          skillId: "openai-docs",
          skillName: "OpenAI Docs",
          description: "OpenAI docs skill description",
          rootDirectory: "skills/.curated",
          skillDirectoryName: "openai-docs",
          downloadUrl: "https://example.com/openai-docs/SKILL.md",
          conflict: null,
        },
      ],
      "github-preview-ssh",
    );

    renderView();
    fireEvent.click(screen.getByRole("button", { name: /Import GitHub repo|导入 GitHub 仓库/i }));

    expect(await screen.findByTestId("github-import-remote-workspace-hint")).toHaveTextContent(
      /Preview workspace is on the active SSH target|预览工作区位于当前 SSH 目标/i,
    );
  });

  it("does not show the remote workspace hint for local previews", async () => {
    storeState.githubImport.preview = makePreview(
      [
        {
          sourcePath: "skills/.curated/openai-docs",
          skillId: "openai-docs",
          skillName: "OpenAI Docs",
          description: "OpenAI docs skill description",
          rootDirectory: "skills/.curated",
          skillDirectoryName: "openai-docs",
          downloadUrl: "https://example.com/openai-docs/SKILL.md",
          conflict: null,
        },
      ],
      "github-preview-local-fixture",
    );

    renderView();
    fireEvent.click(screen.getByRole("button", { name: /Import GitHub repo|导入 GitHub 仓库/i }));

    expect(await screen.findByTestId("github-import-preview-workspace")).toBeInTheDocument();
    expect(screen.queryByTestId("github-import-remote-workspace-hint")).not.toBeInTheDocument();
  });

  it("switches the selected preview skill inside the import wizard", async () => {
    storeState.githubImport.preview = makePreview([
      {
        sourcePath: "skills/.curated/openai-docs",
        skillId: "openai-docs",
        skillName: "OpenAI Docs",
        description: "First skill full description",
        rootDirectory: "skills/.curated",
        skillDirectoryName: "openai-docs",
        downloadUrl: "https://example.com/openai-docs/SKILL.md",
        conflict: null,
      },
      {
        sourcePath: "skills/.system/skill-creator",
        skillId: "skill-creator",
        skillName: "Skill Creator",
        description: "Second skill full description",
        rootDirectory: "skills/.system",
        skillDirectoryName: "skill-creator",
        downloadUrl: "https://example.com/skill-creator/SKILL.md",
        conflict: null,
      },
    ]);

    renderView();
    fireEvent.click(screen.getByRole("button", { name: /Import GitHub repo|导入 GitHub 仓库/i }));

    const detailPane = await screen.findByTestId("github-import-detail-pane");
    expect(within(detailPane).getByText("OpenAI Docs")).toBeInTheDocument();
    expect(within(detailPane).queryByText("Skill Creator")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /Skill Creator/ }));

    await waitFor(() => {
      expect(within(detailPane).getByText("Skill Creator")).toBeInTheDocument();
    });
    expect(within(detailPane).queryByText("OpenAI Docs")).not.toBeInTheDocument();
  });

  it("bulk selects and deselects all preview skills", async () => {
    storeState.githubImport.preview = makePreview([
      {
        sourcePath: "skills/.curated/openai-docs",
        skillId: "openai-docs",
        skillName: "OpenAI Docs",
        description: "OpenAI docs skill description",
        rootDirectory: "skills/.curated",
        skillDirectoryName: "openai-docs",
        downloadUrl: "https://example.com/openai-docs/SKILL.md",
        conflict: null,
      },
      {
        sourcePath: "skills/.system/skill-creator",
        skillId: "skill-creator",
        skillName: "Skill Creator",
        description: "Create skills safely",
        rootDirectory: "skills/.system",
        skillDirectoryName: "skill-creator",
        downloadUrl: "https://example.com/skill-creator/SKILL.md",
        conflict: null,
      },
    ]);

    renderView();
    fireEvent.click(screen.getByRole("button", { name: /GitHub/i }));

    await screen.findByTestId("github-import-preview-workspace");
    const summaryList = screen.getByTestId("github-import-summary-list");
    const getSkillCheckboxes = () =>
      within(summaryList).getAllByRole("checkbox") as HTMLInputElement[];
    const bulkButtons = within(
      screen.getByTestId("github-import-bulk-selection-controls"),
    ).getAllByRole("button") as HTMLButtonElement[];
    const [selectAllButton, deselectAllButton] = bulkButtons;
    const getFooterActionButton = () => {
      const buttons = screen
        .getByTestId("github-import-shell-footer")
        .querySelectorAll("button");
      return buttons[buttons.length - 1] as HTMLButtonElement;
    };

    expect(getSkillCheckboxes()).toHaveLength(2);
    expect(getSkillCheckboxes().every((checkbox) => checkbox.checked)).toBe(true);
    expect(selectAllButton).toBeDisabled();
    expect(deselectAllButton).toBeEnabled();
    expect(getFooterActionButton()).toBeEnabled();

    fireEvent.click(deselectAllButton);

    await waitFor(() => {
      expect(getSkillCheckboxes().every((checkbox) => !checkbox.checked)).toBe(true);
    });
    expect(screen.getByTestId("github-import-repo-toolbar").textContent).toContain("0");
    expect(selectAllButton).toBeEnabled();
    expect(deselectAllButton).toBeDisabled();
    expect(getFooterActionButton()).toBeDisabled();

    fireEvent.click(selectAllButton);

    await waitFor(() => {
      expect(getSkillCheckboxes().every((checkbox) => checkbox.checked)).toBe(true);
    });
    expect(getFooterActionButton()).toBeEnabled();
  });

  it("preserves conflict rename state while bulk toggling preview skills", async () => {
    storeState.githubImport.preview = makePreview([
      {
        sourcePath: "skills/.system/skill-creator",
        skillId: "skill-creator",
        skillName: "Skill Creator",
        description: "Create skills safely",
        rootDirectory: "skills/.system",
        skillDirectoryName: "skill-creator",
        downloadUrl: "https://example.com/skill-creator/SKILL.md",
        conflict: {
          existingSkillId: "skill-creator",
          existingName: "Skill Creator",
          existingCanonicalPath: "/Users/test/.skillsmanage/skills/skill-creator",
          proposedSkillId: "skill-creator",
          proposedName: "Skill Creator",
        },
      },
    ]);

    renderView();
    fireEvent.click(screen.getByRole("button", { name: /GitHub/i }));

    await screen.findByTestId("github-import-preview-workspace");
    fireEvent.click(screen.getByRole("button", { name: /Rename|\u91cd\u547d\u540d/i }));
    fireEvent.change(screen.getByPlaceholderText(/New skill id|\u65b0\u7684\u6280\u80fd ID/i), {
      target: { value: "skill-creator-renamed" },
    });
    fireEvent.click(screen.getByRole("button", { name: /Confirm|\u786e\u8ba4/i }));
    const [selectAllButton, deselectAllButton] = within(
      screen.getByTestId("github-import-bulk-selection-controls"),
    ).getAllByRole("button") as HTMLButtonElement[];
    fireEvent.click(deselectAllButton);
    fireEvent.click(selectAllButton);
    const footerButtons = screen
      .getByTestId("github-import-shell-footer")
      .querySelectorAll("button");
    fireEvent.click(footerButtons[footerButtons.length - 1]);

    const confirmSummary = await screen.findByTestId("github-import-confirm-summary");
    expect(confirmSummary).toHaveTextContent("skill-creator-renamed");
  });

  it("turns conflict resolution into a confirm summary after renaming", async () => {
    storeState.githubImport.preview = makePreview([
      {
        sourcePath: "skills/.system/skill-creator",
        skillId: "skill-creator",
        skillName: "Skill Creator",
        description: "Create skills safely",
        rootDirectory: "skills/.system",
        skillDirectoryName: "skill-creator",
        downloadUrl: "https://example.com/skill-creator/SKILL.md",
        conflict: {
          existingSkillId: "skill-creator",
          existingName: "Skill Creator",
          existingCanonicalPath: "/Users/test/.skillsmanage/skills/skill-creator",
          proposedSkillId: "skill-creator",
          proposedName: "Skill Creator",
        },
      },
    ]);

    renderView();
    fireEvent.click(screen.getByRole("button", { name: /Import GitHub repo|导入 GitHub 仓库/i }));

    await screen.findByTestId("github-import-preview-workspace");
    fireEvent.click(screen.getByRole("button", { name: /Rename|重命名/i }));
    fireEvent.change(screen.getByPlaceholderText(/New skill id|新的技能 ID/i), {
      target: { value: "skill-creator-renamed" },
    });
    fireEvent.click(screen.getByRole("button", { name: /Confirm|确认/i }));
    fireEvent.click(screen.getByRole("button", { name: /Review import|检查导入内容/i }));

    const confirmSummary = await screen.findByTestId("github-import-confirm-summary");
    expect(confirmSummary).toBeInTheDocument();
    expect(confirmSummary).toHaveTextContent("skill-creator-renamed");
    expect(screen.getByTestId("github-import-shell-footer")).toHaveAttribute(
      "data-footer-mode",
      "confirm",
    );
  });

  it("shows inline SSH password repair before remote github import", async () => {
    const storedTarget = {
      id: "ssh-demo",
      kind: "ssh" as const,
      label: "dckj",
      authMethod: "password" as const,
      hasStoredPassword: true,
      credentialStatus: "stored" as const,
      isActive: true,
    };
    const updateSshTargetPassword = vi.fn().mockImplementation(async () => {
      useTargetStore.setState({
        targets: [
          { id: "local", kind: "local", label: "Local", isActive: false },
          storedTarget,
        ],
        activeTarget: storedTarget,
      });
      return {
        ok: true,
        remoteHome: "/home/fixture",
        remoteOs: "Linux",
        credentialStatus: "stored",
        message: "SSH password saved.",
      };
    });
    useTargetStore.setState({
      targets: [
        { id: "local", kind: "local", label: "Local", isActive: false },
        {
          id: "ssh-demo",
          kind: "ssh",
          label: "dckj",
          authMethod: "password",
          hasStoredPassword: false,
          isActive: true,
        },
      ],
      activeTarget: {
        id: "ssh-demo",
        kind: "ssh",
        label: "dckj",
        authMethod: "password",
        hasStoredPassword: false,
        isActive: true,
      },
      updateSshTargetPassword,
    });
    storeState.githubImport.preview = makePreview([
      {
        sourcePath: "skills/.curated/openai-docs",
        skillId: "openai-docs",
        skillName: "OpenAI Docs",
        description: "OpenAI docs skill description",
        rootDirectory: "skills/.curated",
        skillDirectoryName: "openai-docs",
        downloadUrl: "https://example.com/openai-docs/SKILL.md",
        conflict: null,
      },
    ]);

    renderView();
    fireEvent.click(screen.getByRole("button", { name: /Import GitHub repo|导入 GitHub 仓库/i }));
    await screen.findByTestId("github-import-preview-workspace");
    fireEvent.click(screen.getByRole("button", { name: /Review import|检查导入内容/i }));

    const repairPanel = await screen.findByTestId("github-import-ssh-password-repair");
    expect(repairPanel).toHaveTextContent(/Save the active SSH password|保存当前 SSH 密码/i);
    expect(screen.getByRole("button", { name: /^Import$|^导入$/i })).toBeDisabled();

    fireEvent.change(screen.getByLabelText(/SSH password for dckj|dckj 的 SSH 密码/i), {
      target: { value: "secret" },
    });
    const savePasswordButton = screen.getByRole("button", { name: /Save password|保存密码/i });
    await waitFor(() => {
      expect(savePasswordButton).toBeEnabled();
    });
    fireEvent.click(savePasswordButton);
    const footerButtons = screen
      .getByTestId("github-import-shell-footer")
      .querySelectorAll("button");
    const importButton = footerButtons[footerButtons.length - 1] as HTMLButtonElement;

    await waitFor(() => {
      expect(updateSshTargetPassword).toHaveBeenCalledWith("ssh-demo", "secret");
    });
    await waitFor(() => {
      expect(importButton).toBeEnabled();
    });
    await waitFor(() => {
      expect(
        screen.getByText(/SSH password saved for dckj|已保存 dckj 的 SSH 密码/i),
      ).toBeInTheDocument();
    });
  });

  it("allows remote github import with a session-only SSH password", async () => {
    const sessionTarget = {
      id: "ssh-demo",
      kind: "ssh" as const,
      label: "dckj",
      authMethod: "password" as const,
      hasStoredPassword: true,
      credentialStatus: "session" as const,
      isActive: true,
    };
    const updateSshTargetPassword = vi.fn().mockImplementation(async () => {
      useTargetStore.setState({
        targets: [
          { id: "local", kind: "local", label: "Local", isActive: false },
          sessionTarget,
        ],
        activeTarget: sessionTarget,
      });
      return {
        ok: true,
        remoteHome: "/home/fixture",
        remoteOs: "Linux",
        credentialStatus: "session",
        credentialError: "credential vault locked",
        message: "session only",
      };
    });
    useTargetStore.setState({
      targets: [
        { id: "local", kind: "local", label: "Local", isActive: false },
        { ...sessionTarget, hasStoredPassword: false, credentialStatus: "missing" },
      ],
      activeTarget: {
        ...sessionTarget,
        hasStoredPassword: false,
        credentialStatus: "missing",
      },
      updateSshTargetPassword,
    });
    storeState.githubImport.preview = makePreview([
      {
        sourcePath: "skills/.curated/openai-docs",
        skillId: "openai-docs",
        skillName: "OpenAI Docs",
        description: "OpenAI docs skill description",
        rootDirectory: "skills/.curated",
        skillDirectoryName: "openai-docs",
        downloadUrl: "https://example.com/openai-docs/SKILL.md",
        conflict: null,
      },
    ]);

    renderView();
    fireEvent.click(screen.getByRole("button", { name: /Import GitHub repo|导入 GitHub 仓库|å¯¼å…¥ GitHub ä»“åº“/i }));
    await screen.findByTestId("github-import-preview-workspace");
    fireEvent.click(screen.getByRole("button", { name: /Review import|检查导入内容|æ£€æŸ¥å¯¼å…¥å†…å®¹/i }));
    fireEvent.change(screen.getByLabelText(/SSH password for dckj|dckj 的 SSH 密码|dckj çš„ SSH å¯†ç /i), {
      target: { value: "secret" },
    });
    fireEvent.click(screen.getByRole("button", { name: /Save password|保存密码|ä¿å­˜å¯†ç /i }));

    await waitFor(() => {
      expect(screen.getByTestId("github-import-ssh-password-repair").textContent).toMatch(
        /session|本次会话|ä¼šè¯/i,
      );
    });
    const footerButtons = screen
      .getByTestId("github-import-shell-footer")
      .querySelectorAll("button");
    expect(footerButtons[footerButtons.length - 1]).toBeEnabled();
  });

  it("renders the result hub when an import result already exists", async () => {
    storeState.githubImport.importResult = {
      repo: {
        owner: "openai",
        repo: "skills",
        branch: "main",
        normalizedUrl: "https://github.com/openai/skills",
      },
      importedSkills: [
        {
          sourcePath: "skills/.curated/openai-docs",
          originalSkillId: "openai-docs",
          importedSkillId: "openai-docs",
          skillName: "OpenAI Docs",
          targetDirectory: "/Users/test/.skillsmanage/skills/openai-docs",
          resolution: "overwrite",
        },
      ],
      skippedSkills: ["legacy-skill"],
    };

    renderView();
    fireEvent.click(screen.getByRole("button", { name: /Import GitHub repo|导入 GitHub 仓库/i }));

    const resultHub = await screen.findByTestId("github-import-result-hub");
    expect(resultHub).toBeInTheDocument();
    expect(within(resultHub).getByRole("button", { name: /Continue platform setup|继续配置平台安装/i })).toBeInTheDocument();
    expect(within(resultHub).getByRole("button", { name: /Open Central|打开中央技能库/i })).toBeInTheDocument();
    expect(within(resultHub).getByRole("button", { name: /Start another import|开始新的导入/i })).toBeInTheDocument();
    expect(within(resultHub).getByText("legacy-skill")).toBeInTheDocument();
  });

  it("shows settings guidance when github preview fails with auth or rate-limit help", async () => {
    storeState.githubImport.error = "GitHub API rate limit exceeded. Save a Personal Access Token in Settings and retry.";

    renderView();
    fireEvent.click(screen.getByRole("button", { name: /Import GitHub repo|导入 GitHub 仓库/i }));

    expect(
      await screen.findByText(/GitHub Personal Access Token/i),
    ).toBeInTheDocument();
  });
});
