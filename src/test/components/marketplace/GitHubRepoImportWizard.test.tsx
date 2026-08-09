import {
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

import { GitHubRepoImportWizard } from "@/components/marketplace/GitHubRepoImportWizard";
import type { GitHubRepoPreview, GitHubSkillImportSelection } from "@/types";

vi.mock("@/lib/ipc", () => ({
  isTauriRuntime: () => false,
  invoke: vi.fn(),
  listen: vi.fn(),
}));

vi.mock("@/components/central/InstallDialog", () => ({
  InstallDialog: () => null,
}));

vi.mock("@/components/marketplace/githubImportWizardBindings", () => ({
  useGitHubImportWizardBindings: () => ({
    activeTarget: {
      id: "local",
      kind: "local",
      label: "Local",
      isActive: true,
    },
    loadTargets: async () => {},
    updateSshTargetPassword: async () => ({
      ok: true,
      remoteHome: "/home/test",
      remoteOs: "Linux",
      message: "saved",
    }),
    skillMarkdown: {},
    aiSummaries: {},
    fetchGitHubSkillMarkdown: async () => {},
    generateGitHubImportAiSummary: async () => {},
    importProgress: null,
    importStartedAt: null,
  }),
}));

const PREVIEW_FILE_SHA256 = `sha256-v1:${"b".repeat(64)}`;

function makePreview(): GitHubRepoPreview {
  return {
    repo: {
      owner: "mattpocock",
      repo: "skills",
      branch: "main",
      normalizedUrl: "https://github.com/mattpocock/skills",
    },
    skills: [
      {
        sourcePath: "skills/deprecated/design-an-interface/SKILL.md",
        skillId: "design-an-interface",
        skillName: "design-an-interface",
        description: "Generate multiple interface designs.",
        rootDirectory: "skills/deprecated",
        skillDirectoryName: "design-an-interface",
        downloadUrl: "https://example.com/design-an-interface/SKILL.md",
        conflict: null,
        files: [
          { path: "SKILL.md", byteLen: 1024, sha256: PREVIEW_FILE_SHA256 },
          { path: "assets/palette.json", byteLen: 512, sha256: PREVIEW_FILE_SHA256 },
          { path: "references/guide.md", byteLen: 768, sha256: PREVIEW_FILE_SHA256 },
          { path: "references/deep/example.md", byteLen: 256, sha256: PREVIEW_FILE_SHA256 },
        ],
      },
    ],
    previewId: "preview-1",
    resolvedCommitSha: "1234567890abcdef1234567890abcdef12345678",
    snapshotDigest: `sha256-v1:${"a".repeat(64)}`,
    expiresAt: new Date(Date.now() + 30 * 60 * 1000).toISOString(),
  };
}

function makeConflictPreview(): GitHubRepoPreview {
  return {
    ...makePreview(),
    skills: [
      {
        sourcePath: "skills/conflicting/SKILL.md",
        skillId: "conflicting-skill",
        skillName: "conflicting-skill",
        description: "Conflicts with a local skill.",
        rootDirectory: "skills",
        skillDirectoryName: "conflicting-skill",
        downloadUrl: "https://example.com/conflicting-skill/SKILL.md",
        files: [{ path: "SKILL.md", byteLen: 400, sha256: PREVIEW_FILE_SHA256 }],
        conflict: {
          existingSkillId: "conflicting-skill",
          existingName: "Local conflicting skill",
          existingCanonicalPath:
            "C:/Users/test/.agents/skills/conflicting-skill",
          proposedSkillId: "conflicting-skill",
          proposedName: "conflicting-skill",
        },
      },
      {
        sourcePath: "skills/fresh/SKILL.md",
        skillId: "fresh-skill",
        skillName: "fresh-skill",
        description: "A new skill.",
        rootDirectory: "skills",
        skillDirectoryName: "fresh-skill",
        downloadUrl: "https://example.com/fresh-skill/SKILL.md",
        conflict: null,
        files: [{ path: "SKILL.md", byteLen: 300, sha256: PREVIEW_FILE_SHA256 }],
      },
    ],
  };
}

function makeGroupedPreview(): GitHubRepoPreview {
  return {
    ...makePreview(),
    skills: [
      {
        sourcePath: "skills/engineering/ask-matt",
        skillId: "ask-matt",
        skillName: "ask-matt",
        description: "Route to the right engineering skill.",
        pluginName: "mattpocock-skills",
        rootDirectory: "skills/engineering",
        skillDirectoryName: "ask-matt",
        downloadUrl: "https://example.com/ask-matt/SKILL.md",
        conflict: null,
        files: [{ path: "SKILL.md", byteLen: 200, sha256: PREVIEW_FILE_SHA256 }],
      },
      {
        sourcePath: "skills/engineering/code-review",
        skillId: "code-review",
        skillName: "code-review",
        description: "Review a change.",
        pluginName: "mattpocock-skills",
        rootDirectory: "skills/engineering",
        skillDirectoryName: "code-review",
        downloadUrl: "https://example.com/code-review/SKILL.md",
        conflict: null,
        files: [{ path: "SKILL.md", byteLen: 220, sha256: PREVIEW_FILE_SHA256 }],
      },
      {
        sourcePath: "skills/utility/ungrouped",
        skillId: "ungrouped-utility",
        skillName: "Ungrouped Utility",
        description: "A valid skill outside the plugin manifest.",
        rootDirectory: "skills/utility",
        skillDirectoryName: "ungrouped",
        downloadUrl: "https://example.com/ungrouped/SKILL.md",
        conflict: null,
        files: [{ path: "SKILL.md", byteLen: 180, sha256: PREVIEW_FILE_SHA256 }],
      },
    ],
  };
}

function wizardElement({
  preview = makePreview(),
  previewError = null,
  branch = "",
  onBranchChange = vi.fn(),
  onPreview = vi.fn(),
  onImport = vi.fn(),
}: {
  preview?: GitHubRepoPreview | null;
  previewError?: string | null;
  branch?: string;
  onBranchChange?: (value: string) => void;
  onPreview?: (branch: string) =>
    | Promise<GitHubRepoPreview | null>
    | GitHubRepoPreview
    | null;
  onImport?: (selections: GitHubSkillImportSelection[]) => Promise<void> | void;
} = {}) {
  return (
    <MemoryRouter>
      <GitHubRepoImportWizard
        open
        onOpenChange={vi.fn()}
        repoUrl="https://github.com/mattpocock/skills"
        onRepoUrlChange={vi.fn()}
        branch={branch}
        onBranchChange={onBranchChange}
        preview={preview}
        previewError={previewError}
        isPreviewLoading={false}
        isImporting={false}
        importResult={null}
        onPreview={onPreview}
        onImport={onImport}
        onReset={vi.fn()}
        launcherLabel="Central Skills"
      />
    </MemoryRouter>
  );
}

function renderWizard(
  options: Parameters<typeof wizardElement>[0] = {},
) {
  return render(wizardElement(options));
}

async function reviewImport() {
  await screen.findByTestId("github-import-preview-workspace");
  fireEvent.click(
    screen.getByRole("button", { name: /检查导入内容|Review import/i }),
  );
  await screen.findByTestId("github-import-confirm-summary");
}

describe("GitHubRepoImportWizard", () => {
  it("defaults to main and submits main, dev, or a custom branch", async () => {
    const onBranchChange = vi.fn();
    const onPreview = vi.fn().mockResolvedValue(null);
    renderWizard({ preview: null, onBranchChange, onPreview });

    const mainOption = screen.getByRole("radio", { name: "main" });
    expect(mainOption).toHaveAttribute("aria-checked", "true");

    fireEvent.click(
      screen.getByRole("button", { name: /预览导入|Preview import/i }),
    );
    await waitFor(() => expect(onPreview).toHaveBeenLastCalledWith("main"));
    expect(onBranchChange).toHaveBeenCalledWith("main");

    fireEvent.click(screen.getByRole("radio", { name: "dev" }));
    fireEvent.click(
      screen.getByRole("button", { name: /预览导入|Preview import/i }),
    );
    await waitFor(() => expect(onPreview).toHaveBeenLastCalledWith("dev"));
    expect(onBranchChange).toHaveBeenCalledWith("dev");

    fireEvent.click(
      screen.getByRole("radio", { name: /自定义|Custom/i }),
    );
    const customBranchInput = screen.getByRole("textbox", {
      name: /自定义分支|Custom branch/i,
    });
    expect(
      screen.getByRole("button", { name: /预览导入|Preview import/i }),
    ).toBeDisabled();
    fireEvent.change(customBranchInput, {
      target: { value: "feature/branch-picker" },
    });
    fireEvent.click(
      screen.getByRole("button", { name: /预览导入|Preview import/i }),
    );
    await waitFor(() =>
      expect(onPreview).toHaveBeenLastCalledWith("feature/branch-picker"),
    );
    expect(onBranchChange).toHaveBeenCalledWith("feature/branch-picker");
  });

  it("shows the resolved commit short sha and preview expiry without leaking the token", async () => {
    const preview = makePreview();
    renderWizard({ preview });
    await screen.findByTestId("github-import-preview-workspace");

    const provenance = screen.getByTestId("github-import-snapshot-provenance");
    expect(provenance).toHaveTextContent("1234567");
    expect(provenance.textContent).not.toContain(preview.resolvedCommitSha);
    expect(provenance.textContent).not.toContain(preview.previewId);
    expect(provenance.textContent).not.toContain(preview.snapshotDigest);
    expect(
      screen.getByTestId("github-import-repo-toolbar").textContent,
    ).not.toContain(preview.previewId);
  });

  it.each([
    ["github_import.preview_expired:expired", "重新预览"],
    ["github_import.preview_missing:missing", "重新预览"],
    ["github_import.preview_integrity:changed", "重新预览"],
  ])(
    "asks the user to preview again for %s",
    async (previewError, expectedHint) => {
      renderWizard({ preview: null, previewError });

      const hint = await screen.findByTestId("github-import-repreview-hint");
      expect(hint.textContent ?? "").toContain(expectedHint);
      expect(
        screen.queryByText(/Personal Access Token|个人访问令牌/i),
      ).not.toBeInTheDocument();
    },
  );

  it("localizes a conflicting URL and manual branch without exposing backend detail", async () => {
    renderWizard({
      preview: null,
      previewError: "github_import.branch_conflict:private branch detail",
    });

    const message = await screen.findByText(
      /仓库 URL 中的分支与分支输入框不一致|branch in the repository URL differs/i,
    );
    expect(message).toBeInTheDocument();
    expect(message).not.toHaveTextContent("private branch detail");
  });

  it("keeps uncoded preview failures on their historical message", async () => {
    renderWizard({
      preview: null,
      previewError:
        "No importable skills found in this repository. Supported layouts include subpaths.",
    });

    expect(
      await screen.findByText(/No importable skills found/i),
    ).toBeInTheDocument();
    expect(
      screen.queryByTestId("github-import-repreview-hint"),
    ).not.toBeInTheDocument();
  });

  it("shows a virtualized file tree with snapshot totals and expandable deep folders", async () => {
    renderWizard();
    await screen.findByTestId("github-import-preview-workspace");

    fireEvent.click(screen.getByRole("button", { name: /File tree|文件树/i }));

    const tree = screen.getByTestId("github-import-file-tree");
    expect(tree).toHaveTextContent(/4 files|4 个文件/i);
    expect(tree).toHaveTextContent(/3 folders|3 个目录/i);
    expect(tree).toHaveTextContent("design-an-interface");
    expect(within(tree).getByText("references")).toBeInTheDocument();
    expect(within(tree).queryByText("example.md")).not.toBeInTheDocument();

    fireEvent.click(
      within(tree).getByRole("button", { name: /Expand deep|展开 deep/i }),
    );
    expect(within(tree).getByText("example.md")).toBeInTheDocument();
  });

  it("expands and collapses directories from the keyboard", async () => {
    const user = userEvent.setup();
    renderWizard();
    await screen.findByTestId("github-import-preview-workspace");
    await user.click(screen.getByRole("button", { name: /File tree|文件树/i }));

    const deepDirectory = screen.getByRole("button", {
      name: /Expand deep|展开 deep/i,
    });
    expect(deepDirectory).toHaveAttribute("aria-expanded", "false");

    deepDirectory.focus();
    await user.keyboard("{Enter}");
    expect(deepDirectory).toHaveAttribute("aria-expanded", "true");

    await user.keyboard(" ");
    expect(deepDirectory).toHaveAttribute("aria-expanded", "false");
  });

  it("atomically replaces the file tree after re-preview", async () => {
    const initialPreview = makePreview();
    const rendered = renderWizard({ preview: initialPreview });
    await screen.findByTestId("github-import-preview-workspace");
    fireEvent.click(screen.getByRole("button", { name: /File tree|文件树/i }));
    expect(screen.getByTestId("github-import-file-tree")).toHaveTextContent(
      "palette.json",
    );

    const refreshedPreview = makePreview();
    refreshedPreview.skills[0].files = [
      { path: "SKILL.md", byteLen: 1024, sha256: PREVIEW_FILE_SHA256 },
      { path: "scripts/new-command.ts", byteLen: 320, sha256: PREVIEW_FILE_SHA256 },
    ];
    rendered.rerender(wizardElement({ preview: refreshedPreview }));

    const refreshedTree = await screen.findByTestId("github-import-file-tree");
    expect(refreshedTree).toHaveTextContent("new-command.ts");
    expect(refreshedTree).not.toHaveTextContent("palette.json");
  });

  it("keeps the Files tab active when switching skills", async () => {
    renderWizard({ preview: makeConflictPreview() });
    await screen.findByTestId("github-import-preview-workspace");

    const filesTab = screen.getByRole("button", { name: /File tree|文件树/i });
    fireEvent.click(filesTab);
    fireEvent.click(
      within(screen.getByTestId("github-import-summary-list")).getByText(
        "fresh-skill",
      ),
    );

    expect(filesTab).toHaveAttribute("aria-selected", "true");
    expect(screen.getByTestId("github-import-file-tree")).toHaveTextContent(
      "fresh-skill",
    );
  });

  it("blocks review when a selected skill has no trustworthy file manifest", async () => {
    const preview = makePreview();
    preview.skills[0].files = undefined;
    renderWizard({ preview });
    await screen.findByTestId("github-import-preview-workspace");

    expect(
      screen.getByTestId("github-import-file-manifest-blocker"),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: /Review import|检查导入内容/i }),
    ).toBeDisabled();

    fireEvent.click(screen.getByRole("button", { name: /File tree|文件树/i }));
    expect(screen.getByTestId("github-import-file-tree-error")).toBeInTheDocument();
  });

  it("keeps the rendered tree row count bounded for the archive file limit", async () => {
    const preview = makePreview();
    preview.skills[0].files = [
      { path: "SKILL.md", byteLen: 10, sha256: PREVIEW_FILE_SHA256 },
      ...Array.from({ length: 19_999 }, (_, index) => ({
        path: `root-file-${String(index).padStart(5, "0")}.txt`,
        byteLen: 1,
        sha256: PREVIEW_FILE_SHA256,
      })),
    ];
    renderWizard({ preview });
    await screen.findByTestId("github-import-preview-workspace");
    fireEvent.click(screen.getByRole("button", { name: /File tree|文件树/i }));

    expect(screen.getAllByTestId("github-import-file-tree-file").length).toBeLessThan(100);
  });

  it("does not show PAT guidance for non-auth import errors containing subpaths", () => {
    renderWizard({
      preview: null,
      previewError:
        "No importable skills found in this repository. Supported layouts include root SKILL.md, common skill directories such as skills/, .agents/skills/, .claude/skills/, direct repository subpaths, and bounded recursive SKILL.md discovery.",
    });

    expect(
      screen.getByText(/No importable skills found in this repository/i),
    ).toBeInTheDocument();
    expect(
      screen.queryByText(/Open Settings and save a GitHub Personal Access Token|请前往设置并保存 GitHub Personal Access Token/i),
    ).not.toBeInTheDocument();
  });

  it("does not show PAT guidance for GitHub URL validation errors", () => {
    renderWizard({
      preview: null,
      previewError: "Only github.com repository URLs are supported.",
    });

    expect(
      screen.getByText("Only github.com repository URLs are supported."),
    ).toBeInTheDocument();
    expect(
      screen.queryByText(/Open Settings and save a GitHub Personal Access Token|请前往设置并保存 GitHub Personal Access Token/i),
    ).not.toBeInTheDocument();
  });

  it("shows PAT settings guidance for GitHub rate-limit errors", () => {
    renderWizard({
      preview: null,
      previewError: "github_import.rate_limited:GitHub rate limited the request.",
    });

    expect(
      screen.getByText(/Open Settings and save a GitHub Personal Access Token|请前往设置并保存 GitHub Personal Access Token/i),
    ).toBeInTheDocument();
  });

  it("shows configured-token guidance for authenticated access denials", () => {
    renderWizard({
      preview: null,
      previewError:
        "github_import.configured_token_failed:GitHub denied access to the repository.",
    });

    expect(
      screen.getByText(/A configured GitHub token was already used|当前请求已经使用已配置的 GitHub 令牌/i),
    ).toBeInTheDocument();
  });

  it("keeps the confirm step open after clicking review import", async () => {
    renderWizard();

    await reviewImport();

    expect(
      screen.getByTestId("github-import-confirm-summary"),
    ).toBeInTheDocument();
    await waitFor(() => {
      expect(screen.getByTestId("github-import-shell-footer")).toHaveAttribute(
        "data-footer-mode",
        "confirm",
      );
    });
  });

  it("lets conflict skills switch between overwrite and skip from the preview list", async () => {
    const onImport = vi.fn();
    renderWizard({ preview: makeConflictPreview(), onImport });

    await screen.findByTestId("github-import-preview-workspace");

    const resolutionGroup = screen.getByRole("group", {
      name: /conflicting-skill/,
    });
    fireEvent.click(
      within(resolutionGroup).getByRole("button", { name: "覆盖" }),
    );

    await reviewImport();
    fireEvent.click(screen.getByRole("button", { name: /^导入$/ }));

    await waitFor(() => {
      expect(onImport).toHaveBeenCalledWith([
        {
          sourcePath: "skills/conflicting/SKILL.md",
          resolution: "overwrite",
          renamedSkillId: null,
        },
        {
          sourcePath: "skills/fresh/SKILL.md",
          resolution: "overwrite",
          renamedSkillId: null,
        },
      ]);
    });

    onImport.mockClear();
    fireEvent.click(screen.getByRole("button", { name: "返回预览修改" }));
    const nextResolutionGroup = screen.getByRole("group", {
      name: /conflicting-skill/,
    });
    fireEvent.click(
      within(nextResolutionGroup).getByRole("button", { name: "跳过" }),
    );

    await reviewImport();
    expect(
      screen.getByTestId("github-import-confirm-summary"),
    ).toHaveTextContent("conflicting-skill");
    fireEvent.click(screen.getByRole("button", { name: /^导入$/ }));

    await waitFor(() => {
      expect(onImport).toHaveBeenCalledWith([
        {
          sourcePath: "skills/conflicting/SKILL.md",
          resolution: "skip",
          renamedSkillId: null,
        },
        {
          sourcePath: "skills/fresh/SKILL.md",
          resolution: "overwrite",
          renamedSkillId: null,
        },
      ]);
    });
  });

  it("keeps rename available in the detail pane when list controls are present", async () => {
    const onImport = vi.fn();
    renderWizard({ preview: makeConflictPreview(), onImport });

    await screen.findByTestId("github-import-preview-workspace");
    fireEvent.click(screen.getByRole("button", { name: "重命名" }));
    const input = screen.getByPlaceholderText("新的技能 ID");
    fireEvent.change(input, { target: { value: "conflicting-skill-copy" } });
    fireEvent.click(screen.getByRole("button", { name: "确认" }));

    fireEvent.click(screen.getByRole("button", { name: /File tree|文件树/i }));
    expect(screen.getByTestId("github-import-file-tree")).toHaveTextContent(
      "conflicting-skill-copy",
    );

    await reviewImport();
    fireEvent.click(screen.getByRole("button", { name: /^导入$/ }));

    await waitFor(() => {
      expect(onImport).toHaveBeenCalledWith(
        expect.arrayContaining([
          {
            sourcePath: "skills/conflicting/SKILL.md",
            resolution: "rename",
            renamedSkillId: "conflicting-skill-copy",
          },
        ]),
      );
    });
  });

  it("renders plugin grouped preview sections and keeps import payload flat", async () => {
    const onImport = vi.fn();
    renderWizard({ preview: makeGroupedPreview(), onImport });

    await screen.findByTestId("github-import-preview-workspace");
    const summaryList = screen.getByTestId("github-import-summary-list");

    expect(
      within(summaryList).getByText("mattpocock-skills"),
    ).toBeInTheDocument();
    expect(within(summaryList).getByText(/Other|其他/)).toBeInTheDocument();
    expect(within(summaryList).getAllByRole("checkbox")).toHaveLength(3);

    fireEvent.click(
      within(summaryList).getByRole("button", { name: /Ungrouped Utility/ }),
    );
    expect(screen.getByTestId("github-import-detail-pane")).toHaveTextContent(
      "Ungrouped Utility",
    );

    await reviewImport();
    fireEvent.click(screen.getByRole("button", { name: /^导入$/ }));

    await waitFor(() => {
      expect(onImport).toHaveBeenCalled();
    });
    const payload = onImport.mock.calls[0][0];
    expect(payload).toEqual([
      {
        sourcePath: "skills/engineering/ask-matt",
        resolution: "overwrite",
        renamedSkillId: null,
      },
      {
        sourcePath: "skills/engineering/code-review",
        resolution: "overwrite",
        renamedSkillId: null,
      },
      {
        sourcePath: "skills/utility/ungrouped",
        resolution: "overwrite",
        renamedSkillId: null,
      },
    ]);
    expect(payload[0]).not.toHaveProperty("pluginName");
  });

  it("keeps the summary list flat when no preview skill has plugin grouping", async () => {
    renderWizard();

    await screen.findByTestId("github-import-preview-workspace");

    expect(
      screen.queryByTestId("github-import-skill-group"),
    ).not.toBeInTheDocument();
  });
});
