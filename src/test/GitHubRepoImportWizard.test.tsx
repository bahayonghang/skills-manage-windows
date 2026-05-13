import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";

import { GitHubRepoImportWizard } from "@/components/marketplace/GitHubRepoImportWizard";
import type { GitHubRepoPreview } from "@/types";

vi.mock("@/lib/tauri", () => ({
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

describe("GitHubRepoImportWizard", () => {
  it("keeps the confirm step open after clicking review import", async () => {
    render(
      <MemoryRouter>
        <GitHubRepoImportWizard
          open
          onOpenChange={vi.fn()}
          repoUrl="https://github.com/mattpocock/skills"
          onRepoUrlChange={vi.fn()}
          preview={makePreview()}
          previewError={null}
          isPreviewLoading={false}
          isImporting={false}
          importResult={null}
          onPreview={vi.fn()}
          onImport={vi.fn()}
          onReset={vi.fn()}
          launcherLabel="Central Skills"
        />
      </MemoryRouter>,
    );

    await screen.findByTestId("github-import-preview-workspace");

    fireEvent.click(
      screen.getByRole("button", { name: /检查导入内容|Review import/i }),
    );

    expect(await screen.findByTestId("github-import-confirm-summary")).toBeInTheDocument();
    await waitFor(() => {
      expect(screen.getByTestId("github-import-shell-footer")).toHaveAttribute(
        "data-footer-mode",
        "confirm",
      );
    });
  });
});
