import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
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
      },
    ],
    previewWorkspaceId: "preview-1",
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
        conflict: {
          existingSkillId: "conflicting-skill",
          existingName: "Local conflicting skill",
          existingCanonicalPath: "C:/Users/test/.agents/skills/conflicting-skill",
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
      },
    ],
  };
}

function renderWizard({
  preview = makePreview(),
  onImport = vi.fn(),
}: {
  preview?: GitHubRepoPreview;
  onImport?: (
    selections: GitHubSkillImportSelection[],
  ) => Promise<void> | void;
} = {}) {
  render(
    <MemoryRouter>
      <GitHubRepoImportWizard
        open
        onOpenChange={vi.fn()}
        repoUrl="https://github.com/mattpocock/skills"
        onRepoUrlChange={vi.fn()}
        preview={preview}
        previewError={null}
        isPreviewLoading={false}
        isImporting={false}
        importResult={null}
        onPreview={vi.fn()}
        onImport={onImport}
        onReset={vi.fn()}
        launcherLabel="Central Skills"
      />
    </MemoryRouter>,
  );
}

async function reviewImport() {
  await screen.findByTestId("github-import-preview-workspace");
  fireEvent.click(
    screen.getByRole("button", { name: /检查导入内容|Review import/i }),
  );
  await screen.findByTestId("github-import-confirm-summary");
}

describe("GitHubRepoImportWizard", () => {
  it("keeps the confirm step open after clicking review import", async () => {
    renderWizard();

    await reviewImport();

    expect(screen.getByTestId("github-import-confirm-summary")).toBeInTheDocument();
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
    fireEvent.click(within(resolutionGroup).getByRole("button", { name: "覆盖" }));

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
    const nextResolutionGroup = screen.getByRole("group", { name: /conflicting-skill/ });
    fireEvent.click(within(nextResolutionGroup).getByRole("button", { name: "跳过" }));

    await reviewImport();
    expect(screen.getByTestId("github-import-confirm-summary")).toHaveTextContent(
      "conflicting-skill",
    );
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
});
